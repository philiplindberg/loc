# loc — specification

Counts lines of source code per language across a directory tree. The program must agree with this file, byte for byte on `fixtures/cases/`; agreement with cloc, tokei, or scc is not a goal.

## Command line

```
usage: loc [--json] [--no-color] [--no-ignore] [--jobs N] [PATH]

Count lines of code per language under PATH (default: the current directory).

  --json        print the report as JSON
  --no-color    plain output even on a terminal
  --no-ignore   count files that .gitignore excludes
  --jobs N      threads that read and count files (default: all cores)
  -h, --help    show this help
```

`-h` or `--help` anywhere on the command line prints exactly that to stdout and exits `0`; a usage error prints its first line to stderr. `PATH` defaults to `.`; more than one is a usage error. A path that is a file is counted as that file; a directory is walked recursively. Symbolic links are neither followed nor counted anywhere. Hidden files and directories are included. An entry named `.git`, file or directory, is never counted, entered, or reported, with or without `--no-ignore`. `.gitignore` files are applied as described under Ignore rules unless `--no-ignore` is given. `--jobs` sets how many threads read and count files; the default is the machine's available parallelism. It bounds the counting threads, not the process: the walk may run beside them, and a runtime's own threads are outside it. A flag's value is given as `--jobs=N` or `--jobs N`. Neither the value nor visit order affects output: every number is a sum and the table is sorted.

Exit codes: `0` on success, `1` for a usage error (unknown flag, `--jobs` without a positive integer, more than one `PATH`), `2` if any `PATH` does not exist or cannot be read. A file inside a walk that cannot be read is skipped under the label `unreadable` with a warning on stderr and does not change the exit code; a directory that cannot be listed is skipped with a warning and counts nothing. The report goes to stdout; errors go to stderr.

## Output

Default output is a table with one row per language that had at least one recognized file, sorted by code descending, ties broken by language name ascending compared byte-wise (so `JSON` precedes `JavaScript`), framed by rules, then the total row, a blank line, and a breakdown of skipped files:

```
────────────────────────────────────────────────────────────────────────────
Language        Files      Lines      Blank    Comment       Code          %
────────────────────────────────────────────────────────────────────────────
TypeScript         12      3,410        402        611      2,397       78.5
Go                  3        812         97         58        657       21.5
────────────────────────────────────────────────────────────────────────────
Total              15      4,222        499        669      3,054      100.0
────────────────────────────────────────────────────────────────────────────

7 of 22 files skipped (31.8%)
──────────────────────────────
Skipped       Files          %
──────────────────────────────
.types            3       13.6
.symbols          2        9.1
(none)            1        4.5
binary            1        4.5
──────────────────────────────
```

Both tables share one layout. Columns are separated by four spaces. The first column is left-aligned and as wide as its header or its widest value, whichever is longer; every other column is right-aligned and as wide as its header, its widest value, or seven, whichever is longest. Numbers use `,` as a thousands separator. Rules are U+2500 `─` repeated to the width of that table's header line in columns: above and below the header, and above and below the total row. The total row is always present.

Shares are to one decimal, computed in integers: `tenths = (part × 1000 + whole ÷ 2) ÷ whole` with floor division, printed as `tenths ÷ 10`, `.`, `tenths mod 10`; `0.0` when `whole` is 0. The language table's `%` is each language's share of total code; the total row shows `100.0`, or `0.0` when there is no code. Shares need not sum to 100.0. They are derived, so none appear in the JSON.

Skipped files are grouped by label: the lower-cased extension with its dot; `(none)` for a file whose name has no `.` after its first character; `binary` for a recognized file rejected by the `NUL` check, whatever its extension; `unreadable` for a file that could not be opened. Labels sort by file count descending, then label ascending compared byte-wise. The section opens with `S of A files skipped (P%)`, where `A` is recognized plus skipped files and `P` is the skipped share of `A`. With at least one label it continues with a second table, columns `Skipped`, `Files`, `%`: at most ten labels, and with more than ten a final row labeled `N more labels` carrying the file count of the labels not shown; `%` is the row's share of `A`. With nothing skipped the section is the opening line alone. Output ends with a newline.

**Styling.** When stdout is a terminal, `NO_COLOR` is unset or empty, `TERM` is not `dumb`, and `--no-color` was not given, the output is styled; otherwise it is exactly the plain text above. Styled, the first column of the language table carries a two-cell prefix: on a language row, `●` (U+25CF) in the language's color and a space; on the header and total row, two spaces. Widths and rules include the prefix, so the styled table is two columns wider than the plain one. The header line and total row are wrapped in `ESC[1m` … `ESC[0m` (bold); the four rules and every line of the skipped section are wrapped in `ESC[2m` … `ESC[0m` (dim). A color is the 24-bit foreground sequence `ESC[38;2;R;G;Bm`, closed by `ESC[0m`; the values are in the language table.

