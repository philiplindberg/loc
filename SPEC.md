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

`-h` or `--help` anywhere on the command line prints exactly that to stdout and exits `0`; a usage error prints its first line to stderr. `PATH` defaults to `.`; more than one is a usage error. A path that is a file is counted as that file; a directory is walked recursively. `PATH` itself is followed if it is a symbolic link, so a link names its target; a symbolic link met inside a walk is neither followed nor counted. Hidden files and directories are included. An entry named `.git`, file or directory, is never counted, entered, or reported, with or without `--no-ignore`. `.gitignore` files are applied as described under Ignore rules unless `--no-ignore` is given. `--jobs` sets how many threads read and count files; the default is the machine's available parallelism. It bounds the counting threads, not the process: the walk may run beside them, and a runtime's own threads are outside it. A flag's value is given as `--jobs=N` or `--jobs N`. Neither the value nor visit order affects output: every number is a sum and the table is sorted.

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

6 of 21 files skipped, 1.1 MB of 1.3 MB of text (88.7%)
─────────────────────────────────────────
Skipped       Files       Size          %
─────────────────────────────────────────
.types            3     1.1 MB       85.8
.symbols          2    31.3 KB        2.5
Makefile          1     6.4 KB        0.5
─────────────────────────────────────────
1 binary file, 64.0 KB: .png 1
```

Both tables share one layout. Columns are separated by four spaces. The first column is left-aligned and as wide as its header or its widest value, whichever is longer; every other column is right-aligned and as wide as its header, its widest value, or seven, whichever is longest. Numbers use `,` as a thousands separator. Rules are U+2500 `─` repeated to the width of that table's header line in columns: above and below the header, and above and below the total row. The total row is always present.

Shares are to one decimal, computed in integers: `tenths = (part × 1000 + whole ÷ 2) ÷ whole` with floor division, printed as `tenths ÷ 10`, `.`, `tenths mod 10`; `0.0` when `whole` is 0. The language table's `%` is each language's share of total code; the total row shows `100.0`, or `0.0` when there is no code. Shares need not sum to 100.0. They are derived, so none appear in the JSON.

Sizes are bytes shown to one decimal in a unit from `B`, `KB`, `MB`, `GB`, `TB` at powers of 1000. Fewer than 1000 bytes print as the whole number, a space, and `B`. Otherwise, for a unit, `tenths = (bytes × 10 + unit ÷ 2) ÷ unit` with floor division, printed as `tenths ÷ 10`, `.`, `tenths mod 10`, a space, and the unit; the unit is the smallest from `KB` up for which `tenths` is below 10,000, or `TB`. Sizes are also derived, from the byte counts in the JSON.

Skipped files are grouped by label: the lower-cased extension with its dot; the file name, as it is, for a file whose name has no `.` after its first byte; `unreadable` for a file that could not be opened, which counts as text. Each label carries a file count and the files' total size in bytes as the file system reports it; a file whose size cannot be read counts as 0 bytes. Text and binary files are reported apart, because skipped text is a gap in the count and a binary file never had lines to count. Within each, labels sort by size descending, then label ascending compared byte-wise. The section opens with `S of A files skipped, X of Y of text (P%)`, where `S` and `X` are the skipped text files and their size, `A` and `Y` add the recognized files and their size, and `P` is `X`'s share of `Y`. With at least one text label it continues with a second table, columns `Skipped`, `Files`, `Size`, `%`: at most ten labels, and with more than ten a final row labeled `N more labels` carrying the file count and size of the labels not shown; `%` is the row's size as a share of `Y`. With at least one binary file a last line follows: `N binary files, Z: ` (`1 binary file` for one), then each binary label with its file count as `label N`, separated by `, `, at most ten, and with more than ten `, N more`. With no skipped text the section is the opening line, and the binary line if there is one. Output ends with a newline.

**Styling.** When stdout is a terminal, `NO_COLOR` is unset or empty, `TERM` is not `dumb`, and `--no-color` was not given, the output is styled; otherwise it is exactly the plain text above. Styled, the first column of the language table carries a two-cell prefix: on a language row, `●` (U+25CF) in the language's color and a space; on the header and total row, two spaces. Widths and rules include the prefix, so the styled table is two columns wider than the plain one. The header line and total row are wrapped in `ESC[1m` … `ESC[0m` (bold); the four rules and every line of the skipped section are wrapped in `ESC[2m` … `ESC[0m` (dim). A color is the 24-bit foreground sequence `ESC[38;2;R;G;Bm`, closed by `ESC[0m`; the values are in the language table. A terminal that enforces a minimum contrast for text, as VS Code's does by default, lightens a dark dot; the bar is background paint and always shows the exact color.

When total code is non-zero, a language bar precedes the top rule on its own line, as wide as the rule: one segment per language with code, in table order, each a run of spaces on the language's color as background (`ESC[48;2;R;G;Bm` … `ESC[0m`), touching. Every segment gets one cell; with `spare` the width minus the number of segments, each then gets `floor(code × spare ÷ total_code)` more, and the cells still unfilled go one per segment in descending order of `(code × spare) mod total_code`, ties in table order. There is no bar when the segments outnumber the cells.

`--json` emits the same data and nothing else, one object, keys in this order:

```json
{"languages":[{"name":"TypeScript","files":12,"lines":3410,"blank":402,"comment":611,"code":2397,"bytes":118004}],"total":{"files":15,"lines":4222,"blank":499,"comment":669,"code":3054,"bytes":143210},"skipped":{"text":{"files":6,"bytes":1128890,"labels":[{"label":".types","files":3,"bytes":1091220},{"label":".symbols","files":2,"bytes":31250},{"label":"Makefile","files":1,"bytes":6420}]},"binary":{"files":1,"bytes":64010,"labels":[{"label":".png","files":1,"bytes":64010}]}}}
```

JSON numbers are plain; JSON carries every skipped label, text and binary, not just ten. No whitespace outside strings, then a single newline. Languages sorted as in the table; `languages` is `[]` when nothing was recognized. This is the conformance format: `loc --json fixtures/cases` must equal `fixtures/expected.json` exactly.

## Files

A file is recognized by its extension, compared case-insensitively, against the language table; failing that, by its whole name against the names below; failing that, when its name has no `.` after its first byte, by the interpreter of a `#!` first line against the shebangs below. The interpreter is the last `/`-separated component of the line's first word, or, when that is `env`, the first later word that does not start with `-`. A recognized file is counted if its first 8192 bytes contain no `NUL` byte. Any other file is skipped: as binary if its first 8192 bytes contain a `NUL` byte, else as text.

Names: Ruby `Gemfile` `Rakefile`; Shell `.bashrc` `.bash_profile` `.bash_aliases` `.bash_logout` `.profile` `.zshrc` `.zshenv` `.zprofile` `.zlogin` `.zlogout`. Shebangs: JavaScript `node` `nodejs`; Python `python` `python2` `python3`; CoffeeScript `coffee`; PHP `php`; Ruby `ruby`; Shell `sh` `bash` `zsh` `ksh` `dash`.

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

Classification needs one pass over the file's bytes tracking three states: inside a line comment, inside a block comment (with a depth for languages that nest), or inside a string of a given kind. Inside a comment, string delimiters are text. Inside a string, comment markers are text. Where two openers share a prefix (`"` and `"""`, `/` and `//` and `/*`, `r"` and `r#"`, `#` and `###`), the longest match wins. A language may have several line-comment openers (PHP has `//` and `#`); in PHP, `#[` begins an attribute and is code. A block opener that is a run of one byte (`###`) does not match inside a longer run, so `####` opens a line comment. A language may allow a block comment to open only as the first non-whitespace bytes of a line (Rip and CoffeeScript `###`, Ruby `=begin`); elsewhere its opener is read by the other rules, so a mid-line `###` in Rip opens a line comment. Outside a string or comment, a backslash escapes the byte after it and is code; it never extends past the newline.

**Strings.** Each string kind has an opening sequence, a closing sequence, whether backslash escapes the next byte, and whether it may span lines. An unterminated single-line string ends at the newline and the scanner returns to normal state; a backslash immediately before the newline does not extend it. This bounds the damage of a stray quote to one line.

**Interpolation.** Some string kinds interpolate: inside one, an interpolation opener enters code until the matching `}`, counting nested braces; the string resumes after it. The openers are `${` in a TypeScript or JavaScript template literal, in every Dart string, and in the double-quoted kinds of Kotlin, Scala, and Groovy, `{$` in a PHP `"…"` string, `#{` and `${` in a Rip `"…"` string or `"""…"""` heredoc, and `#{` in a Rip heregex and in the CoffeeScript and Ruby kinds marked below. Code inside an interpolation may open strings, comments, regex literals, and further interpolating strings, each tracked the same way. An escaped opener (`\${`, `\#{`) is text.

**Char literals** (Go, Rust, Zig) are single-line string kinds delimited by `'` with escapes. In Go and Zig every `'` outside a string or comment opens one. In Rust, `'` also introduces lifetimes and labels (`'a`, `'outer:`), which have no closing quote, so `'` opens a char literal only if the next byte is `\`, or the byte after the next is `'`; otherwise it is not a delimiter.

**TypeScript, JavaScript, Rip, CoffeeScript, and Ruby regex literals** are single-line string kinds delimited by `/`. A `/` that no comment opener or string kind has claimed opens one in three cases: there is no previous non-whitespace byte on the line; the previous non-whitespace byte is one of `( , = : [ ! & | ? { } ; + - * % < > ~ ^`; or the previous non-whitespace byte ends a word — read back over letters, digits, `_`, and `$` — and that whole word is one of the language's regex words: `return typeof instanceof in of new delete void throw case do else yield await` in TypeScript and JavaScript, those plus `if unless when while until and or not is isnt then` in Rip and CoffeeScript, and `if unless while until and or not then else elsif when case return yield raise` in Ruby. It closes at the next `/` that is not escaped and not inside a `[...]` class. This is a heuristic; the fixtures pin the cases it must get right.

**Zig line strings.** A line whose first non-whitespace bytes are `\\` is a string line from there to the newline. No closing delimiter; each line stands alone.

**Rip and CoffeeScript heregexes** `///…///` are multi-line string kinds with escapes and `#{` interpolation. A `#` comment inside one is string text, so its line is code.

**Heredocs** (Ruby, Shell, PHP). `<<` opens one in Ruby and Shell, unless a third `<` follows; `<<<` opens one in PHP. In Ruby it is followed by an optional `-` or `~`, then the word, optionally in `'`, `"`, or `` ` `` quotes, with no space anywhere; in Shell by an optional `-`, any spaces, then the word, optionally quoted or preceded by `\`; in PHP by any spaces, then the word, optionally in `'` or `"` quotes. The word is a run of letters, digits, and `_`. The rest of the opener's line is scanned as usual; from the next line, every line is string text up to and including the terminator: the first line that equals the word, with leading whitespace allowed before it when the opener had `-` or `~` or the language is PHP, and, in PHP, with anything that is not a letter, digit, or `_` allowed after it, such as `;`. A `<<` that is not followed by a word, as in Ruby's `a << b`, is code.

**Embedded languages** (HTML, Vue, Svelte). Outside a comment, `<script` or `<style`, compared without regard to ASCII case and followed by whitespace, `>`, `/`, or the end of the line, opens a region: the rest of the tag up to its `>` is code, and from there the file is scanned by JavaScript's or CSS's rules until the matching `</script` or `</tag` at a point where those rules are in their normal state, followed by whitespace, `>`, or the end of the line; that closing tag through its `>` is code, and the markup rules resume. A region may instead be delimited by sequences: in PHP, `<?php` or `<?=`, compared without regard to ASCII case, opens one at once, and `?>` closes it. The markup languages have no string syntax of their own, so a quote in text or an attribute is code.

**PHP** files are markup with HTML's regions and the `<?php`/`<?=` regions above, whose bodies are scanned by PHP's code rules: line comments `//` and `#`, `/* */` block comments, `"…"` multi-line with escapes and `{$…}` interpolation, `'…'` multi-line with escapes, `` `…` `` single-line with escapes, and heredocs. Every line of the file counts as PHP.

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
| JSON | `.json` `.jsonc` | `//` | `/* */` | no | `"…"` single, escapes | `#292929` |
| Markdown | `.md` `.markdown` | none | none | — | none | `#083fa1` |
| C | `.c` `.h` | `//` | `/* */` | no | `"…"` single, escapes; `'…'` char | `#555555` |
| C++ | `.cpp` `.cc` `.cxx` `.hpp` `.hh` `.hxx` | `//` | `/* */` | no | identical to C | `#f34b7d` |
| Rip | `.rip` | `#` | `### ###`, at line start only | no | `"…"` single, escapes, `#{}` `${}` interpolation; `'…'` single, escapes; `"""…"""` multi, escapes, `#{}` `${}` interpolation; `'''…'''` multi, escapes; heregex `///…///` multi, escapes, `#{}` interpolation; regex `/…/` | `#ab2317` |
| CSS | `.css` | none | `/* */` | no | `"…"` `'…'` single, escapes | `#663399` |
| YAML | `.yml` `.yaml` | `#` | none | — | `"…"` single, escapes; `'…'` single, no escapes | `#cb171e` |
| TOML | `.toml` | `#` | none | — | `"…"` single, escapes; `'…'` single, no escapes; `"""…"""` multi, escapes; `'''…'''` multi, no escapes | `#9c4221` |
| SQL | `.sql` | `--` | `/* */` | no | `'…'` multi, no escapes; `"…"` single, no escapes | `#e38c00` |
| Dart | `.dart` | `//` | `/* */` | **yes** | `"…"` `'…'` single, escapes; `"""…"""` `'''…'''` multi, escapes; all with `${}` interpolation | `#00b4ab` |
| CoffeeScript | `.coffee` | `#` | `### ###`, at line start only | no | `"…"` single, escapes, `#{}` interpolation; `'…'` single, escapes; `"""…"""` multi, escapes, `#{}` interpolation; `'''…'''` multi, escapes; heregex `///…///` multi, escapes, `#{}` interpolation; `` `…` `` single and ```` ```…``` ```` multi, no escapes; regex `/…/` | `#244776` |
| Ruby | `.rb` `.rake` `.gemspec` | `#` | `=begin =end`, at line start only | no | `"…"` multi, escapes, `#{}` interpolation; `'…'` multi, escapes; `` `…` `` single, escapes; heredocs; regex `/…/` | `#701516` |
| Shell | `.sh` `.bash` `.zsh` `.ksh` | `#` | none | — | `"…"` multi, escapes; `'…'` multi, no escapes; heredocs | `#89e051` |
| HTML | `.html` `.htm` | none | `<!-- -->` | no | none; `<script>` is JavaScript, `<style>` is CSS | `#e34c26` |
| Vue | `.vue` | none | `<!-- -->` | no | identical to HTML | `#41b883` |
| Svelte | `.svelte` | none | `<!-- -->` | no | identical to HTML | `#ff3e00` |
| Java | `.java` | `//` | `/* */` | no | `"…"` single, escapes; `"""…"""` multi, escapes; `'…'` char | `#b07219` |
| Kotlin | `.kt` `.kts` | `//` | `/* */` | **yes** | `"…"` single, escapes, `${}` interpolation; `"""…"""` multi, no escapes, `${}` interpolation; `'…'` char | `#a97bff` |
| Scala | `.scala` `.sc` `.sbt` | `//` | `/* */` | **yes** | `"…"` single, escapes, `${}` interpolation; `"""…"""` multi, no escapes, `${}` interpolation; `'…'` char with lookahead | `#c22d40` |
| Groovy | `.groovy` `.gvy` `.gradle` | `//` | `/* */` | no | `"…"` single, escapes, `${}` interpolation; `'…'` single, escapes; `"""…"""` multi, escapes, `${}` interpolation; `'''…'''` multi, escapes | `#4298b8` |
| Swift | `.swift` | `//` | `/* */` | **yes** | `"…"` single, escapes; `"""…"""` multi, escapes | `#f05138` |
| Objective-C | `.m` `.mm` | `//` | `/* */` | no | identical to C | `#438eff` |
| SCSS | `.scss` | `//` | `/* */` | no | `"…"` `'…'` single, escapes | `#c6538c` |
| Sass | `.sass` | `//` | `/* */` | no | identical to SCSS | `#a53b70` |
| Less | `.less` | `//` | `/* */` | no | identical to SCSS | `#1d365d` |
| XML | `.xml` `.xsd` `.xsl` `.xslt` | none | `<!-- -->` | no | none | `#0060ac` |
| PHP | `.php` `.phtml` | none | `<!-- -->` | no | identical to HTML, plus `<?php`/`<?=` … `?>` regions with the code rules under Scanner | `#4f5d95` |

Colors are GitHub linguist's, except Rip, which linguist does not list.

`///` and `//!` begin with `//` and are comments. A `#!` shebang line is a comment where `#` opens a comment (Python, Rip, CoffeeScript, Ruby, Shell, YAML, TOML), and code in TypeScript and JavaScript, where it does not; there is no special case. A kind marked multi in the table may span lines; the others end at the newline. Doc comments and docstrings get no special treatment.

## Known limitations

TSX and JSX text is not parsed. An apostrophe or quote in JSX text (`<p>don't</p>`) opens a phantom single-line string that ends at the newline and cannot change a count. A bare backtick in JSX text — a markdown fence inside a prompt template — opens a phantom template literal that runs until the next backtick and does change counts; escaped backticks (`\``) are covered by the backslash rule. Python 3.12 f-strings that reuse the enclosing quote inside `{}`, and multi-byte Rust char literals such as `'é'`, fall through to the basic rules with no effect on counts.

The regex rule sees bytes, not grammar: a regex directly after `)` (`if (x) /re/.test(y)`) is read as division, harmless unless the regex contains `/*` or a backtick, whose phantom then runs past the line. Conversely a property named like a keyword (`obj.return / 2`) is read as a regex; that phantom ends at the newline and cannot change a count.

Ignore matching diverges from git in three places. Rules are applied to tracked files too, so a rule that git ignores for a committed tree, such as the `internal/` line in TypeScript's `.gitignore`, removes that tree from the count; `--no-ignore` restores it. Matching is case-sensitive where git on a case-insensitive filesystem is not. POSIX classes such as `[[:alpha:]]` are not recognized: the bracket is read as a literal set.

In a Rip render block, a line whose first non-whitespace bytes are `#` and a letter names an element (`#main`); the scanner reads it as a comment. Word arrays (`%w[…]`) are not parsed: a `#` or quote inside one opens a phantom that ends at the newline. `//` is floor division in Rip, never a comment.

A YAML block scalar (`|` or `>`) has no delimiter, so a `#` line inside one is a comment. Ruby's `%w[…]` and `%q(…)` literals, `__END__` data sections, and a second heredoc opened on the same line are not parsed, and an indented `=begin` opens a block comment where Ruby would not. Shell's backtick substitutions are code, not strings. A `<<` directly followed by a word inside Shell arithmetic (`$((1<<2))`) or a Ruby expression without spaces (`1<<2`) opens a phantom heredoc that runs until a line equal to that word. SQL block comments do not nest as PostgreSQL's do, and MySQL's `#` comments are code. In markup, a `<script` or `<style` inside an attribute value opens a region, and a `</script>` inside a JavaScript string or comment does not close one.

Swift's `\(…)` interpolation is not tracked, so a quote inside one opens a phantom that ends at the newline, and its `#"…"#` raw strings are read as ordinary strings. Scala's `${` is read as interpolation in every double-quoted string, prefixed or not. Groovy's slashy strings (`/…/`) are code. A `.m` file is Objective-C, never MATLAB, and `.mm` counts as Objective-C rather than Objective-C++. An XML `CDATA` section is not parsed, so `<!--` inside one opens a comment. A `<style lang="scss">` block in Vue or Svelte is scanned by CSS's rules, so its `//` comments are code. In PHP, a `?>` inside a `//` or `#` comment does not end the region as it does for PHP itself, `$name` interpolation without braces is not tracked, the short `<?` tag is not recognized, and a `#!` first line lies outside any region and is code.

A `.h` file is C, as GitHub linguist classifies it, whatever language its neighbors are in. C++ raw string literals (`R"(…)"`) are not parsed: the scanner sees an ordinary `"` string, single-line, so a quote inside one opens a phantom that ends at the newline. A filename that is not valid UTF-8 is matched by its bytes.

## Non-goals

git's index, `.git/info/exclude`, global excludes, `.ignore` and tool-specific ignore files, submodules, encoding detection beyond the `NUL` check, per-file output, COCOMO estimates, complexity metrics, languages beyond the table above.

## Conformance

`fixtures/cases/` holds one small file per rule above plus the edge cases: empty file, no trailing newline, CRLF line endings, a leading BOM, whitespace-only lines, a binary file with a recognized extension, an unrecognized extension, an upper-case extension, a whitespace-only line inside a block comment, a hidden file, a hidden directory, a subdirectory, a symlink, an extensionless file with a `#!` line naming an unknown interpreter. `cases/ignore/` holds a `.gitignore` with lines covering the rules above and a nested one that overrides it, with files each line excludes and files it must not. `fixtures/expected.json` is the exact output of `loc --json fixtures/cases` and `fixtures/expected-no-ignore.json` of `loc --json --no-ignore fixtures/cases`; both sit outside `cases/` so they are not counted. `fixtures/expected-files.json` lists every file under `cases/`, sorted by path bytes, with the language and counts `loc --json` reports for that file alone, or the kind and label it is skipped under. The program is correct when its output is byte-identical to the two expected files and every entry of the third; a failing entry names the file that disagrees. The expected files are the authority: a disagreement is a bug in the program until an expected file is changed on purpose. `cases/edge/link` is a symlink to `cases/edge/sub/` and must be neither followed nor counted. A `.DS_Store` inside `cases/` adds a binary file and changes `skipped`; delete it before checking. Files under `cases/ignore/` that its own `.gitignore` matches are tracked only because they were added with `git add -f`; a new one needs the same. The repository's root `.gitignore` applies to the conformance run because `fixtures/` sits inside the repository, so nothing in it may match a fixture. Styled output is not pinned by fixtures either; check it under a pseudo-terminal that the bar is as wide as the rule, every language row starts with a dot, and piped output is unchanged. Three ignore rules are not pinned and are checked by hand: the `.git` rule, since git cannot store a `.git` entry; the reading of ignore files above `PATH`, since the repository root is above `cases/`; and a `\`-escaped trailing space, since a filename ending in a space is a trap for other tools.
