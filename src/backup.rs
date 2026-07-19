use anyhow::Result;
use libc::getuid;
use std::fs;
use std::path::{Path, PathBuf};

pub fn backup(backupdir: &Path, path: &Path) -> Result<()> {
    let mut components = path.components();
    components.next();
    let backup = backupdir.join(components.as_path());
    fs::create_dir_all(backup.parent().unwrap_or(backupdir)).expect("create backup dir");
    Ok(fs::rename(path, backup)?)
}

#[test]
fn test_backup() -> Result<()> {
    let backupdir = PathBuf::from("test/backup");
    let path = PathBuf::from("test/repo/bash/.bashrc");
    backup(&backupdir, &path)?;
    // roll back
    let mut components = path.components();
    components.next();
    let backedup = backupdir.join(components.as_path());
    assert!(backedup.exists());
    fs::rename(&backedup, &path)?;
    assert!(path.exists());
    Ok(())
}

pub fn get_backuppath() -> Result<PathBuf> {
    let mut backupdir = PathBuf::from(".backups");
    backupdir.push(format!("uid{}", unsafe { getuid() }));
    let local = time::OffsetDateTime::now_local()?;
    backupdir.push(format_backuptime(local)?);
    Ok(backupdir)
}

fn format_backuptime(datetime: time::OffsetDateTime) -> Result<String> {
    let format = time::format_description::parse_borrowed::<2>(
        "[year]/[month]/[day]/[hour]:[minute]:[second]",
    )?;
    Ok(datetime.format(&format)?)
}

#[test]
fn formats_backup_time() -> Result<()> {
    let datetime = time::Date::from_calendar_date(2026, time::Month::July, 19)?
        .with_hms(12, 34, 56)?
        .assume_utc();

    assert_eq!(format_backuptime(datetime)?, "2026/07/19/12:34:56");
    Ok(())
}
