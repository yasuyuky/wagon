use crate::config::GlobalConfig;
use std::path::PathBuf;

pub fn load_repo(path: &str) -> anyhow::Result<()> {
    let (site, path) = if let Some(site_path) = path.strip_prefix("https://") {
        let (site, path) = site_path.split_once('/').unwrap_or_default();
        let path = path.trim_end_matches(".git");
        (site, path)
    } else if path.contains(':') {
        let (pat, path) = path.split_once(':').unwrap_or_default();
        let site = match pat {
            "gh" => "github.com",
            "gl" => "gitlab.com",
            "bb" => "bitbucket.org",
            _ => panic!("Unknown site or protocol: {pat}"),
        };
        (site, path)
    } else {
        ("github.com", path)
    };

    let src_base = GlobalConfig::new().src;

    let mut repo_path = dirs::home_dir().unwrap_or_default();
    repo_path.push(src_base);
    repo_path.push(PathBuf::from(site));
    repo_path.push(PathBuf::from(path));

    if repo_path.exists() {
        eprintln!("Repository already exists.");
    } else {
        let url = format!("https://{site}/{path}.git");
        let output = std::process::Command::new("git")
            .args(["clone", &url, repo_path.to_str().unwrap_or_default()])
            .output()?;
        eprintln!("{}: {}", url, String::from_utf8(output.stderr)?);
    }
    println!("{}", repo_path.display());
    Ok(())
}
