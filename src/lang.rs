// The language table from SPEC.md and the byte tables derived from it.

use std::sync::LazyLock;

pub const NUM_LANGS: usize = 10;

// One way a language writes a string literal; char literals and Rust raw strings are kinds too.
pub struct StrKind {
    pub open: &'static [u8],
    pub close: &'static [u8],
    pub escape: bool,
    pub multiline: bool,
    pub stop: [bool; 256], // bytes that end the fast skip inside the string: the closer's first byte, backslash, $
}

const fn kind(open: &'static [u8], close: &'static [u8], escape: bool, multiline: bool) -> StrKind {
    let mut stop = [false; 256];
    stop[close[0] as usize] = true;
    stop[b'\\' as usize] = true;
    stop[b'$' as usize] = true;
    StrKind {
        open,
        close,
        escape,
        multiline,
        stop,
    }
}

const DQ: StrKind = kind(b"\"", b"\"", true, false);
const SQ: StrKind = kind(b"'", b"'", true, false);
const TICK: StrKind = kind(b"`", b"`", true, true);
pub static CHAR: StrKind = kind(b"'", b"'", true, false);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Chars {
    None,
    Always, // every ' outside a string or comment opens a char literal
    Rust,   // ' opens one only before \ or when the byte after next is ': lifetimes have no closer
}

// A language as the scanner sees it. Empty slices and false mean the syntax is absent.
pub struct Lang {
    pub name: &'static str,
    pub line: &'static [u8],       // line-comment opener
    pub block_open: &'static [u8], // block-comment opener; empty when the language has none
    pub block_close: &'static [u8],
    pub nests: bool, // block comments nest (Rust); false ends one at the first closer (Go)
    pub strings: Vec<StrKind>, // longest opener first
    pub chars: Chars,
    pub regex: bool,           // /…/ literals by the previous-byte-or-keyword rule
    pub zig_line_string: bool, // \\ to end of line
    pub rust_raw: bool,        // r#*"…"#*, optional b prefix
    pub color: [u8; 3],
    pub starts: [bool; 256], // bytes that can begin a token in normal state; any other byte is plain code
    pub template: Option<usize>, // index of the backtick kind, whose ${…} interpolations are code
}

const NONE: Lang = Lang {
    name: "",
    line: b"",
    block_open: b"",
    block_close: b"",
    nests: false,
    strings: Vec::new(),
    chars: Chars::None,
    regex: false,
    zig_line_string: false,
    rust_raw: false,
    color: [0; 3],
    starts: [false; 256],
    template: None,
};

fn c_family(name: &'static str, color: [u8; 3]) -> Lang {
    Lang {
        name,
        line: b"//",
        block_open: b"/*",
        block_close: b"*/",
        strings: vec![DQ],
        chars: Chars::Always,
        color,
        ..NONE
    }
}

fn js_family(name: &'static str, color: [u8; 3]) -> Lang {
    Lang {
        name,
        line: b"//",
        block_open: b"/*",
        block_close: b"*/",
        strings: vec![TICK, DQ, SQ],
        regex: true,
        color,
        ..NONE
    }
}

// Colors are GitHub linguist's, except JSON and Markdown, whose linguist colors are unreadable on a dark background and take Seti's.
pub static LANGS: LazyLock<[Lang; NUM_LANGS]> = LazyLock::new(|| {
    prepare([
        js_family("TypeScript", [0x31, 0x78, 0xc6]),
        js_family("JavaScript", [0xf1, 0xe0, 0x5a]),
        Lang {
            name: "Go",
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
            line: b"//",
            strings: vec![DQ],
            chars: Chars::Always,
            zig_line_string: true,
            color: [0xec, 0x91, 0x5c],
            ..NONE
        },
        Lang {
            name: "Python",
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
            line: b"//",
            block_open: b"/*",
            block_close: b"*/",
            strings: vec![DQ],
            color: [0xcb, 0xcb, 0x41],
            ..NONE
        },
        Lang {
            name: "Markdown",
            color: [0x51, 0x9a, 0xba],
            ..NONE
        },
        c_family("C", [0x55, 0x55, 0x55]),
        c_family("C++", [0xf3, 0x4b, 0x7d]),
    ])
});

// Language index by extension, dot included, compared without regard to ASCII case.
pub fn by_ext(ext: &[u8]) -> Option<usize> {
    let mut lower = [0; 9]; // no listed extension is longer than .markdown
    let lower = lower.get_mut(..ext.len())?;
    lower.copy_from_slice(ext);
    lower.make_ascii_lowercase();
    Some(match &*lower {
        b".ts" | b".tsx" | b".mts" | b".cts" => 0,
        b".js" | b".mjs" | b".cjs" | b".jsx" => 1,
        b".go" => 2,
        b".rs" => 3,
        b".zig" => 4,
        b".py" | b".pyi" => 5,
        b".json" | b".jsonc" => 6,
        b".md" | b".markdown" => 7,
        b".c" | b".h" => 8,
        b".cpp" | b".cc" | b".cxx" | b".hpp" | b".hh" | b".hxx" => 9,
        _ => return None,
    })
}

// The scanner only ever tests a byte against these derived tables.
fn prepare(mut langs: [Lang; NUM_LANGS]) -> [Lang; NUM_LANGS] {
    for lang in &mut langs {
        for &b in b" \t\r\x0c\x0b\\" {
            lang.starts[b as usize] = true;
        }
        if let Some(&b) = lang.line.first() {
            lang.starts[b as usize] = true;
        }
        if let Some(&b) = lang.block_open.first() {
            lang.starts[b as usize] = true;
        }
        for (i, kind) in lang.strings.iter().enumerate() {
            lang.starts[kind.open[0] as usize] = true;
            if kind.open == b"`" {
                lang.template = Some(i);
                lang.starts[b'{' as usize] = true;
                lang.starts[b'}' as usize] = true;
            }
        }
        if lang.chars != Chars::None {
            lang.starts[b'\'' as usize] = true;
        }
        if lang.regex {
            lang.starts[b'/' as usize] = true;
        }
        if lang.rust_raw {
            lang.starts[b'r' as usize] = true;
            lang.starts[b'b' as usize] = true;
        }
    }
    langs
}
