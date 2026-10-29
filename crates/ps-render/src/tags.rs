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
    let mut underscores = Vec::new();
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
                flush(text, &mut prose, &underscores, &mut tags, &mut seen);
                excluded = 1;
            }
            Event::Text(_) => {
                if let Some(previous) = &mut prose {
                    if range.start >= previous.end && markers(&text[previous.end..range.start]) {
                        previous.end = range.end;
                    } else {
                        flush(text, &mut prose, &underscores, &mut tags, &mut seen);
                        prose = Some(range);
                    }
                } else {
                    prose = Some(range);
                }
            }
            Event::Start(Tag::Emphasis | Tag::Strong) => {
                let width = if matches!(event, Event::Start(Tag::Strong)) {
                    2
                } else {
                    1
                };
                if text.as_bytes()[range.start] == b'_' {
                    underscores
                        .extend((range.start..range.start + width).map(|p| (p, range.start)));
                }
            }
            Event::Start(Tag::Strikethrough) => {}
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {
                let width = if matches!(event, Event::End(TagEnd::Strong)) {
                    2
                } else {
                    1
                };
                if text.as_bytes()[range.end - 1] == b'_' {
                    underscores.extend((range.end - width..range.end).map(|p| (p, range.start)));
                }
                if let Some(previous) = &mut prose
                    && range.end >= previous.end
                    && markers(&text[previous.end..range.end])
                {
                    previous.end = range.end;
                }
            }
            _ => flush(text, &mut prose, &underscores, &mut tags, &mut seen),
        }
        if tags.len() >= 257 {
            return tags;
        }
    }
    flush(text, &mut prose, &underscores, &mut tags, &mut seen);
    tags
}

fn markers(value: &str) -> bool {
    value.chars().all(|c| matches!(c, '*' | '_' | '~'))
}

// Delimiter positions arrive in source order. Only emphasis that starts before
// the hash surrounds a tag: #_foo_ keeps its underscores, while _#foo_ does not.
fn enclosing_underscore(positions: &[(usize, usize)], position: usize, tag_start: usize) -> bool {
    positions
        .binary_search_by_key(&position, |&(offset, _)| offset)
        .is_ok_and(|index| positions[index].1 < tag_start)
}

// Keep contiguous prose source through emphasis boundaries: a legal tag such
// as #foo_bar_# must not become foo_bar when Markdown splits its Text events.
fn flush(
    text: &str,
    prose: &mut Option<Range<usize>>,
    underscores: &[(usize, usize)],
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
        // Markdown's actual underscore delimiters are boundaries; literal
        // underscores in names such as #snake_case remain part of the tag.
        if !enclosing_underscore(underscores, absolute.saturating_sub(1), absolute)
            && before.is_some_and(|c| {
                c == '\\' || c == '#' || c == '/' || c == ':' || c == '=' || word(c)
            })
        {
            continue;
        }
        let tail = &source[cursor..];
        let single = tail
            .char_indices()
            .find_map(|(offset, c)| {
                ((!word(c) && c != '/')
                    || (c == '_'
                        && enclosing_underscore(underscores, absolute + 1 + offset, absolute)))
                .then_some(offset)
            })
            .unwrap_or(tail.len());
        // Closed tags may contain spaces and punctuation. A hash after
        // whitespace starts another tag; it does not close this one.
        let closing = tail.find(['#', '\n', '\r']).filter(|&end| {
            tail.as_bytes()[end] == b'#'
                && end > 0
                && tail[end + 1..].chars().next().is_none_or(|c| {
                    (!word(c) && c != '/')
                        || (c == '_'
                            && enclosing_underscore(underscores, absolute + end + 2, absolute))
                })
                && !tail[..end].ends_with(char::is_whitespace)
                && !tail.starts_with(char::is_whitespace)
        });
        let end = closing.unwrap_or(single);
        if end == 0 {
            continue;
        }
        if closing.is_none() && tail[..end].chars().all(|c| c.is_numeric() || c == '/') {
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
