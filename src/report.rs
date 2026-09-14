// The report and its two renderings, SPEC.md Output. The JSON is the conformance format; the table is for people.

use std::collections::HashMap;
use std::fmt::Write;

use crate::lang::LANGS;
use crate::walk::{Skipped, Sums};

pub struct LangRow {
    name: &'static str,
    files: usize,
    lines: usize,
    blank: usize,
    comment: usize,
    code: usize,
    bytes: u64,
    color: [u8; 3],
}

#[derive(Default, Clone, Copy)]
pub struct Totals {
    files: usize,
    lines: usize,
    blank: usize,
    comment: usize,
    code: usize,
    bytes: u64,
}

pub struct SkippedLabel {
    label: String,
    files: usize,
    bytes: u64,
}

// Skipped files of one kind, text or binary: labels in report order and their sum.
#[derive(Default)]
pub struct SkippedGroup {
    labels: Vec<SkippedLabel>,
    files: usize,
    bytes: u64,
}

impl SkippedGroup {
    fn new(labels: HashMap<String, Skipped>) -> SkippedGroup {
        let mut group = SkippedGroup::default();
        for (label, skipped) in labels {
            group.labels.push(SkippedLabel {
                label,
                files: skipped.files,
                bytes: skipped.bytes,
            });
            group.files += skipped.files;
            group.bytes += skipped.bytes;
        }
        group
            .labels
            .sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.label.cmp(&b.label)));
        group
    }
}

pub struct Report {
    languages: Vec<LangRow>,
    total: Totals,
    text: SkippedGroup,
    binary: SkippedGroup,
}

impl Report {
    // Orders the sums as the spec requires: languages by code descending then name, labels by size descending then label, both byte-wise.
    pub fn new(sums: Sums) -> Report {
        let mut report = Report {
            languages: Vec::new(),
            total: Totals::default(),
            text: SkippedGroup::default(),
            binary: SkippedGroup::default(),
        };
        for (i, lang) in LANGS.iter().enumerate() {
            if sums.files[i] == 0 {
                continue;
            }
            let counts = sums.langs[i];
            report.languages.push(LangRow {
                name: lang.name,
                files: sums.files[i],
                lines: counts.lines,
                blank: counts.blank,
                comment: counts.comment,
                code: counts.code,
                bytes: sums.bytes[i],
                color: lang.color,
            });
            report.total.files += sums.files[i];
            report.total.lines += counts.lines;
            report.total.blank += counts.blank;
            report.total.comment += counts.comment;
            report.total.code += counts.code;
            report.total.bytes += sums.bytes[i];
        }
        report
            .languages
            .sort_by(|a, b| b.code.cmp(&a.code).then_with(|| a.name.cmp(b.name)));
        report.text = SkippedGroup::new(sums.text);
        report.binary = SkippedGroup::new(sums.binary);
        report
    }

