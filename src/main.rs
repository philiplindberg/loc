// loc counts lines of code per language across a directory tree. Rules: SPEC.md. Conformance: fixtures/.

mod ignore;
mod lang;
mod report;
mod scan;
mod walk;

use report::Report;
use std::env;
use std::ffi::OsString;
use std::io::{self, ErrorKind, IsTerminal, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::exit;

macro_rules! usage {
    () => {
        "Usage: loc [OPTIONS] [PATH...]"
    };
}

const USAGE: &str = usage!();

const HELP: &str = concat!(
    "loc counts lines of code per language across a directory tree.

",
    usage!(),
    "

Arguments:
  [PATH...]                Files or directories to count (default: the current directory)

Options:
  -e, --exclude PATTERN    Skip what this .gitignore line would, under every PATH; repeatable
  -x, --exclude-lang NAME  Skip every file of a language, named as --languages prints it; repeatable
      --no-ignore          Count files that .gitignore files exclude
      --json               Print the report as JSON
      --no-color           Plain output even on a terminal
      --jobs N             Threads that read and count files (default: all cores)
  -l, --languages          List the languages and the files each one claims
  -h, --help               Show this help

Examples:
  loc                      Count the current directory
  loc src tests            Count several paths; a file reached twice counts once
  loc -e '*.min.js' .      Skip what that .gitignore line would
  loc -x c -x c++ .        Skip whole languages
  loc --json . | jq .total.code
"
);

struct Options {
    json: bool,
    no_color: bool,
    no_ignore: bool,
    excludes: Vec<Vec<u8>>,
    excluded_langs: Vec<bool>, // by index into lang::LANGS
    jobs: usize,
    roots: Vec<PathBuf>,
}

enum Exit {
    Help,
    Languages { no_color: bool },
    Usage(String), // the message to print before the usage line; empty means the usage line alone
}

fn parse(args: &[OsString]) -> Result<Options, Exit> {
    let mut opts = Options {
        json: false,
        no_color: false,
        no_ignore: false,
        excludes: Vec::new(),
        excluded_langs: vec![false; lang::LANGS.len()],
        jobs: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        roots: Vec::new(),
    };
    let mut i = 0;
    let mut flags = true; // until --
    // -h or --help anywhere among the flags wins, before any other flag is judged.
    if args
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "-h" || a == "--help")
    {
        return Err(Exit::Help);
    }
    if args
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--languages" || a == "-l")
    {
        return Err(Exit::Languages {
            no_color: args.iter().any(|a| a == "--no-color"),
        });
    }
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        // The value of a flag that takes one: after = or in the next argument.
        let value = |flag: &str, i: &mut usize| -> Option<OsString> {
            if let Some(v) = arg.strip_prefix(flag).and_then(|r| r.strip_prefix('=')) {
                Some(OsString::from(v))
            } else {
                *i += 1;
                args.get(*i).cloned()
            }
        };
        if !flags || !arg.starts_with('-') {
            opts.roots.push(PathBuf::from(&args[i]));
        } else if arg == "--" {
            flags = false;
        } else if arg == "--json" {
            opts.json = true;
        } else if arg == "--no-color" {
            opts.no_color = true;
        } else if arg == "--no-ignore" {
            opts.no_ignore = true;
        } else if arg == "--exclude" || arg == "-e" || arg.starts_with("--exclude=") {
            match value("--exclude", &mut i) {
                Some(pattern) if !pattern.is_empty() => {
                    opts.excludes.push(pattern.as_bytes().to_vec());
                }
                _ => return Err(Exit::Usage("loc: --exclude needs a pattern".to_string())),
            }
        } else if arg == "--exclude-lang" || arg == "-x" || arg.starts_with("--exclude-lang=") {
            let name = value("--exclude-lang", &mut i).unwrap_or_default();
            let name = name.to_string_lossy();
            match lang::by_language(&name) {
                Some(li) => opts.excluded_langs[li] = true,
                None if name.is_empty() => {
                    return Err(Exit::Usage(
                        "loc: --exclude-lang needs a language from --languages".to_string(),
                    ));
                }
                None => return Err(Exit::Usage(format!("loc: unknown language {name}"))),
            }
        } else if arg == "--jobs" || arg.starts_with("--jobs=") {
            let text = value("--jobs", &mut i).unwrap_or_default();
            let text = text.to_string_lossy();
            match text.parse::<usize>() {
                Ok(n) if n >= 1 && !text.starts_with('+') => opts.jobs = n,
                _ => {
                    return Err(Exit::Usage(
                        "loc: --jobs needs a positive integer".to_string(),
                    ));
                }
            }
        } else {
            return Err(Exit::Usage(format!("loc: unknown flag {arg}")));
        }
        i += 1;
    }
    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }
    Ok(opts)
}

fn main() {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    exit(run(&args));
}

fn run(args: &[OsString]) -> i32 {
    let opts = match parse(args) {
        Ok(opts) => opts,
        Err(Exit::Help) => {
            print!("{HELP}");
            return 0;
        }
        Err(Exit::Languages { no_color }) => {
            print!("{}", report::languages(!no_color && terminal()));
            return 0;
        }
        Err(Exit::Usage(message)) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            eprintln!("{USAGE}");
            return 1;
        }
    };
    let mut unreadable = false;
    for root in &opts.roots {
        if let Err(err) = readable(root) {
            eprintln!("loc: {}: {err}", root.display());
            unreadable = true;
        }
    }
    if unreadable {
        return 2;
    }

    let report = Report::new(walk::count_trees(
        &opts.roots,
        opts.no_ignore,
        &opts.excludes,
        &opts.excluded_langs,
        opts.jobs,
    ));
    let out = if opts.json {
        report.json()
    } else {
        report.table(!opts.no_color && terminal())
    };
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => 0,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => 0, // a reader that stops early is a normal end, not a failure
        Err(err) => {
            eprintln!("loc: {err}");
            1
        }
    }
}

// Opens PATH the way the walk will, so a PATH that cannot be read fails here with exit code 2 instead of as a skipped file inside the walk. A symbolic link is followed: PATH names its target.
fn readable(path: &Path) -> io::Result<()> {
    if std::fs::metadata(path)?.is_dir() {
        std::fs::read_dir(path).map(drop)
    } else {
        std::fs::File::open(path).map(drop)
    }
}

fn terminal() -> bool {
    io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && env::var_os("TERM").is_none_or(|t| t != "dumb")
}
