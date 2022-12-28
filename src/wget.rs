use crate::config::GlobalConfig;

pub fn wget(url: &str) -> anyhow::Result<()> {
    let base_path = GlobalConfig::new().src;

    let output = std::process::Command::new("wget")
        .current_dir(&base_path)
        .args(["-r", url])
        .output()
        .expect("installed wget command");

    eprintln!("{}: {}", url, String::from_utf8(output.stderr)?);
    Ok(())
}
