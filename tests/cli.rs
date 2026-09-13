// Exit codes and PATH handling from SPEC.md Command line, which the conformance fixtures cannot pin.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_loc"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run loc")
}

// A fresh directory per test, removed on drop; permissions are restored first so removal succeeds.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "loc-cli-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("create scratch dir");
        Scratch(dir)
    }

    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).expect("write file");
        path
    }

    fn unreadable(path: &Path) {
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for entry in fs::read_dir(&self.0).into_iter().flatten().flatten() {
            let mode = if entry.path().is_dir() { 0o755 } else { 0o644 };
            let _ = fs::set_permissions(entry.path(), fs::Permissions::from_mode(mode));
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exit code")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn help_exits_0_and_usage_errors_exit_1() {
    let s = Scratch::new();
    assert_eq!(code(&run(&["--help"], &s.0)), 0);
    assert_eq!(code(&run(&["--json", "-h"], &s.0)), 0);
    assert_eq!(code(&run(&["--bogus"], &s.0)), 1);
    assert_eq!(code(&run(&["--jobs", "0"], &s.0)), 1);
    assert_eq!(code(&run(&["--jobs"], &s.0)), 1);
}

#[test]
fn missing_path_exits_2() {
    let s = Scratch::new();
    assert_eq!(code(&run(&["nope"], &s.0)), 2);
}

#[test]
fn unreadable_file_path_exits_2() {
    let s = Scratch::new();
    let file = s.file("locked.rs", "x\n");
    Scratch::unreadable(&file);
    let out = run(&["locked.rs"], &s.0);
    assert_eq!(
        code(&out),
        2,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn unreadable_directory_path_exits_2() {
    let s = Scratch::new();
    let dir = s.0.join("locked");
    fs::create_dir(&dir).unwrap();
    Scratch::unreadable(&dir);
    let out = run(&["locked"], &s.0);
    assert_eq!(
        code(&out),
        2,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn unreadable_file_inside_a_walk_is_skipped_and_exits_0() {
    let s = Scratch::new();
    s.file("ok.rs", "x\n");
    let locked = s.file("locked.rs", "y\n");
    Scratch::unreadable(&locked);
    let out = run(&["--json", "."], &s.0);
    assert_eq!(code(&out), 0);
    let json = stdout(&out);
    assert!(json.contains("\"name\":\"Rust\",\"files\":1,"), "{json}");
    assert!(
        json.contains("\"label\":\"unreadable\",\"files\":1,"),
        "{json}"
    );
}

#[test]
fn a_symlink_given_as_path_is_followed() {
    let s = Scratch::new();
    s.file("real/a.rs", "x\n");
    symlink("real", s.0.join("link")).unwrap();
    let out = run(&["--json", "link"], &s.0);
    assert_eq!(code(&out), 0);
    assert!(
        stdout(&out).contains("\"name\":\"Rust\",\"files\":1,"),
        "{}",
        stdout(&out)
    );
    s.file("b.rs", "y\n");
    symlink("b.rs", s.0.join("linkfile.rs")).unwrap();
    let out = run(&["--json", "linkfile.rs"], &s.0);
    assert!(
        stdout(&out).contains("\"name\":\"Rust\",\"files\":1,"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_symlink_inside_a_walk_is_neither_followed_nor_counted() {
    let s = Scratch::new();
    s.file("real/a.rs", "x\n");
    symlink("real", s.0.join("link")).unwrap();
    symlink("real/a.rs", s.0.join("link.rs")).unwrap();
    let out = run(&["--json", "."], &s.0);
    assert!(
        stdout(&out).contains("\"name\":\"Rust\",\"files\":1,"),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("\"skipped\":{\"text\":{\"files\":0,"),
        "{}",
        stdout(&out)
    );
}

fn files_of(json: &str, language: &str) -> usize {
    let key = format!("\"name\":\"{language}\",\"files\":");
    json.find(&key).map_or(0, |i| {
        let rest = &json[i + key.len()..];
        rest[..rest.find(',').unwrap()].parse().unwrap()
    })
}

#[test]
fn help_matches_the_spec() {
    let spec = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/SPEC.md")).unwrap();
    let section = &spec[spec.find("## Command line").unwrap()..];
    let start = section.find("```\n").unwrap() + 4;
    let block = &section[start..start + section[start..].find("```").unwrap()];
    let s = Scratch::new();
    assert_eq!(stdout(&run(&["--help"], &s.0)), block);
}

#[test]
fn several_paths_sum_and_a_file_reached_twice_counts_once() {
    let s = Scratch::new();
    s.file("src/main.rs", "x\n");
    s.file("src/lib/b.rs", "y\n");
    s.file("other/c.rs", "z\n");
    let count = |args: &[&str]| files_of(&stdout(&run(args, &s.0)), "Rust");
    assert_eq!(count(&["--json", "src", "other"]), 3);
    assert_eq!(count(&["--json", "src", "src/lib"]), 2);
    assert_eq!(count(&["--json", "src", "src"]), 2);
    assert_eq!(count(&["--json", "src/main.rs", "src/main.rs"]), 1);
    assert_eq!(count(&["--json", "src/main.rs", "src"]), 2);
}

#[test]
fn a_missing_path_among_several_exits_2_with_no_output() {
    let s = Scratch::new();
    s.file("a.rs", "x\n");
    let out = run(&["--json", "a.rs", "nope"], &s.0);
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty());
}

#[test]
fn double_dash_ends_the_flags() {
    let s = Scratch::new();
    s.file("-weird/a.rs", "x\n");
    assert_eq!(code(&run(&["--json", "-weird"], &s.0)), 1);
    assert_eq!(
        files_of(&stdout(&run(&["--json", "--", "-weird"], &s.0)), "Rust"),
        1
    );
}

#[test]
fn exclude_matches_like_a_gitignore_line() {
    let s = Scratch::new();
    s.file("a.rs", "x\n");
    s.file("vendor/v.rs", "x\n");
    s.file("sub/vendor/w.rs", "x\n");
    s.file("d/x.rs", "x\n");
    s.file("other/d", "x\n");
    let json = |args: &[&str]| stdout(&run(args, &s.0));
    assert_eq!(files_of(&json(&["--json", "."]), "Rust"), 4);
    assert_eq!(
        files_of(&json(&["--json", "--exclude", "vendor", "."]), "Rust"),
        2
    );
    assert_eq!(
        files_of(&json(&["--json", "--exclude=/vendor", "."]), "Rust"),
        3
    );
    let dir_only = json(&["--json", "--exclude", "d/", "."]);
    assert_eq!(files_of(&dir_only, "Rust"), 3);
    assert!(
        dir_only.contains("\"label\":\"d\",\"files\":1,"),
        "{dir_only}"
    );
    assert_eq!(
        files_of(
            &json(&["--json", "--exclude", "vendor", "--exclude", "d", "."]),
            "Rust"
        ),
        1
    );
}

#[test]
fn exclude_decides_before_gitignore_and_applies_with_no_ignore() {
    let s = Scratch::new();
    s.file(".gitignore", "*.md\n");
    s.file("keep.md", "x\n");
    s.file("notes.md", "x\n");
    s.file("a.rs", "x\n");
    s.file("vendor/v.rs", "x\n");
    let json = |args: &[&str]| stdout(&run(args, &s.0));
    assert_eq!(files_of(&json(&["--json", "."]), "Markdown"), 0);
    assert_eq!(
        files_of(&json(&["--json", "--exclude", "!keep.md", "."]), "Markdown"),
        1
    );
    let no_ignore = json(&["--json", "--no-ignore", "--exclude", "vendor", "."]);
    assert_eq!(files_of(&no_ignore, "Markdown"), 2);
    assert_eq!(files_of(&no_ignore, "Rust"), 1);
}

#[test]
fn exclude_is_relative_to_each_path_and_never_path_itself() {
    let s = Scratch::new();
    for root in ["a", "b"] {
        s.file(&format!("{root}/vendor/x.rs"), "x\n");
        s.file(&format!("{root}/y.rs"), "x\n");
    }
    let json = |args: &[&str]| stdout(&run(args, &s.0));
    assert_eq!(
        files_of(&json(&["--json", "--exclude", "/vendor", "a", "b"]), "Rust"),
        2
    );
    assert_eq!(
        files_of(
            &json(&["--json", "--exclude", "vendor", "a/vendor"]),
            "Rust"
        ),
        1
    );
}

#[test]
fn exclude_without_a_pattern_is_a_usage_error() {
    let s = Scratch::new();
    assert_eq!(code(&run(&["--exclude"], &s.0)), 1);
    assert_eq!(code(&run(&["--exclude=", "."], &s.0)), 1);
    assert_eq!(code(&run(&["--exclude=vendor", "."], &s.0)), 0);
}

#[test]
fn languages_lists_the_table() {
    let s = Scratch::new();
    let out = run(&["--json", "--languages", "nope"], &s.0);
    assert_eq!(code(&out), 0);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    let cells = |line: &str| -> Vec<String> {
        line.split("  ")
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .collect()
    };
    assert_eq!(cells(lines[1]), ["Language", "Files"]);
    let body = &lines[3..];
    let firsts: Vec<&str> = body
        .iter()
        .copied()
        .filter(|l| !l.starts_with(' '))
        .collect();
    let readme = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    assert!(
        readme.contains(&format!("in {} languages.", firsts.len())),
        "{} rows",
        firsts.len()
    );
    let names: Vec<&str> = firsts
        .iter()
        .map(|r| r.split_whitespace().next().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    let ruby = firsts
        .iter()
        .find(|r| r.starts_with("Ruby "))
        .expect("Ruby row");
    assert_eq!(cells(ruby), ["Ruby", ".rb .rake .gemspec Gemfile Rakefile"]);
    assert!(
        lines.iter().all(|l| !l.ends_with(' ')),
        "no trailing spaces"
    );
    assert!(!text.contains("PHP code"));
}
