// The language table from SPEC.md and the byte tables derived from it.

use std::collections::HashMap;
use std::sync::LazyLock;

// One way a language writes a string literal; char literals and Rust raw strings are kinds too.
pub struct StrKind {
    pub open: &'static [u8],
    pub close: &'static [u8],
    pub escape: bool,
    pub multiline: bool,
    pub interp: &'static [&'static [u8]], // openers of code inside the string, each closed by its matching }
    pub stop: [bool; 256], // bytes that end the fast skip inside the string: the closer's first byte, backslash, each interpolation opener's first byte
}

const fn kind(open: &'static [u8], close: &'static [u8], escape: bool, multiline: bool) -> StrKind {
    interpolating(open, close, escape, multiline, &[])
}

const fn interpolating(
    open: &'static [u8],
    close: &'static [u8],
    escape: bool,
    multiline: bool,
    interp: &'static [&'static [u8]],
) -> StrKind {
    let mut stop = [false; 256];
    stop[close[0] as usize] = true;
    stop[b'\\' as usize] = true;
    let mut i = 0;
    while i < interp.len() {
        stop[interp[i][0] as usize] = true;
        i += 1;
    }
    StrKind {
        open,
        close,
        escape,
        multiline,
        interp,
        stop,
    }
}

const DOLLAR_INTERP: &[&[u8]] = &[b"${"];
const HASH_INTERP: &[&[u8]] = &[b"#{"];
const RIP_INTERP: &[&[u8]] = &[b"#{", b"${"];

const DQ: StrKind = kind(b"\"", b"\"", true, false);
const SQ: StrKind = kind(b"'", b"'", true, false);
const TICK: StrKind = interpolating(b"`", b"`", true, true, DOLLAR_INTERP);
pub static CHAR: StrKind = kind(b"'", b"'", true, false);

// Words after which a / opens a regex literal.
const JS_REGEX_WORDS: &[&[u8]] = &[
    b"return",
    b"typeof",
    b"instanceof",
    b"in",
    b"of",
    b"new",
    b"delete",
    b"void",
    b"throw",
    b"case",
    b"do",
    b"else",
    b"yield",
    b"await",
];
const RIP_REGEX_WORDS: &[&[u8]] = &[
    b"return",
    b"typeof",
    b"instanceof",
    b"in",
    b"of",
    b"new",
    b"delete",
    b"void",
    b"throw",
    b"case",
    b"do",
    b"else",
    b"yield",
    b"await",
    b"if",
    b"unless",
    b"when",
    b"while",
    b"until",
    b"and",
    b"or",
    b"not",
    b"is",
    b"isnt",
    b"then",
];
const RUBY_REGEX_WORDS: &[&[u8]] = &[
    b"if", b"unless", b"while", b"until", b"and", b"or", b"not", b"then", b"else", b"elsif",
    b"when", b"case", b"return", b"yield", b"raise",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Chars {
    None,
    Always, // every ' outside a string or comment opens a char literal
    Rust,   // ' opens one only before \ or when the byte after next is ': lifetimes have no closer
}

// How a language spells a heredoc opener after <<.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Heredoc {
    None,
    Shell, // optional -, then spaces, then a word that may be quoted or start with \
    Ruby,  // optional - or ~, then a word that may be quoted, with no space anywhere
    Php, // a third <, spaces, then a word that may be quoted; the terminator may be indented and followed by ; or ,
}

// An embedded language: the body between the delimiters is scanned as lang.
pub enum Region {
    Tag {
        name: &'static [u8], // <name …> opens after its >, </name> closes
        lang: &'static str,
    },
    Between {
        open: &'static [u8], // compared without regard to ASCII case
        close: &'static [u8],
        lang: &'static str,
    },
}

impl Region {
    pub fn lang(&self) -> &'static str {
        match self {
            Region::Tag { lang, .. } | Region::Between { lang, .. } => lang,
        }
    }
}

