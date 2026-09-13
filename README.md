# loc

Counts lines of code per language across a directory tree: TypeScript, JavaScript, Go, Rust, Zig, Python, JSON, Markdown, C, and C++. Honors `.gitignore`, skips binaries and symlinks, prints a table on a terminal or JSON with `--json`. The rules are in `SPEC.md`; `fixtures/` pins them.

```
cargo build --release
ln -s "$PWD/target/release/loc" ~/.local/bin/loc
loc PATH
```

`loc` is a link to the release build, so every `cargo build --release` updates it and a debug build does not.

`loc --json fixtures/cases` must equal `fixtures/expected.json` byte for byte, and `loc --json --no-ignore fixtures/cases` must equal `fixtures/expected-no-ignore.json`.

Standard library only. Unix only: file names are handled as bytes.
