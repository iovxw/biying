use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::iter::FromIterator;
use std::ops::Try;
use std::path::PathBuf;
use std::process;
use std::thread;
use std::time::{Duration, SystemTime};

use chrono::prelude::*;
use failure::{self, format_err, Fail};
use lazy_static::lazy_static;
use qmetaobject::*;
use regex::Regex;
use reqwest;
use serde::Deserialize;

use crate::config::Config;
use crate::listmodel::{MutListItem, MutListModel};

const MAX_WP_NUM_IN_A_PAGE: usize = 20;

#[derive(QObject, Default)]
pub struct Wallpapers {
    base: qt_base_class!(trait QObject),
    pub error: qt_signal!(err: QString),
    pub list: qt_property!(RefCell<MutListModel<QWallpaper>>; CONST),
    pub favorites: qt_property!(RefCell<MutListModel<QWallpaper>>; CONST),
    pub list_loading: qt_property!(bool; NOTIFY list_loading_changed),
    pub list_loading_changed: qt_signal!(),
    pub favorites_loading: qt_property!(bool; NOTIFY favorites_loading_changed),
    pub favorites_loading_changed: qt_signal!(),
    pub fetch_next_page: qt_method!(fn (&self)),
    pub next_page_favorites: qt_method!(fn (&self)),
    pub download: qt_method!(fn (&mut self, index: usize, in_favorites_page: bool)),
    pub like: qt_method!(fn (&mut self, index: usize, in_favorites_page: bool)),
    pub set_wallpaper: qt_method!(fn (&self, index: usize, in_favorites_page: bool)),
    pub next_wallpaper: qt_method!(fn (&self)),
    pub diskusage_others: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_favorites: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_changed: qt_signal!(),
    pub clear_other_wallpapers: qt_method!(fn (&mut self)),
    pub config: qt_property!(RefCell<Config>; CONST),
    offset: usize,
    favorites_offset: usize,
}

impl Wallpapers {
    pub fn new() -> Self {
        let mut s = Self {
            config: RefCell::new(Config::open().unwrap_or_default()),
            ..Default::default()
        };
        s.update_diskusage_and_autoclean().unwrap_or_default();
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
                    wallpaper.like = p.config.borrow().likes.iter().any(|x| x == &wallpaper.id);
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
        self.offset += MAX_WP_NUM_IN_A_PAGE;
        thread::spawn(
            move || match fetch_wallpapers(offset, MAX_WP_NUM_IN_A_PAGE) {
                Ok(r) => ok_callback(r),
                Err(e) => err_callback(e.to_string().into()),
            },
        );
    }

