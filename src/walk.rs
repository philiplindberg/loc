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

// Sums kept by one worker: per-language totals by language index, and skipped files by label.
pub struct Sums {
    pub langs: Vec<Counts>,
    pub files: Vec<usize>,
    pub skipped: HashMap<String, usize>,
}

impl Sums {
    fn new() -> Sums {
        Sums {
            langs: vec![Counts::default(); LANGS.len()],
            files: vec![0; LANGS.len()],
            skipped: HashMap::new(),
        }
    }

    fn add(&mut self, other: &Sums) {
        for i in 0..LANGS.len() {
            self.files[i] += other.files[i];
            self.langs[i] += other.langs[i];
        }
        for (label, files) in &other.skipped {
            *self.skipped.entry(label.clone()).or_insert(0) += files;
        }
    }

    fn count(&mut self, path: &Path, reader: &mut Reader) {
        let name = path.file_name().map_or(&b""[..], |n| n.as_bytes());
        let Some(li) = extension(name).and_then(by_ext) else {
            *self.skipped.entry(extension_label(name)).or_insert(0) += 1;
            return;
        };
        let buf = match reader.read(path) {
            Ok(buf) => buf,
            Err(err) => {
                eprintln!("loc: cannot read {}: {}", path.display(), err);
                *self.skipped.entry("unreadable".to_string()).or_insert(0) += 1;
                return;
            }
        };
        if buf[..buf.len().min(NUL_WINDOW)].contains(&0) {
            *self.skipped.entry("binary".to_string()).or_insert(0) += 1;
            return;
        }
        self.files[li] += 1;
        self.langs[li] += scan(buf, &LANGS[li]);
    }
}

const NUL_WINDOW: usize = 8192; // a recognized file is binary if a NUL byte appears this early

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
}

// The bytes from the last '.' in a name, when that dot is not the first byte.
fn extension(name: &[u8]) -> Option<&[u8]> {
    name.iter()
        .rposition(|&b| b == b'.')
        .filter(|&dot| dot > 0)
        .map(|dot| &name[dot..])
}

fn extension_label(name: &[u8]) -> String {
    match extension(name) {
        Some(ext) => String::from_utf8_lossy(ext).to_lowercase(),
        None => "(none)".to_string(),
    }
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
