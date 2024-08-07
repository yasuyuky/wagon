use crate::CONFFILE_NAME;
use anyhow::Result;
use glob::glob;
use serde::Deserialize;
use std::env::consts;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";

#[derive(Deserialize, Debug, Default)]
pub struct GlobalConfig {
    pub src: PathBuf,
}

impl GlobalConfig {
    pub fn new() -> Self {
        let path = Self::get_path();
        if let Ok(mut file) = fs::File::open(path) {
            let mut buf = String::default();
            file.read_to_string(&mut buf).unwrap_or_default();
            toml::from_str::<GlobalConfig>(&buf).unwrap_or_default()
        } else {
            Self {
                src: PathBuf::from("src"),
            }
        }
    }

    fn get_path() -> PathBuf {
        let mut default_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        default_home.push(".config");
        let mut path = std::env::var(XDG_CONFIG_HOME).map_or_else(|_| default_home, PathBuf::from);
        path.push("wagon");
        path.push("config.toml");
        path
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dest: Option<PathBuf>,
    pub init: Option<Vec<Command>>,
    pub update: Option<Vec<Command>>,
    pub dirs: Option<Vec<PathBuf>>,
    pub os: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Command {
    pub command: String,
    pub args: Vec<String>,
    pub os: Option<String>,
}

impl Config {
    pub fn from_path(confpath: &Path) -> Result<Self> {
        let mut file = fs::File::open(confpath)?;
        let mut buf = String::default();
        file.read_to_string(&mut buf)?;
        Ok(toml::from_str::<Config>(&buf)?)
    }
}

pub fn get_config(base: &Path) -> Result<Option<Config>> {
    let longest = base.join(Path::new(CONFFILE_NAME));
    let mut components = longest.components();
    while components.next_back().is_some() {
        let compstr = components.as_path().to_str().unwrap_or_default();
        let confpat = format!("{compstr}/{CONFFILE_NAME}*");
        for confpath in glob(&confpat)?.flatten() {
            if let Ok(config) = Config::from_path(&confpath) {
                if let Some(os) = &config.os {
                    if os == consts::OS {
                        return Ok(Some(config));
                    }
                } else {
                    return Ok(Some(config));
                }
            }
        }
    }
    Ok(None)
}

#[test]
fn test_get_config() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let config = get_config(&test_base)?;
    tracing::info!("config: {config:?}");
    assert!(config.is_some());
    Ok(())
}
