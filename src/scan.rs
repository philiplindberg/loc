// Line classification and the scanner, SPEC.md sections of the same names. One pass over the bytes of a file; the state that survives a line is the open block-comment depth, the open string, and the interpolation stack.

use crate::lang::{CHAR, Chars, Lang, StrKind};

#[derive(Clone, Copy, Default)]
pub struct Counts {
    pub lines: usize,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, other: Counts) {
        self.lines += other.lines;
        self.blank += other.blank;
        self.comment += other.comment;
        self.code += other.code;
    }
}

const fn byte_set(bytes: &[u8]) -> [bool; 256] {
    let mut set = [false; 256];
    let mut i = 0;
    while i < bytes.len() {
        set[bytes[i] as usize] = true;
        i += 1;
    }
    set
}

static REGEX_PREV: [bool; 256] = byte_set(b"(,=:[!&|?{};+-*%<>~^");
static WHITESPACE: [bool; 256] = byte_set(b" \t\r\x0c\x0b");
const REGEX_KEYWORDS: [&[u8]; 14] = [
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

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

// Whether s occurs at buf[i] without crossing end.
fn starts_with(buf: &[u8], i: usize, end: usize, s: &[u8]) -> bool {
    i + s.len() <= end && &buf[i..i + s.len()] == s
}

// The open string: a kind from the language table, a char literal, or a Rust raw string with its hash count, whose closer is a quote followed by that many #.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Open {
    Kind(usize),
    Char,
    Raw(usize),
}

struct Scanner<'a> {
    lang: &'a Lang,
    buf: &'a [u8],
    block_depth: usize,
    open: Option<Open>,
    interp: Vec<usize>, // brace depth of each open ${…}, innermost last
}

pub fn scan(buf: &[u8], lang: &Lang) -> Counts {
    let mut scanner = Scanner {
        lang,
        buf,
        block_depth: 0,
        open: None,
        interp: Vec::new(),
    };
    let mut counts = Counts::default();
    let mut pos = if buf.starts_with(&[0xef, 0xbb, 0xbf]) {
        3
    } else {
        0
    };
    while pos < buf.len() {
        let mut next = buf.len();
        let mut end = next;
        if let Some(nl) = buf[pos..].iter().position(|&b| b == b'\n') {
            end = pos + nl;
            next = end + 1;
            if end > pos && buf[end - 1] == b'\r' {
                end -= 1;
            }
        }
        counts.lines += 1;
        match scanner.line(pos, end) {
            Class::Str | Class::Code => counts.code += 1,
            Class::Blank => counts.blank += 1,
            Class::Comment => counts.comment += 1,
        }
        pos = next;
    }
    counts
}

enum Class {
    Str, // a byte of the line lies inside a string literal
    Blank,
    Code,
    Comment,
}

