use anyhow::{Context, Result};
use chrono::prelude::*;
use colored::*;
use glob::glob;
use serde_derive::Deserialize;
use std::collections::HashSet;
use std::env::consts;
use std::fs;
use std::io::{self, BufRead, Read};
use std::os::unix;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

const CONFFILE_NAME: &str = ".wagon.toml";

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    /// Copy
    #[structopt(alias = "cp")]
    Copy { dir: Vec<PathBuf> },
    /// Link
    #[structopt(alias = "ln")]
    Link { dir: Vec<PathBuf> },
    /// List links
    #[structopt(alias = "ls")]
    List { dir: Vec<PathBuf> },
    /// Init
    Init { dir: Vec<PathBuf> },
}

#[derive(Debug, Clone)]
struct Link {
    source: PathBuf,
    target: PathBuf,
    is_dir: bool,
}

impl Link {
    fn new(source: PathBuf, target: PathBuf, is_dir: bool) -> Self {
        Self {
            source,
            target,
            is_dir,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    dest: Option<PathBuf>,
    init: Option<Vec<InitCommand>>,
    dirs: Option<Vec<PathBuf>>,
}

#[derive(Deserialize, Debug)]
struct InitCommand {
    command: String,
    args: Vec<String>,
    os: Option<String>,
}

enum Content {
    Text(Vec<String>),
    Binary(usize, Vec<u8>),
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
    ignores.insert(base.join(Path::new(CONFFILE_NAME)));
    Ok(ignores)
}

#[test]
fn test_list_ignores() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    fs::File::create("test/repo/bash/test")?;
    let ignores = list_ignores(&test_base)?;
    println!("ignore: {:?}", ignores);
    assert!(ignores.len() > 0);
    Ok(())
}

fn backup(backupdir: &Path, path: &Path) -> Result<()> {
    let mut components = path.components();
    components.next();
    let backup = backupdir.join(components.as_path());
    fs::create_dir_all(backup.parent().unwrap_or(backupdir)).expect("create backup dir");
    Ok(fs::rename(path, backup)?)
}

#[test]
fn test_backup() -> Result<()> {
    let backupdir = PathBuf::from("test/backup");
    let path = PathBuf::from("test/repo/bash/.bashrc");
    backup(&backupdir, &path)?;
    // roll back
    let mut components = path.components();
    components.next();
    let backedup = backupdir.join(components.as_path());
    assert!(backedup.exists());
    fs::rename(&backedup, &path)?;
    assert!(path.exists());
    Ok(())
}

fn get_config(base: &Path) -> Option<Config> {
    let confpath = base.join(Path::new(CONFFILE_NAME));
    Config::from_path(&confpath).ok()
}

#[test]
fn test_get_config() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let config = get_config(&test_base);
    println!("config: {:?}", config);
    assert!(config.is_some());
    Ok(())
}

fn get_dest(src: &Path) -> Result<PathBuf> {
    match get_config(&src.parent().unwrap()).and_then(|c| c.dest) {
        Some(p) => Ok(p),
        None => {
            let maybe_home = dirs::home_dir();
            maybe_home.context("cant get home dir")
        }
    }
}

#[test]
fn test_get_dest() -> Result<()> {
    let test_src = PathBuf::from("test/repo/bash/.bashrc");
    let dest = get_dest(&test_src)?;
    println!("dest: {:?}", dest);
    assert!(dest == PathBuf::from("test/home"));
    Ok(())
}

#[test]
fn test_get_dest_home() -> Result<()> {
    let test_src = PathBuf::from("test/repo/zsh/.zshrc");
    let dest = get_dest(&test_src)?;
    println!("dest: {:?}", dest);
    assert!(dest == dirs::home_dir().unwrap());
    Ok(())
}

fn list_diritems(base: &Path) -> Result<Vec<Link>> {
    let mut items = vec![];
    for d in get_config(&base).and_then(|c| c.dirs).unwrap_or_default() {
        let full = match base.join(&d).canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !fs::metadata(&full)?.is_dir() {
            continue;
        }
        let dst = get_dest(&full)?.canonicalize()?.join(&d);
        items.push(Link::new(full, dst, true))
    }
    Ok(items)
}

struct PathDict {
    dir: HashSet<PathBuf>,
    ign: HashSet<PathBuf>,
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

fn list_items(base: &Path, dirs: &[Link]) -> Result<Vec<Link>> {
    let pathdict = PathDict {
        dir: dirs.iter().map(|d| d.source.clone()).collect(),
        ign: list_ignores(&base)?,
    };
    let items = list_dir(base, base, &pathdict)?;
    Ok(items)
}

#[test]
fn test_list_items() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let items = list_items(&test_base, &vec![])?;
    println!("items: {:?}", items);
    assert!(items.len() > 0);
    Ok(())
}

