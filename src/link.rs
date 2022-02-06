use crate::backup::{backup, get_backuppath};
use crate::list::list_items;
use anyhow::Result;
use colored::Colorize;
use glob::glob;
use log::info;
use std::fs;
use std::os::unix;
use std::path::{Path, PathBuf};

fn link(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(base, false)? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    info!("{} {link} (exists)", "SKIPPED:".cyan());
                    continue;
                }
            }
            info!("{} {:?}", "BACKUP:".yellow(), &link.target);
            backup(backupdir, &link.target)?;
        }
        unix::fs::symlink(&link.source, &link.target)?;
        info!("{} {}", "LINKED:".green(), &link);
    }
    Ok(())
}

fn cleanup_dir(d: Option<&Path>) -> Result<()> {
    if let Some(p) = d {
        let p_str = p.to_str().unwrap_or_default();
        let mut ps = glob(&format!("{p_str}/*"))?;
        if ps.next().is_none() {
            fs::remove_dir(p)?;
            cleanup_dir(p.parent())?;
        }
    }
    Ok(())
}

fn unlink(base: &Path) -> Result<()> {
    for link in list_items(base, false)? {
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    info!("{} {link} (exists)", "UNLINK:".cyan());
                    fs::remove_file(&link.target)?;
                    cleanup_dir(link.target.parent())?;
                }
            }
        }
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
    unlink(&test_base)?;
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

pub fn unlink_dirs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        unlink(&base.join(dir))?
    }
    Ok(())
}
