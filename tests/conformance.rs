// The conformance check from SPEC.md: the binary's JSON on fixtures/cases must equal the expected files byte for byte.

use std::path::Path;
use std::process::Command;

fn check(flags: &[&str], expected: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_loc"))
        .args(flags)
        .arg(root.join("fixtures/cases"))
        .output()
        .expect("run loc");
    assert!(
        out.status.success(),
        "loc exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let want = std::fs::read(root.join("fixtures").join(expected)).expect("read expected file");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&want),
        "loc {} fixtures/cases differs from fixtures/{expected}; a stray file such as .DS_Store inside fixtures/cases changes the skipped counts",
        flags.join(" ")
    );
}

#[test]
fn json_matches_expected() {
    check(&["--json"], "expected.json");
}

#[test]
fn json_no_ignore_matches_expected() {
    check(&["--json", "--no-ignore"], "expected-no-ignore.json");
}
