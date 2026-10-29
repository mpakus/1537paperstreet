//! Bear-style tag workspaces over saved notes in registered project folders.

use crate::{
    Result,
    library::{self, LibraryProject},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

/// Filters applied to the saved tag index; all selections use AND semantics.
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TagQuery {
    /// Selected canonical tags, including descendants, empty for all notes.
    pub tags: Vec<String>,
    /// Registered project ID, empty for all folders.
    pub project_id: String,
    /// Case-insensitive substring in title, path, folder name or tags.
    pub search: String,
    /// Offset into the sorted tag cloud (80 tags per page).
    pub tag_offset: u32,
}

/// A tag and its document/project connections in the current workspace.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CloudTag {
    /// Canonical lowercase key used for selection.
    pub key: String,
    /// Portable Bear syntax, ready for display or copying.
    pub label: String,
    /// Number of distinct matching documents, including descendants.
    pub documents: u32,
    /// Number of distinct registered projects connected by these documents.
    pub projects: u32,
    /// Presentation weight from 0 to 4.
    pub weight: u8,
}

/// A saved document that can be opened through the existing editor flow.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TagNote {
    /// Registry identity.
    pub project_id: String,
    /// Registered folder's display name.
    pub project_name: String,
    /// Project-relative Markdown path.
    pub path: String,
    /// Metadata title or filename.
    pub title: String,
}

/// A bounded cloud page and virtualizable document results.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TagSnapshot {
    /// Current page of related tags, ordered by frequency then name.
    pub tags: Vec<CloudTag>,
    /// Total related tags before pagination.
    pub total_tags: u32,
    /// Effective page offset, clamped after filtering.
    pub tag_offset: u32,
    /// Matching saved documents in title order.
    pub notes: Vec<TagNote>,
    /// Number of projects containing matching notes.
    pub projects: u32,
    /// Explicit partial-result and unreadable-file notices.
    pub notices: Vec<String>,
}

struct IndexedNote {
    note: TagNote,
    tags: BTreeMap<String, String>,
    search: String,
}

/// Rebuildable in-memory metadata index; never stores document bodies.
#[derive(Default)]
pub struct TagIndex {
    roots: Option<Vec<LibraryProject>>,
    notes: Vec<IndexedNote>,
    notices: Vec<String>,
}

/// Validates a tag name (without its # delimiters), preserving display spelling.
/// Empty segments, control characters, oversized tags and deep paths are rejected.
pub fn normalize_tag(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 256
        || value.contains(['#', '\\'])
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let parts: Vec<_> = value.split('/').map(str::trim).collect();
    if parts.len() > 16 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some(parts.join("/"))
}

impl TagIndex {
    /// Rebuilds when requested or registered roots change, then filters in Rust.
    /// Call from a blocking worker with serialized access to this index.
    pub fn query(
        &mut self,
        projects: &[LibraryProject],
        query: &TagQuery,
        refresh: bool,
        extract: impl Fn(&str) -> Vec<String>,
    ) -> Result<TagSnapshot> {
        // Folder scope changes the scan itself, so it can recover notes omitted
        // by the global scan budget rather than only filtering a partial index.
        let scoped: Vec<_> = projects
            .iter()
            .filter(|p| query.project_id.is_empty() || p.id == query.project_id)
            .cloned()
            .collect();
        let projects = scoped.as_slice();
        if refresh || self.roots.as_deref() != Some(projects) {
            let scan = library::scan_with_tags(
                projects,
                &Default::default(),
                &Default::default(),
                |_| vec![],
                extract,
            )?;
            let mut notes = Vec::new();
            let mut notices: Vec<_> = scan
                .notices
                .into_iter()
                .map(|n| n.replace("Library", "Tags Cloud").replace("library", "tag"))
                .collect();
            let mut memberships = 0;
            let mut limited = false;
            for note in scan.notes {
                let mut tags = BTreeMap::new();
                let mut unique = BTreeSet::new();
                for value in note.tags {
                    let Some(value) = normalize_tag(&value) else {
                        continue;
                    };
                    if !unique.insert(value.to_lowercase()) {
                        continue;
                    }
                    if unique.len() > 256 {
                        limited = true;
                        break;
                    }
                    let mut parent = String::new();
                    for part in value.split('/') {
                        if !parent.is_empty() {
                            parent.push('/');
                        }
                        parent.push_str(part);
                        tags.entry(parent.to_lowercase())
                            .or_insert_with(|| parent.clone());
                    }
                }
                memberships += tags.len();
                if memberships > 100_000 {
                    limited = true;
                    break;
                }
                let search = format!(
                    "{} {} {} {}",
                    note.title,
                    note.path,
                    note.project_name,
                    tags.keys().cloned().collect::<Vec<_>>().join(" ")
                )
                .to_lowercase();
                notes.push(IndexedNote {
                    note: TagNote {
                        project_id: note.project_id,
                        project_name: note.project_name,
                        path: note.path,
                        title: note.title,
                    },
                    tags,
                    search,
                });
            }
            if limited {
                notices.push("Partial results: Tags Cloud indexes up to 256 tags per note and 100,000 tag connections. Narrow the source folders or tags.".into());
            }
            self.notes = notes;
            self.notices = notices;
            self.roots = Some(projects.to_vec());
        }
        let selected: Vec<_> = query.tags.iter().map(|tag| tag.to_lowercase()).collect();
        let search = query.search.trim().to_lowercase();
        let matching: Vec<_> = self
            .notes
            .iter()
            .filter(|n| {
                (query.project_id.is_empty() || query.project_id == n.note.project_id)
                    && selected.iter().all(|tag| n.tags.contains_key(tag))
                    && n.search.contains(&search)
            })
            .collect();
        let mut counts: BTreeMap<&str, (&str, u32, BTreeSet<&str>)> = BTreeMap::new();
        for n in &matching {
            for (key, label) in &n.tags {
                let entry = counts.entry(key).or_insert((label, 0, BTreeSet::new()));
                entry.1 += 1;
                entry.2.insert(&n.note.project_id);
            }
        }
        let max = counts
            .values()
            .map(|(_, count, _)| *count)
            .max()
            .unwrap_or(1);
        let mut tags: Vec<_> = counts
            .into_iter()
            .map(|(key, (label, documents, projects))| CloudTag {
                key: key.into(),
                label: if label.chars().all(char::is_numeric)
                    || label
                        .chars()
                        .any(|c| !c.is_alphanumeric() && !matches!(c, '_' | '-' | '/'))
                {
                    format!("#{label}#")
                } else {
                    format!("#{label}")
                },
                documents,
                projects: projects.len() as u32,
                weight: if max == 1 {
                    0
                } else {
                    ((documents - 1) * 4 / (max - 1)) as u8
                },
            })
            .collect();
        tags.sort_by(|a, b| b.documents.cmp(&a.documents).then(a.key.cmp(&b.key)));
        let total_tags = tags.len() as u32;
        let offset = query.tag_offset.min(total_tags.saturating_sub(1) / 80 * 80);
        Ok(TagSnapshot {
            tags: tags.into_iter().skip(offset as usize).take(80).collect(),
            total_tags,
            tag_offset: offset,
            projects: matching
                .iter()
                .map(|n| &n.note.project_id)
                .collect::<BTreeSet<_>>()
                .len() as u32,
            notes: matching.into_iter().map(|n| n.note.clone()).collect(),
            notices: self.notices.clone(),
        })
    }
}
