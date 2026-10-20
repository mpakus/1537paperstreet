//! Bounded, rebuildable document relationship index. Never writes project files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Error, Result, fsops};

const DOCUMENTS: TableDefinition<&str, &str> = TableDefinition::new("documents");
const MAX_FILES: usize = 5_000;
const MAX_ENTRIES: usize = 100_000;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LINKS: usize = 2_000;
const MAX_NODES: usize = 600;
const MAX_EDGES: usize = 6_000;

/// Parameters for the graph view, interpreted entirely in Rust.
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphQuery {
    /// Case-insensitive path substring.
    pub search: String,
    /// Show this note and its immediate incoming/outgoing neighbors.
    pub focus: Option<String>,
    /// Omit documents without any incoming or outgoing connections.
    pub hide_isolated: bool,
    /// Discard this project's disposable cache before indexing.
    pub rebuild: bool,
}

/// One saved Markdown document and its stable display position.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Project-relative document path, also the stable identity.
    pub path: String,
    /// File stem shown on the map.
    pub title: String,
    /// Parent folder, empty at the project root.
    pub folder: String,
    /// Number of incoming edges in the indexed project.
    pub incoming: u32,
    /// Number of outgoing edges in the indexed project.
    pub outgoing: u32,
    /// Horizontal world coordinate.
    pub x: f64,
    /// Vertical world coordinate.
    pub y: f64,
}

/// Directed link between two indices in the returned node array.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// Index of the linking document.
    pub source: u32,
    /// Index of the linked document.
    pub target: u32,
}

/// One bounded graph ready to display, without any document text.
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphSnapshot {
    /// Nodes surviving the query and display limit.
    pub nodes: Vec<GraphNode>,
    /// Deduplicated directed edges between visible nodes.
    pub edges: Vec<GraphEdge>,
    /// Plaintext Markdown files in the bounded project index.
    pub total_nodes: u32,
    /// Unique directed links in the bounded project index.
    pub total_edges: u32,
    /// Files whose link parsing was reused from the database.
    pub cached_files: u32,
    /// Files parsed during this refresh.
    pub parsed_files: u32,
    /// Why any files or results were omitted.
    pub notices: Vec<String>,
}

/// Parser output persisted in the cache; body text must never be included.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentLink {
    /// Relative local link destination (including optional fragment).
    pub destination: String,
    /// Whether wiki-link filename fallback rules apply.
    pub wiki: bool,
}

#[derive(Deserialize, Serialize)]
struct CachedDocument {
    hash: String,
    links: Vec<DocumentLink>,
    truncated: bool,
}

fn cache_error(error: impl std::fmt::Display) -> Error {
    Error::GraphCache {
        detail: error.to_string(),
    }
}

/// Recognizes plaintext Markdown only; age notes must never enter this index.
pub fn is_graph_document(path: &Path) -> bool {
    crate::projects::is_markdown_path(path) && !crate::encrypted::is_encrypted_markdown(path)
}

