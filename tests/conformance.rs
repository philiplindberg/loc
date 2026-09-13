// The conformance check from SPEC.md: the binary's JSON on fixtures/cases must equal the expected files byte for byte, and its report on each file alone must equal the entry in expected-files.json.

use std::fs;
use std::path::{Path, PathBuf};
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

// Every file under cases/, as a path relative to it, sorted by bytes; symbolic links are left out.
fn fixture_files(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read fixtures").flatten() {
        let kind = entry.file_type().expect("file type");
        let rel = rel.join(entry.file_name());
        if kind.is_symlink() {
            continue;
        } else if kind.is_dir() {
            fixture_files(&entry.path(), &rel, out);
        } else {
            out.push(rel);
        }
    }
    out.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
}

// The value after "key": in loc's JSON, up to the next , or }, with quotes stripped.
fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let start = json.find(&format!("\"{key}\":")).expect(key) + key.len() + 3;
    let rest = &json[start..];
    let end = rest.find([',', '}', ']']).unwrap_or(rest.len());
    rest[..end].trim_matches('"')
}

// The expected-files.json entry for one file, derived from loc's report on that file alone.
fn entry(rel: &Path, json: &str) -> String {
    let path = rel.to_string_lossy();
    if json.starts_with("{\"languages\":[]") {
        let kind = if json.contains("\"text\":{\"files\":1") {
            "text"
        } else {
            "binary"
        };
        let labels = &json[json.find(&format!("\"{kind}\":")).unwrap()..];
        format!(
            "{{\"path\":\"{path}\",\"skipped\":\"{kind}\",\"label\":\"{}\"}}",
            field(labels, "label")
        )
    } else {
        format!(
            "{{\"path\":\"{path}\",\"language\":\"{}\",\"lines\":{},\"blank\":{},\"comment\":{},\"code\":{}}}",
            field(json, "name"),
            field(json, "lines"),
            field(json, "blank"),
            field(json, "comment"),
            field(json, "code")
        )
    }
}

#[test]
fn each_file_matches_expected_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cases = root.join("fixtures/cases");
    let mut files = Vec::new();
    fixture_files(&cases, Path::new(""), &mut files);
    let mut lines = Vec::new();
    for rel in &files {
        let out = Command::new(env!("CARGO_BIN_EXE_loc"))
            .arg("--json")
            .arg(cases.join(rel))
            .output()
            .expect("run loc");
        assert!(
            out.status.success(),
            "loc exited {} on {}",
            out.status,
            rel.display()
        );
        lines.push(entry(rel, &String::from_utf8_lossy(&out.stdout)));
    }
    let actual = format!("[\n{}\n]\n", lines.join(",\n"));
    let want = fs::read_to_string(root.join("fixtures/expected-files.json"))
        .expect("read expected-files.json");
    if actual != want {
        let first = actual
            .lines()
            .zip(want.lines())
            .find(|(a, w)| a != w)
            .map_or_else(
                || "a different number of files".to_string(),
                |(a, w)| format!("expected {w}\n  actual {a}"),
            );
        panic!("fixtures/expected-files.json differs; first difference:\n  {first}");
    }
}
