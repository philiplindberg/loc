# loc

Counts lines of code per language across a directory tree. Honors `.gitignore`, skips binaries and symlinks, prints a table on a terminal or JSON with `--json`. The rules are in `SPEC.md`; `fixtures/` pins them.

| Language | Files |
|---|---|
| C | `.c` `.h` |
| C++ | `.cpp` `.cc` `.cxx` `.hpp` `.hh` `.hxx` |
| CoffeeScript | `.coffee`, `#!` `coffee` |
| CSS | `.css` |
| Dart | `.dart` |
| Go | `.go` |
| HTML | `.html` `.htm` |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx`, `#!` `node` `nodejs` |
| JSON | `.json` `.jsonc` |
| Markdown | `.md` `.markdown` |
| Python | `.py` `.pyi`, `#!` `python` `python2` `python3` |
| Rip | `.rip` |
| Ruby | `.rb` `.rake` `.gemspec`, `Gemfile`, `Rakefile`, `#!` `ruby` |
| Rust | `.rs` |
| Shell | `.sh` `.bash` `.zsh` `.ksh`, `.bashrc` and other rc files, `#!` `sh` `bash` `zsh` `ksh` `dash` |
| SQL | `.sql` |
| Svelte | `.svelte` |
| TOML | `.toml` |
| TypeScript | `.ts` `.tsx` `.mts` `.cts` |
| Vue | `.vue` |
| YAML | `.yml` `.yaml` |
| Zig | `.zig` |

A `#!` entry means an extensionless file whose first line names that interpreter is counted as that language.

```
cargo build --release
ln -s "$PWD/target/release/loc" ~/.local/bin/loc
loc PATH
```

`loc` is a link to the release build, so every `cargo build --release` updates it and a debug build does not.

`loc --json fixtures/cases` must equal `fixtures/expected.json` byte for byte, and `loc --json --no-ignore fixtures/cases` must equal `fixtures/expected-no-ignore.json`. `cargo test` checks both.

Standard library only. Unix only: file names are handled as bytes.
