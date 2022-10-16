use crate::CONFFILE_NAME;
use anyhow::Result;
use glob::glob;
use serde::Deserialize;
use std::env::consts;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct GlobalConfig {
    pub src: Option<PathBuf>,
}

impl GlobalConfig {
    pub fn new() -> Result<Self> {
        let mut default_home = dirs::home_dir().unwrap_or(PathBuf::from("/"));
        default_home.push(".config");
        let mut path = std::env::var("XDG_CONFIG_HOME")
            .and_then(|s| Ok(PathBuf::from(s)))
            .unwrap_or(default_home);
        path.push("config.toml");
        let mut file = fs::File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(toml::from_str::<GlobalConfig>(&buf)?)
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
        let mut buf = String::new();
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
