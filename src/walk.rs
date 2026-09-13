// The walk and the pipeline. A walker thread streams paths into a channel as it finds them; workers read and count as they arrive; each worker's sums merge at the end. Walk order and worker count never affect output: every number is a sum.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, sync_channel};
use std::sync::{Condvar, Mutex};
use std::thread;

use crate::ignore::{IgnoreFile, ancestors, ignored, offset_below, read_ignore};
use crate::lang::{LANGS, by_ext};
use crate::scan::{Counts, scan};

// Sums kept by one worker: per-language totals by language index, and skipped files by label, text and binary apart.
pub struct Sums {
    pub langs: Vec<Counts>,
    pub files: Vec<usize>,
    pub bytes: Vec<u64>,
    pub text: HashMap<String, Skipped>,
    pub binary: HashMap<String, Skipped>,
}

#[derive(Clone, Copy, Default)]
pub struct Skipped {
    pub files: usize,
    pub bytes: u64,
}

impl Sums {
    fn new() -> Sums {
        Sums {
            langs: vec![Counts::default(); LANGS.len()],
            files: vec![0; LANGS.len()],
            bytes: vec![0; LANGS.len()],
            text: HashMap::new(),
            binary: HashMap::new(),
        }
    }

    fn add(&mut self, other: &Sums) {
        for i in 0..LANGS.len() {
            self.files[i] += other.files[i];
            self.bytes[i] += other.bytes[i];
            self.langs[i] += other.langs[i];
        }
        for (label, skipped) in &other.text {
            add(&mut self.text, label.clone(), skipped.files, skipped.bytes);
        }
        for (label, skipped) in &other.binary {
            add(
                &mut self.binary,
                label.clone(),
                skipped.files,
                skipped.bytes,
            );
        }
    }

    fn count(&mut self, path: &Path, reader: &mut Reader) {
        let name = path.file_name().map_or(&b""[..], |n| n.as_bytes());
        let Some(li) = extension(name).and_then(by_ext) else {
            match reader.head(path) {
                Ok((head, size)) => {
                    let group = if head.contains(&0) {
                        &mut self.binary
                    } else {
                        &mut self.text
                    };
                    add(group, label(name), 1, size);
                }
                Err(err) => self.unreadable(path, &err),
            }
            return;
        };
        let buf = match reader.read(path) {
            Ok(buf) => buf,
            Err(err) => return self.unreadable(path, &err),
        };
        if buf[..buf.len().min(NUL_WINDOW)].contains(&0) {
            add(&mut self.binary, label(name), 1, buf.len() as u64);
            return;
        }
        self.files[li] += 1;
        self.bytes[li] += buf.len() as u64;
        self.langs[li] += scan(buf, &LANGS[li]);
    }

    fn unreadable(&mut self, path: &Path, err: &io::Error) {
        eprintln!("loc: cannot read {}: {}", path.display(), err);
        add(
            &mut self.text,
            "unreadable".to_string(),
            1,
            size_on_disk(path),
        );
    }
}

const NUL_WINDOW: usize = 8192; // a file is binary if a NUL byte appears this early

fn add(group: &mut HashMap<String, Skipped>, label: String, files: usize, bytes: u64) {
    let entry = group.entry(label).or_default();
    entry.files += files;
    entry.bytes += bytes;
}

fn walk(root: &Path, no_ignore: bool, emit: &mut impl FnMut(PathBuf)) {
    if !fs::metadata(root).is_ok_and(|st| st.is_dir()) {
        return emit(root.to_path_buf());
    }
    let (mut stack, rel) = if no_ignore {
        (Vec::new(), Vec::new())
    } else {
        ancestors(root)
    };
    walk_dir(root, &rel, &mut stack, no_ignore, emit);
}

