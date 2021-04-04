use crate::backup::{backup, get_backuppath};
use crate::list::list_items;
use anyhow::Result;
use colored::Colorize;
use log::info;
use std::fs;
use std::path::{Path, PathBuf};

fn copy(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base, true)? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
        if link.target.exists() {
            let content_src = fs::read(&link.source)?;
            if let Ok(content) = fs::read(&link.target) {
                if content == content_src {
                    info!("{} {} (exists)", "SKIP:".cyan(), &link);
                    continue;
                }
            }
            info!("{} {:?}", "BACKUP:".yellow(), &link.target);
            backup(backupdir, &link.target)?;
        }
        info!("{} {}", "COPY:".green(), &link);
        fs::copy(link.source, link.target)?;
    }
    Ok(())
}

#[test]
fn test_copy() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let test_backupdir = &PathBuf::from("test/backup");
    copy(&test_base, test_backupdir)?;
    let copy_path = PathBuf::from("test/home/.bashrc");
    assert!(copy_path.exists());
    fs::remove_file(&copy_path)?;
    assert!(!copy_path.exists());
    Ok(())
}

pub fn copy_dirs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath();
    for dir in dirs {
        copy(&base.join(dir), &backupdir)?
    }
    Ok(())
}
