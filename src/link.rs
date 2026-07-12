use crate::backup::{backup, get_backuppath};
use crate::list::list_items;
use crate::structs::display_path;
use anyhow::Result;
use colored::Colorize;
use glob::glob;
use std::fs;
use std::io;
use std::os::unix;
use std::path::{Path, PathBuf};

fn target_is_missing(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(false),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(err) => Err(err.into()),
    }
}

fn link(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(base, false)? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
        if let Ok(readlink) = fs::read_link(&link.target) {
            if readlink == link.source {
                eprintln!("{} {link} (exists)", "SKIPPED:".cyan());
                continue;
            } else {
                if target_is_missing(&link.target)? {
                    eprintln!(
                        "{} broken symlink: {} -> {}",
                        "ERROR:".red(),
                        display_path(&link.target),
                        display_path(&readlink)
                    );
                }
                eprintln!("{} {:?}", "LINK BACKUP:".yellow(), &link.target);
                backup(backupdir, &link.target)?;
            }
        } else if link.target.exists() {
            eprintln!("{} {:?}", "BACKUP:".yellow(), &link.target);
            backup(backupdir, &link.target)?;
        }
        unix::fs::symlink(&link.source, &link.target)?;
        eprintln!("{} {}", "LINKED:".green(), &link);
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
        if link.target.exists()
            && let Ok(readlink) = fs::read_link(&link.target)
            && readlink == link.source
        {
            eprintln!("{} {link} (exists)", "UNLINK:".cyan());
            fs::remove_file(&link.target)?;
            cleanup_dir(link.target.parent())?;
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

pub fn link_dirs(dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath()?;
    for dir in dirs {
        link(dir, &backupdir)?
    }
    Ok(())
}

pub fn unlink_dirs(dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        unlink(dir)?
    }
    Ok(())
}