/// Refreshes the persisted index and returns a bounded, laid-out graph.
///
/// Callers serialize access to `cache_dir` and run this blocking operation off
/// the UI thread. Parsing and link resolution are supplied by the renderer so
/// graph edges follow the exact preview link rules. Only saved UTF-8 text is read.
pub fn snapshot(
    cache_dir: &Path,
    project_root: &Path,
    query: &GraphQuery,
    extract: impl Fn(&str) -> Vec<DocumentLink>,
    resolve: impl Fn(&Path, &DocumentLink) -> Option<String>,
) -> Result<GraphSnapshot> {
    let root = fsops::resolve(project_root, Path::new(""))?;
    let key = blake3::hash(root.as_os_str().as_encoded_bytes()).to_hex();
    fs::create_dir_all(cache_dir).map_err(|e| Error::io("create the graph cache", cache_dir, e))?;
    let file = cache_dir.join(format!("v1-{key}.redb"));
    if query.rebuild && file.exists() {
        fs::remove_file(&file).map_err(|e| Error::io("rebuild the graph cache", &file, e))?;
    }
    let db = Database::builder()
        .set_cache_size(8 * 1024 * 1024)
        .create(&file)
        .map_err(cache_error)?;
    let write = db.begin_write().map_err(cache_error)?;
    let mut records = BTreeMap::new();
    let mut output = GraphSnapshot::default();
    let mut budget = ScanBudget::default();
    let mut files = Vec::new();
    collect(&root, Path::new(""), 0, &mut files, &mut budget)?;
    files.sort();
    {
        let mut table = write.open_table(DOCUMENTS).map_err(cache_error)?;
        for path in files {
            let resolved = fsops::resolve(&root, &path)?;
            let metadata = fs::symlink_metadata(&resolved)
                .map_err(|e| Error::io("inspect a graph document", &path, e))?;
            if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
                budget.skipped += 1;
                continue;
            }
            if budget.bytes + metadata.len() as usize > MAX_BYTES {
                budget.capped = true;
                break;
            }
            let mut bytes = Vec::new();
            fs::File::open(&resolved)
                .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
                .map_err(|e| Error::io("read a graph document", &path, e))?;
            budget.bytes += bytes.len();
            if bytes.len() as u64 > MAX_FILE_BYTES {
                budget.skipped += 1;
                continue;
            }
            let Ok(text) = std::str::from_utf8(&bytes) else {
                budget.skipped += 1;
                continue;
            };
            let path_text = display_path(&path);
            let hash = blake3::hash(&bytes).to_hex().to_string();
            let cached = table
                .get(path_text.as_str())
                .map_err(cache_error)?
                .and_then(|value| serde_json::from_str::<CachedDocument>(value.value()).ok());
            let record = if let Some(cached) = cached.filter(|cached| cached.hash == hash) {
                output.cached_files += 1;
                cached
            } else {
                let mut links = extract(text);
                let truncated = links.len() > MAX_LINKS;
                if links.len() > MAX_LINKS {
                    budget.capped = true;
                    links.truncate(MAX_LINKS);
                }
                output.parsed_files += 1;
                let record = CachedDocument {
                    hash,
                    links,
                    truncated,
                };
                let json = serde_json::to_string(&record).map_err(cache_error)?;
                table
                    .insert(path_text.as_str(), json.as_str())
                    .map_err(cache_error)?;
                record
            };
            budget.capped |= record.truncated;
            records.insert(path_text, record);
        }
        let obsolete: Vec<String> = table
            .iter()
            .map_err(cache_error)?
            .map(|row| {
                row.map(|(key, _)| key.value().to_owned())
                    .map_err(cache_error)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|key| !records.contains_key(key))
            .collect();
        for key in obsolete {
            table.remove(key.as_str()).map_err(cache_error)?;
        }
    }
    write.commit().map_err(cache_error)?;
    if budget.capped {
        output.notices.push("Partial index: limited to 5,000 notes, 100,000 entries, 64 MB of text and 2,000 links per note.".into());
    }
    if budget.skipped > 0 {
        output.notices.push(format!(
            "Skipped {} files or folders (over 2 MB, non-UTF-8, or deeper than 64 folders).",
            budget.skipped
        ));
    }
    let mut edges = BTreeSet::new();
    let mut resolved_links = 0;
    'documents: for (path, record) in &records {
        for link in &record.links {
            if resolved_links == 50_000 {
                output.notices.push(
                    "Partial connections: limited to resolving 50,000 links per refresh.".into(),
                );
                break 'documents;
            }
            resolved_links += 1;
            if let Some(target) = resolve(Path::new(path), link)
                && target != *path
                && records.contains_key(&target)
            {
                edges.insert((path.clone(), target));
            }
        }
    }
    output.total_nodes = records.len() as u32;
    output.total_edges = edges.len() as u32;
    let mut counts: BTreeMap<&str, (u32, u32)> =
        records.keys().map(|key| (key.as_str(), (0, 0))).collect();
    for (source, target) in &edges {
        if let Some(count) = counts.get_mut(source.as_str()) {
            count.1 += 1;
        }
        if let Some(count) = counts.get_mut(target.as_str()) {
            count.0 += 1;
        }
    }
    let neighborhood: BTreeSet<&str> = query
        .focus
        .as_deref()
        .map(|focus| {
            let mut found = BTreeSet::from([focus]);
            for (source, target) in &edges {
                if source == focus {
                    found.insert(target.as_str());
                }
                if target == focus {
                    found.insert(source.as_str());
                }
            }
            found
        })
        .unwrap_or_default();
    let search = query.search.trim().to_lowercase();
    // Reserve the focus node's place before the display cap applies to neighbors.
    let focus = query
        .focus
        .as_deref()
        .and_then(|path| counts.remove_entry(path));
    for (path, (incoming, outgoing)) in focus.into_iter().chain(counts) {
        if !path.to_lowercase().contains(&search)
            || (query.hide_isolated && incoming + outgoing == 0)
            || (query.focus.is_some() && !neighborhood.contains(path))
        {
            continue;
        }
        if output.nodes.len() == MAX_NODES {
            output.notices.push("Showing the first 600 matching notes. Search or focus a neighborhood to explore more.".into());
            break;
        }
        let file = Path::new(path);
        output.nodes.push(GraphNode {
            path: path.into(),
            title: file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .into(),
            folder: file.parent().map(display_path).unwrap_or_default(),
            incoming,
            outgoing,
            x: 0.0,
            y: 0.0,
        });
    }
    let indices: BTreeMap<&str, u32> = output
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.path.as_str(), i as u32))
        .collect();
    for (source, target) in edges {
        if let (Some(&source), Some(&target)) =
            (indices.get(source.as_str()), indices.get(target.as_str()))
        {
            if output.edges.len() == MAX_EDGES {
                output.notices.push(
                    "Showing the first 6,000 connections. Focus a neighborhood to see more detail."
                        .into(),
                );
                break;
            }
            output.edges.push(GraphEdge { source, target });
        }
    }
    layout(&mut output.nodes, &output.edges);
    Ok(output)
}

