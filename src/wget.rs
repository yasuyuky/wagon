pub fn wget(url: &str) -> anyhow::Result<()> {
    let mut base_path = dirs::home_dir().unwrap_or_default();
    base_path.push("src");

    let output = std::process::Command::new("wget")
        .current_dir(&base_path)
        .args(&["-r", &url])
        .output()
        .expect("installed wget command");

    eprintln!("{}: {}", url, String::from_utf8(output.stderr)?);
    Ok(())
}
