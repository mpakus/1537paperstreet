use pulldown_cmark::{Event, LinkType, Tag, TagEnd, html};

use crate::blocks::SpannedEvent;
use crate::links::has_scheme;

pub(crate) struct WikiHtml<I> {
    events: I,
    wiki_open: usize,
}

impl<I> WikiHtml<I> {
    pub(crate) fn new(events: I) -> Self {
        Self {
            events,
            wiki_open: 0,
        }
    }
}

impl<'input, I> Iterator for WikiHtml<I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    type Item = SpannedEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let (event, source_range) = self.events.next()?;
        match event {
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                title,
                ..
            }) => {
                self.wiki_open += 1;
                let href = wiki_href(&dest_url);
                let missing = href == "#missing-wiki";
                let class = if missing {
                    "wiki-link is-missing"
                } else {
                    "wiki-link"
                };
                let mut tag = String::from("<a href=\"");
                tag.push_str(&percent_encode_href(&href));
                tag.push_str("\" class=\"");
                tag.push_str(class);
                tag.push('"');
                if !title.is_empty() {
                    tag.push_str(" title=\"");
                    html::push_html(&mut tag, [Event::Text(title)].into_iter());
                    tag.push('"');
                }
                tag.push('>');
                Some((Event::Html(tag.into()), source_range))
            }
            Event::End(TagEnd::Link) if self.wiki_open > 0 => {
                self.wiki_open -= 1;
                Some((Event::Html("</a>".into()), source_range))
            }
            event => Some((event, source_range)),
        }
    }
}

fn ensure_markdown_href(destination: &str) -> String {
    let (path, suffix) = crate::links::split_suffix(destination);
    if path.is_empty() {
        return destination.to_owned();
    }
    if has_doc_extension(path) {
        return destination.to_owned();
    }
    format!("{path}.md{suffix}")
}

fn wiki_href(destination: &str) -> String {
    if destination.starts_with('#') || destination.starts_with("asset:") || has_scheme(destination)
    {
        destination.to_owned()
    } else {
        ensure_markdown_href(destination)
    }
}

fn has_doc_extension(path: &str) -> bool {
    let last = path.rsplit(['/', '\\']).next().unwrap_or(path);
    last.rsplit_once('.')
        .is_some_and(|(_, ext)| !ext.is_empty())
}

fn percent_encode_href(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b'/'
            | b':'
            | b'#'
            | b'?'
            | b'='
            | b'&'
            | b'+' => encoded.push(char::from(byte)),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