    // JSON as JSON.stringify writes it: one object, no whitespace, then a newline.
    pub fn json(&self) -> String {
        let mut out = String::from("{\"languages\":[");
        for (i, row) in self.languages.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"name\":{},\"files\":{},\"lines\":{},\"blank\":{},\"comment\":{},\"code\":{},\"bytes\":{}}}",
                json_str(row.name),
                row.files,
                row.lines,
                row.blank,
                row.comment,
                row.code,
                row.bytes
            );
        }
        let total = self.total;
        let _ = write!(
            out,
            "],\"total\":{{\"files\":{},\"lines\":{},\"blank\":{},\"comment\":{},\"code\":{},\"bytes\":{}}},\"skipped\":{{",
            total.files, total.lines, total.blank, total.comment, total.code, total.bytes
        );
        for (i, (key, group)) in [("text", &self.text), ("binary", &self.binary)]
            .into_iter()
            .enumerate()
        {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "\"{key}\":{{\"files\":{},\"bytes\":{},\"labels\":[",
                group.files, group.bytes
            );
            for (i, row) in group.labels.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "{{\"label\":{},\"files\":{},\"bytes\":{}}}",
                    json_str(&row.label),
                    row.files,
                    row.bytes
                );
            }
            out.push_str("]}");
        }
        out.push_str("}}\n");
        out
    }

    // The language bar: one run of background-colored spaces per language with code, touching, each at least one cell, the rest by code share with floor and largest remainder, ties in table order.
    fn bar(&self, width: usize) -> String {
        let rows: Vec<&LangRow> = self.languages.iter().filter(|l| l.code > 0).collect();
        if rows.is_empty() || width < rows.len() {
            return String::new();
        }
        let spare = width - rows.len();
        let total = self.total.code;
        let mut cells: Vec<usize> = rows.iter().map(|l| 1 + l.code * spare / total).collect();
        let rem: Vec<usize> = rows.iter().map(|l| l.code * spare % total).collect();
        let left = width - cells.iter().sum::<usize>();
        let mut order: Vec<usize> = (0..rows.len()).collect();
        order.sort_by(|&a, &b| rem[b].cmp(&rem[a]));
        for &i in &order[..left] {
            cells[i] += 1;
        }
        let mut out = String::new();
        for (lang, &n) in rows.iter().zip(&cells) {
            out.push_str(&bg(lang.color));
            out.push_str(&" ".repeat(n));
            out.push_str(RESET);
        }
        out
    }

    pub fn table(&self, styled: bool) -> String {
        let style = |code: &str, text: String| {
            if styled {
                format!("{code}{text}{RESET}")
            } else {
                text
            }
        };
        let prefix = if styled { "  " } else { "" }; // room for the dot on language rows
        let row = |name: &str, totals: Totals| -> Vec<String> {
            vec![
                format!("{prefix}{name}"),
                with_commas(totals.files),
                with_commas(totals.lines),
                with_commas(totals.blank),
                with_commas(totals.comment),
                with_commas(totals.code),
                share(totals.code as u64, self.total.code as u64),
            ]
        };
        let body: Vec<Vec<String>> = self
            .languages
            .iter()
            .map(|lang| {
                row(
                    lang.name,
                    Totals {
                        files: lang.files,
                        lines: lang.lines,
                        blank: lang.blank,
                        comment: lang.comment,
                        code: lang.code,
                        bytes: lang.bytes,
                    },
                )
            })
            .collect();
        let total_row = row("Total", self.total);
        let mut all_rows = body.clone();
        all_rows.push(total_row.clone());
        let headers: Vec<String> = [
            &format!("{prefix}Language")[..],
            "Files",
            "Lines",
            "Blank",
            "Comment",
            "Code",
            "%",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let grid = Grid::new(headers, &all_rows);

        let mut out: Vec<String> = Vec::new();
        if styled && self.total.code > 0 {
            out.push(self.bar(grid.width()));
        }
        out.push(style(DIM, grid.rule()));
        out.push(style(BOLD, grid.line(&grid.headers)));
        out.push(style(DIM, grid.rule()));
        for (lang, cells) in self.languages.iter().zip(&body) {
            let line = grid.line(cells);
            out.push(if styled {
                format!("{}●{RESET} {}", fg(lang.color), &line[2..])
            } else {
                line
            });
        }
        out.push(style(DIM, grid.rule()));
        out.push(style(BOLD, grid.line(&total_row)));
        out.push(style(DIM, grid.rule()));
        out.push(String::new());

        let all_files = self.total.files + self.text.files;
        let all_bytes = self.total.bytes + self.text.bytes;
        out.push(style(
            DIM,
            format!(
                "{} of {} files skipped, {} of {} of text ({}%)",
                with_commas(self.text.files),
                with_commas(all_files),
                size(self.text.bytes),
                size(all_bytes),
                share(self.text.bytes, all_bytes)
            ),
        ));
        let labels = &self.text.labels;
        if !labels.is_empty() {
            let shown = labels.len().min(10);
            let mut rows: Vec<Vec<String>> = labels[..shown]
                .iter()
                .map(|skipped| {
                    vec![
                        skipped.label.clone(),
                        with_commas(skipped.files),
                        size(skipped.bytes),
                        share(skipped.bytes, all_bytes),
                    ]
                })
                .collect();
            if labels.len() > 10 {
                let rest_files: usize = labels[10..].iter().map(|skipped| skipped.files).sum();
                let rest_bytes: u64 = labels[10..].iter().map(|skipped| skipped.bytes).sum();
                rows.push(vec![
                    format!("{} more labels", with_commas(labels.len() - 10)),
                    with_commas(rest_files),
                    size(rest_bytes),
                    share(rest_bytes, all_bytes),
                ]);
            }
            let skipped_grid = Grid::new(
                ["Skipped", "Files", "Size", "%"]
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                &rows,
            );
            out.push(style(DIM, skipped_grid.rule()));
            out.push(style(DIM, skipped_grid.line(&skipped_grid.headers)));
            out.push(style(DIM, skipped_grid.rule()));
            for cells in &rows {
                out.push(style(DIM, skipped_grid.line(cells)));
            }
            out.push(style(DIM, skipped_grid.rule()));
        }
        if self.binary.files > 0 {
            let labels = &self.binary.labels;
            let mut line = format!(
                "{} binary file{}, {}: ",
                with_commas(self.binary.files),
                if self.binary.files == 1 { "" } else { "s" },
                size(self.binary.bytes)
            );
            let shown = labels.len().min(10);
            let listed: Vec<String> = labels[..shown]
                .iter()
                .map(|skipped| format!("{} {}", skipped.label, with_commas(skipped.files)))
                .collect();
            line.push_str(&listed.join(", "));
            if labels.len() > 10 {
                let _ = write!(line, ", {} more", with_commas(labels.len() - 10));
            }
            out.push(style(DIM, line));
        }
        out.join("\n") + "\n"
    }
}

