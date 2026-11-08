//! PDF files shown through the project `asset://` protocol.

use std::path::Path;

use ps_core::projects::is_pdf_path;

use crate::images::{escape_text, project_asset_url};

/// One HTML chunk that shows a project PDF through `asset://`.
///
/// The file bytes stay on disk. The webview's PDF viewer loads the asset URL.
/// A path that is not a PDF, or that leaves the project, becomes a short message.
///
/// ```
/// use std::path::Path;
/// use ps_render::render_pdf;
///
/// let html = render_pdf("project-1", Path::new("papers/note 1.pdf"));
/// assert!(html.contains("data=\"asset://localhost/project-1/papers/note%201.pdf\""));
/// assert!(html.contains("type=\"application/pdf\""));
/// assert!(html.contains("class=\"pdf-file\""));
/// assert!(!html.contains("<script"));
/// ```
#[must_use]
pub fn render_pdf(project_scope: &str, rel_path: &Path) -> String {
    let Some(src) = pdf_src(project_scope, rel_path) else {
        return String::from(
            "<section class=\"chunk\"><p class=\"pdf-file\">This PDF cannot be shown.</p></section>\n",
        );
    };
    let title = escape_text(
        rel_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("PDF"),
    );
    format!(
        "<section class=\"chunk\"><div class=\"pdf-file\"><object data=\"{src}\" type=\"application/pdf\" title=\"{title}\"><p>This PDF could not be shown in the preview. Use Open in External Editor.</p></object></div></section>\n"
    )
}

/// One HTML chunk of already-rendered PDF page images.
///
/// `sources` are `data:` or `asset:` URLs. A source containing a quote is skipped.
/// When `total` is larger than `shown`, the chunk says the rest is omitted.
#[must_use]
pub fn render_pdf_page_images(sources: &[impl AsRef<str>], shown: usize, total: usize) -> String {
    let mut html = String::from("<section class=\"chunk\"><div class=\"pdf-pages\">\n");
    for (index, source) in sources.iter().enumerate() {
        let source = source.as_ref();
        if source.is_empty() || source.contains(['"', '<', '>']) {
            continue;
        }
        let page = index + 1;
        html.push_str(&format!("<img src=\"{source}\" alt=\"Page {page}\">\n"));
    }
    if total > shown {
        html.push_str(&format!(
            "<p>Showing the first {shown} pages of {total}.</p>\n"
        ));
    }
    html.push_str("</div></section>\n");
    html
}

fn pdf_src(project_scope: &str, rel_path: &Path) -> Option<String> {
    if !is_pdf_path(rel_path) {
        return None;
    }
    project_asset_url(project_scope, rel_path)
}
