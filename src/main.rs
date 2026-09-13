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
use std::path::PathBuf;
use std::process::exit;

macro_rules! usage {
    () => {
        "usage: loc [--json] [--no-color] [--no-ignore] [--jobs N] [PATH]"
    };
}

const USAGE: &str = usage!();

const HELP: &str = concat!(
    usage!(),
    "

Count lines of code per language under PATH (default: the current directory).

  --json        print the report as JSON
  --no-color    plain output even on a terminal
  --no-ignore   count files that .gitignore excludes
  --jobs N      threads that read and count files (default: all cores)
  -h, --help    show this help
"
);

struct Options {
    json: bool,
    no_color: bool,
    no_ignore: bool,
    jobs: usize,
    root: PathBuf,
}

enum Exit {
    Help,
    Usage(String), // the message to print before the usage line; empty means the usage line alone
}

fn parse(args: &[OsString]) -> Result<Options, Exit> {
    let mut opts = Options {
        json: false,
        no_color: false,
        no_ignore: false,
        jobs: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        root: PathBuf::from("."),
    };
    if args.iter().any(|a| a == "-h" || a == "--help") {
        // Anywhere on the line, before any other flag is judged.
        return Err(Exit::Help);
    }
    let mut paths: Vec<&OsString> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        if arg == "--json" {
            opts.json = true;
        } else if arg == "--no-color" {
            opts.no_color = true;
        } else if arg == "--no-ignore" {
            opts.no_ignore = true;
        } else if arg == "--jobs" || arg.starts_with("--jobs=") {
            let value = if let Some(v) = arg.strip_prefix("--jobs=") {
                v.to_string()
            } else {
                i += 1;
                args.get(i)
                    .map(|v| v.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            match value.parse::<usize>() {
                Ok(n) if n >= 1 && !value.starts_with('+') => opts.jobs = n,
                _ => {
                    return Err(Exit::Usage(
                        "loc: --jobs needs a positive integer".to_string(),
                    ));
                }
            }
        } else if arg.starts_with('-') {
            return Err(Exit::Usage(format!("loc: unknown flag {arg}")));
        } else {
            paths.push(&args[i]);
        }
        i += 1;
    }
    if paths.len() > 1 {
        return Err(Exit::Usage(String::new()));
    }
    if let Some(path) = paths.first() {
        opts.root = PathBuf::from(path);
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
        Err(Exit::Usage(message)) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            eprintln!("{USAGE}");
            return 1;
        }
    };
    if std::fs::symlink_metadata(&opts.root).is_err() {
        eprintln!("loc: {}: no such file or directory", opts.root.display());
        return 2;
    }

    let report = Report::new(walk::count_tree(&opts.root, opts.no_ignore, opts.jobs));
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

fn terminal() -> bool {
    io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && env::var_os("TERM").is_none_or(|t| t != "dumb")
}
