// .gitignore matching, SPEC.md Ignore rules. A pattern is compiled once into a small program of steps; matching is a backtracking walk over the path's bytes, which is what git's fnmatch does without a regex engine.

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

enum Step {
    Byte(u8),                                      // one literal byte
    Any,                                           // * : zero or more bytes other than /
    One,                                           // ? : one byte other than /
    Class { set: Box<[bool; 256]>, inside: bool }, // […] : one byte other than /, in the set (true) or outside it (false)
    AnyDirs, // a leading **/ or a middle /**/ : nothing, or any run of bytes ending in /
    Rest,    // a trailing /** : anything at all
}

struct Pattern {
    negate: bool,
    dir_only: bool,
    anchored: bool,
    steps: Vec<Step>,
}

// An ignore file and how far below the base directory it sits, as a byte offset into the relative path of the entries it applies to: rel[offset..] is an entry's path relative to the file's own directory.
pub struct IgnoreFile {
    offset: usize,
    patterns: Vec<Pattern>,
}

fn compile_pattern(mut line: &[u8]) -> Option<Pattern> {
    let negate = line[0] == b'!';
    if negate {
        line = &line[1..];
    }
    let dir_only = line.last() == Some(&b'/');
    if dir_only {
        line = &line[..line.len() - 1];
    }
    let anchored = line.contains(&b'/');
    if anchored && line.first() == Some(&b'/') {
        line = &line[1..];
    }
    if line.is_empty() {
        return None;
    }
    Some(Pattern {
        negate,
        dir_only,
        anchored,
        steps: compile_glob(line)?,
    })
}

// None when a [ has no closing ]: git's matcher gives up on such a line, so it matches nothing.
fn compile_glob(glob: &[u8]) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    let mut i = 0;
    if glob.starts_with(b"**/") {
        steps.push(Step::AnyDirs);
        i = 3;
    }
    while i < glob.len() {
        let c = glob[i];
        let after_slash = i > 0 && glob[i - 1] == b'/';
        if c == b'*' && after_slash && glob[i..].starts_with(b"**/") {
            steps.push(Step::AnyDirs);
            i += 3;
        } else if c == b'*' && after_slash && &glob[i..] == b"**" {
            steps.push(Step::Rest);
            i += 2;
        } else if c == b'*' {
            steps.push(Step::Any);
            i += 1;
            if i < glob.len() && glob[i] == b'*' {
                // any other ** is *
                i += 1;
            }
        } else if c == b'?' {
            steps.push(Step::One);
            i += 1;
        } else if c == b'\\' && i + 1 < glob.len() {
            steps.push(Step::Byte(glob[i + 1]));
            i += 2;
        } else if c == b'[' {
            let (step, n) = compile_class(&glob[i..])?;
            steps.push(step);
            i += n;
        } else {
            steps.push(Step::Byte(c));
            i += 1;
        }
    }
    Some(steps)
}

// Reads a bracket set starting at glob[0] == '[' and returns it with the number of bytes consumed.
fn compile_class(glob: &[u8]) -> Option<(Step, usize)> {
    let mut set = [false; 256];
    let mut inside = true;
    let mut j = 1;
    if j < glob.len() && (glob[j] == b'!' || glob[j] == b'^') {
        inside = false;
        j += 1;
    }
    let mut member = false; // whether the previous byte can start a range
    while j < glob.len() {
        let c = glob[j];
        if c == b']' {
            return Some((
                Step::Class {
                    set: Box::new(set),
                    inside,
                },
                j + 1,
            ));
        } else if c == b'\\' && j + 1 < glob.len() {
            set[glob[j + 1] as usize] = true;
            member = true;
            j += 2;
        } else if c == b'-' && member && j + 1 < glob.len() && glob[j + 1] != b']' {
            for b in glob[j - 1]..=glob[j + 1] {
                set[b as usize] = true;
            }
            member = false;
            j += 2;
        } else {
            set[c as usize] = true;
            member = true;
            j += 1;
        }
    }
    None
}

