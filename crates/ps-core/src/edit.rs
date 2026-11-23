//! Formatting and syntax diagnostics for editor buffers.

use std::path::Path;

use crate::projects::{config_language, is_markdown_path};
use crate::{Error, Result};

/// An exact rendered text run and the selected UTF-16 offsets within it.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct HighlightRange {
    /// UTF-8 start of the source run.
    pub start: usize,
    /// UTF-8 end of the source run.
    pub end: usize,
    /// Selection start in the displayed run (UTF-16 code units).
    pub from: usize,
    /// Selection end in the displayed run (UTF-16 code units).
    pub to: usize,
}

/// Temporary, sanitized source mapping generated only for a highlight action.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct HighlightSource {
    /// HTML containing authenticated text-run positions.
    pub html: String,
    /// BLAKE3 of the exact source buffer, also included in each mapping attribute.
    pub hash: String,
}

/// Formats Markdown tables and YAML, JSON, or TOML buffers.
pub fn format_text(rel_path: &Path, text: &str) -> Result<String> {
    if is_markdown_path(rel_path) {
        return Ok(format_markdown(text));
    }
    match config_language(rel_path) {
        Some("json") => format_json(text),
        Some("yaml") => {
            check_config("yaml", text)?;
            let mut options = pretty_yaml::config::FormatOptions::default();
            options.language.trim_trailing_whitespaces = false;
            let mut formatted = pretty_yaml::format_text(text, &options)
                .map_err(|error| config_error("YAML", text, error.offset(), error.message()))?;
            if !text.ends_with('\n') && formatted.ends_with('\n') {
                formatted.pop();
            }
            Ok(formatted)
        }
        Some("toml") => format_toml(text),
        _ => Err(Error::FormatUnavailable),
    }
}

/// A syntax diagnostic attached to a one-based editor line.
#[derive(serde::Serialize, ts_rs::TS)]
pub struct ConfigDiagnostic {
    /// One-based source line.
    pub line: usize,
    /// Human-readable parser message.
    pub message: String,
}

/// Checks a YAML or TOML editor buffer using the same validation as Format.
pub fn lint_config(rel_path: &Path, text: &str) -> Vec<ConfigDiagnostic> {
    match config_language(rel_path).map(|language| check_config(language, text)) {
        Some(Err(Error::InvalidConfigBuffer { line, message, .. })) => {
            vec![ConfigDiagnostic { line, message }]
        }
        _ => Vec::new(),
    }
}

fn check_config(language: &str, text: &str) -> Result<()> {
    // The formatting parsers recurse. Validate with bounded parsers first.
    match language {
        "yaml" => {
            let mut options = serde_saphyr::Options::default();
            options.with_snippet = false;
            serde_saphyr::from_multiple_with_options::<serde::de::IgnoredAny>(text, options)
                .map(|_| ())
                .map_err(|error| Error::InvalidConfigBuffer {
                    format: "YAML",
                    line: error
                        .location()
                        .map_or(1, |location| location.line() as usize),
                    message: error.to_string(),
                })
        }
        "toml" => parse_toml(text).map(|_| ()),
        _ => Ok(()),
    }
}

fn parse_toml(text: &str) -> Result<toml_edit::DocumentMut> {
    text.parse::<toml_edit::DocumentMut>().map_err(|error| {
        config_error(
            "TOML",
            text,
            error.span().map_or(0, |span| span.start),
            error.message(),
        )
    })
}

