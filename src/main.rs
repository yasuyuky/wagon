extern crate anyhow;
extern crate chrono;
extern crate dirs;
extern crate glob;
extern crate structopt;

use anyhow::Result;
use chrono::prelude::*;
use glob::glob;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    /// Link
    #[structopt(alias = "ln")]
    Link {
        #[structopt(parse(from_str))]
        target: Vec<String>,
    },
    /// List links
    #[structopt(alias = "ls")]
    List,
}

fn list_ignores(base: &Path) -> Result<Vec<PathBuf>> {
    let mut ignores: Vec<PathBuf> = Vec::new();
    let ifilespat = format!("{}/**/.gitignore", base.to_str().unwrap_or_default());
    for ref path in glob(&ifilespat)?.flatten() {
        for line in io::BufReader::new(fs::File::open(path)?).lines().flatten() {
            ignores.extend(glob(&line)?.flatten());
        }
    }
    Ok(ignores)
}

fn backup(backupdir: &Path, path: &Path) -> Result<()> {
    fs::create_dir_all(backupdir).expect("create backup dir");
    let backup = backupdir.join(path);
    Ok(fs::rename(path, backup)?)
}

fn list_candidates(base: &Path, target: &str) -> Result<Vec<(PathBuf, PathBuf)>> {
    let basetarget = base.join(target);
    let ignores = list_ignores(&basetarget)?;
    let pat = format!("{}/**/*", basetarget.to_str().unwrap_or_default());
    let mut candidates = vec![];
    for src in glob(&pat)?.flatten() {
        if !fs::metadata(&src)?.is_file() {
            continue;
        }
        if ignores.iter().any(|ip| src.starts_with(ip)) {
            continue;
        }
        let f = src.strip_prefix(&basetarget).unwrap();
        let dst: PathBuf = dirs::home_dir().expect("home dir").join(f);
        candidates.push((src, dst));
    }
    Ok(candidates)
}

fn link(base: &Path, target: &str, backupdir: &Path) -> Result<()> {
    for (src, dst) in list_candidates(base, target)? {
        fs::create_dir_all(dst.parent().unwrap_or(Path::new("/")))?;
        if dst.exists() {
            if let Ok(_link) = fs::read_link(&dst) {
                // TODO: check link == src
                println!("skip link {:?} -> {:?} (exists)", &dst, &src);
                continue;
            }
        }
        backup(backupdir, &dst)?;
        unix::fs::symlink(src, dst)?;
    }
    Ok(())
}

fn link_targets(base: &Path, targets: &[String], backupdir: &Path) -> Result<()> {
    for target in targets {
        link(base, target, backupdir)?
    }
    Ok(())
}

fn list_links(base: &Path, root: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut links = Vec::new();
    let pat = format!("{}/*", root.to_str().unwrap());
    for ref path in glob(&pat)?.flatten() {
        if let Ok(link) = fs::read_link(path) {
            if link.starts_with(base) {
                links.push((path.clone(), link));
            }
        } else if path.is_dir() {
            links.extend(list_links(base, &path)?)
        }
    }
    Ok(links)
}

fn print_links(base: &Path) -> Result<()> {
    for (p, l) in list_links(base, &dirs::home_dir().expect("home"))? {
        println!("{} -> {}", p.to_str().unwrap(), l.to_str().unwrap())
    }
    Ok(())
}

fn main() -> Result<()> {
    let command = Command::from_args();
    let base = std::env::current_dir().expect("current dir");
    let local: DateTime<Local> = Local::now();
    let mut backupdir = PathBuf::new().join(".backups");
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    match command {
        Command::Link { target } => link_targets(&base, &target, &backupdir)?,
        Command::List => print_links(&base)?,
    };
    Ok(())
}