When total code is non-zero, a language bar precedes the top rule on its own line, as wide as the rule: one segment per language with code, in table order, each a run of spaces on the language's color as background (`ESC[48;2;R;G;Bm` … `ESC[0m`), touching. Every segment gets one cell; with `spare` the width minus the number of segments, each then gets `floor(code × spare ÷ total_code)` more, and the cells still unfilled go one per segment in descending order of `(code × spare) mod total_code`, ties in table order. There is no bar when the segments outnumber the cells.

`--json` emits the same data and nothing else, one object, keys in this order:

```json
{"languages":[{"name":"TypeScript","files":12,"lines":3410,"blank":402,"comment":611,"code":2397}],"total":{"files":15,"lines":4222,"blank":499,"comment":669,"code":3054},"skipped":{"files":7,"labels":[{"label":".types","files":3},{"label":".symbols","files":2},{"label":"(none)","files":1},{"label":"binary","files":1}]}}
```

JSON numbers are plain; JSON carries every skipped label, not just ten. No whitespace outside strings, then a single newline. Languages sorted as in the table; `languages` is `[]` when nothing was recognized. This is the conformance format: `loc --json fixtures/cases` must equal `fixtures/expected.json` exactly.

## Files

A file is counted if its extension, compared case-insensitively, is in the language table. Any other file is skipped under its extension label. A recognized file whose first 8192 bytes contain a `NUL` byte is binary: skipped under the label `binary`.

Files are read as bytes. A leading UTF-8 byte-order mark (`EF BB BF`) is dropped. No other decoding happens; invalid UTF-8 is counted like any other bytes.

A line is a run of bytes terminated by `\n`. A `\r` immediately before the `\n` is part of the terminator. A final run with no `\n` is a line if it is non-empty. An empty file has zero lines and is still counted as a file.

Whitespace, for the purpose of the blank rule, is exactly these bytes: space, tab, `\r`, form feed (`0x0C`), and vertical tab (`0x0B`). Not the standard library's notion of whitespace — a non-breaking space or any other multi-byte sequence is content.

## Ignore rules

`.gitignore` files are applied as git applies them, within the subset below, unless `--no-ignore` is given. An ignored file is not counted and not reported: it is absent from the table, from `skipped`, and from the skipped section's file total. An ignored directory is not entered, so nothing beneath it is counted, reported, or re-included by a later `!` line. `PATH` itself is never ignored.

**Which files apply.** A `.gitignore` in a walked directory applies to that directory and everything beneath it. When an ancestor of `PATH` (or `PATH` itself) contains an entry named `.git`, the `.gitignore` files in the nearest such ancestor and in every directory between it and `PATH` apply the same way, so `loc src` inside a repository ignores what `loc .` would. Nothing above the nearest `.git` is read. `.git/info/exclude`, global excludes, `.ignore`, and tool-specific ignore files are not read. Rules apply to every file on disk; whether git tracks a file is not consulted.

**Precedence.** A path's status is decided by the last matching line of the deepest `.gitignore` that has a matching line: a deeper file overrides a shallower one, and within a file a later line overrides an earlier one. A `!` line re-includes a path that an earlier line or a shallower file excluded. A path inside an excluded directory is never reached, so no `!` line can re-include it.

**Lines.** The file is split on `\n`; a `\r` before the `\n` is dropped. A blank line, or one whose first byte is `#`, has no effect. Trailing spaces are removed unless the last is preceded by `\`. A leading `!` negates the line. `\` escapes the byte after it everywhere else, so `\#`, `\!`, `\*`, and `\ ` are those bytes literally. A line that is empty after processing has no effect.

**Anchoring.** A line ending in `/` matches only directories; that `/` is then dropped before the rules below. A line with a `/` at its start or in its middle is anchored: a leading `/` is dropped and the rest is matched against the path relative to the `.gitignore`'s directory. Any other line is matched against the path's last component alone, at any depth. `**/` at the start matches in every directory, `/**` at the end matches everything inside the named directory, `/**/` in the middle matches zero or more directories; any other `**` is `*`.

