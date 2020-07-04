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
    Link { target: Vec<PathBuf> },
    /// List links
    #[structopt(alias = "ls")]
    List { target: Vec<PathBuf> },
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

fn list_items(base: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let ignores = list_ignores(&base)?;
    let pat = format!("{}/**/*", base.to_str().unwrap_or_default());
    let mut items = vec![];
    for src in glob(&pat)?.flatten() {
        if !fs::metadata(&src)?.is_file() {
            continue;
        }
        if ignores.iter().any(|ip| src.starts_with(ip)) {
            continue;
        }
        let f = src.strip_prefix(&base).unwrap();
        let dst: PathBuf = dirs::home_dir().expect("home dir").join(f);
        items.push((src, dst));
    }
    Ok(items)
}

fn link(base: &Path, target: &Path, backupdir: &Path) -> Result<()> {
    for (src, dst) in list_items(&base.join(target))? {
        fs::create_dir_all(dst.parent().unwrap_or(Path::new("/")))?;
        if dst.exists() {
            if let Ok(link) = fs::read_link(&dst) {
                if link == src {
                    println!("SKIP: {:?} -> {:?} (exists)", &dst, &src);
                    continue;
                }
            }
            println!("BACKUP: {:?}", &dst);
            backup(backupdir, &dst)?;
        }
        println!("LINK: {:?} -> {:?}", &dst, &src);
        unix::fs::symlink(src, dst)?;
    }
    Ok(())
}

fn link_targets(base: &Path, targets: &[PathBuf], backupdir: &Path) -> Result<()> {
    for target in targets {
        link(base, target, backupdir)?
    }
    Ok(())
}

fn print_links(base: &Path, targets: &[PathBuf]) -> Result<()> {
    let alltargets: Vec<PathBuf> = if targets.is_empty() {
        let pat = format!("{}/*", base.to_str().unwrap());
        glob(&pat)?.flatten().collect()
    } else {
        targets.iter().map(PathBuf::from).collect()
    };
    for ref target in alltargets {
        for (src, dst) in list_items(&base.join(target))? {
            if dst.exists() {
                if let Ok(link) = fs::read_link(&dst) {
                    if link == src {
                        println!("{} -> {}", &dst.to_str().unwrap(), &src.to_str().unwrap());
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let command = Command::from_args();
    let base = std::env::current_dir().expect("current dir");
    let local: DateTime<Local> = Local::now();
    let mut backupdir = PathBuf::from(".backups");
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    match command {
        Command::Link { target } => link_targets(&base, &target, &backupdir)?,
        Command::List { target } => print_links(&base, &target)?,
    };
    Ok(())
}
