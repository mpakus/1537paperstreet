use std::path::Path;

use ps_core::graph::{DocumentLink, GraphQuery, GraphSnapshot};
use pulldown_cmark::{Event, LinkType, Parser, Tag};

use crate::links::{Links, has_scheme, split_suffix};

/// Builds a cached graph using the same Markdown and wiki-link rules as preview.
/// Call off the UI thread, with serialized access to the cache directory.
pub fn project_graph(
    cache: &Path,
    root: &Path,
    query: &GraphQuery,
) -> ps_core::Result<GraphSnapshot> {
    ps_core::graph::snapshot(cache, root, query, extract, |document, link| {
        let links = Links::new(std::iter::empty(), root, document, "graph");
        let destination = if link.wiki {
            links.wiki_url(&link.destination)
        } else {
            links.asset_url(&link.destination)
        }
        .ok()??;
        let path = destination.strip_prefix("asset://localhost/graph/")?;
        let decoded = if link.wiki {
            split_suffix(path).0.to_owned()
        } else {
            crate::images::percent_decode(split_suffix(path).0)?
        };
        ps_core::fsops::resolve(root, Path::new(&decoded)).ok()?;
        Some(decoded)
    })
}

fn extract(text: &str) -> Vec<DocumentLink> {
    Parser::new_ext(text, crate::pipeline::markdown_options())
        .filter_map(|event| {
            let Event::Start(Tag::Link {
                dest_url,
                link_type,
                ..
            }) = event
            else {
                return None;
            };
            let (path, _) = split_suffix(&dest_url);
            if path.is_empty()
                || dest_url.len() > 4096
                || has_scheme(&dest_url)
                || Path::new(path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("age"))
            {
                return None;
            }
            Some(DocumentLink {
                destination: dest_url.into_string(),
                wiki: matches!(link_type, LinkType::WikiLink { .. }),
            })
        })
        .take(2001)
        .collect()
}