// The --languages table: every language a file can be recognized as, by name, with the extensions and file names it claims, in the language table's layout. A cell's items wrap onto continuation lines so no column exceeds WRAP characters.
pub fn languages(styled: bool) -> String {
    let style = |code: &str, text: String| {
        if styled {
            format!("{code}{text}{RESET}")
        } else {
            text
        }
    };
    let prefix = if styled { "  " } else { "" };
    let mut langs: Vec<&crate::lang::Lang> = LANGS.iter().filter(|lang| lang.listed()).collect();
    langs.sort_by(|a, b| a.name.cmp(b.name));
    // Each language becomes one or more physical rows; only the first carries the name.
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut first_row: Vec<usize> = Vec::new();
    for lang in &langs {
        let files: Vec<&str> = lang.extensions.iter().chain(lang.names).copied().collect();
        let cells = [wrap(&files)];
        let height = cells.iter().map(Vec::len).max().unwrap_or(0).max(1);
        first_row.push(rows.len());
        for line in 0..height {
            let mut row = vec![if line == 0 {
                format!("{prefix}{}", lang.name)
            } else {
                String::new()
            }];
            row.extend(
                cells
                    .iter()
                    .map(|c| c.get(line).cloned().unwrap_or_default()),
            );
            rows.push(row);
        }
    }
    let headers = [&format!("{prefix}Language")[..], "Files"]
        .iter()
        .map(ToString::to_string)
        .collect();
    let grid = Grid::left(headers, &rows);
    let mut out = vec![
        style(DIM, grid.rule()),
        style(BOLD, grid.line(&grid.headers).trim_end().to_string()),
        style(DIM, grid.rule()),
    ];
    for (i, cells) in rows.iter().enumerate() {
        let line = grid.line(cells).trim_end().to_string();
        let lang = first_row.iter().position(|&f| f == i).map(|k| langs[k]);
        out.push(match lang {
            Some(lang) if styled => format!("{}●{RESET} {}", fg(lang.color), &line[2..]),
            _ => line,
        });
    }
    out.join("\n") + "\n"
}

const WRAP: usize = 40; // widest a --languages column grows before its items continue on the next line

// Joins items with single spaces into lines no longer than WRAP, breaking only between items.
fn wrap(items: &[&str]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for item in items {
        match lines.last_mut() {
            Some(last) if last.len() + 1 + item.len() <= WRAP => {
                last.push(' ');
                last.push_str(item);
            }
            _ => lines.push((*item).to_string()),
        }
    }
    lines
}

