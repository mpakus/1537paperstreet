use std::collections::VecDeque;
use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

use crate::blocks::SpannedEvent;

/// Rewrites GFM task items into checkboxes the preview can toggle.
///
/// Tight items emit the marker immediately. Loose items wrap it in a
/// paragraph, so the marker is hoisted in front of that paragraph and the
/// item still gets `task-list-item`. A paragraph whose lines are bare
/// `[ ]` / `[x]` markers (no list dash) becomes the same checkbox list.
pub(crate) struct TaskLists<'input, 'source, I> {
    events: I,
    markdown: &'source str,
    pending: VecDeque<SpannedEvent<'input>>,
    html_items: Vec<bool>,
    skip_paragraph: bool,
}

impl<'input, 'source, I> TaskLists<'input, 'source, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    pub(crate) fn new(events: I, markdown: &'source str) -> Self {
        Self {
            events,
            markdown,
            pending: VecDeque::new(),
            html_items: Vec::new(),
            skip_paragraph: false,
        }
    }

    fn fill(&mut self, count: usize) {
        while self.pending.len() < count {
            let Some(event) = self.events.next() else {
                return;
            };
            self.pending.push_back(event);
        }
    }

    fn pop(&mut self) -> Option<SpannedEvent<'input>> {
        if let Some(event) = self.pending.pop_front() {
            return Some(event);
        }
        self.events.next()
    }

    /// The next event is a task marker, or a paragraph whose first child is.
    fn lookahead_is_task(&mut self) -> bool {
        self.fill(1);
        if matches!(
            self.pending.front().map(|(event, _)| event),
            Some(Event::TaskListMarker(_))
        ) {
            return true;
        }
        let paragraph = matches!(
            self.pending.front().map(|(event, _)| event),
            Some(Event::Start(Tag::Paragraph))
        );
        if !paragraph {
            return false;
        }
        self.fill(2);
        matches!(
            self.pending.get(1).map(|(event, _)| event),
            Some(Event::TaskListMarker(_))
        )
    }

    /// Takes a buffered task marker without consuming anything else.
    fn pop_task_marker(&mut self) -> Option<(bool, Range<usize>)> {
        self.fill(1);
        let is_marker = matches!(
            self.pending.front().map(|(event, _)| event),
            Some(Event::TaskListMarker(_))
        );
        if !is_marker {
            return None;
        }
        match self.pending.pop_front() {
            Some((Event::TaskListMarker(checked), range)) => Some((checked, range)),
            Some(other) => {
                self.pending.push_front(other);
                None
            }
            None => None,
        }
    }
}

impl<'input, 'source, I> Iterator for TaskLists<'input, 'source, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    type Item = SpannedEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (event, source_range) = self.pop()?;
            if self.skip_paragraph {
                if matches!(event, Event::End(TagEnd::Paragraph)) {
                    self.skip_paragraph = false;
                }
                continue;
            }
            if let Some(item) = self.map_event(event, source_range) {
                return Some(item);
            }
        }
    }
}

impl<'input, 'source, I> TaskLists<'input, 'source, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    fn map_event(
        &mut self,
        event: Event<'input>,
        source_range: Range<usize>,
    ) -> Option<SpannedEvent<'input>> {
        match event {
            Event::Start(Tag::Item) => {
                let is_task = self.lookahead_is_task();
                self.html_items.push(is_task);
                if is_task {
                    Some((
                        Event::Html("<li class=\"task-list-item\">".into()),
                        source_range,
                    ))
                } else {
                    Some((Event::Start(Tag::Item), source_range))
                }
            }
            Event::End(TagEnd::Item) => {
                if self.html_items.pop().unwrap_or(false) {
                    Some((Event::Html("</li>\n".into()), source_range))
                } else {
                    Some((Event::End(TagEnd::Item), source_range))
                }
            }
            Event::Start(Tag::Paragraph) => {
                if self.html_items.last().copied() != Some(true)
                    && let Some(html) = bare_task_list_html(self.markdown, &source_range)
                {
                    self.skip_paragraph = true;
                    return Some((Event::Html(html.into()), source_range));
                }
                if self.html_items.last().copied() == Some(true)
                    && let Some((checked, range)) = self.pop_task_marker()
                {
                    self.pending
                        .push_front((Event::Start(Tag::Paragraph), source_range));
                    return Some(checkbox_event(checked, range));
                }
                Some((Event::Start(Tag::Paragraph), source_range))
            }
            Event::TaskListMarker(checked) => Some(checkbox_event(checked, source_range)),
            event => Some((event, source_range)),
        }
    }
}

fn checkbox_event(checked: bool, source_range: Range<usize>) -> SpannedEvent<'static> {
    let checked_attr = if checked { " checked=\"\"" } else { "" };
    let html = format!(
        "<input type=\"checkbox\" data-task-at=\"{}\"{checked_attr}/>\n",
        source_range.start
    );
    (Event::Html(html.into()), source_range)
}

/// A paragraph made only of `[ ]` / `[x]` lines, without a list marker.
fn bare_task_list_html(markdown: &str, range: &Range<usize>) -> Option<String> {
    let source = markdown.get(range.start..range.end)?;
    let mut lines: Vec<&str> = source.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return None;
    }
    let mut items = Vec::with_capacity(lines.len());
    let mut line_at = range.start;
    for (index, line) in lines.iter().enumerate() {
        let body = line.strip_suffix('\r').unwrap_or(line);
        let (checked, indent, label) = parse_bare_task_line(body)?;
        items.push((checked, line_at + indent, label));
        line_at += line.len();
        if index + 1 < lines.len() {
            line_at += 1;
        }
    }
    let mut html = String::from("<ul>\n");
    for (checked, at, label) in items {
        let checked_attr = if checked { " checked=\"\"" } else { "" };
        html.push_str(&format!(
            "<li class=\"task-list-item\"><input type=\"checkbox\" data-task-at=\"{at}\"{checked_attr}/>\n{}</li>\n",
            task_label_html(label)
        ));
    }
    html.push_str("</ul>\n");
    Some(html)
}

/// `[ ] label` or `[x] label` at the start of a line. Indent is the byte offset of `[`.
fn parse_bare_task_line(line: &str) -> Option<(bool, usize, &str)> {
    let rest = line.trim_start_matches([' ', '\t']);
    let indent = line.len() - rest.len();
    let bytes = rest.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'[' || bytes[2] != b']' {
        return None;
    }
    let checked = match bytes[1] {
        b' ' => false,
        b'x' | b'X' => true,
        _ => return None,
    };
    let after = rest.get(3..)?;
    if !after.is_empty() && !after.starts_with([' ', '\t']) {
        return None;
    }
    Some((checked, indent, after.trim_start_matches([' ', '\t'])))
}

fn task_label_html(label: &str) -> String {
    if label.is_empty() {
        return String::new();
    }
    let mut output = String::new();
    let parser = Parser::new_ext(
        label,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_SMART_PUNCTUATION,
    );
    html::push_html(&mut output, parser);
    let trimmed = output.trim();
    trimmed
        .strip_prefix("<p>")
        .and_then(|rest| rest.strip_suffix("</p>"))
        .unwrap_or(trimmed)
        .to_string()
}