    pub fn next_page_favorites(&mut self) {
        if self.favorites.borrow().len() == self.config.borrow().likes.len() {
            return;
        }
        self.favorites_loading = true;
        self.favorites_loading_changed();
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |images: Vec<RawImage>| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                for img in images {
                    let mut wallpaper: QWallpaper = (&img).into();
                    wallpaper.like = true;
                    mutp.favorites.borrow_mut().push(wallpaper);
                }
                mutp.favorites_loading = false;
                mutp.favorites_loading_changed();
            });
        });
        let ptr = QPointer::from(&*self);
        let err_callback = queued_callback(move |e: QString| {
            ptr.as_ref().map(|p| p.error(e));
        });

        let favorites = &self.config.borrow().likes;
        let mut end = self.favorites_offset + MAX_WP_NUM_IN_A_PAGE;
        end = std::cmp::min(end, favorites.len());
        let favorites = favorites[self.favorites_offset..end].to_vec();
        self.favorites_offset = end;
        thread::spawn(move || match fetch_wallpapers_by_id(&favorites) {
            Ok(r) => ok_callback(r),
            Err(e) => err_callback(e.to_string().into()),
        });
    }

    pub fn download(&mut self, index: usize, in_favorites_page: bool) {
        let ptr = QPointer::from(&*self);
        let ok_callback = queued_callback(move |v: String| {
            ptr.as_ref().map(|p| {
                let mutp = unsafe { &mut *(p as *const _ as *mut Self) };
                mutp.update_diskusage_and_autoclean().unwrap_or_default();
                let mut list = if in_favorites_page {
                    mutp.favorites.borrow_mut()
                } else {
                    mutp.list.borrow_mut()
                };
                list[index].image = ("file:".to_owned() + &v).into();
                list[index].loading = false;
                let idx = (&mut *list as &mut dyn QAbstractListModel).row_index(index as i32);
                (&mut *list as &mut dyn QAbstractListModel).data_changed(idx, idx);
            });
        });
        let err_callback = queued_callback(move |e: String| eprintln!("{}", e));

        let mut list = if in_favorites_page {
            self.favorites.borrow_mut()
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

        let idx = (&mut *list as &mut dyn QAbstractListModel).row_index(index as i32);
        (&mut *list as &mut dyn QAbstractListModel).data_changed(idx, idx);

        thread::spawn(move || {
            let resolution = resolution.to_str().unwrap();
            match download_image(&id, &urlbase, resolution, &download_dir) {
                Ok(path) => ok_callback(path),
                Err(e) => err_callback(e.to_string()),
            }
        });
    }

    pub fn set_wallpaper(&self, index: usize, in_favorites_page: bool) {
        let wallpaper = &if in_favorites_page {
            self.favorites.borrow()
        } else {
            self.list.borrow()
        }[index];

        let config = self.config.borrow();
        let resolution =
            config.resolution.download[config.resolution.download_index].to_qbytearray();
        let resolution = resolution.to_str().unwrap();

        let file = download_image(
            &wallpaper.id,
            &wallpaper.urlbase,
            resolution,
            &self.config.borrow().download_dir,
        );

        let file = match file {
            Ok(v) => v,
            Err(e) => {
                self.error(e.to_string().into());
                return;
            }
        };

        self.set_wallpaper_cmd(&file);
    }

    pub fn like(&mut self, index: usize, in_favorites_page: bool) {
        // NOTE: `self.favorites` is favorites in favorites page, not all favorites
        // `self.config.likes` is the full list

        let id: String;
        let main_index: Option<usize>;
        let favorites_index: Option<usize>;
        let favorited: bool;

        if in_favorites_page {
            id = self.favorites.borrow()[index].id.clone();
            main_index = linear_search_by(&self.list.borrow(), |v| v.id == id);
            favorites_index = Some(index);
            favorited = true; // In favorites page, all wallpapres are favorited
        } else {
            id = self.list.borrow()[index].id.clone();
            favorites_index = linear_search_by(&self.favorites.borrow(), |v| &*v.id == id);
            main_index = Some(index);
            favorited = self.list.borrow()[index].like;
        }

        if let Some(index) = favorites_index {
            self.favorites.borrow_mut().remove(index);
            self.favorites_offset -= 1;
        }

        if let Some(index) = main_index {
            let wallpapers = &mut *self.list.borrow_mut();
            wallpapers[index].like = !favorited;

            if !favorited {
                self.favorites
                    .borrow_mut()
                    .insert(0, wallpapers[index].clone());
                self.favorites_offset += 1;
            }

            let idx = (wallpapers as &mut dyn QAbstractListModel).row_index(index as i32);
            (wallpapers as &mut dyn QAbstractListModel).data_changed(idx, idx);
        }

        if !favorited {
            self.config.borrow_mut().likes.insert(0, id);
        } else {
            self.config.borrow_mut().likes.remove_item(&id);
        }
        self.config.borrow().save().expect("Failed to save config!");

        self.update_diskusage_and_autoclean().unwrap_or_default();
    }

    pub fn clear_other_wallpapers(&mut self) {
        let config = self.config.borrow();
        let r: Result<(), failure::Error> = try {
            let download_dir = fs::read_dir(&config.download_dir)?;

            for entry in download_dir {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(path)?;
                    continue;
                }
                let name = entry
                    .file_name()
                    .into_string()
                    .expect("file name to String");
                let id = name.split('_').next().expect("Illegal file name");
                if !config.likes.iter().any(|x| x == id) {
                    fs::remove_file(path)?;
                }
            }

            self.diskusage_others = 0;
            self.diskusage_changed();
        };
        if let Err(e) = r {
            self.error(e.to_string().into());
        }
    }

    pub fn next_wallpaper(&self) {
        let r: Result<(), failure::Error> = try {
            let config = self.config.borrow();
            let resolution =
                config.resolution.download[config.resolution.download_index].to_qbytearray();
            let resolution = resolution.to_str().unwrap();
            match config.auto_change.mode {
                // Newest
                0 => {
                    let wallpaper = if let Some(r) = fetch_wallpapers(0, 1)?.pop() {
                        r
                    } else {
                        return;
                    };

                    let path = download_image(
                        &wallpaper.object_id,
                        &wallpaper.urlbase,
                        resolution,
                        &config.download_dir,
                    )?;

                    self.set_wallpaper_cmd(&path);
                }
                // Favorites
                1 => {
                    let idx = rand::random::<usize>() % config.likes.len();
                    let id = &config.likes[idx];
                    let wallpaper = if let Some(x) = fetch_wallpapers_by_id(&[id])?.pop() {
                        x
                    } else {
                        return;
                    };
                    let path = download_image(
                        &wallpaper.object_id,
                        &wallpaper.urlbase,
                        resolution,
                        &config.download_dir,
                    )?;
                    self.set_wallpaper_cmd(&path);
                }
                // Random
                2 => {
                    let mut downloaded = fs::read_dir(&config.download_dir)?
                        .filter_map(Result::ok)
                        .map(|entry| (entry.path(), entry.file_name()))
                        .filter(|(_path, name)| {
                            let name = name.to_string_lossy();
                            parse_wallpaper_filename(&name)
                                .map(|(_, res)| res == resolution)
                                .unwrap_or(false)
                        })
                        .map(|(path, _name)| path)
                        .collect::<Vec<_>>();
                    if downloaded.is_empty() {
                        return;
                    }

                    let idx = rand::random::<usize>() % downloaded.len();

                    let file = downloaded.remove(idx);

                    self.set_wallpaper_cmd(&file.to_string_lossy());
                }
                _ => unreachable!(),
            }
        };
        if let Err(e) = r {
            self.error(e.to_string().into());
        }
    }

    fn set_wallpaper_cmd(&self, file: &str) {
        let config = self.config.borrow();
        let de = &config.de.borrow()[config.de_index];
        let cmd = String::from_utf16_lossy(de.cmd.to_slice());
        process::Command::new("sh")
            .env("WALLPAPER", file)
            .arg("-c")
            .arg(&cmd)
            .spawn()
            .expect("");
    }

    fn update_diskusage_and_autoclean(&mut self) -> Result<(), failure::Error> {
        let config = self.config.borrow();
        let download_dir = fs::read_dir(&config.download_dir)?;
        let mut favorites = 0;
        let mut others = 0;
        for entry in download_dir {
            let entry = entry?;
            let metadata = entry.metadata().expect("Wallpaper file metadata");
            if metadata.is_dir() {
                // remove all other files
                fs::remove_dir_all(entry.path())?;
                continue;
            }

            let created = metadata.modified().expect("read metadata created");
            let outdated = SystemTime::now().duration_since(created)?
                > Duration::from_secs(config.autoremove * 24 * 60 * 60);

            let name = entry
                .file_name()
                .into_string()
                .expect("file name to String");
            if let Some((id, res)) = parse_wallpaper_filename(&name) {
                let favorited = config.likes.iter().any(|x| x == id);
                let resolution =
                    config.resolution.download[config.resolution.download_index].to_qbytearray();
                let resolution = resolution.to_str().unwrap();
                let file_size = metadata.len();
                if res != resolution || (outdated && !favorited) {
                    fs::remove_file(entry.path())?;
                } else if favorited {
                    favorites += file_size;
                } else {
                    others += file_size;
                }
            } else {
                // remove all other files
                fs::remove_file(entry.path())?;
            };
        }
        self.diskusage_favorites = favorites;
        self.diskusage_others = others;
        self.diskusage_changed();
        Ok(())
    }
}

