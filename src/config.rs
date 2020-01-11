use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::path::PathBuf;

use qmetaobject::*;
use serde::{Deserialize, Serialize};
use toml;

use crate::listmodel::{MutListItem, MutListModel};

#[derive(QObject, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    base: qt_base_class!(trait QObject),
    #[serde(rename = "custom_cmd", with = "custom_cmd")]
    pub de: qt_property!(RefCell<MutListModel<DesktopEnviroment>>; CONST),
    pub de_index: qt_property!(usize; NOTIFY s1),
    pub auto_change: qt_property!(AutoChangeConfig; NOTIFY s2),
    pub resolution: qt_property!(Resolution; NOTIFY s3),
    pub autoremove: qt_property!(u64; NOTIFY s4),
    #[serde(skip)]
    s1: qt_signal!(),
    #[serde(skip)]
    s2: qt_signal!(),
    #[serde(skip)]
    s3: qt_signal!(),
    #[serde(skip)]
    s4: qt_signal!(),
    pub download_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub likes: Vec<String>,
}

impl Config {
    pub fn open() -> Result<Self, failure::Error> {
        let mut f = fs::File::open(Self::config_dir().join("config.toml"))?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        let config = toml::from_str(&s)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), failure::Error> {
        let path = Self::config_dir();
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        let mut f = fs::File::create(path.join("config.toml"))?;
        let s = toml::Value::try_from(self).unwrap();
        let mut v = toml::to_vec(&s)?;
        f.write_all(&mut v)?;
        Ok(())
    }

    fn config_dir() -> PathBuf {
        env::var("XDG_CONFIG_HOME")
            .map_or_else(
                |_| env::var("HOME").expect("") + "/.config/" + env!("CARGO_PKG_NAME"),
                |path| path + "/" + env!("CARGO_PKG_NAME"),
            )
            .into()
    }
}

// Only read and save the `cmd` of "Other", ignore default values
mod custom_cmd {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        s: &RefCell<MutListModel<DesktopEnviroment>>,
        ser: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let cmd = String::from_utf16_lossy(s.borrow().iter().last().unwrap().cmd.to_slice());
        String::serialize(&cmd, ser)
    }

    pub fn deserialize<'de, D>(de: D) -> Result<RefCell<MutListModel<DesktopEnviroment>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cmd = String::deserialize(de)?;
        let mut de = default_de_list();
        de.push(DesktopEnviroment {
            name: "Other".into(),
            cmd: cmd.into(),
        });
        Ok(RefCell::new(<_>::from_iter(de)))
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut de = default_de_list();
        de.push(DesktopEnviroment {
            name: "Other".into(),
            cmd: "".into(),
        });
        Self {
            base: Default::default(),
            de: RefCell::new(<_>::from_iter(de)),
            de_index: current_de(),
            auto_change: Default::default(),
            resolution: Default::default(),
            autoremove: 30,
            s1: Default::default(),
            s2: Default::default(),
            s3: Default::default(),
            s4: Default::default(),
            download_dir: env::var("XDG_DATA_HOME")
                .map_or_else(
                    |_| env::var("HOME").expect("") + "/.local/share/" + env!("CARGO_PKG_NAME"),
                    |path| path + "/" + env!("CARGO_PKG_NAME"),
                )
                .into(),
            cache_dir: env::var("XDG_CACHE_HOME")
                .map_or_else(
                    |_| env::var("HOME").expect("") + "/.cache/" + env!("CARGO_PKG_NAME"),
                    |path| path + "/" + env!("CARGO_PKG_NAME"),
                )
                .into(),
            likes: Default::default(),
        }
    }
}

