use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

mod backup;
mod config;
mod copy;
mod dest;
mod init;
mod link;
mod list;
mod pull;
mod show;
mod structs;
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
    #[structopt(alias = "rm")]
    Unlink { dir: Vec<PathBuf> },
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

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let command = opt.cmd;
    if opt.color {
        std::env::set_var("CLICOLOR_FORCE", "1");
    }
    let current_dir = std::env::current_dir().expect("current dir");
    let base = opt.base.unwrap_or(current_dir);
    match command {
        Command::Copy { dir } => copy::copy_dirs(&base, &dir)?,
        Command::Link { dir } => link::link_dirs(&base, &dir)?,
        Command::Unlink { dir } => link::unlink_dirs(&base, &dir)?,
        Command::List { dir } => show::show_list(&base, &dir)?,
        Command::Init { dir } => init::run_inits(&base, &dir)?,
        Command::Pull { dir, target } => pull::pull_files(&base, &dir, &target)?,
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
