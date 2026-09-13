// The report and its two renderings, SPEC.md Output. The JSON is the conformance format; the table is for people.

use std::fmt::Write;

use crate::lang::LANGS;
use crate::walk::Sums;

pub struct LangRow {
    name: &'static str,
    files: usize,
    lines: usize,
    blank: usize,
    comment: usize,
    code: usize,
    color: [u8; 3],
}

#[derive(Default, Clone, Copy)]
pub struct Totals {
    files: usize,
    lines: usize,
    blank: usize,
    comment: usize,
    code: usize,
}

pub struct SkippedLabel {
    label: String,
    files: usize,
}

pub struct Report {
    languages: Vec<LangRow>,
    total: Totals,
    skipped_files: usize,
    skipped: Vec<SkippedLabel>,
}

impl Report {
    // Orders the sums as the spec requires: languages by code descending then name, labels by files descending then label, both byte-wise.
    pub fn new(sums: Sums) -> Report {
        let mut report = Report {
            languages: Vec::new(),
            total: Totals::default(),
            skipped_files: 0,
            skipped: Vec::new(),
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
                color: lang.color,
            });
            report.total.files += sums.files[i];
            report.total.lines += counts.lines;
            report.total.blank += counts.blank;
            report.total.comment += counts.comment;
            report.total.code += counts.code;
        }
        report
            .languages
            .sort_by(|a, b| b.code.cmp(&a.code).then_with(|| a.name.cmp(b.name)));
        for (label, files) in sums.skipped {
            report.skipped.push(SkippedLabel { label, files });
            report.skipped_files += files;
        }
        report
            .skipped
            .sort_by(|a, b| b.files.cmp(&a.files).then_with(|| a.label.cmp(&b.label)));
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
                "{{\"name\":{},\"files\":{},\"lines\":{},\"blank\":{},\"comment\":{},\"code\":{}}}",
                json_str(row.name),
                row.files,
                row.lines,
                row.blank,
                row.comment,
                row.code
            );
        }
        let total = self.total;
        let _ = write!(
            out,
            "],\"total\":{{\"files\":{},\"lines\":{},\"blank\":{},\"comment\":{},\"code\":{}}},\"skipped\":{{\"files\":{},\"labels\":[",
            total.files, total.lines, total.blank, total.comment, total.code, self.skipped_files
        );
        for (i, row) in self.skipped.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"label\":{},\"files\":{}}}",
                json_str(&row.label),
                row.files
            );
        }
        out.push_str("]}}\n");
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
                share(totals.code, self.total.code),
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

        let all = self.total.files + self.skipped_files;
        out.push(style(
            DIM,
            format!(
                "{} of {} files skipped ({}%)",
                with_commas(self.skipped_files),
                with_commas(all),
                share(self.skipped_files, all)
            ),
        ));
        if !self.skipped.is_empty() {
            let shown = self.skipped.len().min(10);
            let mut rows: Vec<Vec<String>> = self.skipped[..shown]
                .iter()
                .map(|skipped| {
                    vec![
                        skipped.label.clone(),
                        with_commas(skipped.files),
                        share(skipped.files, all),
                    ]
                })
                .collect();
            if self.skipped.len() > 10 {
                let rest: usize = self.skipped[10..].iter().map(|skipped| skipped.files).sum();
                rows.push(vec![
                    format!("{} more labels", with_commas(self.skipped.len() - 10)),
                    with_commas(rest),
                    share(rest, all),
                ]);
            }
            let skipped_grid = Grid::new(
                ["Skipped", "Files", "%"]
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
        out.join("\n") + "\n"
    }
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
fn share(part: usize, total: usize) -> String {
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

// A grid lays out rows under a header: first column left-aligned, the rest right-aligned and at least MIN_WIDTH wide, four-space gutters, a rule as wide as the header. Widths are in characters.
struct Grid {
    headers: Vec<String>,
    widths: Vec<usize>,
}

impl Grid {
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
        Grid { headers, widths }
    }

    fn line(&self, row: &[String]) -> String {
        let mut out = String::new();
        for (col, cell) in row.iter().enumerate() {
            if col > 0 {
                out.push_str(GAP);
            }
            let pad = " ".repeat(self.widths[col] - cell.chars().count());
            if col == 0 {
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
