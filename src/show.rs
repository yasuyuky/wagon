use crate::{Content, Link, list::list_items};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\x1b' => out.push_str("\\x1b"),
            '\x07' => out.push_str("\\x07"),
            '\x08' => out.push_str("\\x08"),
            '\x0c' => out.push_str("\\x0c"),
            '\x7f' => out.push_str("\\x7f"),
            ch if ('\u{80}'..='\u{9f}').contains(&ch) => {
                out.push_str(&format!("\\u{{{:x}}}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out
}

fn safe_path(path: &Path) -> String {
    sanitize(&path.to_string_lossy())
}

fn safe_link(link: &Link) -> String {
    format!("{} -> {}", safe_path(&link.target), safe_path(&link.source))
}

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
            let line = sanitize(line.trim_end());
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

fn show_link(base: &Path) -> Result<String> {
    let mut vs = vec![];
    for link in list_items(base, false)? {
        if link.target.exists() {
            if let Ok(readlink) = fs::read_link(&link.target) {
                if readlink == link.source {
                    vs.push(format!("{}: {}", "LINKING".cyan(), safe_link(&link)))
                }
            } else {
                vs.push(format!(
                    "{}: {}",
                    "EXISTS".magenta(),
                    safe_path(&link.target)
                ));
                if !link.is_dir {
                    vs.push(show_content_diff(&link)?)
                }
            }
        } else {
            vs.push(format!("{}: {}", "NOLINK".yellow(), safe_link(&link)))
        }
    }
    Ok(vs.join("\n"))
}

pub fn show_list(dirs: &[PathBuf]) -> Result<()> {
    for dir in dirs {
        if fs::metadata(dir)?.is_dir() {
            if let Some(name) = dir.file_name() {
                eprintln!("{}", sanitize(&name.to_string_lossy()).bold());
            }
            eprintln!("{}", show_link(dir)?)
        }
    }
    Ok(())
}
