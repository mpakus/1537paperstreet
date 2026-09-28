use std::collections::VecDeque;
use std::ops::Range;

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::blocks::SpannedEvent;

/// Rewrites GFM task items into checkboxes the preview can toggle.
///
/// Tight items emit the marker immediately. Loose items wrap it in a
/// paragraph, so the marker is hoisted in front of that paragraph and the
/// item still gets `task-list-item`.
pub(crate) struct TaskLists<'input, I> {
    events: I,
    pending: VecDeque<SpannedEvent<'input>>,
    html_items: Vec<bool>,
}

impl<'input, I> TaskLists<'input, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    pub(crate) fn new(events: I) -> Self {
        Self {
            events,
            pending: VecDeque::new(),
            html_items: Vec::new(),
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

impl<'input, I> Iterator for TaskLists<'input, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    type Item = SpannedEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let (event, source_range) = self.pop()?;
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
