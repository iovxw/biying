use std::cell::RefCell;
use std::fs;
use std::iter::FromIterator;
use std::ops::Try;
use std::path::PathBuf;
use std::process;
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
    pub list: qt_property!(RefCell<MutListModel<QWallpaper>>; CONST),
    pub favourites: qt_property!(RefCell<MutListModel<QWallpaper>>; CONST),
    pub list_loading: qt_property!(bool; NOTIFY list_loading_changed),
    pub list_loading_changed: qt_signal!(),
    pub favourites_loading: qt_property!(bool; NOTIFY favourites_loading_changed),
    pub favourites_loading_changed: qt_signal!(),
    pub fetch_next_page: qt_method!(fn (&self)),
    pub next_page_favourites: qt_method!(fn (&self)),
    pub download: qt_method!(fn (&mut self, index: usize, in_favourites: bool)),
    pub like: qt_method!(fn (&mut self, index: usize, in_favourites: bool)),
    pub set_wallpaper: qt_method!(fn (&self, index: usize, in_favourites: bool)),
    pub diskusage_others: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_favourites: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_changed: qt_signal!(),
    pub config: qt_property!(RefCell<Config>; CONST),
    offset: usize,
    favourites_offset: usize,
}

impl Wallpapers {
    pub fn new() -> Self {
        let mut s = Self {
            config: RefCell::new(Config::open().unwrap_or_default()),
            ..Default::default()
        };
        s.update_diskusage().unwrap_or_default();
        s
    }

    pub fn fetch_next_page(&mut self) {
        self.list_loading = true;
        self.list_loading_changed();
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: Vec<RawImage>| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                for v in v {
                    let mut wallpaper: QWallpaper = (&v).into();
                    wallpaper.like = p.config.borrow().likes.contains(&wallpaper.id);
                    mutp.list.borrow_mut().push(wallpaper);
                }
                mutp.list_loading = false;
                mutp.list_loading_changed();
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

    pub fn next_page_favourites(&mut self) {
        if self.favourites.borrow().len() == self.config.borrow().likes.len() {
            return;
        }
        self.favourites_loading = true;
        self.favourites_loading_changed();
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: Vec<RawImage>| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                for v in v {
                    let mut wallpaper: QWallpaper = (&v).into();
                    wallpaper.like = p.config.borrow().likes.contains(&wallpaper.id);
                    mutp.favourites.borrow_mut().push(wallpaper);
                }
                mutp.favourites_loading = false;
                mutp.favourites_loading_changed();
            });
        });
        let ptr = QPointer::from(&*self);
        let err_callback = queued_callback(move |e: QString| {
            ptr.as_ref().map(|p| p.error(e));
        });
        // TODO: offset
        let favourites: Vec<_> = self.config.borrow().likes.clone().into_iter().collect();
        thread::spawn(move || match fetch_wallpapers_by_id(&favourites) {
            Ok(r) => ok_callback(r),
            Err(e) => err_callback(e.to_string().into()),
        });
    }

    pub fn download(&mut self, index: usize, in_favourites: bool) {
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: String| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                mutp.update_diskusage().unwrap_or_default();
                let mut list = if in_favourites {
                    mutp.favourites.borrow_mut()
                } else {
                    mutp.list.borrow_mut()
                };
                list[index].image = ("file:".to_owned() + &v).into();
                list[index].loading = false;
                let idx = (&mut *list as &mut QAbstractListModel).row_index(index as i32);
                (&mut *list as &mut QAbstractListModel).data_changed(idx, idx);
            });
        });
        let err_callback = queued_callback(move |e: String| eprintln!("{}", e));

        let mut list = if in_favourites {
            self.favourites.borrow_mut()
        } else {
            self.list.borrow_mut()
        };
        let wp = &mut list[index];
        wp.loading = true;
        let id = wp.id.clone();
        let urlbase = wp.urlbase.clone();
        let config = self.config.borrow();
        let resolution =
            config.resolution.download[config.resolution.download_index].to_qbytearray();
        let download_dir = config.download_dir.clone();

        let idx = (&mut *list as &mut QAbstractListModel).row_index(index as i32);
        (&mut *list as &mut QAbstractListModel).data_changed(idx, idx);

        thread::spawn(move || {
            let resolution = resolution.to_str().unwrap();
            match download_image(&id, &urlbase, resolution, &download_dir) {
                Ok(path) => ok_callback(path),
                Err(e) => err_callback(e.to_string()),
            }
        });
    }

    pub fn set_wallpaper(&self, index: usize, in_favourites: bool) {
        let config = self.config.borrow();
        let wallpaper = &if in_favourites {
            self.favourites.borrow()
        } else {
            self.list.borrow()
        }[index];

        let resolution =
            config.resolution.download[config.resolution.download_index].to_qbytearray();
        let resolution = resolution.to_str().unwrap();

        let file = download_image(
            &wallpaper.id,
            &wallpaper.urlbase,
            resolution,
            &config.download_dir,
        );

        let file = match file {
            Ok(v) => v,
            Err(e) => {
                self.error(e.to_string().into());
                return;
            }
        };

        let de = &config.de.borrow()[config.de_index];
        let cmd = String::from_utf16_lossy(de.cmd.to_slice());
        process::Command::new("sh")
            .env("WALLPAPER", &file)
            .arg("-c")
            .arg(&cmd)
            .spawn()
            .expect("");
    }

    pub fn like(&mut self, mut index: usize, in_favourites: bool) {
        if in_favourites {
            let id = self.favourites.borrow()[index].id.clone();
            self.favourites.borrow_mut().remove(index);
            let r = linear_search_by(&self.list.borrow(), |v| v.id == id);
            match r {
                Some(i) => index = i,
                None => {
                    self.update_diskusage().unwrap_or_default();
                    return;
                }
            }
        }
        {
            let wallpapers = &mut *self.list.borrow_mut();
            wallpapers[index].like = !wallpapers[index].like;
            if wallpapers[index].like {
                self.config
                    .borrow_mut()
                    .likes
                    .insert(wallpapers[index].id.clone());
                self.favourites.borrow_mut().push(wallpapers[index].clone());
            } else {
                let id = &wallpapers[index].id;
                self.config.borrow_mut().likes.remove(id);
                let i = linear_search_by(&self.favourites.borrow(), |v| &*v.id == id);
                if let Some(i) = i {
                    self.favourites.borrow_mut().remove(i);
                }
            }
            self.config.borrow().save().expect("Failed to save config!");
            let idx = (wallpapers as &mut QAbstractListModel).row_index(index as i32);
            (wallpapers as &mut QAbstractListModel).data_changed(idx, idx);
        }
        self.update_diskusage().unwrap_or_default();
    }

    pub fn update_diskusage(&mut self) -> Result<(), failure::Error> {
        let config = self.config.borrow();
        let download_dir = fs::read_dir(&config.download_dir)?;
        let mut favourites = 0;
        let mut others = 0;
        for entry in download_dir {
            let entry = entry?;
            let metadata = entry.metadata().expect("Wallpaper file metadata");
            if metadata.is_dir() {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .expect("file name to String");
            let id = name.split('_').next().expect("Illegal file name");
            let file_size = metadata.len();
            if config.likes.contains(id) {
                favourites += file_size;
            } else {
                others += file_size;
            }
        }
        self.diskusage_favourites = favourites;
        self.diskusage_others = others;
        self.diskusage_changed();
        Ok(())
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

#[derive(Default, Clone)]
pub struct QWallpaper {
    pub name: qt_property!(QString),
    pub preview: qt_property!(QString),
    pub copyright: qt_property!(QString),
    pub metas: qt_property!(QVariantList),
    pub wp: qt_property!(bool),
    pub like: qt_property!(bool),
    pub image: qt_property!(QString),
    pub loading: qt_property!(bool),
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
            7 => QMetaType::to_qvariant(&self.loading),
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
            7 => <_>::from_qvariant(value.clone()).map(|v| self.loading = v),
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
            QByteArray::from("loading"),
        ]
    }
}

