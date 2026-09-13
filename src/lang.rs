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

const JS_INTERP: &[&[u8]] = &[b"${"];
const RIP_INTERP: &[&[u8]] = &[b"#{", b"${"];
const RIP_HEREGEX_INTERP: &[&[u8]] = &[b"#{"];

const DQ: StrKind = kind(b"\"", b"\"", true, false);
const SQ: StrKind = kind(b"'", b"'", true, false);
const TICK: StrKind = interpolating(b"`", b"`", true, true, JS_INTERP);
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Chars {
    None,
    Always, // every ' outside a string or comment opens a char literal
    Rust,   // ' opens one only before \ or when the byte after next is ': lifetimes have no closer
}

// A language as the scanner sees it. Empty slices, None, and false mean the syntax is absent.
pub struct Lang {
    pub name: &'static str,
    pub extensions: &'static [&'static str], // lower-case, dot included
    pub line: &'static [u8],                 // line-comment opener
    pub block_open: &'static [u8], // block-comment opener; empty when the language has none
    pub block_close: &'static [u8],
    pub nests: bool, // block comments nest (Rust); false ends one at the first closer (Go)
    pub block_at_line_start: bool, // a block comment opens only as the first non-whitespace bytes of a line (Rip)
    pub strings: Vec<StrKind>,
    pub chars: Chars,
    pub regex: Option<&'static [&'static [u8]]>, // /…/ literals by the previous-byte-or-word rule, with the words
    pub zig_line_string: bool,                   // \\ to end of line
    pub rust_raw: bool,                          // r#*"…"#*, optional b prefix
    pub color: [u8; 3],
    pub starts: [bool; 256], // bytes that can begin a token in normal state; any other byte is plain code
}

const NONE: Lang = Lang {
    name: "",
    extensions: &[],
    line: b"",
    block_open: b"",
    block_close: b"",
    nests: false,
    block_at_line_start: false,
    strings: Vec::new(),
    chars: Chars::None,
    regex: None,
    zig_line_string: false,
    rust_raw: false,
    color: [0; 3],
    starts: [false; 256],
};

fn c_family(name: &'static str, extensions: &'static [&'static str], color: [u8; 3]) -> Lang {
    Lang {
        name,
        extensions,
        line: b"//",
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
        line: b"//",
        block_open: b"/*",
        block_close: b"*/",
        strings: vec![TICK, DQ, SQ],
        regex: Some(JS_REGEX_WORDS),
        color,
        ..NONE
    }
}

// Colors are GitHub linguist's, except JSON and Markdown, whose linguist colors are unreadable on a dark background and take Seti's, and Rip, which linguist does not list.
pub static LANGS: LazyLock<Vec<Lang>> = LazyLock::new(|| {
    prepare(vec![
        js_family(
            "TypeScript",
            &[".ts", ".tsx", ".mts", ".cts"],
            [0x31, 0x78, 0xc6],
        ),
        js_family(
            "JavaScript",
            &[".js", ".mjs", ".cjs", ".jsx"],
            [0xf1, 0xe0, 0x5a],
        ),
        Lang {
            name: "Go",
            extensions: &[".go"],
            line: b"//",
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
            line: b"//",
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
            line: b"//",
            strings: vec![DQ],
            chars: Chars::Always,
            zig_line_string: true,
            color: [0xec, 0x91, 0x5c],
            ..NONE
        },
        Lang {
            name: "Python",
            extensions: &[".py", ".pyi"],
            line: b"#",
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
            line: b"//",
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ],
            color: [0xcb, 0xcb, 0x41],
            ..NONE
        },
        Lang {
            name: "Markdown",
            extensions: &[".md", ".markdown"],
            color: [0x51, 0x9a, 0xba],
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
            line: b"#",
            block_open: b"###",
            block_close: b"###",
            block_at_line_start: true,
            strings: vec![
                interpolating(b"\"\"\"", b"\"\"\"", true, true, RIP_INTERP),
                kind(b"'''", b"'''", true, true),
                interpolating(b"///", b"///", true, true, RIP_HEREGEX_INTERP),
                interpolating(b"\"", b"\"", true, false, RIP_INTERP),
                SQ,
            ],
            regex: Some(RIP_REGEX_WORDS),
            color: [0xab, 0x23, 0x17],
            ..NONE
        },
    ])
});

const EXT_MAX: usize = 16; // an extension longer than this is never recognized

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

// Language index by extension, dot included, compared without regard to ASCII case.
pub fn by_ext(ext: &[u8]) -> Option<usize> {
    let mut lower = [0; EXT_MAX];
    let lower = lower.get_mut(..ext.len())?;
    lower.copy_from_slice(ext);
    lower.make_ascii_lowercase();
    BY_EXT.get(&*lower).copied()
}

// The scanner only ever tests a byte against these derived tables.
fn prepare(mut langs: Vec<Lang>) -> Vec<Lang> {
    for lang in &mut langs {
        // Longest opener first, so `"""` is tried before `"`; the sort is stable, so equal lengths keep table order.
        lang.strings
            .sort_by_key(|kind| std::cmp::Reverse(kind.open.len()));
        for &b in b" \t\r\x0c\x0b\\" {
            lang.starts[b as usize] = true;
        }
        if let Some(&b) = lang.line.first() {
            lang.starts[b as usize] = true;
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
    }
    langs
}
