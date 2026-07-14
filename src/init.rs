use crate::{
    config::get_config,
    structs::{sanitize_display, sanitize_output},
};
use anyhow::Result;
use colored::Colorize;
use std::env::consts;
use std::path::{Path, PathBuf};

fn run_init(base: &Path) -> Result<()> {
    if let Some(conf) = get_config(base)? {
        for initc in conf.init.unwrap_or_default() {
            if let Some(os) = initc.os
                && !os.starts_with(consts::OS)
            {
                continue;
            }
            eprintln!(
                "{}: {} {}",
                "COMMAND".cyan(),
                sanitize_display(&initc.command),
                sanitize_display(&initc.args.join(" "))
            );
            match std::process::Command::new(initc.command)
                .args(initc.args)
                .output()
            {
                Ok(out) => eprintln!("{}", sanitize_output(&String::from_utf8(out.stdout)?)),
                Err(e) => eprintln!("Error: {e:?}"),
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

pub fn run_inits(dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        run_init(dir)?
    }
    Ok(())
}
