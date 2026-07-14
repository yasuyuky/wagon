use crate::backup::{backup, get_backuppath};
use crate::list::list_items;
use crate::structs::display_path;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

fn copy(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(base, true)? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
        if link.target.exists() {
            let content_src = fs::read(&link.source)?;
            if let Ok(content) = fs::read(&link.target)
                && content == content_src
            {
                eprintln!("{} {link} (exists)", "SKIP:".cyan());
                continue;
            }
            eprintln!("{} {}", "BACKUP:".yellow(), display_path(&link.target));
            backup(backupdir, &link.target)?;
        }
        eprintln!("{} {}", "COPY:".green(), &link);
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

pub fn copy_dirs(dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath()?;
    for dir in dirs {
        copy(dir, &backupdir)?
    }
    Ok(())
}