fn format_toml(text: &str) -> Result<String> {
    use toml_edit::visit_mut::VisitMut;

    struct FormatToml;

    impl VisitMut for FormatToml {
        fn visit_table_like_kv_mut(
            &mut self,
            mut key: toml_edit::KeyMut<'_>,
            item: &mut toml_edit::Item,
        ) {
            if let Some(value) = item.as_value_mut() {
                key.leaf_decor_mut().set_suffix(" ");
                value.decor_mut().set_prefix(" ");
            }
            self.visit_item_mut(item);
        }

        fn visit_array_mut(&mut self, array: &mut toml_edit::Array) {
            let single_line = |trivia: &toml_edit::RawString| {
                trivia
                    .as_str()
                    .is_some_and(|text| text.bytes().all(|byte| matches!(byte, b' ' | b'\t')))
            };
            // Only compact arrays whose layout contains no comments or line breaks.
            if single_line(array.trailing())
                && array.iter().all(|value| {
                    let decor = value.decor();
                    decor.prefix().is_none_or(single_line) && decor.suffix().is_none_or(single_line)
                })
            {
                array.fmt();
            }
            toml_edit::visit_mut::visit_array_mut(self, array);
        }
    }

    let mut doc = parse_toml(text)?;
    FormatToml.visit_document_mut(&mut doc);
    let mut formatted = doc.to_string();
    if !text.ends_with('\n') && formatted.ends_with('\n') {
        formatted.pop();
    }
    Ok(formatted)
}

fn config_error(format: &'static str, text: &str, offset: usize, message: &str) -> Error {
    Error::InvalidConfigBuffer {
        format,
        line: 1 + text
            .bytes()
            .take(offset)
            .filter(|byte| *byte == b'\n')
            .count(),
        message: message.to_owned(),
    }
}

fn format_json(text: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| Error::InvalidJsonBuffer)?;
    let mut pretty = serde_json::to_string_pretty(&value).map_err(|_| Error::InvalidJsonBuffer)?;
    if text.ends_with('\n') && !pretty.ends_with('\n') {
        pretty.push('\n');
    }
    Ok(pretty)
}

fn format_markdown(text: &str) -> String {
    let ends_with_newline = text.ends_with('\n');
    let mut lines: Vec<String> = line_slices(text).into_iter().map(str::to_owned).collect();
    let mut fence: Option<Fence> = None;
    let mut index = 0usize;
    while index < lines.len() {
        if let Some(open) = fence {
            if closes_fence(&lines[index], open) {
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some(open) = opens_fence(&lines[index]) {
            fence = Some(open);
            index += 1;
            continue;
        }
        let slices: Vec<&str> = lines.iter().map(String::as_str).collect();
        if let Some(table) = parse_table(&slices, index) {
            let formatted = render_table(&table);
            let next = table.start + formatted.len();
            lines.splice(table.start..table.end, formatted);
            index = next;
            continue;
        }
        index += 1;
    }
    let mut out = lines.join("\n");
    if ends_with_newline {
        out.push('\n');
    }
    out
}

#[derive(Clone, Copy)]
struct Fence {
    marker: char,
    len: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    None,
    Left,
    Center,
    Right,
}

struct Table {
    start: usize,
    end: usize,
    header: Vec<String>,
    aligns: Vec<Align>,
    rows: Vec<Vec<String>>,
}

fn line_slices(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn fence_marker(line: &str) -> Option<(Fence, &str)> {
    let (indent, rest) = split_indent(line);
    if indent > 3 {
        return None;
    }
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = rest
        .chars()
        .take_while(|character| *character == marker)
        .count();
    if len < 3 {
        return None;
    }
    let info = rest.get(len..).unwrap_or("").trim();
    Some((Fence { marker, len }, info))
}

fn opens_fence(line: &str) -> Option<Fence> {
    fence_marker(line).map(|(fence, _)| fence)
}

fn closes_fence(line: &str, open: Fence) -> bool {
    match fence_marker(line) {
        Some((close, info)) => {
            close.marker == open.marker && close.len >= open.len && info.is_empty()
        }
        None => false,
    }
}

fn split_indent(line: &str) -> (usize, &str) {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    (indent, line.get(indent..).unwrap_or(""))
}

fn parse_table(lines: &[&str], start: usize) -> Option<Table> {
    let header_line = lines.get(start)?;
    let sep_line = lines.get(start + 1)?;
    if !looks_like_table_row(header_line) || !is_separator_row(sep_line) {
        return None;
    }
    let header = split_cells(header_line);
    let aligns = split_cells(sep_line)
        .into_iter()
        .map(|cell| alignment(&cell))
        .collect::<Vec<_>>();
    if header.is_empty() || aligns.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    let mut end = start + 2;
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() || opens_fence(line).is_some() || !looks_like_table_row(line) {
            break;
        }
        if is_separator_row(line) {
            break;
        }
        rows.push(split_cells(line));
        end += 1;
    }
    Some(Table {
        start,
        end,
        header,
        aligns,
        rows,
    })
}

fn looks_like_table_row(line: &str) -> bool {
    line.contains('|')
}

fn is_separator_row(line: &str) -> bool {
    if !line.contains('|') && !line.contains('-') {
        return false;
    }
    let cells = split_cells(line);
    !cells.is_empty() && cells.iter().all(|cell| is_separator_cell(cell))
}

fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut saw_dash = false;
    for character in trimmed.chars() {
        match character {
            ':' => {}
            '-' => saw_dash = true,
            _ => return false,
        }
    }
    saw_dash
}

fn alignment(cell: &str) -> Align {
    let trimmed = cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    match (left, right) {
        (true, true) => Align::Center,
        (true, false) => Align::Left,
        (false, true) => Align::Right,
        (false, false) => Align::None,
    }
}

fn split_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let body = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let body = body.strip_suffix('|').unwrap_or(body);
    body.split('|').map(|cell| cell.trim().to_owned()).collect()
}