fn download_image(
    id: &str,
    urlbase: &str,
    resolution: &str,
    output_dir: &PathBuf,
) -> Result<String, failure::Error> {
    let output = output_dir.join(format!("{}_{}.jpg", id, resolution));
    if !output.exists() {
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
    pub info: qt_property!(QString),
}

impl From<&ImageMeta> for QWallpaperInfo {
    fn from(v: &ImageMeta) -> QWallpaperInfo {
        QWallpaperInfo {
            market: v.market.as_str().into(),
            info: v.info.as_str().into(),
        }
    }
}

fn fetch_wallpapers(offset: usize, limit: usize) -> Result<Vec<RawImage>, failure::Error> {
    let client = build_client();

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
    let images = resp?;

    fill_wallpapers_metadata(&client, images)
}

fn fetch_wallpapers_by_id(id_list: &[String]) -> Result<Vec<RawImage>, failure::Error> {
    let client = build_client();

    let where_query: Vec<String> = id_list
        .iter()
        .map(|img| format!(r#"{{"objectId":"{}"}}"#, img))
        .collect();
    let where_query = format!("{{\"$or\":[{}]}}", where_query.join(","));

    let url = reqwest::Url::parse_with_params(
        "https://leancloud.cn/1.1/classes/Image",
        &[("where", &where_query)],
    )
    .expect("parse url");

    let resp: Response<RawImage> = client.get(url).send()?.json()?;
    let images = resp?;

    fill_wallpapers_metadata(&client, images)
}

fn fill_wallpapers_metadata(
    client: &reqwest::Client,
    mut images: Vec<RawImage>,
) -> Result<Vec<RawImage>, failure::Error> {
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

fn build_client() -> reqwest::Client {
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
    client
}

fn linear_search_by<T>(s: &[T], f: impl Fn(&T) -> bool) -> Option<usize> {
    for (i, v) in s.iter().enumerate() {
        if f(v) {
            return Some(i);
        }
    }
    None
}
