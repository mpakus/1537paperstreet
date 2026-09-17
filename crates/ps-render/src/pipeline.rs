use std::borrow::Cow;
use std::cell::Cell;
use std::path::Path;

use pulldown_cmark::{Options, Parser, html};

use crate::blocks::{BlockEvent, Blocks, RenderedBlock, SpannedEvent};
use crate::chunks::{CLOSE_SECTION, Chunks, OPEN_SECTION};
use crate::front_matter::FrontMatter;
use crate::gfm::TaskLists;
use crate::highlight::Highlight;
use crate::links::Links;
use crate::mermaid::Mermaid;
use crate::sanitize;
use crate::toc::{HeadingIds, TocItem};
use crate::wiki::WikiHtml;

/// Options that affect Markdown rendering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderOptions {
    /// Whether raw HTML bypasses sanitization.
    pub allow_raw_html: bool,
}

/// Rendered HTML and the headings needed to build its table of contents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedDocument {
    /// The rendered Markdown.
    pub html: String,
    /// Headings in source order.
    pub toc: Vec<TocItem>,
    /// Top-level blocks with source mapping and content hashes.
    pub blocks: Vec<RenderedBlock>,
}

/// Renders Markdown as HTML with the extensions supported by 1537paperstreet.
///
/// ```
/// let html = ps_render::render("# Hello");
/// assert!(html.contains("<h1"));
/// assert!(html.contains("Hello"));
/// ```
#[must_use]
pub fn render(markdown: &str) -> String {
    render_with_options(markdown, RenderOptions::default())
}

/// Renders Markdown with explicit rendering options.
///
/// ```
/// use ps_render::{render_with_options, RenderOptions};
///
/// let dirty = "<script>alert(1)</script>";
/// let html = render_with_options(dirty, RenderOptions { allow_raw_html: false });
/// assert!(!html.contains("<script"));
/// ```
#[must_use]
pub fn render_with_options(markdown: &str, options: RenderOptions) -> String {
    render_document_with_options(markdown, options).html
}

/// Renders Markdown and collects its table of contents.
#[must_use]
pub fn render_document(markdown: &str) -> RenderedDocument {
    render_document_with_options(markdown, RenderOptions::default())
}

/// Renders Markdown and collects its table of contents with explicit options.
#[must_use]
pub fn render_document_with_options(
    markdown: &str,
    render_options: RenderOptions,
) -> RenderedDocument {
    let parser_input = normalize_parser_input(markdown);
    let raw_html_seen = Cell::new(false);
    let events = sanitize::Events::new(
        Parser::new_ext(&parser_input, markdown_options()).into_offset_iter(),
        &raw_html_seen,
    );
    let mut blocks = Vec::new();
    let (html, toc) = finish_render(
        markdown,
        events,
        render_options,
        &raw_html_seen,
        &mut blocks,
    );
    RenderedDocument { html, toc, blocks }
}

/// Renders Markdown with project-relative links restricted to one project root.
#[must_use]
pub fn render_project(
    markdown: &str,
    project_root: &Path,
    document_path: &Path,
    project_scope: &str,
) -> RenderedDocument {
    render_project_with_options(
        markdown,
        project_root,
        document_path,
        project_scope,
        RenderOptions::default(),
    )
}

/// Renders project-relative Markdown with explicit rendering options.
#[must_use]
pub fn render_project_with_options(
    markdown: &str,
    project_root: &Path,
    document_path: &Path,
    project_scope: &str,
    render_options: RenderOptions,
) -> RenderedDocument {
    let parser_input = normalize_parser_input(markdown);
    let raw_html_seen = Cell::new(false);
    let events = sanitize::Events::new(
        Parser::new_ext(&parser_input, markdown_options()).into_offset_iter(),
        &raw_html_seen,
    );
    let mut blocks = Vec::new();
    let (html, toc) = finish_render(
        markdown,
        Links::new(events, project_root, document_path, project_scope),
        render_options,
        &raw_html_seen,
        &mut blocks,
    );
    RenderedDocument {
        html: crate::images::reserve_sizes(&html, project_root, project_scope),
        toc,
        blocks,
    }
}

