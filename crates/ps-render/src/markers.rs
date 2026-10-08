use std::ops::Range;

use ps_core::edit::{HighlightRange, HighlightSource};
use ps_core::{Error, Result};
use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd, html};

use crate::blocks::SpannedEvent;

const PRESETS: [(&str, &str); 6] = [
    ("red", "🔴"),
    ("orange", "🟠"),
    ("yellow", "🟡"),
    ("green", "🟢"),
    ("blue", "🔵"),
    ("purple", "🟣"),
];

#[derive(Clone)]
struct Marker {
    start: usize,
    content: Range<usize>,
    end: usize,
    color: String,
}

/// Builds a temporary source map without adding DOM nodes to ordinary previews.
#[must_use]
pub fn highlight_source(markdown: &str) -> HighlightSource {
    HighlightSource {
        html: crate::pipeline::render_mapped(markdown).html,
        hash: blake3::hash(markdown.as_bytes()).to_hex().to_string(),
    }
}

fn color(value: &str) -> Result<String> {
    if value == "default" || PRESETS.iter().any(|(name, _)| *name == value) {
        return Ok(value.to_owned());
    }
    let hex = value
        .strip_prefix('#')
        .ok_or(Error::InvalidHighlightColor)?;
    if !matches!(hex.len(), 3 | 6) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::InvalidHighlightColor);
    }
    let hex = hex.to_ascii_lowercase();
    Ok(if hex.len() == 3 {
        format!("#{}", hex.chars().flat_map(|c| [c, c]).collect::<String>())
    } else {
        format!("#{hex}")
    })
}

fn opening(color: &str) -> String {
    if color == "default" {
        "==".to_owned()
    } else if let Some((_, emoji)) = PRESETS.iter().find(|(name, _)| *name == color) {
        format!("=={emoji}")
    } else {
        format!("=={{{color}}}")
    }
}

fn prefix(source: &str, start: usize) -> Option<(String, usize)> {
    let rest = source.get(start + 2..)?;
    if let Some(hex) = rest.strip_prefix('{') {
        let end = hex.find('}')?;
        return Some((color(&hex[..end]).ok()?, start + 2 + end + 2));
    }
    for (name, emoji) in PRESETS {
        if rest.starts_with(emoji) {
            return Some((name.to_owned(), start + 2 + emoji.len()));
        }
    }
    Some(("default".to_owned(), start + 2))
}

fn inline_start(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. }
    )
}

fn inline_end(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link
    )
}

#[derive(Default)]
struct Editable {
    stack: Vec<bool>,
    excluded: usize,
}

impl Editable {
    fn includes(&mut self, event: &Event<'_>) -> bool {
        match event {
            Event::Start(tag) => {
                let skip = matches!(
                    tag,
                    Tag::CodeBlock(_) | Tag::Image { .. } | Tag::MetadataBlock(_)
                ) || matches!(tag, Tag::Link { link_type, .. } if !matches!(link_type, LinkType::Inline | LinkType::Reference | LinkType::ReferenceUnknown));
                self.stack.push(skip);
                self.excluded += usize::from(skip);
            }
            Event::End(_) => {
                self.excluded -= usize::from(self.stack.pop().unwrap_or(false));
            }
            _ => {}
        }
        self.excluded == 0
    }
}

