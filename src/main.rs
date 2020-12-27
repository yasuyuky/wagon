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

#[derive(StructOpt)]
struct Opt {
    #[structopt(long)]
    color: bool,
    #[structopt(long)]
    base: Option<PathBuf>,
    #[structopt(subcommand)]
    cmd: Command,
}

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
    /// Pull
    Pull { dir: PathBuf, target: Vec<PathBuf> },
    /// Completion
    Completion {
        #[structopt(subcommand)]
        shell: Shell,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Shell {
    Bash,
    Fish,
    Zsh,
    PowerShell,
    Elvish,
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
    os: Option<String>,
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
    let confpat = format!("{}/{}*", base.to_str().unwrap_or_default(), CONFFILE_NAME);
    ignores.extend(glob(&confpat)?.flatten());
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

fn get_config(base: &Path) -> Result<Option<Config>> {
    let longest = base.join(Path::new(CONFFILE_NAME));
    let mut components = longest.components();
    while components.next_back().is_some() {
        let compstr = components.as_path().to_str().unwrap_or_default();
        let confpat = format!("{}/{}*", compstr, CONFFILE_NAME);
        for confpath in glob(&confpat)?.flatten() {
            if let Ok(config) = Config::from_path(&confpath) {
                if let Some(os) = &config.os {
                    if os == consts::OS {
                        return Ok(Some(config));
                    }
                } else {
                    return Ok(Some(config));
                }
            }
        }
    }
    Ok(None)
}

#[test]
fn test_get_config() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    let config = get_config(&test_base)?;
    println!("config: {:?}", config);
    assert!(config.is_some());
    Ok(())
}

fn get_dest(src: &Path) -> Result<PathBuf> {
    match get_config(&src.parent().unwrap())?.and_then(|c| c.dest) {
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
    for d in get_config(&base)?.and_then(|c| c.dirs).unwrap_or_default() {
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

fn list_items(base: &Path, ignore_dirlink: bool) -> Result<Vec<Link>> {
    let dirs = if ignore_dirlink {
        vec![]
    } else {
        list_diritems(base)?
    };
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
    let items = list_items(&test_base, true)?;
    println!("items: {:?}", items);
    assert!(items.len() > 0);
    Ok(())
}

fn link(base: &Path, backupdir: &Path) -> Result<()> {
    for link in list_items(&base, false)? {
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
    for link in list_items(&base, true)? {
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
    if let Some(conf) = get_config(base)? {
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

#[test]
fn test_run_init() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    run_init(&test_base)?;
    let file_path = PathBuf::from("testtouch");
    assert!(file_path.exists());
    fs::remove_file(&file_path)?;
    assert!(!file_path.exists());
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

fn get_text_diff(ss: &[String], ts: &[String], sp: &str, tp: &str, sd: &str, td: &str) -> String {
    difflib::unified_diff(ss, ts, sp, tp, sd, td, 3)
        .iter()
        .map(|line| {
            if line.starts_with('+') {
                format!("{}", line.trim_end().green())
            } else if line.starts_with('-') {
                format!("{}", line.trim_end().red())
            } else {
                line.trim_end().to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn check_binary_diff(ssz: usize, sb: Vec<u8>, tsz: usize, tb: Vec<u8>) -> String {
    if sb != tb {
        format!(
            "{} src size:{}, dst size:{}",
            "binary files do not match.".red(),
            ssz,
            tsz
        )
    } else {
        String::new()
    }
}

fn show_content_diff(link: &Link) -> Result<String> {
    let (srcc, sp, srcd) = read_content(&link.source)?;
    let (tgtc, tp, tgtd) = read_content(&link.target)?;
    Ok(match (srcc, tgtc) {
        (Content::Text(ss), Content::Text(ts)) => get_text_diff(&ss, &ts, &sp, &tp, &srcd, &tgtd),
        (Content::Binary(ssz, sb), Content::Binary(tsz, tb)) => check_binary_diff(ssz, sb, tsz, tb),
        _ => "file types do not match".to_owned(),
    })
}

fn show_link(base: &Path) -> Result<String> {
    let mut vs = vec![];
    for link in list_items(base, false)? {
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    vs.push(format!("{}: {}", "LINKED".cyan(), &link))
                }
            } else {
                let tgt = link.target.to_str().unwrap_or_default();
                vs.push(format!("{}: {}", "EXISTS".magenta(), tgt));
                if !link.is_dir {
                    vs.push(show_content_diff(&link)?)
                }
            }
        } else {
            vs.push(format!("{}: {}", "NOLINK".yellow(), &link))
        }
    }
    Ok(vs.join("\n"))
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
            println!("{}", show_link(&dir)?)
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

fn pull_files(base: &Path, dir: &Path, targets: &[PathBuf]) -> Result<()> {
    if let Some(conf) = get_config(&base.join(&dir))? {
        let dest = conf.dest.unwrap_or(dirs::home_dir().unwrap_or_default());
        for target in targets {
            if target.is_file() {
                let to = dir.join(target.strip_prefix(&dest)?);
                println!(
                    "{}: {} -> {}",
                    "PULL".cyan(),
                    target.to_str().unwrap_or_default(),
                    to.to_str().unwrap_or_default(),
                );
                fs::create_dir_all(to.parent().unwrap_or(dir))?;
                fs::copy(target, to)?;
            } else {
                println!(
                    "{}: {} is directory",
                    "SKIPPED".yellow(),
                    target.to_str().unwrap_or_default()
                );
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let command = opt.cmd;
    if opt.color {
        std::env::set_var("CLICOLOR_FORCE", "1");
    }
    let current_dir = std::env::current_dir().expect("current dir");
    let base = opt.base.unwrap_or(current_dir);
    match command {
        Command::Copy { dir } => copy_dirs(&base, &dir)?,
        Command::Link { dir } => link_dirs(&base, &dir)?,
        Command::List { dir } => print_links(&base, &dir)?,
        Command::Init { dir } => run_inits(&base, &dir)?,
        Command::Pull { dir, target } => pull_files(&base, &dir, &target)?,
        Command::Completion { shell } => {
            let shell = match shell {
                Shell::Bash => structopt::clap::Shell::Bash,
                Shell::Fish => structopt::clap::Shell::Fish,
                Shell::Zsh => structopt::clap::Shell::Zsh,
                Shell::PowerShell => structopt::clap::Shell::PowerShell,
                Shell::Elvish => structopt::clap::Shell::Elvish,
            };
            Command::clap().gen_completions_to("wagon", shell, &mut std::io::stdout());
        }
    }
    Ok(())
}
