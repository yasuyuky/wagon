use anyhow::Result;
use chrono::prelude::*;
use colored::*;
use glob::glob;
use serde_derive::Deserialize;
use std::env::consts;
use std::fs;
use std::io::{self, BufRead, Error, ErrorKind, Read};
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
    /// Diff
    Diff { dir: Vec<PathBuf> },
}

#[derive(Debug)]
struct Link {
    source: PathBuf,
    target: PathBuf,
}

impl Link {
    fn new(source: PathBuf, target: PathBuf) -> Self {
        Self { source, target }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    dest: Option<PathBuf>,
    init: Option<Vec<InitCommand>>,
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

fn list_ignores(base: &Path) -> Result<Vec<PathBuf>> {
    let mut ignores: Vec<PathBuf> = Vec::new();
    let ifilespat = format!("{}/**/.gitignore", base.to_str().unwrap_or_default());
    for ref path in glob(&ifilespat)?.flatten() {
        for line in io::BufReader::new(fs::File::open(path)?).lines().flatten() {
            let pat = path.parent().unwrap().join(&line);
            ignores.extend(glob(&pat.as_os_str().to_str().unwrap())?.flatten());
        }
    }
    let mut ifiles = glob(&ifilespat)?.flatten().collect();
    ignores.append(&mut ifiles);
    ignores.push(base.join(Path::new(CONFFILE_NAME)));
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
            dirs::home_dir().ok_or_else(|| anyhow::Error::new(Error::from(ErrorKind::NotFound)))
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

fn list_items(base: &Path) -> Result<Vec<Link>> {
    let ignores = list_ignores(&base)?;
    let pat = format!("{}/**/*", base.to_str().unwrap_or_default());
    let mut items = vec![];
    for src in glob(&pat)?.flatten() {
        if !fs::metadata(&src)?.is_file() {
            continue;
        }
        if ignores.iter().any(|ip| src == *ip) {
            continue;
        }
        let f = src.strip_prefix(&base).unwrap();
        let dst = get_dest(&src)?.join(f);
        items.push(Link::new(src, dst));
    }
    Ok(items)
}

#[test]
fn test_list_items() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let items = list_items(&test_base)?;
    println!("items: {:?}", items);
    assert!(items.len() > 0);
    Ok(())
}

fn link(base: &Path, dir: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base.join(dir))? {
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
    let test_base = PathBuf::from("test/repo");
    let dir = &PathBuf::from("bash");
    let test_backupdir = &PathBuf::from("test/backup");
    link(&test_base, dir, test_backupdir)?;
    let link_path = PathBuf::from("test/home/.bashrc");
    assert!(link_path.exists());
    fs::remove_file(&link_path)?;
    assert!(!link_path.exists());
    Ok(())
}

fn link_dirs(base: &Path, dirs: &[PathBuf], backupdir: &Path) -> Result<()> {
    for dir in dirs {
        link(base, dir, backupdir)?
    }
    Ok(())
}

fn copy(base: &Path, dir: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base.join(dir))? {
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

fn copy_dirs(base: &Path, dirs: &[PathBuf], backupdir: &Path) -> Result<()> {
    for dir in dirs {
        copy(base, dir, backupdir)?
    }
    Ok(())
}

fn print_links(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let alldirs: Vec<PathBuf> = if dirs.is_empty() {
        let pat = format!("{}/*", base.to_str().unwrap());
        glob(&pat)?.flatten().collect()
    } else {
        dirs.iter().map(PathBuf::from).collect()
    };
    for ref dir in alldirs {
        for link in list_items(&base.join(dir))? {
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

fn run_init(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        if let Some(conf) = get_config(&base.join(dir)) {
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
    let ps = path.as_os_str().to_str().unwrap_or_default().to_owned();
    Ok((read_text(&mut f).unwrap_or(read_binary(&mut f)?), ps, date))
}

fn print_diffs(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    let alldirs: Vec<PathBuf> = if dirs.is_empty() {
        let pat = format!("{}/*", base.to_str().unwrap());
        glob(&pat)?.flatten().collect()
    } else {
        dirs.iter().map(PathBuf::from).collect()
    };
    for ref dir in alldirs {
        for link in list_items(&base.join(dir))? {
            println!("{}", link.target.to_str().unwrap_or_default().yellow());
            if link.target.exists() {
                if let Ok(readlink) = fs::read_link(&link.target) {
                    if readlink == link.source {
                        println!("{} {}", "LINK".cyan(), &link);
                    }
                } else {
                    let (srcc, sp, srcd) = read_content(&link.source)?;
                    let (tgtc, tp, tgtd) = read_content(&link.target)?;
                    match (srcc, tgtc) {
                        (Content::Text(ss), Content::Text(ts)) => {
                            let diff = difflib::unified_diff(&ss, &ts, &sp, &tp, &srcd, &tgtd, 3);
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
                        (Content::Binary(ssz, sb), Content::Binary(tsz, tb)) => {
                            if sb != tb {
                                println!(
                                    "{} src size:{}, dst size:{}",
                                    "binary files do not match.".red(),
                                    ssz,
                                    tsz
                                )
                            }
                        }
                        _ => println!("file types do not match"),
                    }
                }
            } else {
                println!("target does not exist");
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
        Command::Copy { dir } => copy_dirs(&base, &dir, &backupdir)?,
        Command::Link { dir } => link_dirs(&base, &dir, &backupdir)?,
        Command::List { dir } => print_links(&base, &dir)?,
        Command::Init { dir } => run_init(&base, &dir)?,
        Command::Diff { dir } => print_diffs(&base, &dir)?,
    }
    Ok(())
}