impl<'a> Scanner<'a> {
    fn kind(&self, open: Open) -> &'a StrKind {
        match open {
            Open::Kind(i) => &self.lang.strings[i],
            Open::Char => &CHAR,
            Open::Raw(_) => unreachable!("raw strings have no table kind"),
        }
    }

    fn multiline(&self, open: Open) -> bool {
        match open {
            Open::Kind(i) => self.lang.strings[i].multiline,
            Open::Char => false,
            Open::Raw(_) => true,
        }
    }

    // Scans buf[pos..end] and classifies it, leaving the cross-line state for the next line.
    fn line(&mut self, pos: usize, end: usize) -> Class {
        let (buf, lang) = (self.buf, self.lang);
        let mut saw_string = self.open.is_some(); // an empty or whitespace-only line inside a multi-line string is code
        let (mut saw_code, mut saw_comment) = (false, false);
        let mut i = pos;

        if lang.zig_line_string && self.open.is_none() {
            let mut j = pos;
            while j < end && WHITESPACE[buf[j] as usize] {
                j += 1;
            }
            if j + 1 < end && buf[j] == b'\\' && buf[j + 1] == b'\\' {
                saw_string = true;
                i = end;
            }
        }

        while i < end {
            if let Some(open) = self.open {
                saw_string = true;
                if let Open::Raw(hashes) = open {
                    while i < end && buf[i] != b'"' {
                        i += 1;
                    }
                    if i >= end {
                        break;
                    }
                    i += 1;
                    if i + hashes <= end && buf[i..i + hashes].iter().all(|&b| b == b'#') {
                        self.open = None;
                        i += hashes;
                    }
                    continue;
                }
                let kind = self.kind(open);
                while i < end && !kind.stop[buf[i] as usize] {
                    i += 1;
                }
                if i >= end {
                    break;
                }
                let b = buf[i];
                if kind.escape && b == b'\\' {
                    i += 2;
                } else if Some(open) == lang.template.map(Open::Kind)
                    && b == b'$'
                    && i + 1 < end
                    && buf[i + 1] == b'{'
                {
                    self.interp.push(0);
                    self.open = None;
                    i += 2;
                } else if starts_with(buf, i, end, kind.close) {
                    self.open = None;
                    i += kind.close.len();
                } else {
                    i += 1;
                }
                continue;
            }

            if self.block_depth > 0 {
                saw_comment = true;
                while i < end
                    && buf[i] != lang.block_close[0]
                    && !(lang.nests && buf[i] == lang.block_open[0])
                {
                    i += 1;
                }
                if i >= end {
                    break;
                }
                if lang.nests && starts_with(buf, i, end, lang.block_open) {
                    self.block_depth += 1;
                    i += lang.block_open.len();
                } else if starts_with(buf, i, end, lang.block_close) {
                    self.block_depth -= 1;
                    i += lang.block_close.len();
                } else {
                    i += 1;
                }
                continue;
            }

            let b = buf[i];
            if !lang.starts[b as usize] {
                // plain code: skip the whole run
                saw_code = true;
                i += 1;
                while i < end && !lang.starts[buf[i] as usize] {
                    i += 1;
                }
                continue;
            }
            let interp = !self.interp.is_empty();
            match b {
                _ if WHITESPACE[b as usize] => i += 1,
                b'\\' => {
                    // escapes the next byte even outside a string
                    saw_code = true;
                    i += 2;
                }
                b'{' if interp => {
                    *self.interp.last_mut().unwrap() += 1;
                    saw_code = true;
                    i += 1;
                }
                b'}' if interp => {
                    let depth = self.interp.last_mut().unwrap();
                    if *depth == 0 {
                        // back into the template
                        self.interp.pop();
                        self.open = lang.template.map(Open::Kind);
                        saw_string = true;
                    } else {
                        *depth -= 1;
                        saw_code = true;
                    }
                    i += 1;
                }
                _ if !lang.line.is_empty() && starts_with(buf, i, end, lang.line) => {
                    saw_comment = true;
                    i = end;
                }
                _ if !lang.block_open.is_empty() && starts_with(buf, i, end, lang.block_open) => {
                    self.block_depth = 1;
                    saw_comment = true;
                    i += lang.block_open.len();
                }
                b'b' | b'r'
                    if lang.rust_raw
                        && let Some((hashes, after)) = rust_raw_opener(buf, i, end) =>
                {
                    self.open = Some(Open::Raw(hashes));
                    saw_string = true;
                    i = after;
                }
                _ if let Some(ki) = self.open_string(i, end) => {
                    self.open = Some(Open::Kind(ki));
                    saw_string = true;
                    i += lang.strings[ki].open.len();
                }
                b'\'' if lang.chars != Chars::None => {
                    if lang.chars == Chars::Always
                        || (i + 1 < buf.len() && buf[i + 1] == b'\\')
                        || (i + 2 < end && buf[i + 2] == b'\'')
                    {
                        self.open = Some(Open::Char);
                        saw_string = true;
                    } else {
                        saw_code = true;
                    }
                    i += 1;
                }
                // the next byte is not / or *, which the comment openers took
                b'/' if lang.regex && regex_opens(buf, pos, i) => {
                    i = regex_end(buf, i + 1, end);
                    saw_string = true;
                }
                _ => {
                    saw_code = true;
                    i += 1;
                }
            }
        }

        if let Some(open) = self.open
            && !self.multiline(open)
        {
            // an unterminated single-line string ends at the newline
            self.open = None;
        }
        if saw_string {
            Class::Str
        } else if !saw_code && blank(&buf[pos..end]) {
            Class::Blank
        } else if saw_code {
            Class::Code
        } else if saw_comment {
            Class::Comment
        } else {
            Class::Code
        }
    }

    fn open_string(&self, i: usize, end: usize) -> Option<usize> {
        self.lang
            .strings
            .iter()
            .position(|k| starts_with(self.buf, i, end, k.open))
    }
}

fn blank(line: &[u8]) -> bool {
    line.iter().all(|&b| WHITESPACE[b as usize])
}

// A raw string opener r#*" (optionally b-prefixed) at buf[i]: its hash count and where it ends.
fn rust_raw_opener(buf: &[u8], i: usize, end: usize) -> Option<(usize, usize)> {
    let mut j = i;
    if j < end && buf[j] == b'b' {
        j += 1;
    }
    if j >= end || buf[j] != b'r' {
        return None;
    }
    j += 1;
    let hashes = buf[j..end].iter().take_while(|&&b| b == b'#').count();
    j += hashes;
    if j >= end || buf[j] != b'"' {
        return None;
    }
    Some((hashes, j + 1))
}

// The previous-byte-or-keyword rule for a / at buf[i] on the line starting at line_start.
fn regex_opens(buf: &[u8], line_start: usize, i: usize) -> bool {
    let mut j = i;
    while j > line_start && WHITESPACE[buf[j - 1] as usize] {
        j -= 1;
    }
    if j == line_start {
        return true;
    }
    let prev = buf[j - 1];
    if REGEX_PREV[prev as usize] {
        return true;
    }
    if !is_ident(prev) {
        return false;
    }
    let mut k = j - 1;
    while k > line_start && is_ident(buf[k - 1]) {
        k -= 1;
    }
    REGEX_KEYWORDS.contains(&&buf[k..j])
}

fn regex_end(buf: &[u8], mut i: usize, end: usize) -> usize {
    let mut in_class = false;
    while i < end {
        match buf[i] {
            b'\\' => i += 1,
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => return i + 1,
            _ => {}
        }
        i += 1;
    }
    end
}
