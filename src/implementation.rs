use std::cell::RefCell;
use std::fs;
use std::iter::FromIterator;
use std::ops::Try;
use std::path::PathBuf;
use std::rc::Rc;
use std::thread;

use chrono::prelude::*;
use failure::{self, format_err, Fail};
use qmetaobject::*;
use reqwest;
use serde::Deserialize;

use crate::config::Config;
use crate::listmodel::{MutListItem, MutListModel};

#[derive(QObject, Default)]
pub struct Wallpapers {
    base: qt_base_class!(trait QObject),
    pub error: qt_signal!(err: QString),
    pub list: qt_property!(RefCell<MutListModel<QWallpaper>>; NOTIFY list_changed),
    pub list_changed: qt_signal!(),
    pub fetch_next_page: qt_method!(fn (&self)),
    config: Rc<RefCell<Config>>,
    offset: usize,
}

impl Wallpapers {
    pub fn new() -> Self {
        Self {
            config: Rc::new(RefCell::new(Config::open().unwrap_or_default())),
            ..Default::default()
        }
    }

    pub fn fetch_next_page(&mut self) {
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: Vec<RawImage>| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                for v in v {
                    let mut wallpaper: QWallpaper = (&v).into();
                    wallpaper.config = p.config.clone();
                    mutp.list.borrow_mut().push(wallpaper);
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

#[derive(Default, QObject)]
pub struct QWallpaper {
    base: qt_base_class!(trait QObject),
    pub name: qt_property!(QString),
    pub preview: qt_property!(QString),
    pub copyright: qt_property!(QString),
    pub metas: qt_property!(QVariantList),
    pub wp: qt_property!(bool),
    pub like: qt_property!(bool; NOTIFY like_changed WRITE set_like),
    pub like_changed: qt_signal!(),
    pub image: qt_property!(QString; NOTIFY image_changed READ download),
    pub image_changed: qt_signal!(),
    config: Rc<RefCell<Config>>,
    id: String,
    urlbase: String,
}

impl MutListItem for QWallpaper {
    fn get(&self, idx: i32) -> QVariant {
        match idx {
            0 => QMetaType::to_qvariant(&self.name),
            1 => QMetaType::to_qvariant(&self.preview),
            2 => QMetaType::to_qvariant(&self.copyright),
            3 => QMetaType::to_qvariant(&self.metas),
            4 => QMetaType::to_qvariant(&self.wp),
            5 => QMetaType::to_qvariant(&self.like),
            6 => QMetaType::to_qvariant(&self.image),
            _ => QVariant::default(),
        }
    }
    fn set(&mut self, value: &QVariant, idx: i32) -> bool {
        match idx {
            0 => <_>::from_qvariant(value.clone()).map(|v| self.name = v),
            1 => <_>::from_qvariant(value.clone()).map(|v| self.preview = v),
            2 => <_>::from_qvariant(value.clone()).map(|v| self.copyright = v),
            3 => <_>::from_qvariant(value.clone()).map(|v| self.metas = v),
            4 => <_>::from_qvariant(value.clone()).map(|v| self.wp = v),
            5 => <_>::from_qvariant(value.clone()).map(|v| self.like = v),
            6 => <_>::from_qvariant(value.clone()).map(|v| self.image = v),
            _ => None,
        }
        .is_some()
    }
    fn names() -> Vec<QByteArray> {
        vec![
            QByteArray::from("name"),
            QByteArray::from("preview"),
            QByteArray::from("copyright"),
            QByteArray::from("metas"),
            QByteArray::from("wp"),
            QByteArray::from("like"),
            QByteArray::from("image"),
        ]
    }
}

impl QWallpaper {
    fn set_like(&mut self, val: bool) {
        println!("like! {}", val)
    }

    fn download(&mut self) -> QString {
        println!("start!");
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: String| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                mutp.image = v.into();
                p.image_changed();
            });
        });
        let ptr = QPointer::from(&*self);
        let err_callback = queued_callback(move |e: String| {});
        let id = self.id.clone();
        let urlbase = self.urlbase.clone();
        let resolution = "1920x1080";
        let download_dir = self.config.borrow().download_dir.clone();
        thread::spawn(
            move || match download_image(&id, &urlbase, resolution, &download_dir) {
                Ok(path) => ok_callback(path),
                Err(e) => err_callback(e.to_string()),
            },
        );
        QString::default()
    }
}

fn download_image(
    id: &str,
    urlbase: &str,
    resolution: &str,
    output_dir: &PathBuf,
) -> Result<String, failure::Error> {
    let mut r = reqwest::get(&format!(
        "https://wpdn.bohan.co{}_{}.jpg",
        urlbase, resolution
    ))?;
    if !r.status().is_success() {
        return Err(format_err!("Server Error: {}", r.status()));
    }
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }
    let output = output_dir.with_file_name(format!("{}_{}.jpg", id, resolution));
    if !output.exists() {
        let mut file = fs::File::create(&output)?;
        r.copy_to(&mut file)?;
    }
    Ok(output.to_string_lossy().into())
}

impl From<&RawImage> for QWallpaper {
    fn from(v: &RawImage) -> QWallpaper {
        QWallpaper {
            name: v.name.as_str().into(),
            preview: format!("https://wpdn.bohan.co{}_800x480.jpg", v.urlbase).into(),
            copyright: v.copyright.as_str().into(),
            metas: QVariantList::from_iter(
                v.metas
                    .iter()
                    .map(Into::<QWallpaperInfo>::into)
                    .map(|v| v.to_qvariant()),
            ),
            wp: v.wp,
            id: v.object_id.clone(),
            urlbase: v.urlbase.clone(),
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
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "X-AVOSCloud-Application-Id",
        reqwest::header::HeaderValue::from_static(env!("AVOS_ID")),
    );
    headers.insert(
        "X-AVOSCloud-Application-Key",
        reqwest::header::HeaderValue::from_static(env!("AVOS_KEY")),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("build client");

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

    let resp: Response<RawImage> = client.get(url).send()?.json()?;
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

    let resp: Response<ImageMeta> = client.get(url).send()?.json()?;
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
