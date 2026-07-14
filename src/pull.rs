use crate::{config::get_config, structs::display_path};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn absolute_path(value: &str) -> std::result::Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("target must be an absolute path".to_owned())
    }
}

pub fn pull_files(base: &Path, dir: &Path, targets: &[PathBuf]) -> Result<()> {
    if let Some(conf) = get_config(&base.join(dir))? {
        let dest = conf.dest.unwrap_or_else(|| dirs::home_dir().unwrap());
        for target in targets {
            if target.is_file() {
                let to = dir.join(target.strip_prefix(&dest)?);
                eprintln!(
                    "{}: {} -> {}",
                    "PULL".cyan(),
                    display_path(target),
                    display_path(&to),
                );
                fs::create_dir_all(to.parent().unwrap_or(dir))?;
                fs::copy(target, to)?;
            } else {
                eprintln!(
                    "{}: {} is directory",
                    "SKIPPED".yellow(),
                    display_path(target)
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Opt};
    use clap::Parser;

    #[test]
    fn pull_accepts_targets_without_dir() {
        let target_path = "/example/.zshrc";
        let opt = Opt::try_parse_from(["wagon", "pull", target_path]).unwrap();
        let Command::Pull { target } = opt.cmd else {
            panic!("expected pull command");
        };
        assert_eq!(target, vec![PathBuf::from(target_path)]);
    }

    #[test]
    fn pull_rejects_relative_targets() {
        let Err(err) = Opt::try_parse_from(["wagon", "pull", "zsh"]) else {
            panic!("expected parse error");
        };
        assert!(err.to_string().contains("target must be an absolute path"));
    }
}