fn parse_wallpaper_filename(file: &str) -> Option<(&str, &str)> {
    lazy_static! {
        static ref WALLPAPER_FILE_NAME: Regex =
            Regex::new(r#"([[:alnum:]]+)_(\d+x\d+)\.\w+"#).unwrap();
    }
    let re = WALLPAPER_FILE_NAME.captures(file)?;
    let id = re.get(1)?;
    let res = re.get(2)?;
    Some((id.as_str(), res.as_str()))
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
            Response::Ok { results } => Ok(results),
            Response::Err { code, error } => Err(ServerError { code, error }),
        }
    }
    fn from_error(v: Self::Error) -> Self {
        Response::Err {
            code: v.code,
            error: v.error,
        }
    }
    fn from_ok(v: <Response<T> as Try>::Ok) -> Self {
        Response::Ok { results: v }
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
        "https://leanapi.bohan.co/1.1/classes/Image",
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

fn fetch_wallpapers_by_id<'a, T: AsRef<str>>(
    id_list: &[T],
) -> Result<Vec<RawImage>, failure::Error> {
    let client = build_client();

    let where_query: Vec<String> = id_list
        .iter()
        .map(|img| format!(r#"{{"objectId":"{}"}}"#, img.as_ref()))
        .collect();
    let where_query = format!("{{\"$or\":[{}]}}", where_query.join(","));

    let url = reqwest::Url::parse_with_params(
        "https://leanapi.bohan.co/1.1/classes/Image",
        &[("where", &where_query)],
    )
    .expect("parse url");

    let resp: Response<RawImage> = client.get(url).send()?.json()?;
    let mut images = resp?;
    // Keep the order
    let id_index: HashMap<&str, usize> = id_list
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_ref(), index))
        .collect();
    images.sort_by_key(|img| id_index.get(&*img.object_id));

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
        "https://leanapi.bohan.co/1.1/classes/Archive",
        // Default limit is 100, maximum is 1000
        &[("where", &*where_query), ("limit", "1000")],
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
