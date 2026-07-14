use crate::{
    CONFFILE_NAME, IGNOREFILE_NAME, Link, config::get_config, dest::get_dest, structs::display_path,
};
use anyhow::Result;
use colored::Colorize;
use ignore::{DirEntry, WalkBuilder};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn list_diritems(base: &Path) -> Result<HashSet<PathBuf>> {
    let mut items = HashSet::new();
    for d in get_config(base)?.and_then(|c| c.dirs).unwrap_or_default() {
        let Ok(full) = base.join(&d).canonicalize() else {
            continue;
        };
        if !fs::metadata(full)?.is_dir() {
            continue;
        }
        items.insert(base.join(d));
    }
    Ok(items)
}

#[test]
fn test_list_diritems() -> Result<()> {
    let test_base = PathBuf::from("test/repo/zsh");
    let diritems = list_diritems(&test_base)?;
    tracing::info!("diritems: {diritems:?}");
    assert!(!diritems.is_empty());
    Ok(())
}

fn filter_ignores(e: &DirEntry) -> bool {
    let p = e.path().file_name().unwrap_or_default().to_string_lossy();
    !(p == CONFFILE_NAME
        || p == IGNOREFILE_NAME
        || p == ".git"
        || p == ".gitignore"
        || p == ".gitmodules")
}

fn metadata_or_report_broken_link(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(err) => {
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err.into());
            }
            match fs::symlink_metadata(path) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    let target = fs::read_link(path)
                        .map(|target| format!(" -> {}", display_path(&target)))
                        .unwrap_or_default();
                    eprintln!(
                        "{} broken symlink: {}{} ({err})",
                        "ERROR:".red(),
                        display_path(path),
                        target
                    );
                    Ok(None)
                }
                _ => Err(err.into()),
            }
        }
    }
}

fn list_dir(base: &Path, dir: &Path, dir_items: &HashSet<PathBuf>) -> Result<Vec<Link>> {
    let mut items = vec![];
    let pat = dir.to_str().unwrap_or_default().to_string();
    'walk: for r in WalkBuilder::new(pat)
        .standard_filters(true)
        .hidden(false)
        .add_custom_ignore_filename(IGNOREFILE_NAME)
        .filter_entry(filter_ignores)
        .build()
    {
        match r {
            Ok(dent) => {
                let p = PathBuf::from(dent.path());
                let Some(meta) = metadata_or_report_broken_link(&p)? else {
                    continue;
                };
                let f = p.strip_prefix(base).unwrap_or(&p);
                let dst = get_dest(&p)?.canonicalize()?.join(f);
                if meta.is_file() {
                    for dir_item in dir_items {
                        if p.starts_with(dir_item) {
                            continue 'walk;
                        }
                    }
                    items.push(Link::new(p.canonicalize()?, dst, false));
                } else if meta.is_dir() && dir_items.contains(&p) {
                    items.push(Link::new(p.canonicalize()?, dst, true));
                }
            }
            Err(err) => println!("{err:?}"),
        }
    }
    Ok(items)
}

pub fn list_items(base: &Path, ignore_dirlink: bool) -> Result<Vec<Link>> {
    let dirs = if ignore_dirlink {
        HashSet::new()
    } else {
        list_diritems(base)?
    };
    let items = list_dir(base, base, &dirs)?;
    Ok(items)
}

#[test]
fn test_list_items() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let items = list_items(&test_base, true)?;
    tracing::info!("items: {items:?}");
    assert!(!items.is_empty());
    Ok(())
}

#[test]
fn test_list_items_with_diritems() -> Result<()> {
    let test_base = PathBuf::from("test/repo/zsh");
    let items = list_items(&test_base, false)?;
    tracing::info!("items: {items:#?}");
    assert!(items.len() == 2);
    Ok(())
}
