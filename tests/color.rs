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
