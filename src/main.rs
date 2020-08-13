use anyhow::Result;
use chrono::prelude::*;
use colored::*;
use glob::glob;
use serde_derive::Deserialize;
use std::fs;
use std::io::{self, BufRead, Error, ErrorKind, Read};
use std::os::unix;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    /// Copy
    #[structopt(alias = "cp")]
    Copy { target: Vec<PathBuf> },
    /// Link
    #[structopt(alias = "ln")]
    Link { target: Vec<PathBuf> },
    /// List links
    #[structopt(alias = "ls")]
    List { target: Vec<PathBuf> },
}

struct Link {
    source: PathBuf,
    target: PathBuf,
}

impl Link {
    fn new(source: PathBuf, target: PathBuf) -> Self {
        Self { source, target }
    }
}

#[derive(Deserialize)]
struct Config {
    dest: Option<PathBuf>,
}

#[derive(Deserialize)]
struct InitCommand {
    command: String,
    args: Vec<String>,
}

impl Config {
    pub fn from_path(confpath: &Path) -> Result<Self> {
        let mut file = fs::File::open(confpath)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(toml::from_str::<Config>(&buf)?)
    }
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}",
            self.target.to_str().unwrap_or_default(),
            self.source.to_str().unwrap_or_default()
        )
    }
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
    let mut components = path.components();
    components.next();
    let backup = backupdir.join(components.as_path());
    fs::create_dir_all(backup.parent().unwrap_or(backupdir)).expect("create backup dir");
    Ok(fs::rename(path, backup)?)
}

fn get_config(base: &Path) -> Option<Config> {
    let confpath = base.join(Path::new(".wagon.toml"));
    Config::from_path(&confpath).ok()
}

fn get_dest(src: &Path) -> Result<PathBuf> {
    match get_config(&src.parent().unwrap()).and_then(|c| c.dest) {
        Some(p) => Ok(p),
        None => dirs::home_dir().ok_or(anyhow::Error::new(Error::from(ErrorKind::NotFound))),
    }
}

fn list_items(base: &Path) -> Result<Vec<Link>> {
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
        let dst = get_dest(&src)?.join(f);
        items.push(Link::new(src, dst));
    }
    Ok(items)
}

fn link(base: &Path, target: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base.join(target))? {
        fs::create_dir_all(link.target.parent().unwrap_or(Path::new("/")))?;
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

fn link_targets(base: &Path, targets: &[PathBuf], backupdir: &Path) -> Result<()> {
    for target in targets {
        link(base, target, backupdir)?
    }
    Ok(())
}

fn copy(base: &Path, target: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base.join(target))? {
        fs::create_dir_all(link.target.parent().unwrap_or(Path::new("/")))?;
        if link.target.exists() {
            let content_src = fs::read(&link.source)?;
            if let Ok(content) = fs::read(&link.target) {
                if content == content_src {
                    println!("{} {} (exists)", "SKIP:".cyan(), &link);
                    continue;
                }
            }
            println!("{} {:?}", "BACKUP:".yellow(), &link.target);
            backup(backupdir, &link.target)?;
        }
        println!("{} {}", "COPY:".green(), &link);
        fs::copy(link.source, link.target)?;
    }
    Ok(())
}

fn copy_targets(base: &Path, targets: &[PathBuf], backupdir: &Path) -> Result<()> {
    for target in targets {
        copy(base, target, backupdir)?
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
        for link in list_items(&base.join(target))? {
            if link.target.exists() {
                if let Ok(readlink) = fs::read_link(&link.target) {
                    if readlink == link.source {
                        println!("{}", &link);
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
        Command::Copy { target } => copy_targets(&base, &target, &backupdir)?,
        Command::Link { target } => link_targets(&base, &target, &backupdir)?,
        Command::List { target } => print_links(&base, &target)?,
    }
    Ok(())
}
