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
    assert_eq!(code(&run(&["a", "b"], &s.0)), 1);
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
