use crate::{
    Content, Link,
    list::list_items,
    structs::{display_path, sanitize_display},
};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn read_text(f: &mut fs::File) -> Result<Content> {
    let mut buf = String::default();
    f.read_to_string(&mut buf)?;
    let ss = buf.lines().map(String::from).collect();
    Ok(Content::Text(ss))
}

fn read_binary(f: &mut fs::File) -> Result<Content> {
    let mut buf = Vec::new();
    let size = f.read_to_end(&mut buf)?;
    Ok(Content::Binary(size, buf))
}

fn read_content(path: &Path) -> Result<(Content, String, String)> {
    let mut f = fs::File::open(path)?;
    let meta = f.metadata()?;
    let date = format!("{}", time::OffsetDateTime::from(meta.modified()?));
    let ps = path.to_str().unwrap_or_default().to_owned();
    Ok((read_text(&mut f).unwrap_or(read_binary(&mut f)?), ps, date))
}

fn get_text_diff(ss: &[String], ts: &[String], sp: &str, tp: &str, sd: &str, td: &str) -> String {
    difflib::unified_diff(ss, ts, sp, tp, sd, td, 3)
        .iter()
        .map(|line| {
            let line = sanitize_display(line.trim_end());
            if line.starts_with('+') {
                format!("{}", line.green())
            } else if line.starts_with('-') {
                format!("{}", line.red())
            } else {
                line
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn check_binary_diff(ssz: usize, sb: Vec<u8>, tsz: usize, tb: Vec<u8>) -> String {
    if sb != tb {
        format!(
            "{} src size:{}, dst size:{}",
            "binary files do not match.".red(),
            ssz,
            tsz
        )
    } else {
        String::default()
    }
}

fn show_content_diff(link: &Link) -> Result<String> {
    let (srcc, sp, srcd) = read_content(&link.source)?;
    let (tgtc, tp, tgtd) = read_content(&link.target)?;
    Ok(match (srcc, tgtc) {
        (Content::Text(ss), Content::Text(ts)) => get_text_diff(&ss, &ts, &sp, &tp, &srcd, &tgtd),
        (Content::Binary(ssz, sb), Content::Binary(tsz, tb)) => check_binary_diff(ssz, sb, tsz, tb),
        _ => "file types do not match".to_owned(),
    })
}

fn show_existing_target(link: &Link) -> Result<Vec<String>> {
    let mut lines = vec![format!(
        "{}: {}",
        "EXISTS".magenta(),
        display_path(&link.target)
    )];
    if !link.is_dir && fs::metadata(&link.target)?.is_file() {
        lines.push(show_content_diff(link)?)
    }
    Ok(lines)
}

fn show_link(base: &Path) -> Result<String> {
    let mut vs = vec![];
    for link in list_items(base, false)? {
        if let Ok(readlink) = fs::read_link(&link.target) {
            if readlink == link.source {
                vs.push(format!("{}: {}", "LINKING".cyan(), &link))
            } else if link.target.exists() {
                vs.extend(show_existing_target(&link)?)
            } else {
                vs.push(format!(
                    "{} broken symlink: {} -> {}",
                    "ERROR:".red(),
                    display_path(&link.target),
                    display_path(&readlink)
                ));
                vs.push(format!("{}: {}", "NOLINK".yellow(), &link))
            }
        } else if link.target.exists() {
            vs.extend(show_existing_target(&link)?)
        } else {
            vs.push(format!("{}: {}", "NOLINK".yellow(), &link))
        }
    }
    Ok(vs.join("\n"))
}

pub fn show_list(dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        if fs::metadata(dir)?.is_dir() {
            if let Some(name) = dir.file_name() {
                // Keep ls as direct CLI output so fixed labels can use ANSI
                // without disabling tracing sanitization globally.
                eprintln!("{}", sanitize_display(&name.to_string_lossy()).bold());
            }
            eprintln!("{}", show_link(dir)?)
        }
    }
    Ok(())
}
