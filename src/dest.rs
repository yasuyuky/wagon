use crate::config;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn get_dest(src: &Path) -> Result<PathBuf> {
    match config::get_config(src.parent().unwrap())?.and_then(|c| c.dest) {
        Some(p) => Ok(p),
        None => {
            let maybe_home = dirs::home_dir();
            maybe_home.context("cant get home dir")
        }
    }
}

#[test]
fn test_get_dest() -> Result<()> {
    let test_src = PathBuf::from("test/repo/bash/.bashrc");
    let dest = get_dest(&test_src)?;
    log::info!("dest: {:?}", dest);
    assert!(dest == PathBuf::from("test/home"));
    Ok(())
}

#[test]
fn test_get_dest_home() -> Result<()> {
    let test_src = PathBuf::from("test/repo/zsh/.zshrc");
    let dest = get_dest(&test_src)?;
    log::info!("dest: {:?}", dest);
    assert!(dest == dirs::home_dir().unwrap());
    Ok(())
}