**Wildcards.** Matching is byte-wise and case-sensitive. `*` matches zero or more bytes other than `/`; `?` matches one byte other than `/`; `[…]` matches one byte other than `/` from the set, where `a-z` is a range, a leading `!` or `^` negates the set, and `\` escapes.

## Line classification

Every line is exactly one of blank, comment, or code, so `blank + comment + code = lines`. Rules apply in order; the first that matches wins:

1. Any byte of the line lies inside a string literal → **code**. This covers blank lines inside multi-line strings, comment markers inside strings, and Python docstrings.
2. The line is whitespace-only → **blank**. A whitespace-only line inside a block comment is blank, not comment.
3. Every non-whitespace byte of the line lies inside a comment → **comment**. This includes a line that only opens or closes a block comment, and a line that is entirely inside one.
4. Otherwise → **code**. A line with code and a trailing comment is code.

Markdown has no comment or string syntax, so its lines are blank or code.

## Scanner

Classification needs one pass over the file's bytes tracking three states: inside a line comment, inside a block comment (with a depth for languages that nest), or inside a string of a given kind. Inside a comment, string delimiters are text. Inside a string, comment markers are text. Where two openers share a prefix (`"` and `"""`, `/` and `//` and `/*`, `r"` and `r#"`), the longest match wins. Outside a string or comment, a backslash escapes the byte after it and is code; it never extends past the newline.

**Strings.** Each string kind has an opening sequence, a closing sequence, whether backslash escapes the next byte, and whether it may span lines. An unterminated single-line string ends at the newline and the scanner returns to normal state; a backslash immediately before the newline does not extend it. This bounds the damage of a stray quote to one line.

**Template literals** (TypeScript, JavaScript). Inside a template literal, `${` enters code until the matching `}`, counting nested braces; the template resumes after it. Code inside an interpolation may open strings, comments, regex literals, and further template literals, each tracked the same way. `\${` is text.

**Char literals** (Go, Rust, Zig) are single-line string kinds delimited by `'` with escapes. In Go and Zig every `'` outside a string or comment opens one. In Rust, `'` also introduces lifetimes and labels (`'a`, `'outer:`), which have no closing quote, so `'` opens a char literal only if the next byte is `\`, or the byte after the next is `'`; otherwise it is not a delimiter.

**TypeScript and JavaScript regex literals** are single-line string kinds delimited by `/`. A `/` whose next byte is not `/` or `*` opens one in three cases: there is no previous non-whitespace byte on the line; the previous non-whitespace byte is one of `( , = : [ ! & | ? { } ; + - * % < > ~ ^`; or the previous non-whitespace byte ends a word — read back over letters, digits, `_`, and `$` — and that whole word is one of `return typeof instanceof in of new delete void throw case do else yield await`. It closes at the next `/` that is not escaped and not inside a `[...]` class. This is a heuristic; the fixtures pin the cases it must get right.

**Zig line strings.** A line whose first non-whitespace bytes are `\\` is a string line from there to the newline. No closing delimiter; each line stands alone.

**Python string prefixes** (`r`, `b`, `f`, `u`, and their combinations, any case) immediately before a quote are part of the string opener and change nothing about classification. Backslash escapes apply in all Python string kinds, including raw strings, because `r"\""` is a valid literal containing `\"`.

**Rust raw strings** are `r"…"`, `r#"…"#`, and so on with any number of `#`; the closing sequence must match the count. No escapes. Byte-string prefixes `b` and `br` are part of the opener.

## Language table

| language | extensions | line comment | block comment | nests | strings | color |
|---|---|---|---|---|---|---|
| TypeScript | `.ts` `.tsx` `.mts` `.cts` | `//` | `/* */` | no | `"…"` single, escapes; `'…'` single, escapes; `` `…` `` multi, escapes, `${}` interpolation; regex `/…/` | `#3178c6` |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx` | `//` | `/* */` | no | identical to TypeScript, regex rule included | `#f1e05a` |
| Go | `.go` | `//` | `/* */` | no | `"…"` single, escapes; `` `…` `` multi, no escapes; `'…'` char | `#00add8` |
| Rust | `.rs` | `//` | `/* */` | **yes** | `"…"` **multi**, escapes; `r#*"…"#*` multi, no escapes; `'…'` char with lookahead | `#dea584` |
| Zig | `.zig` | `//` | none | — | `"…"` single, escapes; `'…'` char; `\\` line strings | `#ec915c` |
| Python | `.py` `.pyi` | `#` | none | — | `"…"` `'…'` single, escapes; `"""…"""` `'''…'''` multi, escapes; prefixes | `#3572a5` |
| JSON | `.json` `.jsonc` | `//` | `/* */` | no | `"…"` single, escapes | `#cbcb41` |
| Markdown | `.md` `.markdown` | none | none | — | none | `#519aba` |
| C | `.c` `.h` | `//` | `/* */` | no | `"…"` single, escapes; `'…'` char | `#555555` |
| C++ | `.cpp` `.cc` `.cxx` `.hpp` `.hh` `.hxx` | `//` | `/* */` | no | identical to C | `#f34b7d` |