fn default_de_list() -> Vec<DesktopEnviroment> {
    vec![
        DesktopEnviroment {
            name: "Budgie".into(),
            cmd: r#"gsettings set org.gnome.desktop.background picture-uri "file://$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "Cinnamon".into(),
            cmd: r#"gsettings set org.cinnamon.desktop.background picture-uri "file://$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "Deepin".into(),
            cmd: r#"gsettings set com.deepin.wrap.gnome.desktop.background picture-uri "file://$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "GNOME".into(),
            cmd: r#"gsettings set org.gnome.desktop.background picture-uri "file://$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "LXDE".into(),
            cmd: r#"pcmanfm --set-wallpaper "$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "LXQt".into(),
            cmd: r#"pcmanfm-qt --set-wallpaper "$WALLPAPER""#.into(),
        },
        DesktopEnviroment {
            name: "MATE".into(),
            cmd: r#"gsettings set org.mate.background picture-filename "$WALLPAPER""#.into(),
        },
    ]
}

fn current_de() -> usize {
    let v: String = (&[
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
        "GDMSESSION",
    ])
        .iter()
        .map(|e| env::var(e))
        .filter(|r| r.is_ok())
        .map(Result::unwrap)
        .filter(|r| !r.is_empty())
        .map(|s| s.to_lowercase())
        .next()
        .unwrap_or_default();
    match &*v {
        "budgie" => 0,
        "cinnamon" | "x-cinnamon" => 1,
        "deepin" => 2,
        "gnome" | "ubuntu:gnome" | "gnome-xorg" | "gnome-wayland" => 3,
        "lxde" => 4,
        "lxqt" => 5,
        "mate" => 6,
        _ => 7,
    }
}

#[derive(QGadget, Clone, Serialize, Deserialize)]
pub struct AutoChangeConfig {
    pub enable: qt_property!(bool),
    pub interval: qt_property!(u32),
    pub mode: qt_property!(u8),
}

impl Default for AutoChangeConfig {
    fn default() -> Self {
        Self {
            enable: false,
            interval: 5,
            mode: 0,
        }
    }
}

#[derive(QGadget, Clone, Serialize, Deserialize)]
pub struct Resolution {
    #[serde(skip, default = "default_preview")]
    pub preview: qt_property!(QVariantList),
    pub preview_index: qt_property!(usize),
    #[serde(skip, default = "default_download")]
    pub download: qt_property!(QVariantList),
    pub download_index: qt_property!(usize),
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            preview: default_preview(),
            preview_index: 0,
            download: default_download(),
            download_index: 0,
        }
    }
}

fn default_preview() -> QVariantList {
    <_>::from_iter(vec![QString::from("800x480"), QString::from("480x800")])
}

fn default_download() -> QVariantList {
    <_>::from_iter(vec![
        QString::from("1920x1080"),
        QString::from("1366x768"),
        QString::from("1080x1920"),
        QString::from("768x1280"),
    ])
}

#[derive(Serialize, Deserialize)]
pub struct DesktopEnviroment {
    #[serde(with = "qstring")]
    pub name: QString,
    #[serde(with = "qstring")]
    pub cmd: QString,
}

mod qstring {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(s: &QString, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        String::serialize(&String::from_utf16_lossy(s.to_slice()), ser)
    }

    pub fn deserialize<'de, D>(de: D) -> Result<QString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(de)?;
        Ok(v.into())
    }
}

impl MutListItem for DesktopEnviroment {
    fn get(&self, idx: i32) -> QVariant {
        match idx {
            0 => QMetaType::to_qvariant(&self.name),
            1 => QMetaType::to_qvariant(&self.cmd),
            _ => QVariant::default(),
        }
    }
    fn set(&mut self, value: &QVariant, idx: i32) -> bool {
        match idx {
            0 => <_>::from_qvariant(value.clone()).map(|v| self.name = v),
            1 => <_>::from_qvariant(value.clone()).map(|v| self.cmd = v),
            _ => None,
        }
        .is_some()
    }
    fn names() -> Vec<QByteArray> {
        vec![QByteArray::from("name"), QByteArray::from("cmd")]
    }
}