#[derive(Default)]
struct ScanBudget {
    entries: usize,
    bytes: usize,
    capped: bool,
    skipped: usize,
}

fn collect(
    root: &Path,
    rel: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
    budget: &mut ScanBudget,
) -> Result<()> {
    if depth > 64 {
        budget.skipped += 1;
        return Ok(());
    }
    let dir = fsops::resolve(root, rel)?;
    let entries = fs::read_dir(&dir).map_err(|e| Error::io("read the graph folder", &dir, e))?;
    // Sort before applying limits so repeated scans have the same bounded subset.
    let mut entries = entries
        .take(MAX_ENTRIES.saturating_sub(budget.entries) + 1)
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| Error::io("read a graph entry", &dir, e))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if budget.entries >= MAX_ENTRIES || files.len() >= MAX_FILES {
            budget.capped = true;
            break;
        }
        budget.entries += 1;
        let path = rel.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|e| Error::io("inspect a graph entry", &path, e))?;
        if kind.is_dir()
            && ![".git", "node_modules", "target", ".1537paperstreet"]
                .iter()
                .any(|name| entry.file_name() == *name)
        {
            collect(root, &path, depth + 1, files, budget)?;
        } else if kind.is_file() && is_graph_document(&path) && path.to_str().is_some() {
            files.push(path);
        }
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn layout(nodes: &mut [GraphNode], edges: &[GraphEdge]) {
    // Deterministic bounded force layout, outside the UI thread. No idle simulation.
    let count = nodes.len();
    for (i, node) in nodes.iter_mut().enumerate() {
        let angle = i as f64 * 2.399_963_229_728_653;
        let radius = 25.0 * (i as f64 + 1.0).sqrt();
        node.x = angle.cos() * radius;
        node.y = angle.sin() * radius;
    }
    for step in 0..90 {
        let mut forces = vec![(0.0, 0.0); count];
        for i in 0..count {
            for j in i + 1..count {
                let dx = nodes[i].x - nodes[j].x;
                let dy = nodes[i].y - nodes[j].y;
                let repulsion = 1_200.0 / (dx * dx + dy * dy + 25.0);
                let (fx, fy) = (dx * repulsion, dy * repulsion);
                forces[i].0 += fx;
                forces[i].1 += fy;
                forces[j].0 -= fx;
                forces[j].1 -= fy;
            }
        }
        for edge in edges {
            let (a, b) = (edge.source as usize, edge.target as usize);
            let dx = nodes[b].x - nodes[a].x;
            let dy = nodes[b].y - nodes[a].y;
            let distance = dx.hypot(dy).max(1.0);
            let pull = (distance - 90.0) / distance * 0.08;
            forces[a].0 += dx * pull;
            forces[a].1 += dy * pull;
            forces[b].0 -= dx * pull;
            forces[b].1 -= dy * pull;
        }
        let cooling = 1.0 - step as f64 / 100.0;
        for (node, (fx, fy)) in nodes.iter_mut().zip(forces) {
            node.x += (fx - node.x * 0.02).clamp(-15.0, 15.0) * cooling;
            node.y += (fy - node.y * 0.02).clamp(-15.0, 15.0) * cooling;
        }
    }
}
