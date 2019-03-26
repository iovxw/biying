use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use toml;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub download_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub likes: HashSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
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

impl Config {
    pub fn open() -> Result<Self, failure::Error> {
        let mut f = fs::File::open(Self::config_dir().with_file_name(env!("CARGO_PKG_NAME")))?;
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
        let mut f = fs::File::create(path.with_file_name(env!("CARGO_PKG_NAME")))?;
        let mut v = toml::to_vec(self)?;
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
