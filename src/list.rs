use crate::{config::get_config, dest::get_dest, Link, PathDict, CONFFILE_NAME};
use anyhow::Result;
use glob::glob;
use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

fn list_ignores(base: &Path) -> Result<HashSet<PathBuf>> {
    let mut ignores = HashSet::new();
    let ifilespat = format!("{}/**/.gitignore", base.to_str().unwrap_or_default());
    for ref path in glob(&ifilespat)?.flatten() {
        for line in io::BufReader::new(fs::File::open(path)?).lines().flatten() {
            let pat = path.parent().unwrap().join(&line);
            ignores.extend(glob(&pat.to_str().unwrap())?.flatten());
        }
    }
    ignores.extend(glob(&ifilespat)?.flatten());
    let confpat = format!("{}/{}*", base.to_str().unwrap_or_default(), CONFFILE_NAME);
    ignores.extend(glob(&confpat)?.flatten());
    Ok(ignores)
}

#[test]
fn test_list_ignores() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    fs::File::create("test/repo/bash/test")?;
    let ignores = list_ignores(&test_base)?;
    log::info!("ignore: {:?}", ignores);
    assert!(ignores.len() > 0);
    Ok(())
}

fn list_diritems(base: &Path) -> Result<HashSet<PathBuf>> {
    let mut items = HashSet::new();
    for d in get_config(&base)?.and_then(|c| c.dirs).unwrap_or_default() {
        let full = match base.join(&d).canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !fs::metadata(&full)?.is_dir() {
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
    log::info!("diritems: {:?}", diritems);
    assert!(diritems.len() > 0);
    Ok(())
}

fn list_dir(base: &Path, dir: &Path, pathdict: &PathDict) -> Result<Vec<Link>> {
    let mut items = vec![];
    let pat = format!("{}/*", dir.to_str().unwrap_or_default());
    for p in glob(&pat)?.flatten() {
        if pathdict.ign.contains(&p) {
            continue;
        }
        let f = p.strip_prefix(&base)?;
        let dst = get_dest(&p)?.canonicalize()?.join(f);
        if fs::metadata(&p)?.is_file() {
            items.push(Link::new(p.canonicalize()?, dst, false));
        } else if fs::metadata(&p)?.is_dir() {
            if pathdict.dir.contains(&p) {
                items.push(Link::new(p.canonicalize()?, dst, true));
            } else {
                items.extend(list_dir(base, &p, pathdict)?);
            }
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
    let pathdict = PathDict {
        dir: dirs,
        ign: list_ignores(&base)?,
    };
    let items = list_dir(base, base, &pathdict)?;
    Ok(items)
}

#[test]
fn test_list_items() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let items = list_items(&test_base, true)?;
    log::info!("items: {:?}", items);
    assert!(items.len() > 0);
    Ok(())
}