// Whether steps match subject to the end of both.
fn matches(steps: &[Step], subject: &[u8]) -> bool {
    let Some(step) = steps.first() else {
        return subject.is_empty();
    };
    let rest = &steps[1..];
    match step {
        Step::Byte(b) => subject.first() == Some(b) && matches(rest, &subject[1..]),
        Step::One => subject.first().is_some_and(|&c| c != b'/') && matches(rest, &subject[1..]),
        Step::Class { set, inside } => {
            subject
                .first()
                .is_some_and(|&c| c != b'/' && set[c as usize] == *inside)
                && matches(rest, &subject[1..])
        }
        Step::Any => {
            for k in 0..=subject.len() {
                if matches(rest, &subject[k..]) {
                    return true;
                }
                if k == subject.len() || subject[k] == b'/' {
                    return false;
                }
            }
            false
        }
        Step::AnyDirs => {
            matches(rest, subject)
                || (0..subject.len())
                    .any(|k| subject[k] == b'/' && matches(rest, &subject[k + 1..]))
        }
        Step::Rest => true,
    }
}

fn parse_ignore(text: &[u8], offset: usize) -> IgnoreFile {
    let mut patterns = Vec::new();
    for line in text.split(|&b| b == b'\n') {
        let mut line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        while line.last() == Some(&b' ') && !(line.len() > 1 && line[line.len() - 2] == b'\\') {
            line = &line[..line.len() - 1];
        }
        if let Some(pattern) = compile_pattern(line) {
            patterns.push(pattern);
        }
    }
    IgnoreFile { offset, patterns }
}

pub fn read_ignore(dir: &Path, offset: usize) -> Option<IgnoreFile> {
    fs::read(dir.join(".gitignore"))
        .ok()
        .map(|text| parse_ignore(&text, offset))
}

// --exclude patterns as one ignore file, each pattern a line, measured from the directory at offset.
pub fn exclude_file(patterns: &[Vec<u8>], offset: usize) -> IgnoreFile {
    parse_ignore(&patterns.join(&b'\n'), offset)
}

// The status a file's lines give a path: that of the last matching line, or None when no line matches.
fn decides(file: &IgnoreFile, rel: &[u8], name: &[u8], is_dir: bool) -> Option<bool> {
    file.patterns.iter().rev().find_map(|pattern| {
        if pattern.dir_only && !is_dir {
            return None;
        }
        let subject = if pattern.anchored {
            &rel[file.offset..]
        } else {
            name
        };
        matches(&pattern.steps, subject).then_some(!pattern.negate)
    })
}

// The --exclude patterns decide first; then the deepest file with a matching line, and within a file the last matching line.
pub fn ignored(
    excludes: Option<&IgnoreFile>,
    stack: &[IgnoreFile],
    rel: &[u8],
    name: &[u8],
    is_dir: bool,
) -> bool {
    excludes
        .and_then(|file| decides(file, rel, name, is_dir))
        .or_else(|| {
            stack
                .iter()
                .rev()
                .find_map(|file| decides(file, rel, name, is_dir))
        })
        .unwrap_or(false)
}

fn has_git(dir: &Path) -> bool {
    fs::symlink_metadata(dir.join(".git")).is_ok()
}

// Removes . and .. components lexically, without touching the filesystem.
fn clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            component => out.push(component.as_os_str()),
        }
    }
    out
}

// The ignore files between the nearest ancestor holding .git and root, and root's path relative to that ancestor. Both are empty when root holds .git itself or no ancestor does.
pub fn ancestors(root: &Path) -> (Vec<IgnoreFile>, Vec<u8>) {
    let Ok(abs) = std::path::absolute(root).map(|path| clean(&path)) else {
        return (Vec::new(), Vec::new());
    };
    if has_git(&abs) {
        return (Vec::new(), Vec::new());
    }
    let mut chain = Vec::new(); // parents of abs, nearest first, ending with the one holding .git
    let mut dir = abs.as_path();
    loop {
        let Some(parent) = dir.parent() else {
            return (Vec::new(), Vec::new());
        };
        chain.push(parent);
        if has_git(parent) {
            break;
        }
        dir = parent;
    }
    let base = chain[chain.len() - 1];
    let rel_of = |path: &Path| {
        path.strip_prefix(base)
            .unwrap_or(Path::new(""))
            .as_os_str()
            .as_bytes()
            .to_vec()
    };
    let rel = rel_of(&abs);
    let mut stack = Vec::new();
    for dir in chain.iter().rev() {
        // base first, then each directory down to root's parent
        if let Some(file) = read_ignore(dir, offset_below(&rel_of(dir))) {
            stack.push(file);
        }
    }
    (stack, rel)
}

// Where an entry's path relative to the base continues after the directory at dir_rel.
pub fn offset_below(dir_rel: &[u8]) -> usize {
    if dir_rel.is_empty() || dir_rel == b"." {
        0
    } else {
        dir_rel.len() + 1
    }
}