// A language as the scanner sees it. Empty slices, None, and false mean the syntax is absent.
pub struct Lang {
    pub name: &'static str,
    pub extensions: &'static [&'static str], // lower-case, dot included
    pub names: &'static [&'static str],      // whole file names recognized without an extension
    pub shebangs: &'static [&'static str], // interpreters whose #! line recognizes an extensionless file
    pub line: &'static [&'static [u8]], // line-comment openers; none shares a prefix with another
    pub hash_attribute: bool,           // `#[` begins an attribute, not a comment (PHP)
    pub block_open: &'static [u8],      // block-comment opener; empty when the language has none
    pub block_close: &'static [u8],
    pub nests: bool, // block comments nest (Rust); false ends one at the first closer (Go)
    pub block_at_line_start: bool, // a block comment opens only as the first non-whitespace bytes of a line (Rip)
    pub strings: Vec<StrKind>,
    pub chars: Chars,
    pub regex: Option<&'static [&'static [u8]]>, // /…/ literals by the previous-byte-or-word rule, with the words
    pub zig_line_string: bool,                   // \\ to end of line
    pub rust_raw: bool,                          // r#*"…"#*, optional b prefix
    pub heredoc: Heredoc,
    pub regions: &'static [Region],
    pub region_langs: Vec<usize>, // the language index of each region, from prepare
    pub color: [u8; 3],
    pub starts: [bool; 256], // bytes that can begin a token in normal state; any other byte is plain code
}

impl Lang {
    // Whether --languages lists it: an entry no file can reach, such as an embedded-only language, is not a name the user can give.
    pub fn listed(&self) -> bool {
        !self.extensions.is_empty() || !self.names.is_empty() || !self.shebangs.is_empty()
    }
}

const NONE: Lang = Lang {
    name: "",
    extensions: &[],
    names: &[],
    shebangs: &[],
    line: &[],
    hash_attribute: false,
    block_open: b"",
    block_close: b"",
    nests: false,
    block_at_line_start: false,
    strings: Vec::new(),
    chars: Chars::None,
    regex: None,
    zig_line_string: false,
    rust_raw: false,
    heredoc: Heredoc::None,
    regions: &[],
    region_langs: Vec::new(),
    color: [0; 3],
    starts: [false; 256],
};

