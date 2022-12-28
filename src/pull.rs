use crate::config::get_config;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn pull_files(base: &Path, dir: &Path, targets: &[PathBuf]) -> Result<()> {
    if let Some(conf) = get_config(&base.join(dir))? {
        let dest = conf.dest.unwrap_or_else(|| dirs::home_dir().unwrap());
        for target in targets {
            if target.is_file() {
                let to = dir.join(target.strip_prefix(&dest)?);
                info!(
                    "{}: {} -> {}",
                    "PULL".cyan(),
                    target.to_str().unwrap_or_default(),
                    to.to_str().unwrap_or_default(),
                );
                fs::create_dir_all(to.parent().unwrap_or(dir))?;
                fs::copy(target, to)?;
            } else {
                info!(
                    "{}: {} is directory",
                    "SKIPPED".yellow(),
                    target.to_str().unwrap_or_default()
                );
            }
        }
    }
    Ok(())
}