fn render_table(table: &Table) -> Vec<String> {
    let columns = table
        .header
        .len()
        .max(table.aligns.len())
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
    let mut widths = vec![3usize; columns];
    for (index, cell) in table.header.iter().enumerate() {
        widths[index] = widths[index].max(cell.chars().count());
    }
    for row in &table.rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }
    let mut lines = Vec::with_capacity(2 + table.rows.len());
    lines.push(render_row(
        &pad_row(&table.header, columns),
        &widths,
        &table.aligns,
    ));
    lines.push(render_separator(&widths, &table.aligns, columns));
    for row in &table.rows {
        lines.push(render_row(&pad_row(row, columns), &widths, &table.aligns));
    }
    lines
}

fn pad_row(row: &[String], columns: usize) -> Vec<String> {
    let mut cells = row.to_vec();
    cells.resize(columns, String::new());
    cells
}

fn render_row(cells: &[String], widths: &[usize], aligns: &[Align]) -> String {
    let mut line = String::from("|");
    for (index, cell) in cells.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(0);
        let align = aligns.get(index).copied().unwrap_or(Align::None);
        line.push(' ');
        line.push_str(&pad_cell(cell, width, align));
        line.push_str(" |");
    }
    line
}

fn render_separator(widths: &[usize], aligns: &[Align], columns: usize) -> String {
    let mut line = String::from("|");
    for index in 0..columns {
        let width = widths.get(index).copied().unwrap_or(3).max(3);
        let align = aligns.get(index).copied().unwrap_or(Align::None);
        line.push(' ');
        line.push_str(&separator_cell(width, align));
        line.push_str(" |");
    }
    line
}

fn pad_cell(cell: &str, width: usize, align: Align) -> String {
    let len = cell.chars().count();
    let extra = width.saturating_sub(len);
    match align {
        Align::Right => format!("{}{cell}", " ".repeat(extra)),
        Align::Center => {
            let left = extra / 2;
            let right = extra - left;
            format!("{}{cell}{}", " ".repeat(left), " ".repeat(right))
        }
        Align::Left | Align::None => format!("{cell}{}", " ".repeat(extra)),
    }
}

fn separator_cell(width: usize, align: Align) -> String {
    match align {
        Align::Left => format!(":{}", "-".repeat(width.saturating_sub(1).max(3))),
        Align::Right => format!("{}:", "-".repeat(width.saturating_sub(1).max(3))),
        Align::Center => format!(":{}:", "-".repeat(width.saturating_sub(2).max(3))),
        Align::None => "-".repeat(width.max(3)),
    }
}
