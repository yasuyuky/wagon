use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dest: Option<PathBuf>,
    pub init: Option<Vec<InitCommand>>,
    pub dirs: Option<Vec<PathBuf>>,
    pub os: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct InitCommand {
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
