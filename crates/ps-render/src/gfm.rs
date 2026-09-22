use std::iter::Peekable;

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::blocks::SpannedEvent;

pub(crate) struct TaskLists<I: Iterator> {
    events: Peekable<I>,
    html_items: Vec<bool>,
}

impl<I: Iterator> TaskLists<I> {
    pub(crate) fn new(events: I) -> Self {
        Self {
            events: events.peekable(),
            html_items: Vec::new(),
        }
    }
}

impl<'input, I> Iterator for TaskLists<I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    type Item = SpannedEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let (event, source_range) = self.events.next()?;
        match event {
            Event::Start(Tag::Item) => {
                let is_task = matches!(self.events.peek(), Some((Event::TaskListMarker(_), _)));
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
            Event::TaskListMarker(checked) => {
                let checked_attr = if checked { " checked=\"\"" } else { "" };
                let html = format!(
                    "<input type=\"checkbox\" data-task-at=\"{}\"{checked_attr}/>\n",
                    source_range.start
                );
                Some((Event::Html(html.into()), source_range))
            }
            event => Some((event, source_range)),
        }
    }
}