fn markers(events: &[SpannedEvent<'_>], source: &str) -> Vec<Marker> {
    let mut found = Vec::new();
    let mut pending = None;
    let mut editable = Editable::default();
    for (event, range) in events {
        let allowed = editable.includes(event);
        if matches!(event, Event::Start(tag) if !inline_start(tag))
            || matches!(event, Event::End(tag) if !inline_end(tag))
        {
            pending = None;
        }
        let Event::Text(text) = event else { continue };
        if !allowed || source.get(range.clone()) != Some(text.as_ref()) {
            continue;
        }
        for (offset, _) in text.match_indices("==") {
            let at = range.start + offset;
            let escapes = source.as_bytes()[..at]
                .iter()
                .rev()
                .take_while(|c| **c == b'\\')
                .count();
            if escapes % 2 != 0 {
                continue;
            }
            if let Some(start) = pending.take() {
                if let Some((color, content_start)) = prefix(source, start)
                    && content_start < at
                {
                    found.push(Marker {
                        start,
                        content: content_start..at,
                        end: at + 2,
                        color,
                    });
                }
            } else {
                pending = Some(at);
            }
        }
    }
    found
}

/// Splits a text event at marker delimiters while preserving parser source ranges.
fn pieces<'a, 'm>(
    text: &pulldown_cmark::CowStr<'a>,
    range: &Range<usize>,
    source: &str,
    marks: &'m [Marker],
) -> Vec<(pulldown_cmark::CowStr<'a>, Range<usize>, Option<&'m Marker>)> {
    let first = marks.partition_point(|m| m.end <= range.start);
    let relevant: Vec<_> = marks[first..]
        .iter()
        .take_while(|m| m.start < range.end)
        .collect();
    if relevant.is_empty() {
        return vec![(text.clone(), range.clone(), None)];
    }
    if source.get(range.clone()) != Some(text.as_ref()) {
        return vec![(text.clone(), range.clone(), relevant.first().copied())];
    }
    let mut bounds = vec![range.start, range.end];
    for marker in &relevant {
        for at in [
            marker.start,
            marker.content.start,
            marker.content.end,
            marker.end,
        ] {
            if at > range.start && at < range.end {
                bounds.push(at);
            }
        }
    }
    bounds.sort_unstable();
    bounds.dedup();
    bounds
        .windows(2)
        .filter_map(|pair| {
            let part = pair[0]..pair[1];
            let marker = relevant
                .iter()
                .find(|m| m.start <= part.start && m.end >= part.end);
            if marker.is_some_and(|m| part.start < m.content.start || part.end > m.content.end) {
                return None;
            }
            Some((
                source[part.clone()].to_owned().into(),
                part,
                marker.copied(),
            ))
        })
        .collect()
}

fn mark_html(color: &str) -> String {
    if let Some(hex) = color.strip_prefix('#') {
        let rgb = u32::from_str_radix(hex, 16).unwrap_or(0);
        let luminance = |channel: u32| {
            let v = f64::from(channel) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        let l = 0.2126 * luminance((rgb >> 16) & 255)
            + 0.7152 * luminance((rgb >> 8) & 255)
            + 0.0722 * luminance(rgb & 255);
        let fg = if l > 0.179 { "#000000" } else { "#ffffff" };
        format!(
            "<mark class=\"text-highlight\" data-highlight=\"{color}\" style=\"background-color:{color};color:{fg}\">"
        )
    } else {
        format!("<mark class=\"text-highlight\" data-highlight=\"{color}\">")
    }
}

/// The only inline style accepted for a marker, including raw HTML input.
pub(crate) fn safe_style(value: &str) -> bool {
    let Some((bg, fg)) = value
        .strip_prefix("background-color:")
        .and_then(|v| v.split_once(";color:"))
    else {
        return false;
    };
    bg.starts_with('#') && bg.len() == 7 && color(bg).is_ok() && matches!(fg, "#000000" | "#ffffff")
}

pub(crate) struct Markers<'a, I> {
    original: Option<I>,
    mapped: std::vec::IntoIter<SpannedEvent<'a>>,
}

impl<'a, I: Iterator<Item = SpannedEvent<'a>>> Markers<'a, I> {
    pub(crate) fn new(events: I, source: &str, mapping: bool) -> Self {
        if !mapping && !source.contains("==") {
            return Self {
                original: Some(events),
                mapped: Vec::new().into_iter(),
            };
        }
        let events: Vec<_> = events.collect();
        let marks = markers(&events, source);
        if marks.is_empty() && !mapping {
            return Self {
                original: None,
                mapped: events.into_iter(),
            };
        }
        let hash = if mapping {
            blake3::hash(source.as_bytes()).to_hex().to_string()
        } else {
            String::new()
        };
        let mut output = Vec::with_capacity(events.len());
        let mut editable = Editable::default();
        let mut open: Option<&Marker> = None;
        for (event, range) in events {
            let allowed = editable.includes(&event);
            if let Event::Text(ref text) = event
                && allowed
            {
                for (text, part, marker) in pieces(text, &range, source, &marks) {
                    if open.map(|m| m.start) != marker.map(|m| m.start) {
                        if open.is_some() {
                            output.push((Event::InlineHtml("</mark>".into()), part.clone()));
                        }
                        if let Some(mark) = marker {
                            output.push((
                                Event::InlineHtml(mark_html(&mark.color).into()),
                                part.clone(),
                            ));
                        }
                        open = marker;
                    }
                    if mapping {
                        output.push((
                            Event::InlineHtml(
                                format!(
                                    "<span data-ps-map=\"{hash}:{}:{}\">",
                                    part.start, part.end
                                )
                                .into(),
                            ),
                            part.clone(),
                        ));
                    }
                    output.push((Event::Text(text), part.clone()));
                    if mapping {
                        output.push((Event::InlineHtml("</span>".into()), part));
                    }
                }
            } else {
                let inside = matches!(event, Event::SoftBreak | Event::HardBreak)
                    && open.is_some_and(|m| {
                        m.content.start <= range.start && range.end <= m.content.end
                    });
                if !inside && open.take().is_some() {
                    output.push((Event::InlineHtml("</mark>".into()), range.clone()));
                }
                output.push((event, range));
            }
        }
        Self {
            original: None,
            mapped: output.into_iter(),
        }
    }
}

impl<'a, I: Iterator<Item = SpannedEvent<'a>>> Iterator for Markers<'a, I> {
    type Item = SpannedEvent<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(original) = &mut self.original {
            original.next()
        } else {
            self.mapped.next()
        }
    }
}

