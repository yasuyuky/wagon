use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, shells};
use std::path::PathBuf;

mod backup;
mod config;
mod copy;
mod dest;
mod init;
mod link;
mod list;
mod pull;
mod repo;
mod show;
mod structs;
mod update;
mod wget;

use structs::{Content, Link};

const CONFFILE_NAME: &str = ".wagon.toml";
const IGNOREFILE_NAME: &str = ".wagonignore";
const CLICOLOR_FORCE: &str = "CLICOLOR_FORCE";
/// Manage dotfiles and project configs with symlinks and copies.
///
/// wagon scans a repository-like directory tree for files and directories,
/// links or copies them into a destination (defaults to your home), and
/// provides utilities to preview differences and run init/update hooks.
#[derive(Parser)]
struct Opt {
    #[clap(long)]
    color: bool,
    #[clap(long)]
    base: Option<PathBuf>,
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Parser)]
#[clap(rename_all = "kebab-case")]
enum Command {
    /// Copy
    #[clap(alias = "cp")]
    Copy { dir: Vec<PathBuf> },
    /// Link
    #[clap(alias = "ln")]
    Link { dir: Vec<PathBuf> },
    #[clap(alias = "rm")]
    Unlink { dir: Vec<PathBuf> },
    /// List links
    #[clap(alias = "ls")]
    List { dir: Vec<PathBuf> },
    /// Init
    Init { dir: Vec<PathBuf> },
    /// Update
    Update { dir: Vec<PathBuf> },
    /// Pull
    Pull { dir: PathBuf, target: Vec<PathBuf> },
    /// Repo
    Repo { pathlikes: Vec<String> },
    /// Wget
    Wget { url: String },
    /// Completion
    Completion {
        #[clap(subcommand)]
        shell: Shell,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Parser)]
#[clap(rename_all = "kebab-case")]
enum Shell {
    Bash,
    Fish,
    Zsh,
    PowerShell,
    Elvish,
}

fn init_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_level(false)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn generate_completion(shell: Shell) {
    let shell = match shell {
        Shell::Bash => shells::Shell::Bash,
        Shell::Fish => shells::Shell::Fish,
        Shell::Zsh => shells::Shell::Zsh,
        Shell::PowerShell => shells::Shell::PowerShell,
        Shell::Elvish => shells::Shell::Elvish,
    };
    let mut cmd = Opt::command();
    generate(shell, &mut cmd, "wagon", &mut std::io::stdout());
}

fn main() -> Result<()> {
    init_tracing();
    let opt = Opt::parse();
    let command = opt.cmd;
    if opt.color {
        unsafe { std::env::set_var(CLICOLOR_FORCE, "1") }
    }
    let current_dir = std::env::current_dir().expect("current dir");
    let base = opt.base.unwrap_or(current_dir);
    let cwd_or = |dirs: Vec<PathBuf>| -> Vec<PathBuf> {
        if dirs.is_empty() {
            vec![std::env::current_dir().expect("current dir")]
        } else {
            dirs
        }
    };
    match command {
        Command::Copy { dir } => copy::copy_dirs(&base, &cwd_or(dir))?,
        Command::Link { dir } => link::link_dirs(&base, &cwd_or(dir))?,
        Command::Unlink { dir } => link::unlink_dirs(&base, &cwd_or(dir))?,
        Command::List { dir } => show::show_list(&base, &cwd_or(dir))?,
        Command::Init { dir } => init::run_inits(&base, &cwd_or(dir))?,
        Command::Update { dir } => update::run_updates(&base, &cwd_or(dir))?,
        Command::Pull { dir, target } => pull::pull_files(&base, &dir, &cwd_or(target))?,
        Command::Repo { pathlikes } => {
            for pathlike in pathlikes {
                if pathlike == "checkout" {
                    continue;
                }
                repo::load_repo(&pathlike)?;
            }
        }
        Command::Wget { url } => wget::wget(&url)?,
        Command::Completion { shell } => generate_completion(shell),
    }
    Ok(())
}