fn c_family(name: &'static str, extensions: &'static [&'static str], color: [u8; 3]) -> Lang {
    Lang {
        name,
        extensions,
        line: &[b"//"],
        block_open: b"/*",
        block_close: b"*/",
        strings: vec![DQ],
        chars: Chars::Always,
        color,
        ..NONE
    }
}

fn js_family(name: &'static str, extensions: &'static [&'static str], color: [u8; 3]) -> Lang {
    Lang {
        name,
        extensions,
        line: &[b"//"],
        block_open: b"/*",
        block_close: b"*/",
        strings: vec![TICK, DQ, SQ],
        regex: Some(JS_REGEX_WORDS),
        color,
        ..NONE
    }
}

// Markup with <!-- --> comments, JavaScript inside <script> and CSS inside <style>, and no string syntax of its own.
fn markup(name: &'static str, extensions: &'static [&'static str], color: [u8; 3]) -> Lang {
    Lang {
        name,
        extensions,
        block_open: b"<!--",
        block_close: b"-->",
        regions: &[
            Region::Tag {
                name: b"script",
                lang: "JavaScript",
            },
            Region::Tag {
                name: b"style",
                lang: "CSS",
            },
        ],
        color,
        ..NONE
    }
}

// Colors are GitHub linguist's, except Rip, which linguist does not list.
pub static LANGS: LazyLock<Vec<Lang>> = LazyLock::new(|| {
    prepare(vec![
        js_family(
            "TypeScript",
            &[".ts", ".tsx", ".mts", ".cts"],
            [0x31, 0x78, 0xc6],
        ),
        Lang {
            shebangs: &["node", "nodejs"],
            ..js_family(
                "JavaScript",
                &[".js", ".mjs", ".cjs", ".jsx"],
                [0xf1, 0xe0, 0x5a],
            )
        },
        Lang {
            name: "Go",
            extensions: &[".go"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![kind(b"`", b"`", false, true), DQ],
            chars: Chars::Always,
            color: [0x00, 0xad, 0xd8],
            ..NONE
        },
        Lang {
            name: "Rust",
            extensions: &[".rs"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            nests: true,
            strings: vec![kind(b"\"", b"\"", true, true)],
            chars: Chars::Rust,
            rust_raw: true,
            color: [0xde, 0xa5, 0x84],
            ..NONE
        },
        Lang {
            name: "Zig",
            extensions: &[".zig"],
            line: &[b"//"],
            strings: vec![DQ],
            chars: Chars::Always,
            zig_line_string: true,
            color: [0xec, 0x91, 0x5c],
            ..NONE
        },
        Lang {
            name: "Python",
            extensions: &[".py", ".pyi"],
            shebangs: &["python", "python2", "python3"],
            line: &[b"#"],
            strings: vec![
                kind(b"\"\"\"", b"\"\"\"", true, true),
                kind(b"'''", b"'''", true, true),
                DQ,
                SQ,
            ],
            color: [0x35, 0x72, 0xa5],
            ..NONE
        },
        Lang {
            name: "JSON",
            extensions: &[".json", ".jsonc"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ],
            color: [0x29, 0x29, 0x29],
            ..NONE
        },
        Lang {
            name: "Markdown",
            extensions: &[".md", ".markdown"],
            color: [0x08, 0x3f, 0xa1],
            ..NONE
        },
        c_family("C", &[".c", ".h"], [0x55, 0x55, 0x55]),
        c_family(
            "C++",
            &[".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx"],
            [0xf3, 0x4b, 0x7d],
        ),
        Lang {
            name: "Rip",
            extensions: &[".rip"],
            line: &[b"#"],
            block_open: b"###",
            block_close: b"###",
            block_at_line_start: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", true, true, RIP_INTERP),
                kind(b"'''", b"'''", true, true),
                interpolating(b"///", b"///", true, true, HASH_INTERP),
                interpolating(b"\"", b"\"", true, false, RIP_INTERP),
                SQ,
            ],
            regex: Some(RIP_REGEX_WORDS),
            color: [0xab, 0x23, 0x17],
            ..NONE
        },
        Lang {
            name: "CSS",
            extensions: &[".css"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ, SQ],
            color: [0x66, 0x33, 0x99],
            ..NONE
        },
        Lang {
            name: "YAML",
            extensions: &[".yml", ".yaml"],
            line: &[b"#"],
            strings: vec![DQ, kind(b"'", b"'", false, false)],
            color: [0xcb, 0x17, 0x1e],
            ..NONE
        },
        Lang {
            name: "TOML",
            extensions: &[".toml"],
            line: &[b"#"],
            strings: vec![
                kind(b"\"\"\"", b"\"\"\"", true, true),
                kind(b"'''", b"'''", false, true),
                DQ,
                kind(b"'", b"'", false, false),
            ],
            color: [0x9c, 0x42, 0x21],
            ..NONE
        },
        Lang {
            name: "SQL",
            extensions: &[".sql"],
            line: &[b"--"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![
                kind(b"'", b"'", false, true),
                kind(b"\"", b"\"", false, false),
            ],
            color: [0xe3, 0x8c, 0x00],
            ..NONE
        },
        Lang {
            name: "Dart",
            extensions: &[".dart"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            nests: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", true, true, DOLLAR_INTERP),
                interpolating(b"'''", b"'''", true, true, DOLLAR_INTERP),
                interpolating(b"\"", b"\"", true, false, DOLLAR_INTERP),
                interpolating(b"'", b"'", true, false, DOLLAR_INTERP),
            ],
            color: [0x00, 0xb4, 0xab],
            ..NONE
        },
        Lang {
            name: "CoffeeScript",
            extensions: &[".coffee"],
            shebangs: &["coffee"],
            line: &[b"#"],
            block_open: b"###",
            block_close: b"###",
            block_at_line_start: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", true, true, HASH_INTERP),
                kind(b"'''", b"'''", true, true),
                interpolating(b"///", b"///", true, true, HASH_INTERP),
                kind(b"```", b"```", false, true),
                interpolating(b"\"", b"\"", true, false, HASH_INTERP),
                SQ,
                kind(b"`", b"`", false, false),
            ],
            regex: Some(RIP_REGEX_WORDS),
            color: [0x24, 0x47, 0x76],
            ..NONE
        },
        Lang {
            name: "Ruby",
            extensions: &[".rb", ".rake", ".gemspec"],
            names: &["Gemfile", "Rakefile"],
            shebangs: &["ruby"],
            line: &[b"#"],
            block_open: b"=begin",
            block_close: b"=end",
            block_at_line_start: true,
            strings: vec![
                interpolating(b"\"", b"\"", true, true, HASH_INTERP),
                kind(b"'", b"'", true, true),
                kind(b"`", b"`", true, false),
            ],
            regex: Some(RUBY_REGEX_WORDS),
            heredoc: Heredoc::Ruby,
            color: [0x70, 0x15, 0x16],
            ..NONE
        },
        Lang {
            name: "Shell",
            extensions: &[".sh", ".bash", ".zsh", ".ksh"],
            names: &[".bashrc", ".bash_profile", ".profile", ".zshrc"],
            shebangs: &["sh", "bash", "zsh", "ksh", "dash"],
            line: &[b"#"],
            strings: vec![
                kind(b"\"", b"\"", true, true),
                kind(b"'", b"'", false, true),
            ],
            heredoc: Heredoc::Shell,
            color: [0x89, 0xe0, 0x51],
            ..NONE
        },
        markup("HTML", &[".html", ".htm"], [0xe3, 0x4c, 0x26]),
        markup("Vue", &[".vue"], [0x41, 0xb8, 0x83]),
        markup("Svelte", &[".svelte"], [0xff, 0x3e, 0x00]),
        Lang {
            name: "Java",
            extensions: &[".java"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![kind(b"\"\"\"", b"\"\"\"", true, true), DQ],
            chars: Chars::Always,
            color: [0xb0, 0x72, 0x19],
            ..NONE
        },
        Lang {
            name: "Kotlin",
            extensions: &[".kt", ".kts"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            nests: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", false, true, DOLLAR_INTERP),
                interpolating(b"\"", b"\"", true, false, DOLLAR_INTERP),
            ],
            chars: Chars::Always,
            color: [0xa9, 0x7b, 0xff],
            ..NONE
        },
        Lang {
            name: "Scala",
            extensions: &[".scala", ".sc", ".sbt"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            nests: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", false, true, DOLLAR_INTERP),
                interpolating(b"\"", b"\"", true, false, DOLLAR_INTERP),
            ],
            chars: Chars::Rust,
            color: [0xc2, 0x2d, 0x40],
            ..NONE
        },
        Lang {
            name: "Groovy",
            extensions: &[".groovy", ".gvy", ".gradle"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", true, true, DOLLAR_INTERP),
                kind(b"'''", b"'''", true, true),
                interpolating(b"\"", b"\"", true, false, DOLLAR_INTERP),
                SQ,
            ],
            color: [0x42, 0x98, 0xb8],
            ..NONE
        },
        Lang {
            name: "Swift",
            extensions: &[".swift"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            nests: true,
            strings: vec![kind(b"\"\"\"", b"\"\"\"", true, true), DQ],
            color: [0xf0, 0x51, 0x38],
            ..NONE
        },
        c_family("Objective-C", &[".m", ".mm"], [0x43, 0x8e, 0xff]),
        Lang {
            name: "SCSS",
            extensions: &[".scss"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ, SQ],
            color: [0xc6, 0x53, 0x8c],
            ..NONE
        },
        Lang {
            name: "Sass",
            extensions: &[".sass"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ, SQ],
            color: [0xa5, 0x3b, 0x70],
            ..NONE
        },
        Lang {
            name: "Less",
            extensions: &[".less"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ, SQ],
            color: [0x1d, 0x36, 0x5d],
            ..NONE
        },
        Lang {
            name: "PHP",
            extensions: &[".php", ".phtml"],
            shebangs: &["php"],
            regions: &[
                Region::Tag {
                    name: b"script",
                    lang: "JavaScript",
                },
                Region::Tag {
                    name: b"style",
                    lang: "CSS",
                },
                Region::Between {
                    open: b"<?php",
                    close: b"?>",
                    lang: "PHP code",
                },
                Region::Between {
                    open: b"<?=",
                    close: b"?>",
                    lang: "PHP code",
                },
            ],
            ..markup("PHP", &[], [0x4f, 0x5d, 0x95])
        },
        // The rules inside PHP's <?php … ?> regions; never recognized by extension, so never a row of its own.
        Lang {
            name: "PHP code",
            line: &[b"//", b"#"],
            hash_attribute: true,
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![
                interpolating(b"\"", b"\"", true, true, &[b"{$"]),
                kind(b"'", b"'", true, true),
                kind(b"`", b"`", true, false),
            ],
            heredoc: Heredoc::Php,
            ..NONE
        },
        Lang {
            name: "C#",
            extensions: &[".cs"],
            line: &[b"//"],
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![
                kind(b"\"\"\"", b"\"\"\"", false, true),
                kind(b"@$\"", b"\"", false, true),
                kind(b"$@\"", b"\"", false, true),
                kind(b"@\"", b"\"", false, true),
                DQ,
            ],
            chars: Chars::Always,
            color: [0x73, 0x55, 0xdd],
            ..NONE
        },
        Lang {
            name: "XML",
            extensions: &[".xml", ".xsd", ".xsl", ".xslt"],
            block_open: b"<!--",
            block_close: b"-->",
            color: [0x00, 0x60, 0xac],
            ..NONE
        },
    ])
});

const EXT_MAX: usize = 16; // an extension longer than this is never recognized

// Extensions that are never text and common enough to be numerous in an ordinary tree: a file with one is binary by its name alone, so it is never opened. Anything rarer is left to the NUL check, which is exact.
pub const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp", ".ttf", ".otf", ".woff", ".woff2", ".zip",
    ".gz", ".tgz", ".bz2", ".xz", ".zst", ".7z", ".jar", ".mp3", ".mp4", ".mov", ".wav", ".webm",
    ".pyc", ".o", ".a", ".so", ".dylib", ".dll", ".exe", ".class", ".wasm", ".rlib", ".rmeta",
    ".pdf",
];

static BINARY_EXT: LazyLock<HashMap<&'static [u8], ()>> = LazyLock::new(|| {
    BINARY_EXTENSIONS
        .iter()
        .map(|ext| (ext.as_bytes(), ()))
        .collect()
});

// Whether an extension, dot included, compared without regard to ASCII case, names a binary file by convention.
pub fn binary_ext(ext: &[u8]) -> bool {
    let mut lower = [0; EXT_MAX];
    let Some(lower) = lower.get_mut(..ext.len()) else {
        return false;
    };
    lower.copy_from_slice(ext);
    lower.make_ascii_lowercase();
    BINARY_EXT.contains_key(&*lower)
}

static BY_EXT: LazyLock<HashMap<&'static [u8], usize>> = LazyLock::new(|| {
    LANGS
        .iter()
        .enumerate()
        .flat_map(|(i, lang)| {
            lang.extensions.iter().map(move |ext| {
                assert!(
                    ext.len() <= EXT_MAX,
                    "extension {ext} is too long to look up"
                );
                (ext.as_bytes(), i)
            })
        })
        .collect()
});

static BY_NAME: LazyLock<HashMap<&'static [u8], usize>> = LazyLock::new(|| {
    LANGS
        .iter()
        .enumerate()
        .flat_map(|(i, lang)| lang.names.iter().map(move |name| (name.as_bytes(), i)))
        .collect()
});

