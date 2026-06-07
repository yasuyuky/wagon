use std::fs;
use std::process::Command;

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn color_output_keeps_escape_sequences() {
    let output = Command::new(env!("CARGO_BIN_EXE_wagon"))
        .args(["--color", "ls", "test/repo/bash"])
        .output()
        .expect("run wagon");

    assert!(output.status.success(), "command failed: {output:?}");
    let output = [output.stdout, output.stderr].concat();
    assert!(contains(&output, b"\x1b[1mba"));
    assert!(!contains(&output, br"\x1b"));
}

#[test]
fn color_output_sanitizes_repo_paths() {
    let base = std::env::temp_dir().join(format!("wagon-color-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create temp repo");
    fs::write(base.join("bad\x1b]2;owned\x07"), "").expect("write temp file");

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
    assert!(contains(&output, br"bad\x1b]2;owned\x07"));
    assert!(!contains(&output, b"\x1b]2;owned"));
}
