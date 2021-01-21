use anyhow::{Context, Result};
use chrono::prelude::*;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod config;
mod copy;
mod init;
mod link;
mod list;
mod show;
mod structs;
use config::get_config;
use copy::copy_dirs;
use init::run_inits;
use link::link_dirs;
use list::list_items;
use show::show_list;
use structs::{Content, Link, PathDict};

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

fn get_backuppath() -> PathBuf {
    let mut backupdir = PathBuf::from(".backups");
    let local: DateTime<Local> = Local::now();
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    backupdir
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
        Command::List { dir } => show_list(&base, &dir)?,
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
