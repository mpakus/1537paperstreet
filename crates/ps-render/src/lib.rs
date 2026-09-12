//! Markdown rendering for 1537paperstreet.

#![warn(missing_docs)]

mod blocks;
mod cache;
mod chunks;
mod front_matter;
mod gfm;
mod highlight;
mod images;
mod links;
mod mermaid;
mod pipeline;
mod sanitize;
mod toc;
mod wiki;

pub use blocks::RenderedBlock;
pub use cache::{CacheError, CacheResult, RenderCache};
pub use chunks::html_chunks;
pub use highlight::render_source;
pub use pipeline::{
    RenderOptions, RenderedDocument, render, render_document, render_document_with_options,
    render_project, render_project_with_options, render_with_options,
};
pub use toc::TocItem;
