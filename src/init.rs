use crate::config::get_config;
use anyhow::Result;
use colored::Colorize;
use log::info;
use std::env::consts;
use std::path::{Path, PathBuf};

fn run_init(base: &Path) -> Result<()> {
    if let Some(conf) = get_config(base)? {
        for initc in conf.init.unwrap_or_default() {
            if let Some(os) = initc.os {
                if !os.starts_with(consts::OS) {
                    continue;
                }
            }
            info!(
                "{}: {} {}",
                "COMMAND".cyan(),
                initc.command,
                initc.args.join(" ")
            );
            match std::process::Command::new(initc.command)
                .args(initc.args)
                .output()
            {
                Ok(out) => info!("{}", String::from_utf8(out.stdout)?),
                Err(e) => info!("Error: {:?}", e),
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
    std::fs::remove_file(&file_path)?;
    assert!(!file_path.exists());
    Ok(())
}

pub fn run_inits(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        run_init(&base.join(dir))?
    }
    Ok(())
}
