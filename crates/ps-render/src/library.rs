use ps_core::library::NoteSection;
use pulldown_cmark::{Event, Parser, Tag};

/// Finds selectable Markdown sections using the preview's parser and heading IDs.
/// Nested headings are included until the next heading at the same or higher level.
pub fn library_sections(text: &str) -> Vec<NoteSection> {
    let mut toc = Vec::new();
    let events = Parser::new_ext(text, crate::pipeline::markdown_options()).into_offset_iter();
    let headings = crate::toc::HeadingIds::new(events, &mut toc);
    let mut positions = Vec::new();
    for (event, range) in headings {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            positions.push((level as u8, range.start));
        }
    }
    toc.into_iter()
        .enumerate()
        .map(|(index, heading)| {
            let (level, start) = positions[index];
            let end = positions[index + 1..]
                .iter()
                .find(|(next, _)| *next <= level)
                .map_or(text.len(), |(_, start)| *start);
            NoteSection {
                id: heading.id,
                title: heading.title,
                start: start as u32,
                end: end as u32,
            }
        })
        .collect()
}