fn utf16_byte(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units == offset {
            return Some(byte);
        }
        units += c.len_utf16();
    }
    (units == offset).then_some(text.len())
}

/// Applies validated marker edits to a buffer; never writes a user file.
/// Selecting any part of an existing marker recolors/removes that entire marker.
pub fn apply_highlight(
    source: &str,
    ranges: &[HighlightRange],
    requested: Option<&str>,
) -> Result<String> {
    let requested = requested.map(color).transpose()?;
    if ranges.is_empty() {
        return Err(Error::InvalidHighlightSelection);
    }
    let normalized = crate::pipeline::normalize_parser_input(source);
    let events: Vec<_> = Parser::new_ext(&normalized, crate::pipeline::markdown_options())
        .into_offset_iter()
        .collect();
    let marks = markers(&events, source);
    let mut runs = Vec::new();
    let mut editable = Editable::default();
    let mut group = 0;
    for (event, range) in &events {
        let allowed = editable.includes(event);
        if !allowed
            || (!matches!(event, Event::Text(_) | Event::SoftBreak | Event::HardBreak)
                && !matches!(event, Event::Start(tag) if inline_start(tag))
                && !matches!(event, Event::End(tag) if inline_end(tag)))
        {
            group += 1;
        }
        if let Event::Text(text) = event
            && allowed
        {
            runs.extend(
                pieces(text, range, source, &marks)
                    .into_iter()
                    .map(|(text, run, marker)| (text, run, marker, group)),
            );
        }
    }
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut touched = std::collections::BTreeSet::new();
    let mut selected = Vec::new();
    let mut previous = 0;
    for selection in ranges {
        let (text, run, marker, group) = runs
            .iter()
            .find(|(_, run, _, _)| run.start == selection.start && run.end == selection.end)
            .ok_or(Error::InvalidHighlightSelection)?;
        let from = utf16_byte(text, selection.from).ok_or(Error::InvalidHighlightSelection)?;
        let to = utf16_byte(text, selection.to).ok_or(Error::InvalidHighlightSelection)?;
        if from >= to {
            return Err(Error::InvalidHighlightSelection);
        }
        let mut part = if source.get(run.clone()) == Some(text.as_ref()) {
            run.start + from..run.start + to
        } else if from == 0 && to == text.len() {
            run.clone()
        } else {
            return Err(Error::InvalidHighlightSelection);
        };
        if part.start == run.start
            && source.as_bytes()[..part.start]
                .iter()
                .rev()
                .take_while(|c| **c == b'\\')
                .count()
                % 2
                == 1
        {
            part.start -= 1;
        }
        if part.start < previous {
            return Err(Error::InvalidHighlightSelection);
        }
        previous = part.end;
        if text[from..to].trim().is_empty() {
            continue;
        }
        if let Some(mark) = marker {
            if touched.insert(mark.start) {
                selected.push((mark.start..mark.end, *group));
            }
        } else if requested.is_some() {
            selected.push((part, *group));
        }
    }

    selected.sort_by_key(|(range, _)| range.start);
    let mut merged: Vec<(Range<usize>, usize)> = Vec::new();
    for (part, group) in selected {
        if let Some((last, last_group)) = merged.last_mut()
            && *last_group == group
            && runs.iter().all(|(text, run, _, _)| {
                let from = run.start.max(last.end);
                let to = run.end.min(part.start);
                from >= to
                    || if source.get(run.clone()) == Some(text.as_ref()) {
                        source[from..to].trim().is_empty()
                    } else {
                        text.trim().is_empty()
                    }
            })
        {
            last.end = last.end.max(part.end);
        } else {
            merged.push((part, group));
        }
    }
    for (part, _) in merged {
        let mut content = source[part.clone()].to_owned();
        for mark in marks
            .iter()
            .rev()
            .filter(|m| part.start <= m.start && m.end <= part.end)
        {
            content.replace_range(mark.content.end - part.start..mark.end - part.start, "");
            content.replace_range(mark.start - part.start..mark.content.start - part.start, "");
        }
        let replacement = requested.as_deref().map_or_else(
            || content.clone(),
            |color| format!("{}{content}==", opening(color)),
        );
        edits.push((part, replacement));
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut output = source.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    // Delimiters must not change emphasis, links, escapes, or visible text.
    if unmarked_html(source) != unmarked_html(&output) {
        return Err(Error::InvalidHighlightSelection);
    }
    Ok(output)
}

fn unmarked_html(source: &str) -> String {
    let normalized = crate::pipeline::normalize_parser_input(source);
    let events =
        Parser::new_ext(&normalized, crate::pipeline::markdown_options()).into_offset_iter();
    let events = Markers::new(events, source, false).filter_map(|(event, _)| {
        if matches!(&event, Event::InlineHtml(tag) if tag.starts_with("<mark class=\"text-highlight\"") || tag.as_ref() == "</mark>") {
            None
        } else {
            Some(event)
        }
    });
    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}
