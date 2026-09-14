# loc

Counts lines of code across a directory tree in 34 languages. Honors `.gitignore` and `--exclude` patterns, skips binaries and symlinks, and reports what it skipped by size so you can tell whether the count is representative.

`loc -e fixtures .` on this repository:

```
──────────────────────────────────────────────────────────────────────────
Language      Files      Lines      Blank    Comment       Code          %
──────────────────────────────────────────────────────────────────────────
Rust              8      3,640        203         91      3,346       93.0
Markdown          2        340         94          0        246        6.8
TOML              1          4          0          0          4        0.1
──────────────────────────────────────────────────────────────────────────
Total            11      3,984        297         91      3,596      100.0
──────────────────────────────────────────────────────────────────────────

2 of 13 files skipped, 165 B of 165.5 KB of text (0.1%)
───────────────────────────────────────────
Skipped         Files       Size          %
───────────────────────────────────────────
.lock               1      147 B        0.1
.gitignore          1       18 B        0.0
───────────────────────────────────────────
```

The skipped section separates text loc could not count, which is a gap, from binaries, which never had lines. On a terminal the table is colored, with a bar of each language's share above it.

## Install

```
mkdir -p ~/.local/bin
cargo build --release
ln -sf "$PWD/target/release/loc" ~/.local/bin/loc
```

`~/.local/bin` or any directory on your PATH. `loc` is a link to the release build, so every `cargo build --release` updates it and a debug build does not. `cargo install --path .` also works when `~/.cargo/bin` is on your PATH, but it copies the binary, so it needs rerunning after each change.

## Usage

```
loc counts lines of code per language across a directory tree.

Usage: loc [OPTIONS] [PATH...]

Arguments:
  [PATH...]                Files or directories to count (default: the current directory)

Options:
  -e, --exclude PATTERN    Skip what this .gitignore line would, under every PATH; repeatable
  -x, --exclude-lang NAME  Skip every file of a language, named as --languages prints it; repeatable
      --no-ignore          Count files that .gitignore files exclude
  -f, --by-file            Add a row per file under its language
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
```

`--exclude` takes one `.gitignore` line per flag, with the same rules: `vendor` at any depth, `/vendor` only at the root, `vendor/` only directories, `!name` to re-include. It applies before any `.gitignore` file and with `--no-ignore` too. `--exclude-lang` drops a whole language the same way, by name and regardless of case, so `-x c` skips every `.c` and `.h` file. `--jobs N` bounds the counting threads and never changes the output.

## Languages

```
───────────────────────────────────────────────────
Language        Files
───────────────────────────────────────────────────
C               .c .h
C#              .cs
C++             .cpp .cc .cxx .hpp .hh .hxx
CSS             .css
CoffeeScript    .coffee
Dart            .dart
Go              .go
Groovy          .groovy .gvy .gradle
HTML            .html .htm
JSON            .json .jsonc
Java            .java
JavaScript      .js .mjs .cjs .jsx
Kotlin          .kt .kts
Less            .less
Markdown        .md .markdown
Objective-C     .m .mm
PHP             .php .phtml
Python          .py .pyi
Rip             .rip
Ruby            .rb .rake .gemspec Gemfile Rakefile
Rust            .rs
SCSS            .scss
SQL             .sql
Sass            .sass
Scala           .scala .sc .sbt
Shell           .sh .bash .zsh .ksh .bashrc
                .bash_profile .profile .zshrc
Svelte          .svelte
Swift           .swift
TOML            .toml
TypeScript      .ts .tsx .mts .cts
Vue             .vue
XML             .xml .xsd .xsl .xslt
YAML            .yml .yaml
Zig             .zig
```

The table is the output of `loc --languages`. A file is recognized by extension, by whole name (`Gemfile`, `.bashrc`), or, when it has no extension, by the interpreter on its `#!` line. Embedded languages count under the file's own: `<script>` and `<style>` inside HTML, Vue, Svelte, and PHP are scanned by JavaScript's and CSS's rules.

## Rules and tests

`SPEC.md` defines every rule and the exact output; the program must agree with it byte for byte. `fixtures/cases/` holds a small file per rule, and four expected documents pin the program's output on it: `fixtures/expected.json`, `fixtures/expected-no-ignore.json`, `fixtures/expected.txt`, and `fixtures/expected-files.json`, the last the `--by-file --no-ignore` output, so every fixture's own counts are pinned and a failure names the file. `tests/cli.rs` pins what fixtures cannot: exit codes, several paths, `--exclude`, and the ignore rules that git cannot store. `cargo test` runs all of it.

The expected documents are the authority: a disagreement is a bug in the program until an expected document is changed on purpose. To add a language: a row in the spec's language table, an entry in `src/lang.rs`, a fixture per rule, the fixture's counts predicted by hand before running the program, then the expected documents regenerated.

Standard library only. Unix only: file names are handled as bytes.