Colors are GitHub linguist's, except JSON and Markdown, whose linguist colors are unreadable on a dark background and take Seti's.

`///` and `//!` begin with `//` and are comments. A `#!` shebang line is a comment in Python, where `#` opens a comment, and code in TypeScript and JavaScript, where it does not; there is no special case. Rust `"…"` may span lines; the single-line kinds of TypeScript, JavaScript, Go, Zig, and Python may not. Doc comments and docstrings get no special treatment.

## Known limitations

TSX and JSX text is not parsed. An apostrophe or quote in JSX text (`<p>don't</p>`) opens a phantom single-line string that ends at the newline and cannot change a count. A bare backtick in JSX text — a markdown fence inside a prompt template — opens a phantom template literal that runs until the next backtick and does change counts; escaped backticks (`\``) are covered by the backslash rule. Python 3.12 f-strings that reuse the enclosing quote inside `{}`, and multi-byte Rust char literals such as `'é'`, fall through to the basic rules with no effect on counts.

The regex rule sees bytes, not grammar: a regex directly after `)` (`if (x) /re/.test(y)`) is read as division, harmless unless the regex contains `/*` or a backtick, whose phantom then runs past the line. Conversely a property named like a keyword (`obj.return / 2`) is read as a regex; that phantom ends at the newline and cannot change a count.

Ignore matching diverges from git in three places. Rules are applied to tracked files too, so a rule that git ignores for a committed tree, such as the `internal/` line in TypeScript's `.gitignore`, removes that tree from the count; `--no-ignore` restores it. Matching is case-sensitive where git on a case-insensitive filesystem is not. POSIX classes such as `[[:alpha:]]` are not recognized: the bracket is read as a literal set.

A `.h` file is C, as GitHub linguist classifies it, whatever language its neighbors are in. C++ raw string literals (`R"(…)"`) are not parsed: the scanner sees an ordinary `"` string, single-line, so a quote inside one opens a phantom that ends at the newline. A filename that is not valid UTF-8 is matched by its bytes.

## Non-goals

git's index, `.git/info/exclude`, global excludes, `.ignore` and tool-specific ignore files, submodules, encoding detection beyond the `NUL` check, per-file output, COCOMO estimates, complexity metrics, languages beyond the ten above.

## Conformance

`fixtures/cases/` holds one small file per rule above plus the edge cases: empty file, no trailing newline, CRLF line endings, a leading BOM, whitespace-only lines, a binary file with a recognized extension, an unrecognized extension, an upper-case extension, a whitespace-only line inside a block comment, a hidden file, a hidden directory, a subdirectory, a symlink. `cases/ignore/` holds a `.gitignore` with lines covering the rules above and a nested one that overrides it, with files each line excludes and files it must not. `fixtures/expected.json` is the exact output of `loc --json fixtures/cases` and `fixtures/expected-no-ignore.json` of `loc --json --no-ignore fixtures/cases`; both sit outside `cases/` so they are not counted. The program is correct when its output is byte-identical to both. To find which file a failing run disagrees on, count `cases/` one subdirectory at a time. The expected files are the authority: a disagreement is a bug in the program until an expected file is changed on purpose. `cases/edge/link` is a symlink to `cases/edge/sub/` and must be neither followed nor counted. A `.DS_Store` inside `cases/` adds a `(none)` label and changes `skipped`; delete it before checking. Files under `cases/ignore/` that its own `.gitignore` matches are tracked only because they were added with `git add -f`; a new one needs the same. The repository's root `.gitignore` applies to the conformance run because `fixtures/` sits inside the repository, so nothing in it may match a fixture. Styled output is not pinned by fixtures either; check it under a pseudo-terminal that the bar is as wide as the rule, every language row starts with a dot, and piped output is unchanged. Three ignore rules are not pinned and are checked by hand: the `.git` rule, since git cannot store a `.git` entry; the reading of ignore files above `PATH`, since the repository root is above `cases/`; and a `\`-escaped trailing space, since a filename ending in a space is a trap for other tools.
