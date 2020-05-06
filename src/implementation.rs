use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::iter::FromIterator;
use std::ops::Try;
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use chrono::prelude::*;
use failure::{self, format_err, Fail};
use lazy_static::lazy_static;
use qmetaobject::{future::execute_async, *};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use regex::Regex;
use reqwest;
use serde::Deserialize;
use tokio::fs::File;
use tokio::prelude::*;

use crate::async_utils::enter_tokio;
use crate::config::Config;
use crate::listmodel::{MutListItem, MutListModel};

const MAX_WP_NUM_IN_A_PAGE: usize = 20;
const ORIGINAL_RESOLUTION: &str = "1920x1200";

lazy_static! {
    static ref CURRENT_WP: Mutex<Option<String>> = Mutex::new(None);
    static ref CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "X-AVOSCloud-Application-Id",
            reqwest::header::HeaderValue::from_static(env!("AVOS_ID")),
        );
        headers.insert(
            "X-AVOSCloud-Application-Key",
            reqwest::header::HeaderValue::from_static(env!("AVOS_KEY")),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("build client")
    };
}

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
    pub fetch_next_page: qt_method!(fn(&self)),
    pub next_page_favorites: qt_method!(fn(&self)),
    pub download: qt_method!(fn(&mut self, index: usize, in_favorites_page: bool)),
    pub like: qt_method!(fn(&mut self, index: usize, in_favorites_page: bool)),
    pub set_wallpaper: qt_method!(fn(&self, index: usize, in_favorites_page: bool)),
    pub next_wallpaper: qt_method!(fn(&self)),
    pub diskusage_others: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_favorites: qt_property!(u64; NOTIFY diskusage_changed),
    pub diskusage_changed: qt_signal!(),
    pub clear_other_wallpapers: qt_method!(fn(&mut self)),
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

        let offset = self.offset;
        self.offset += MAX_WP_NUM_IN_A_PAGE;
        let this = QPointer::from(&*self);
        execute_async(enter_tokio(async move {
            let this = this.as_ref().expect("");
            match fetch_wallpapers(&CLIENT, offset, MAX_WP_NUM_IN_A_PAGE).await {
                Ok(images) => {
                    let mutp = unsafe { &mut *(this as *const _ as *mut Self) };
                    for v in images {
                        let mut wallpaper: QWallpaper = (&v).into();
                        wallpaper.like = mutp
                            .config
                            .borrow()
                            .likes
                            .iter()
                            .any(|x| x == &wallpaper.id);
                        mutp.list.borrow_mut().push(wallpaper);
                    }
                    mutp.list_loading = false;
                    mutp.list_loading_changed();
                }
                Err(e) => {
                    this.error(e.to_string().into());
                }
            }
        }));
    }

    pub fn next_page_favorites(&mut self) {
        if self.favorites.borrow().len() == self.config.borrow().likes.len() {
            return;
        }
        self.favorites_loading = true;
        self.favorites_loading_changed();

        let favorites = &self.config.borrow().likes;
        let mut end = self.favorites_offset + MAX_WP_NUM_IN_A_PAGE;
        end = std::cmp::min(end, favorites.len());
        let favorites = favorites[self.favorites_offset..end].to_vec();
        self.favorites_offset = end;

        let this = QPointer::from(&*self);
        execute_async(enter_tokio(async move {
            let this = this.as_ref().expect("");
            match fetch_wallpapers_by_id(&CLIENT, &favorites).await {
                Ok(images) => {
                    let mutp = unsafe { &mut *(this as *const _ as *mut Self) };
                    for img in images {
                        let mut wallpaper: QWallpaper = (&img).into();
                        wallpaper.like = true;
                        mutp.favorites.borrow_mut().push(wallpaper);
                    }
                    mutp.favorites_loading = false;
                    mutp.favorites_loading_changed();
                }
                Err(e) => {
                    this.error(e.to_string().into());
                }
            }
        }));
    }

    pub fn download(&mut self, index: usize, in_favorites_page: bool) {
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
        let try_original = wp.wp && config.resolution.original;

        let idx = (&mut *list as &mut dyn QAbstractListModel).row_index(index as i32);
        (&mut *list as &mut dyn QAbstractListModel).data_changed(idx, idx);

        std::mem::drop(list);
        let this = QPointer::from(&*self);
        execute_async(enter_tokio(async move {
            let resolution = resolution.to_str().unwrap();
            let this = this.as_ref().expect("");
            // Safety: is this safe?
            let this = unsafe { &mut *(this as *const _ as *mut Self) };
            match download_image(&id, &urlbase, resolution, &download_dir, try_original).await {
                Ok(path) => {
                    let mut list = if in_favorites_page {
                        this.favorites.borrow_mut()
                    } else {
                        this.list.borrow_mut()
                    };
                    list[index].image = ("file:".to_owned() + &path).into();
                    list[index].loading = false;
                    let idx = (&mut *list as &mut dyn QAbstractListModel).row_index(index as i32);
                    (&mut *list as &mut dyn QAbstractListModel).data_changed(idx, idx);
                }
                Err(e) => {
                    this.error(e.to_string().into());
                }
            }
        }));
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
        let id = wallpaper.id.clone();
        let urlbase = wallpaper.urlbase.clone();
        let download_dir = config.download_dir.clone();
        let try_original = wallpaper.wp && config.resolution.original;

        let this = QPointer::from(&*self);
        execute_async(enter_tokio(async move {
            let resolution = resolution.to_str().unwrap();
            let this = this.as_ref().expect("");
            let file = download_image(&id, &urlbase, resolution, &download_dir, try_original).await;

            match file {
                Ok(file) => set_wallpaper(&this.config.borrow(), id, &file),
                Err(e) => {
                    this.error(e.to_string().into());
                    return;
                }
            };
        }));
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
        let this = QPointer::from(&*self);
        execute_async(enter_tokio(async move {
            let this = this.as_ref().expect("");
            if let Err(e) = next_wallpaper(&this.config.borrow()).await {
                this.error(e.to_string().into());
            }
        }));
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
                // TODO
                let valid_resolution =
                    (res == ORIGINAL_RESOLUTION && config.resolution.original) || res == resolution;
                if !valid_resolution || (outdated && !favorited) {
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

async fn download_image(
    id: &str,
    urlbase: &str,
    resolution: &str,
    output_dir: &PathBuf,
    try_original: bool,
) -> Result<String, failure::Error> {
    let mut resolutions: &[&str] = &[resolution];
    if try_original && resolution == "1920x1080" {
        resolutions = &["1920x1200", "1920x1080"];
    }
    for resolution in resolutions {
        let output = output_dir.join(format!("{}_{}.jpg", id, resolution));
        if !output.exists() {
            let mut r = reqwest::get(&format!(
                "https://wpdn.bohan.co{}_{}.jpg",
                urlbase, resolution
            ))
            .await?;
            if r.status() == reqwest::StatusCode::NOT_FOUND {
                continue;
            }
            if !r.status().is_success() {
                return Err(format_err!("Server Error: {}", r.status()));
            }
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir)?;
            }
            let mut file = File::create(&output).await?;
            while let Some(chunk) = r.chunk().await? {
                file.write(&chunk).await?;
            }
        }
        return Ok(output.to_string_lossy().into());
    }
    unreachable!()
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

async fn fetch_wallpapers(
    client: &reqwest::Client,
    offset: usize,
    limit: usize,
) -> Result<Vec<RawImage>, failure::Error> {
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

    let resp: Response<RawImage> = client.get(url).send().await?.json().await?;
    let images = resp?;

    fill_wallpapers_metadata(client, images).await
}

async fn fetch_wallpapers_by_id<'a, T: AsRef<str>>(
    client: &reqwest::Client,
    id_list: &[T],
) -> Result<Vec<RawImage>, failure::Error> {
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

    let resp: Response<RawImage> = client.get(url).send().await?.json().await?;
    let mut images = resp?;
    // Keep the order
    let id_index: HashMap<&str, usize> = id_list
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_ref(), index))
        .collect();
    images.sort_by_key(|img| id_index.get(&*img.object_id));

    fill_wallpapers_metadata(client, images).await
}

