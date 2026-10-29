use std::collections::HashSet;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Extracts Bear-style inline tags from prose, retaining source escapes.
/// Returns at most 257 distinct tags (the last is a truncation sentinel).
pub fn inline_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = HashSet::new();
    let mut excluded = 0usize;
    let mut prose: Option<Range<usize>> = None;
    for (event, range) in
        Parser::new_ext(text, crate::pipeline::markdown_options()).into_offset_iter()
    {
        if excluded > 0 {
            match event {
                Event::Start(_) => excluded += 1,
                Event::End(_) => excluded -= 1,
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(
                Tag::CodeBlock(_)
                | Tag::MetadataBlock(_)
                | Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::HtmlBlock,
            ) => {
                flush(text, &mut prose, &mut tags, &mut seen);
                excluded = 1;
            }
            Event::Text(_) => {
                if let Some(previous) = &mut prose {
                    if range.start >= previous.end && markers(&text[previous.end..range.start]) {
                        previous.end = range.end;
                    } else {
                        flush(text, &mut prose, &mut tags, &mut seen);
                        prose = Some(range);
                    }
                } else {
                    prose = Some(range);
                }
            }
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {}
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {
                if let Some(previous) = &mut prose
                    && range.end >= previous.end
                    && markers(&text[previous.end..range.end])
                {
                    previous.end = range.end;
                }
            }
            _ => flush(text, &mut prose, &mut tags, &mut seen),
        }
        if tags.len() >= 257 {
            return tags;
        }
    }
    flush(text, &mut prose, &mut tags, &mut seen);
    tags
}

fn markers(value: &str) -> bool {
    value.chars().all(|c| matches!(c, '*' | '_' | '~'))
}

// Keep contiguous prose source through emphasis boundaries: a legal tag such
// as #foo_bar_# must not become foo_bar when Markdown splits its Text events.
fn flush(
    text: &str,
    prose: &mut Option<Range<usize>>,
    tags: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(range) = prose.take() else {
        return;
    };
    if tags.len() >= 257 {
        return;
    }
    let source = &text[range.clone()];
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find('#') {
        let start = cursor + relative;
        cursor = start + 1;
        let absolute = range.start + start;
        let before = text[..absolute].chars().next_back();
        if before
            .is_some_and(|c| c == '\\' || c == '#' || c == '/' || c == ':' || c == '=' || word(c))
        {
            continue;
        }
        let tail = &source[cursor..];
        let single = tail.find(|c| !word(c) && c != '/').unwrap_or(tail.len());
        // Closed tags may contain spaces and punctuation. A hash after
        // whitespace starts another tag; it does not close this one.
        let closing = tail.find(['#', '\n', '\r']).filter(|&end| {
            tail.as_bytes()[end] == b'#'
                && end > 0
                && tail[end + 1..]
                    .chars()
                    .next()
                    .is_none_or(|c| !word(c) && c != '/')
                && !tail[..end].ends_with(char::is_whitespace)
                && !tail.starts_with(char::is_whitespace)
        });
        let end = closing.unwrap_or(single);
        if end == 0 {
            continue;
        }
        if closing.is_none() && tail[..end].chars().all(char::is_numeric) {
            cursor += end;
            continue;
        }
        let candidate = &tail[..end];
        if let Some(tag) = ps_core::tags::normalize_tag(candidate)
            && seen.insert(tag.to_lowercase())
        {
            tags.push(tag);
        }
        cursor += end;
        if source[cursor..].starts_with('#') {
            cursor += 1;
        }
        if tags.len() >= 257 {
            return;
        }
    }
}

fn word(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || c == '-'
        || (!c.is_ascii()
            && !c.is_whitespace()
            && !c.is_control()
            && !matches!(
                c,
                '。' | '、'
                    | '，'
                    | '！'
                    | '？'
                    | '：'
                    | '；'
                    | '（'
                    | '）'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
            ))
}