fn finish_render<'events>(
    markdown: &str,
    events: impl Iterator<Item = SpannedEvent<'events>>,
    render_options: RenderOptions,
    raw_html_seen: &Cell<bool>,
    blocks: &mut Vec<RenderedBlock>,
) -> (String, Vec<TocItem>) {
    let mut output = String::new();
    let mut toc = Vec::new();
    let mut mermaid_figures = Vec::new();
    let has_headings = may_have_heading(markdown);
    let events = WikiHtml::new(TaskLists::new(FrontMatter::new(events)));

    let mermaid_prefix = if markdown.contains("mermaid") {
        let prefix = format!("PSMERMAID{}__", blake3::hash(markdown.as_bytes()).to_hex());
        render_events(
            Highlight::new(Mermaid::new(events, &prefix, &mut mermaid_figures)),
            has_headings,
            &mut output,
            &mut toc,
            markdown,
            blocks,
        );
        Some(prefix)
    } else {
        render_events(
            Highlight::new(events),
            has_headings,
            &mut output,
            &mut toc,
            markdown,
            blocks,
        );
        None
    };

    if !render_options.allow_raw_html && raw_html_seen.get() {
        output = sanitize::clean(&output);
    }
    if let Some(prefix) = mermaid_prefix {
        for (index, figure) in mermaid_figures.iter().enumerate() {
            output = output.replacen(&format!("{prefix}{index}__"), figure, 1);
        }
    }
    output = wrap_tables(output);

    (output, toc)
}

/// Wide GFM tables scroll inside the reading column; cell text still wraps.
fn wrap_tables(html: String) -> String {
    if !html.contains("<table") {
        return html;
    }
    let html = wrap_named_cells(&html, "th");
    let html = wrap_named_cells(&html, "td");
    html.replace("<table", "<div class=\"table-scroll\"><table")
        .replace("</table>", "</table></div>")
}

/// Inner wrappers make `max-width` wrap in WebKit table layout.
fn wrap_named_cells(html: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut output = String::with_capacity(html.len().saturating_add(html.len() / 8));
    let mut index = 0;
    while let Some(relative) = html.get(index..).and_then(|rest| rest.find(&open)) {
        let start = index + relative;
        let after_name = start + open.len();
        let next = html.get(after_name..).and_then(|rest| rest.chars().next());
        if !matches!(next, Some(' ' | '\n' | '\t' | '\r' | '>' | '/')) {
            output.push_str(&html[index..after_name]);
            index = after_name;
            continue;
        }
        let Some(gt_relative) = html.get(after_name..).and_then(|rest| rest.find('>')) else {
            output.push_str(&html[index..]);
            return output;
        };
        let open_end = after_name + gt_relative + 1;
        output.push_str(&html[index..open_end]);
        output.push_str("<div class=\"cell\">");
        let Some(close_relative) = html.get(open_end..).and_then(|rest| rest.find(&close)) else {
            output.push_str(&html[open_end..]);
            return output;
        };
        let close_at = open_end + close_relative;
        output.push_str(&html[open_end..close_at]);
        output.push_str("</div>");
        output.push_str(&close);
        index = close_at + close.len();
    }
    output.push_str(&html[index..]);
    output
}

fn render_events<'events>(
    events: impl Iterator<Item = SpannedEvent<'events>>,
    has_headings: bool,
    output: &mut String,
    toc: &mut Vec<TocItem>,
    markdown: &str,
    blocks: &mut Vec<RenderedBlock>,
) {
    if has_headings {
        let events = HeadingIds::new(events, toc);
        render_chunks(Blocks::new(events, markdown, blocks), output);
    } else {
        render_chunks(Blocks::new(events, markdown, blocks), output);
    }
}

fn render_chunks<'input>(events: impl Iterator<Item = BlockEvent<'input>>, output: &mut String) {
    let mut chunks = Chunks::new(events);
    if chunks.has_events() {
        output.push_str(OPEN_SECTION);
        html::push_html(output, chunks);
        output.push_str(CLOSE_SECTION);
    }
}

fn may_have_heading(markdown: &str) -> bool {
    markdown
        .bytes()
        .any(|byte| matches!(byte, b'#' | b'=' | b'-'))
}

fn normalize_parser_input(markdown: &str) -> Cow<'_, str> {
    if !needs_parser_normalization(markdown) {
        return Cow::Borrowed(markdown);
    }

    let mut normalized = String::with_capacity(markdown.len());
    let mut characters = markdown.chars().peekable();
    while let Some(character) = characters.next() {
        if is_unsupported_control(character) {
            normalized.push('?');
        } else if character == '\r' && characters.peek() != Some(&'\n') {
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    Cow::Owned(normalized)
}

fn is_unsupported_control(character: char) -> bool {
    matches!(
        character,
        '\0'..='\u{0008}' | '\u{000B}'..='\u{000C}' | '\u{000E}'..='\u{001F}' | '\u{007F}'
    )
}

fn needs_parser_normalization(markdown: &str) -> bool {
    let bytes = markdown.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| {
        matches!(*byte, 0..=8 | 11..=12 | 14..=31 | 127)
            || (*byte == b'\r' && bytes.get(index + 1) != Some(&b'\n'))
    })
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_WIKILINKS
}