fn link(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base, &list_diritems(&base)?)? {
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

fn link_dirs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath();
    for dir in dirs {
        link(&base.join(dir), &backupdir)?
    }
    Ok(())
}

fn copy(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base, &vec![])? {
        fs::create_dir_all(link.target.parent().unwrap_or_else(|| Path::new("/")))?;
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

fn copy_dirs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let backupdir = get_backuppath();
    for dir in dirs {
        copy(&base.join(dir), &backupdir)?
    }
    Ok(())
}

fn run_init(base: &Path) -> Result<()> {
    if let Some(conf) = get_config(base) {
        for initc in conf.init.unwrap_or_default() {
            if let Some(os) = initc.os {
                if !os.starts_with(consts::OS) {
                    continue;
                }
            }
            match std::process::Command::new(initc.command)
                .args(initc.args)
                .output()
            {
                Ok(out) => println!("{}", String::from_utf8(out.stdout)?),
                Err(e) => println!("Error: {:?}", e),
            }
        }
    }
    Ok(())
}

fn run_inits(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        run_init(&base.join(dir))?
    }
    Ok(())
}

fn read_text(f: &mut fs::File) -> Result<Content> {
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    let ss = buf.lines().map(String::from).collect();
    Ok(Content::Text(ss))
}

fn read_binary(f: &mut fs::File) -> Result<Content> {
    let mut buf = Vec::new();
    let size = f.read(&mut buf)?;
    Ok(Content::Binary(size, buf))
}

fn read_content(path: &Path) -> Result<(Content, String, String)> {
    let mut f = fs::File::open(path)?;
    let meta = f.metadata()?;
    let date = format!("{}", DateTime::<Local>::from(meta.modified()?));
    let ps = path.to_str().unwrap_or_default().to_owned();
    Ok((read_text(&mut f).unwrap_or(read_binary(&mut f)?), ps, date))
}

fn print_text_diff(ss: &[String], ts: &[String], sp: &str, tp: &str, sd: &str, td: &str) {
    let diff = difflib::unified_diff(ss, ts, sp, tp, sd, td, 3);
    for line in &diff {
        if line.starts_with('+') {
            println!("{}", line.trim_end().green());
        } else if line.starts_with('-') {
            println!("{}", line.trim_end().red());
        } else {
            println!("{}", line.trim_end());
        }
    }
}

fn print_binary_diff(ssz: usize, sb: Vec<u8>, tsz: usize, tb: Vec<u8>) {
    if sb != tb {
        println!(
            "{} src size:{}, dst size:{}",
            "binary files do not match.".red(),
            ssz,
            tsz
        )
    }
}

fn print_content_diff(link: &Link) -> Result<()> {
    let (srcc, sp, srcd) = read_content(&link.source)?;
    let (tgtc, tp, tgtd) = read_content(&link.target)?;
    match (srcc, tgtc) {
        (Content::Text(ss), Content::Text(ts)) => print_text_diff(&ss, &ts, &sp, &tp, &srcd, &tgtd),
        (Content::Binary(ssz, sb), Content::Binary(tsz, tb)) => print_binary_diff(ssz, sb, tsz, tb),
        _ => println!("file types do not match"),
    }
    Ok(())
}

fn print_link(base: &Path) -> Result<()> {
    for link in list_items(base, &list_diritems(base)?)? {
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    println!("{}: {}", "LINKED".cyan(), &link);
                }
            } else {
                println!("{}: {}", "EXISTS".magenta(), &link.target.to_str().unwrap());
                if !link.is_dir {
                    print_content_diff(&link)?
                }
            }
        } else {
            println!("{}: {}", "NOLINK".yellow(), &link)
        }
    }
    Ok(())
}

fn collect_dirs(base: &Path, dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if dirs.is_empty() {
        let pat = format!("{}/[0-9A-Za-z]*", base.to_str().unwrap());
        Ok(glob(&pat)?.flatten().collect())
    } else {
        Ok(dirs.iter().map(PathBuf::from).collect())
    }
}

fn print_links(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in collect_dirs(base, dirs)? {
        if fs::metadata(&dir)?.is_dir() {
            println!("{}", dir.file_name().unwrap().to_str().unwrap().bold());
            print_link(&dir)?
        }
    }
    Ok(())
}

fn get_backuppath() -> PathBuf {
    let mut backupdir = PathBuf::from(".backups");
    let local: DateTime<Local> = Local::now();
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    backupdir
}

fn main() -> Result<()> {
    let command = Command::from_args();
    let base = std::env::current_dir().expect("current dir");
    match command {
        Command::Copy { dir } => copy_dirs(&base, &dir)?,
        Command::Link { dir } => link_dirs(&base, &dir)?,
        Command::List { dir } => print_links(&base, &dir)?,
        Command::Init { dir } => run_inits(&base, &dir)?,
    }
    Ok(())
}