// rel is the directory's path relative to the base the ignore files are measured from.
fn walk_dir(
    dir: &Path,
    rel: &[u8],
    stack: &mut Vec<IgnoreFile>,
    no_ignore: bool,
    emit: &mut impl FnMut(PathBuf),
) {
    let mut pushed = false;
    if !no_ignore && let Some(file) = read_ignore(dir, offset_below(rel)) {
        stack.push(file);
        pushed = true;
    }
    match fs::read_dir(dir) {
        Ok(entries) => {
            let mut prefix = rel.to_vec();
            if !prefix.is_empty() {
                prefix.push(b'/');
            }
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if name == ".git" || kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
                    continue;
                }
                let mut child_rel = prefix.clone();
                child_rel.extend_from_slice(name.as_bytes());
                if !no_ignore && ignored(stack, &child_rel, name.as_bytes(), kind.is_dir()) {
                    continue;
                }
                let path = entry.path();
                if kind.is_dir() {
                    walk_dir(&path, &child_rel, stack, no_ignore, emit);
                } else {
                    emit(path);
                }
            }
        }
        Err(err) => eprintln!("loc: cannot read {}: {}", dir.display(), err),
    }
    if pushed {
        stack.pop();
    }
}

// At most READ_CAP workers are inside open, stat, read, and close at once. Opening and reading tens of thousands of small files from every core contends inside the kernel and costs more than the parallelism returns; scanning does not, so the scan runs on every worker while the reads take turns.
const READ_CAP: usize = 6;

struct Semaphore {
    free: Mutex<usize>,
    changed: Condvar,
}

struct Turn<'a>(&'a Semaphore);

impl Semaphore {
    fn acquire(&self) -> Turn<'_> {
        let mut free = self.free.lock().unwrap();
        while *free == 0 {
            free = self.changed.wait(free).unwrap();
        }
        *free -= 1;
        Turn(self)
    }
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().unwrap() += 1;
        self.0.changed.notify_one();
    }
}

// A reader keeps one buffer that grows to the largest file it has read, so a worker allocates only on growth.
struct Reader<'a> {
    buf: Vec<u8>,
    turns: &'a Semaphore,
}

impl Reader<'_> {
    fn read(&mut self, path: &Path) -> io::Result<&[u8]> {
        let _turn = self.turns.acquire();
        let f = File::open(path)?;
        let size = f.metadata()?.len();
        self.buf.clear();
        self.buf.reserve(size as usize);
        f.take(size).read_to_end(&mut self.buf)?;
        Ok(&self.buf)
    }

    // The first NUL_WINDOW bytes, enough for the binary check, and the file's size.
    fn head(&mut self, path: &Path) -> io::Result<(&[u8], u64)> {
        let _turn = self.turns.acquire();
        let f = File::open(path)?;
        let size = f.metadata()?.len();
        self.buf.clear();
        f.take(NUL_WINDOW as u64).read_to_end(&mut self.buf)?;
        Ok((&self.buf, size))
    }
}

// The bytes from the last '.' in a name, when that dot is not the first byte.
fn extension(name: &[u8]) -> Option<&[u8]> {
    name.iter()
        .rposition(|&b| b == b'.')
        .filter(|&dot| dot > 0)
        .map(|dot| &name[dot..])
}

// The skipped label of an unrecognized file: its lower-cased extension, or its name when it has none.
fn label(name: &[u8]) -> String {
    match extension(name) {
        Some(ext) => String::from_utf8_lossy(ext).to_lowercase(),
        None => String::from_utf8_lossy(name).into_owned(),
    }
}

// The size a file without opening it; 0 when even that fails.
fn size_on_disk(path: &Path) -> u64 {
    fs::symlink_metadata(path).map_or(0, |m| m.len())
}

pub fn count_tree(root: &Path, no_ignore: bool, jobs: usize) -> Sums {
    let (tx, rx) = sync_channel::<PathBuf>(1024);
    let rx: Mutex<Receiver<PathBuf>> = Mutex::new(rx);
    let turns = Semaphore {
        free: Mutex::new(READ_CAP),
        changed: Condvar::new(),
    };
    thread::scope(|s| {
        s.spawn(move || walk(root, no_ignore, &mut |path| drop(tx.send(path))));
        let workers: Vec<_> = (0..jobs)
            .map(|_| {
                s.spawn(|| {
                    let mut sums = Sums::new();
                    let mut reader = Reader {
                        buf: Vec::new(),
                        turns: &turns,
                    };
                    loop {
                        let next = rx.lock().unwrap().recv();
                        let Ok(path) = next else { break };
                        sums.count(&path, &mut reader);
                    }
                    sums
                })
            })
            .collect();
        let mut total = Sums::new();
        for worker in workers {
            total.add(&worker.join().unwrap());
        }
        total
    })
}
