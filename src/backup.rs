use anyhow::Result;
use chrono::prelude::*;
use libc::getuid;
use std::fs;
use std::path::{Path, PathBuf};

pub fn backup(backupdir: &Path, path: &Path) -> Result<()> {
    let mut components = path.components();
    components.next();
    let backup = backupdir.join(components.as_path());
    fs::create_dir_all(backup.parent().unwrap_or(backupdir)).expect("create backup dir");
    Ok(fs::rename(path, backup)?)
}

#[test]
fn test_backup() -> Result<()> {
    let backupdir = PathBuf::from("test/backup");
    let path = PathBuf::from("test/repo/bash/.bashrc");
    backup(&backupdir, &path)?;
    // roll back
    let mut components = path.components();
    components.next();
    let backedup = backupdir.join(components.as_path());
    assert!(backedup.exists());
    fs::rename(&backedup, &path)?;
    assert!(path.exists());
    Ok(())
}

pub fn get_backuppath() -> PathBuf {
    let mut backupdir = PathBuf::from(".backups");
    backupdir.push(format!("uid{}", unsafe { getuid() }));
    let local: DateTime<Local> = Local::now();
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    backupdir
}