static BY_SHEBANG: LazyLock<HashMap<&'static [u8], usize>> = LazyLock::new(|| {
    LANGS
        .iter()
        .enumerate()
        .flat_map(|(i, lang)| lang.shebangs.iter().map(move |s| (s.as_bytes(), i)))
        .collect()
});

// Language index by extension, dot included, compared without regard to ASCII case.
pub fn by_ext(ext: &[u8]) -> Option<usize> {
    let mut lower = [0; EXT_MAX];
    let lower = lower.get_mut(..ext.len())?;
    lower.copy_from_slice(ext);
    lower.make_ascii_lowercase();
    BY_EXT.get(&*lower).copied()
}

// Language index by the name --languages prints, compared without regard to ASCII case.
pub fn by_language(name: &str) -> Option<usize> {
    LANGS
        .iter()
        .position(|lang| lang.listed() && lang.name.eq_ignore_ascii_case(name))
}

// Language index by whole file name, compared exactly.
pub fn by_name(name: &[u8]) -> Option<usize> {
    BY_NAME.get(name).copied()
}

// Language index by the interpreter of a #! line, compared exactly.
pub fn by_shebang(interpreter: &[u8]) -> Option<usize> {
    BY_SHEBANG.get(interpreter).copied()
}

// The scanner only ever tests a byte against these derived tables.
fn prepare(mut langs: Vec<Lang>) -> Vec<Lang> {
    let names: Vec<&str> = langs.iter().map(|lang| lang.name).collect();
    for lang in &mut langs {
        // Longest opener first, so `"""` is tried before `"`; the sort is stable, so equal lengths keep table order.
        lang.strings
            .sort_by_key(|kind| std::cmp::Reverse(kind.open.len()));
        lang.region_langs = lang
            .regions
            .iter()
            .map(|region| {
                names
                    .iter()
                    .position(|&name| name == region.lang())
                    .expect("a region names a language in the table")
            })
            .collect();
        for &b in b" \t\r\x0c\x0b\\" {
            lang.starts[b as usize] = true;
        }
        for opener in lang.line {
            lang.starts[opener[0] as usize] = true;
        }
        if let Some(&b) = lang.block_open.first() {
            lang.starts[b as usize] = true;
        }
        for kind in &lang.strings {
            lang.starts[kind.open[0] as usize] = true;
            if !kind.interp.is_empty() {
                lang.starts[b'{' as usize] = true;
                lang.starts[b'}' as usize] = true;
            }
        }
        if lang.chars != Chars::None {
            lang.starts[b'\'' as usize] = true;
        }
        if lang.regex.is_some() {
            lang.starts[b'/' as usize] = true;
        }
        if lang.rust_raw {
            lang.starts[b'r' as usize] = true;
            lang.starts[b'b' as usize] = true;
        }
        if lang.heredoc != Heredoc::None || !lang.regions.is_empty() {
            lang.starts[b'<' as usize] = true;
        }
    }
    langs
}

#[cfg(test)]
mod tests {
    use super::LANGS;

    // The README's language count is the number of table entries a file can be recognized as; entries that exist only as region rules do not count.
    #[test]
    fn readme_language_count_matches_the_table() {
        let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
            .expect("read README.md");
        let recognizable = LANGS
            .iter()
            .filter(|lang| {
                !lang.extensions.is_empty() || !lang.names.is_empty() || !lang.shebangs.is_empty()
            })
            .count();
        let claimed = format!("in {recognizable} languages.");
        assert!(
            readme.contains(&claimed),
            "README.md does not say {claimed:?}; the table has {recognizable} recognizable languages"
        );
    }
}