async fn fill_wallpapers_metadata(
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

    let resp: Response<ImageMeta> = client.get(url).send().await?.json().await?;
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

// TODO: allow systray run this
async fn next_wallpaper(config: &Config) -> Result<(), failure::Error> {
    let resolution = config.resolution.download[config.resolution.download_index].to_qbytearray();
    let resolution = resolution.to_str().unwrap();
    let wallpaper = match config.auto_change.mode {
        // Newest
        0 => fetch_wallpapers(&CLIENT, 0, 1).await?.pop().expect(""),
        // Favorites
        1 => {
            let idx = rand::random::<usize>() % config.likes.len();
            let id = &config.likes[idx];
            fetch_wallpapers_by_id(&CLIENT, &[id])
                .await?
                .pop()
                .expect("")
        }
        // Random
        2 => random_wallpaper(&CLIENT).await?,
        _ => unreachable!(),
    };
    let path = download_image(
        &wallpaper.object_id,
        &wallpaper.urlbase,
        resolution,
        &config.download_dir,
        wallpaper.wp && config.resolution.original,
    )
    .await?;
    set_wallpaper(&config, wallpaper.object_id, &path);
    Ok(())
}

async fn random_wallpaper(client: &reqwest::Client) -> Result<RawImage, failure::Error> {
    static mut WP_COUNT: Option<usize> = None;
    thread_local! {
        static SMALL_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
    }

    let wp_count = unsafe {
        if let Some(count) = WP_COUNT {
            count
        } else {
            let url = reqwest::Url::parse_with_params(
                "https://leanapi.bohan.co/1.1/classes/Image",
                &[("count", "1")],
            )
            .expect("parse url");
            #[derive(Deserialize)]
            struct Resp {
                count: usize,
            }
            let resp: Resp = client.get(url).send().await?.json().await?;
            WP_COUNT = Some(resp.count);
            resp.count
        }
    };

    let n = SMALL_RNG.with(|rng| rng.borrow_mut().gen::<usize>()) % wp_count;

    let url = reqwest::Url::parse_with_params(
        "https://leanapi.bohan.co/1.1/classes/Image",
        &[("limit", "1"), ("skip", &n.to_string())],
    )
    .expect("parse url");

    let resp: Response<RawImage> = client.get(url).send().await?.json().await?;
    let mut images = resp?;

    Ok(images.pop().expect(""))
}

fn set_wallpaper(config: &Config, id: String, file: &str) {
    let de = &config.de.borrow()[config.de_index];
    let cmd = String::from_utf16_lossy(de.cmd.to_slice());
    process::Command::new("sh")
        .env("WALLPAPER", file)
        .arg("-c")
        .arg(&cmd)
        .spawn()
        .expect("");
    *CURRENT_WP.lock().unwrap() = Some(id);
}

fn linear_search_by<T>(s: &[T], f: impl Fn(&T) -> bool) -> Option<usize> {
    for (i, v) in s.iter().enumerate() {
        if f(v) {
            return Some(i);
        }
    }
    None
}
