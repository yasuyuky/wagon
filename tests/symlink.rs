use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("wagon-{name}-{}-{now}", std::process::id()))
}

fn write_repo(base: &Path, dest: &Path) {
    fs::create_dir_all(base).expect("create repo");
    fs::create_dir_all(dest).expect("create dest");
    fs::write(base.join(".wagon.toml"), format!("dest = {:?}\n", dest)).expect("write config");
    fs::write(base.join(".bashrc"), "new\n").expect("write source");
}

fn output_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&[output.stdout.as_slice(), output.stderr.as_slice()].concat())
        .into_owned()
}

#[test]
fn link_reports_and_replaces_broken_target_symlink() {
    let root = temp_dir("broken-target");
    let base = root.join("repo");
    let dest = root.join("home");
    write_repo(&base, &dest);
    symlink(root.join("missing"), dest.join(".bashrc")).expect("create broken symlink");

    let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
        .current_dir(&root)
        .args(["--base"])
        .arg(&base)
        .arg("link")
        .output()
        .expect("run wagon");

    assert!(output.status.success(), "command failed: {output:?}");
    assert_eq!(
        fs::read_link(dest.join(".bashrc")).expect("read replaced symlink"),
        base.join(".bashrc")
            .canonicalize()
            .expect("canonical source")
    );
    assert!(
        output_text(&output).contains("ERROR: broken symlink:"),
        "output: {output:?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn link_reports_broken_source_symlink_and_continues() {
    let root = temp_dir("broken-source");
    let base = root.join("repo");
    let dest = root.join("home");
    write_repo(&base, &dest);
    symlink(root.join("missing"), base.join("broken")).expect("create broken symlink");

    let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
        .current_dir(&root)
        .args(["--base"])
        .arg(&base)
        .arg("link")
        .output()
        .expect("run wagon");

    assert!(output.status.success(), "command failed: {output:?}");
    assert_eq!(
        fs::read_link(dest.join(".bashrc")).expect("read linked file"),
        base.join(".bashrc")
            .canonicalize()
            .expect("canonical source")
    );
    assert!(fs::symlink_metadata(dest.join("broken")).is_err());
    assert!(
        output_text(&output).contains("ERROR: broken symlink:"),
        "output: {output:?}"
    );

    let _ = fs::remove_dir_all(root);
}
