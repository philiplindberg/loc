// The conformance check from SPEC.md: the binary's output on fixtures/cases must equal the four expected files byte for byte.

use std::fs;
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
fn table_matches_expected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_loc"))
        .arg(root.join("fixtures/cases"))
        .output()
        .expect("run loc");
    assert!(out.status.success());
    let want = fs::read_to_string(root.join("fixtures/expected.txt")).expect("read expected.txt");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        want,
        "loc fixtures/cases differs from fixtures/expected.txt"
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

#[test]
fn by_file_json_matches_expected_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_loc"))
        .args(["--json", "--by-file", "--no-ignore", "fixtures/cases"])
        .current_dir(root) // paths in the output are as given, so they must not depend on where the checkout lives
        .output()
        .expect("run loc");
    assert!(out.status.success());
    let actual = String::from_utf8_lossy(&out.stdout).into_owned();
    let want = fs::read_to_string(root.join("fixtures/expected-files.json"))
        .expect("read expected-files.json");
    if actual != want {
        // Entries are compared one file at a time so the failure names the file.
        let entries = |json: &str| -> Vec<String> {
            json.split("{\"path\":")
                .skip(1)
                .map(|e| e.split('}').next().unwrap().to_string())
                .collect()
        };
        let (a, w) = (entries(&actual), entries(&want));
        let first = a.iter().zip(&w).find(|(a, w)| a != w).map_or_else(
            || {
                format!(
                    "{} files reported, {} expected, or the totals differ",
                    a.len(),
                    w.len()
                )
            },
            |(a, w)| format!("expected {w}\n  actual {a}"),
        );
        panic!("fixtures/expected-files.json differs; first difference:\n  {first}");
    }
}
