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
mod repo;
mod show;
mod structs;
mod update;
use structs::{Content, Link};

const CONFFILE_NAME: &str = ".wagon.toml";
const IGNOREFILE_NAME: &str = ".wagonignore";

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
    /// Update
    Update { dir: Vec<PathBuf> },
    /// Pull
    Pull { dir: PathBuf, target: Vec<PathBuf> },
    /// Repo
    Repo { pathlike: String },
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

fn init_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_level(false)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn main() -> Result<()> {
    init_tracing();
    let opt = Opt::from_args();
    let command = opt.cmd;
    if opt.color {
        std::env::set_var("CLICOLOR_FORCE", "1");
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
        Command::Repo { pathlike } => repo::load_repo(&pathlike)?,
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
