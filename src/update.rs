use crate::config::get_config;
use anyhow::Result;
use std::env::consts;
use std::path::{Path, PathBuf};

fn run_update(base: &Path) -> Result<()> {
    if let Some(conf) = get_config(base)? {
        for updatec in conf.update.unwrap_or_default() {
            if let Some(os) = updatec.os {
                if !os.starts_with(consts::OS) {
                    continue;
                }
            }
            match std::process::Command::new(updatec.command)
                .args(updatec.args)
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
fn test_run_update() -> Result<()> {
    let test_base = PathBuf::from("test/repo/bash");
    run_update(&test_base)?;
    let file_path = PathBuf::from("testtouch");
    assert!(file_path.exists());
    std::fs::remove_file(&file_path)?;
    assert!(!file_path.exists());
    Ok(())
}

pub fn run_updates(base: &Path, dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        run_update(&base.join(dir))?
    }
    Ok(())
}
