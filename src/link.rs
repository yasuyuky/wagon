use crate::{backup, list_items, get_backuppath};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::os::unix;
use std::path::{Path, PathBuf};

fn link(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base, false)? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    println!("{} {} (exists)", "SKIP:".cyan(), &link);
                    continue;
                }
            }
            println!("{} {:?}", "BACKUP:".yellow(), &link.target);
            backup(backupdir, &link.target)?;
        }
        println!("{} {}", "LINK:".green(), &link);
        unix::fs::symlink(link.source, link.target)?;
    }
    Ok(())
}

#[test]
fn test_link() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let test_backupdir = &PathBuf::from("test/backup");
    link(&test_base, test_backupdir)?;
    let link_path = PathBuf::from("test/home/.bashrc");
    assert!(link_path.exists());
    assert!(fs::read_link(&link_path).is_ok());
    fs::remove_file(&link_path)?;
    assert!(!link_path.exists());
    Ok(())
}

pub fn link_dirs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath();
    for dir in dirs {
        link(&base.join(dir), &backupdir)?
    }
    Ok(())
}