// JSON.stringify's string escaping: quote, backslash, and control bytes; everything else as is.
fn json_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// Bytes to one decimal in the smallest unit from KB up that keeps the tenths below 10,000, or TB; under 1000 bytes whole.
fn size(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    const UNITS: [(u64, &str); 4] = [
        (1_000, "KB"),
        (1_000_000, "MB"),
        (1_000_000_000, "GB"),
        (1_000_000_000_000, "TB"),
    ];
    let bytes = u128::from(bytes); // bytes × 10 must not overflow
    for (i, &(unit, name)) in UNITS.iter().enumerate() {
        let unit = u128::from(unit);
        let tenths = (bytes * 10 + unit / 2) / unit;
        if tenths < 10_000 || i == UNITS.len() - 1 {
            return format!("{}.{} {name}", tenths / 10, tenths % 10);
        }
    }
    unreachable!("the last unit always returns")
}

fn with_commas(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// part/total to one decimal in integer arithmetic, so every implementation rounds alike.
fn share(part: u64, total: u64) -> String {
    if total == 0 {
        return "0.0".to_string();
    }
    let tenths = (part * 1000 + total / 2) / total;
    format!("{}.{}", tenths / 10, tenths % 10)
}

const GAP: &str = "    ";
const MIN_WIDTH: usize = 7;
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn fg(c: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m", c[0], c[1], c[2])
}

fn bg(color: [u8; 3]) -> String {
    format!("\x1b[48;2;{};{};{}m", color[0], color[1], color[2])
}

// A grid lays out rows under a header: first column left-aligned, the rest right-aligned (or all left-aligned) and at least MIN_WIDTH wide, four-space gutters, a rule as wide as the header. Widths are in characters.
struct Grid {
    headers: Vec<String>,
    widths: Vec<usize>,
    left: bool, // every column left-aligned
}

impl Grid {
    fn left(headers: Vec<String>, rows: &[Vec<String>]) -> Grid {
        Grid {
            left: true,
            ..Grid::new(headers, rows)
        }
    }

    fn new(headers: Vec<String>, rows: &[Vec<String>]) -> Grid {
        let widths = headers
            .iter()
            .enumerate()
            .map(|(col, header)| {
                let mut width = header.chars().count();
                if col > 0 {
                    width = width.max(MIN_WIDTH);
                }
                rows.iter()
                    .fold(width, |width, row| width.max(row[col].chars().count()))
            })
            .collect();
        Grid {
            headers,
            widths,
            left: false,
        }
    }

    fn line(&self, row: &[String]) -> String {
        let mut out = String::new();
        for (col, cell) in row.iter().enumerate() {
            if col > 0 {
                out.push_str(GAP);
            }
            let pad = " ".repeat(self.widths[col] - cell.chars().count());
            if col == 0 || self.left {
                out.push_str(cell);
                out.push_str(&pad);
            } else {
                out.push_str(&pad);
                out.push_str(cell);
            }
        }
        out
    }

    fn width(&self) -> usize {
        self.line(&self.headers).chars().count()
    }

    fn rule(&self) -> String {
        "─".repeat(self.width())
    }
}

#[cfg(test)]
mod tests {
    use super::size;

    // The size rule from SPEC.md at its unit boundaries and rounding edges.
    #[test]
    fn sizes_follow_the_spec_rule() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1), "1 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(1000), "1.0 KB");
        assert_eq!(size(1049), "1.0 KB");
        assert_eq!(size(1050), "1.1 KB");
        assert_eq!(size(9999), "10.0 KB");
        assert_eq!(size(999949), "999.9 KB");
        assert_eq!(size(999950), "1.0 MB");
        assert_eq!(size(1000000), "1.0 MB");
        assert_eq!(size(1234567), "1.2 MB");
        assert_eq!(size(999950000), "1.0 GB");
        assert_eq!(size(1000000000), "1.0 GB");
        assert_eq!(size(1000000000000), "1.0 TB");
        assert_eq!(size(1230000000000000), "1230.0 TB");
        assert_eq!(size(18446744073709551615), "18446744.1 TB");
    }
}
