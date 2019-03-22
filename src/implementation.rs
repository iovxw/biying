use std::cell::RefCell;
use std::iter::FromIterator;
use std::ops::Try;
use std::thread;

use chrono::prelude::*;
use failure::{self, Fail};
use qmetaobject::*;
use reqwest;
use serde::Deserialize;

use crate::listmodel::{MutListItem, MutListModel};

#[derive(QObject, Default)]
pub struct Wallpapers {
    base: qt_base_class!(trait QObject),
    pub error: qt_signal!(err: QString),
    pub list: qt_property!(RefCell<MutListModel<QWallpaper>>; NOTIFY list_changed),
    pub list_changed: qt_signal!(),
    pub fetch_next_page: qt_method!(fn (&self)),
    offset: usize,
}

impl Wallpapers {
    pub fn fetch_next_page(&mut self) {
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: Vec<RawImage>| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Wallpapers) };
                for v in v {
                    mutp.list.borrow_mut().push((&v).into());
                }
                p.list_changed();
            });
        });
        let ptr = QPointer::from(&*self);
        let err_callback = queued_callback(move |e: QString| {
            ptr.as_ref().map(|p| p.error(e));
        });
        let offset = self.offset;
        self.offset += 20;
        thread::spawn(move || match fetch_wallpapers(offset, 20) {
            Ok(r) => ok_callback(r),
            Err(e) => err_callback(e.to_string().into()),
        });
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Response<T> {
    Ok { results: Vec<T> },
    Err { code: i32, error: String },
}

impl<T> Try for Response<T> {
    type Ok = Vec<T>;
    type Error = ServerError;
    fn into_result(self) -> Result<<Response<T> as Try>::Ok, Self::Error> {
        match self {
            Self::Ok { results } => Ok(results),
            Self::Err { code, error } => Err(ServerError { code, error }),
        }
    }
    fn from_error(v: Self::Error) -> Self {
        Self::Err {
            code: v.code,
            error: v.error,
        }
    }
    fn from_ok(v: <Response<T> as Try>::Ok) -> Self {
        Self::Ok { results: v }
    }
}

#[derive(Fail, Debug)]
#[fail(display = "Server Error: {}", error)]
struct ServerError {
    code: i32,
    error: String,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
struct ImageMeta {
    date: String,
    link: String,
    info: String,
    market: String,
    image: ImagePointer,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
struct ImagePointer {
    object_id: String,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
struct RawImage {
    name: String,
    urlbase: String,
    copyright: String,
    object_id: String,
    wp: bool,
    created_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    metas: Vec<ImageMeta>,
}

#[derive(Default)]
pub struct QWallpaper {
    pub name: qt_property!(QString),
    pub preview: qt_property!(QString),
    pub copyright: qt_property!(QString),
    pub id: qt_property!(QString),
    pub metas: qt_property!(QVariantList),
    pub wp: qt_property!(bool),
    pub like: qt_property!(bool; NOTIFY like_changed WRITE set_like),
    pub like_changed: qt_signal!(),
}

impl MutListItem for QWallpaper {
    fn get(&self, idx: i32) -> QVariant {
        match idx {
            0 => QMetaType::to_qvariant(&self.name),
            1 => QMetaType::to_qvariant(&self.preview),
            2 => QMetaType::to_qvariant(&self.copyright),
            3 => QMetaType::to_qvariant(&self.id),
            4 => QMetaType::to_qvariant(&self.metas),
            5 => QMetaType::to_qvariant(&self.wp),
            6 => QMetaType::to_qvariant(&self.like),
            _ => QVariant::default(),
        }
    }
    fn set(&mut self, value: &QVariant, idx: i32) -> bool {
        match idx {
            0 => <_>::from_qvariant(value.clone()).map(|v| self.name = v),
            1 => <_>::from_qvariant(value.clone()).map(|v| self.preview = v),
            2 => <_>::from_qvariant(value.clone()).map(|v| self.copyright = v),
            3 => <_>::from_qvariant(value.clone()).map(|v| self.id = v),
            4 => <_>::from_qvariant(value.clone()).map(|v| self.metas = v),
            5 => <_>::from_qvariant(value.clone()).map(|v| self.wp = v),
            6 => <_>::from_qvariant(value.clone()).map(|v| self.like = v),
            _ => None,
        }
        .is_some()
    }
    fn names() -> Vec<QByteArray> {
        vec![
            QByteArray::from("name"),
            QByteArray::from("preview"),
            QByteArray::from("copyright"),
            QByteArray::from("id"),
            QByteArray::from("metas"),
            QByteArray::from("wp"),
            QByteArray::from("like"),
        ]
    }
}

impl QWallpaper {
    fn set_like(&mut self, val: bool) {
        println!("like! {}", val)
    }
}

impl From<&RawImage> for QWallpaper {
    fn from(v: &RawImage) -> QWallpaper {
        QWallpaper {
            name: v.name.as_str().into(),
            preview: format!("https://wpdn.bohan.co{}_800x480.jpg", v.urlbase).into(),
            copyright: v.copyright.as_str().into(),
            id: v.object_id.as_str().into(),
            metas: QVariantList::from_iter(
                v.metas
                    .iter()
                    .map(Into::<QWallpaperInfo>::into)
                    .map(|v| v.to_qvariant()),
            ),
            wp: v.wp,
            ..QWallpaper::default()
        }
    }
}

#[derive(QGadget, Clone, Default)]
pub struct QWallpaperInfo {
    pub market: qt_property!(QString),
    pub link: qt_property!(QString),
    pub info: qt_property!(QString),
}

impl From<&ImageMeta> for QWallpaperInfo {
    fn from(v: &ImageMeta) -> QWallpaperInfo {
        QWallpaperInfo {
            market: v.market.as_str().into(),
            link: v.link.as_str().into(),
            info: v.info.as_str().into(),
        }
    }
}

fn fetch_wallpapers(offset: usize, limit: usize) -> Result<Vec<RawImage>, failure::Error> {
    let offset = offset.to_string();
    let limit = limit.to_string();
    let url = reqwest::Url::parse_with_params(
        "https://leancloud.cn/1.1/classes/Image",
        &[
            ("order", "-createdAt"),
            ("skip", &offset),
            ("limit", &limit),
        ],
    )
    .expect("parse url");

    let resp: Response<RawImage> = reqwest::Client::new()
        .get(url)
        .header("X-AVOSCloud-Application-Id", env!("AVOS_ID"))
        .header("X-AVOSCloud-Application-Key", env!("AVOS_KEY"))
        .send()?
        .json()?;
    let mut images = resp?;

    let where_query: Vec<String> = images
        .iter()
        .map(|img| {
            format!(
                r#"{{"image":{{"__type":"Pointer","className":"Image","objectId":"{}"}}}}"#,
                img.object_id
            )
        })
        .collect();
    let where_query = format!("{{\"$or\":[{}]}}", where_query.join(","));

    let url = reqwest::Url::parse_with_params(
        "https://leancloud.cn/1.1/classes/Archive",
        &[("where", &where_query)],
    )
    .expect("parse url");

    let resp: Response<ImageMeta> = reqwest::Client::new()
        .get(url)
        .header("X-AVOSCloud-Application-Id", env!("AVOS_ID"))
        .header("X-AVOSCloud-Application-Key", env!("AVOS_KEY"))
        .send()?
        .json()?;
    let metas = resp?;

    for meta in metas {
        for img in &mut images {
            if meta.image.object_id == img.object_id {
                img.metas.push(meta);
                break;
            }
        }
    }

    Ok(images)
}
