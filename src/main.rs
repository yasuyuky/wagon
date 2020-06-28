extern crate chrono;
extern crate dirs;
extern crate glob;
extern crate structopt;

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

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<fs::File>>>
where
    P: AsRef<Path>,
{
    let file = fs::File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn list_ignores(base: &Path, target: &str) -> Result<Vec<PathBuf>, io::Error> {
    let mut ignores: Vec<PathBuf> = Vec::new();
    let i_pat = format!(
        "{}/{}/**/.gitignore",
        base.as_os_str().to_str().unwrap_or_default(),
        target
    );
    for entry in glob(&i_pat).expect("valid pattern") {
        match entry {
            Ok(path) => {
                if let Ok(lines) = read_lines(path) {
                    for line in lines {
                        if let Ok(pat) = line {
                            ignores.extend(glob(&pat).expect("valid").filter_map(|p| p.ok()));
                        }
                    }
                }
            }
            Err(err) => println!("{}", err),
        }
    }
    Ok(ignores)
}

fn backup(backupdir: &Path, path: &Path) {
    fs::create_dir_all(backupdir).expect("create backup dir");
    let backup = backupdir.join(path);
    fs::rename(path, backup).expect("backed up");
}

fn link(base: &Path, target: &str, backupdir: &Path) {
    let ignores = list_ignores(base, target).unwrap_or_default();
    let basetarget = base.join(target);
    let pat = format!(
        "{}/**/*",
        basetarget.as_os_str().to_str().unwrap_or_default()
    );
    for entry in glob(&pat).expect("valid pattern") {
        if let Ok(ref path) = entry {
            if !fs::metadata(path).expect("get metadata").is_file() {
                continue;
            }
            if ignores.iter().any(|ip| path.starts_with(ip)) {
                continue;
            }

            let f = path.strip_prefix(&basetarget).unwrap();
            let dst: PathBuf = dirs::home_dir().expect("home dir").join(f);
            fs::create_dir_all(dst.parent().unwrap()).expect("create dirs");
            if dst.exists() {
                if let Ok(_link) = fs::read_link(&dst)
                {
                    // TODO: check link == dst
                    println!("skip link {:?} -> {:?} (exists)", &dst, &path);
                    continue;
                }
            }
            backup(backupdir, path);
            unix::fs::symlink(path, dst).expect("create symlink");
        }
    }
}

fn retrieve_targets(base: &Path, targets: &[String], backupdir: &Path) {
    for target in targets {
        link(base, target, backupdir)
    }
}

fn main() -> Result<(), io::Error> {
    let command = Command::from_args();
    let base = std::env::current_dir().expect("current dir");
    let local: DateTime<Local> = Local::now();
    let mut backupdir = PathBuf::new().join(".backups");
    backupdir.push(local.format("%Y/%m/%d/%H:%M:%S").to_string());
    match command {
        Command::Link { target } => retrieve_targets(&base, &target, &backupdir),
        Command::List => println!("Not implemented"),
    };
    Ok(())
}
