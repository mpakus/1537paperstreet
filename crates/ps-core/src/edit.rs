//! Format for Markdown and JSON editor buffers.

use std::path::Path;

use crate::projects::is_markdown_path;
use crate::{Error, Result};

/// Pretty-prints Markdown tables or JSON. Other files are left to an external editor.
pub fn format_text(rel_path: &Path, text: &str) -> Result<String> {
    if is_markdown_path(rel_path) {
        return Ok(format_markdown(text));
    }
    if is_json_path(rel_path) {
        return format_json(text);
    }
    Err(Error::FormatUnavailable)
}

fn is_json_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
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
