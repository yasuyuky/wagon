use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn color_output_keeps_escape_sequences() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/repo/bash");
    let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
        .args(["--color", "ls"])
        .arg(base)
        .output()
        .expect("run wagon");

    assert!(output.status.success(), "command failed: {output:?}");
    let output = [output.stdout, output.stderr].concat();
    assert!(contains(&output, b"\x1b["));
    assert!(!contains(&output, br"\x1b"));
}

#[test]
fn link_and_copy_outputs_keep_escape_sequences() {
    let root =
        std::env::temp_dir().join(format!("wagon-command-color-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    for command in ["link", "copy"] {
        let base = root.join(command).join("repo");
        let dest = root.join(command).join("home");
        fs::create_dir_all(&base).expect("create temp repo");
        fs::create_dir_all(&dest).expect("create temp destination");
        fs::write(base.join(".wagon.toml"), format!("dest = {dest:?}\n")).expect("write config");
        fs::write(base.join("file"), "content").expect("write source file");

        let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
            .args(["--color", "--base"])
            .arg(&base)
            .arg(command)
            .output()
            .expect("run wagon");

        assert!(output.status.success(), "command failed: {output:?}");
        assert!(contains(&output.stderr, b"\x1b["), "output: {output:?}");
        assert!(!contains(&output.stderr, br"\x1b"), "output: {output:?}");
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn color_output_sanitizes_repo_paths() {
    let base = std::env::temp_dir().join(format!("wagon-color-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create temp repo");
    fs::write(base.join("bad\x1b]2;owned\x07\nrow\rcol\tend"), "").expect("write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
        .args(["--color", "--base"])
        .arg(&base)
        .arg("ls")
        .output()
        .expect("run wagon");
    let _ = fs::remove_dir_all(&base);

    assert!(output.status.success(), "command failed: {output:?}");
    let output = [output.stdout, output.stderr].concat();
    assert!(contains(&output, b"\x1b["));
    assert!(contains(&output, br"bad\x1b]2;owned\x07\nrow\rcol\tend"));
    assert!(!contains(&output, b"\x1b]2;owned"));
    assert!(!contains(&output, b"\rcol\tend"));
}
