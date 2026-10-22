use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A registered project supplied by the application, never by an IPC path.
#[derive(Clone, Debug)]
pub struct LibraryProject {
    /// Registry identity.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Registered root.
    pub root: PathBuf,
}

/// Fields for a new Markdown note. Existing notes are edited in the editor.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    /// New project-relative Markdown path; never overwrites.
    pub path: String,
    /// Human-readable title.
    pub title: String,
    /// Built-in template identifier.
    pub note_type: String,
    /// User-defined workflow status.
    pub status: String,
    /// Comma-separated topics.
    pub tags: String,
    /// Optional source URL or reference, stored as text only.
    pub source: String,
}

/// Metadata filters evaluated in Rust.
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    /// Selected project identities. Empty means all supplied projects.
    pub project_ids: Vec<String>,
    /// Substring in title, path or project name.
    pub search: String,
    /// Exact type filter, empty for all.
    pub note_type: String,
    /// Exact status filter, empty for all.
    pub status: String,
    /// Exact topic filter, empty for all.
    pub tag: String,
}

/// A Markdown heading and the byte range of its complete section.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoteSection {
    /// Unique preview-compatible heading anchor.
    pub id: String,
    /// Plain-text heading label.
    pub title: String,
    /// Inclusive UTF-8 byte offset.
    pub start: u32,
    /// Exclusive UTF-8 byte offset, including nested headings.
    pub end: u32,
}

/// Metadata for a saved plaintext note.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryNote {
    /// Registered project identity.
    pub project_id: String,
    /// Project display name.
    pub project_name: String,
    /// Project-relative path.
    pub path: String,
    /// Front-matter title or filename.
    pub title: String,
    /// Front-matter type, or note.
    pub note_type: String,
    /// Front-matter status.
    pub status: String,
    /// Front-matter topics.
    pub tags: Vec<String>,
    /// Optional source reference.
    pub source: String,
    /// Hash of exact saved bytes.
    pub hash: String,
    /// unreviewed, reviewed or changed; never an accuracy judgment.
    pub freshness: String,
    /// Selectable sections.
    pub sections: Vec<NoteSection>,
}

/// An ordered source reference with an explicitly accepted baseline.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySource {
    /// Registered project identity.
    pub project_id: String,
    /// Project-relative plaintext Markdown path.
    pub path: String,
    /// Heading anchor; null selects the complete document.
    pub section: Option<String>,
    /// Hash accepted when saving or reviewing the selection.
    pub hash: String,
}

/// Purpose of a saved ordered selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SavedSetKind {
    /// A reusable document collection.
    Collection,
    /// A reusable context pack.
    Pack,
}

/// A named collection or context pack. Contains no document bodies.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SavedSet {
    /// ULID; empty only when creating a selection.
    pub id: String,
    /// User-defined name.
    pub name: String,
    /// Selection purpose.
    pub kind: SavedSetKind,
    /// Sources in user-defined order, at most 100.
    pub sources: Vec<LibrarySource>,
}

/// Persistent application metadata, independent of projects/config schemas.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibraryData {
    /// Version of this metadata document.
    pub schema_version: u32,
    /// Optimistic concurrency token.
    pub revision: String,
    /// User-authored selections, at most 100.
    pub sets: Vec<SavedSet>,
    /// Explicit whole-document review baselines.
    pub reviews: Vec<LibrarySource>,
}

impl Default for LibraryData {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: String::new(),
            sets: vec![],
            reviews: vec![],
        }
    }
}

impl crate::store::VersionedDocument for LibraryData {
    const SCHEMA_VERSION: u32 = 1;
}

/// Library results without document bodies.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    /// Filtered notes, sorted by title then identity.
    pub notes: Vec<LibraryNote>,
    /// Saved collections and context packs.
    pub sets: Vec<SavedSet>,
    /// Token required for mutations.
    pub revision: String,
    /// Partial-result and metadata warnings.
    pub notices: Vec<String>,
    /// Available built-in template identifiers.
    pub note_types: Vec<String>,
}

/// Explicit metadata changes, serialized by the application.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum LibraryMutation {
    /// Create or replace a named selection after checking source hashes.
    Save {
        /// Selection to save.
        set: SavedSet,
    },
    /// Remove a selection; source documents are untouched.
    Delete {
        /// Saved selection identity.
        id: String,
    },
    /// Accept the displayed saved version of a note.
    Review {
        /// Source whose hash must still match.
        source: LibrarySource,
    },
}

/// Current state of one context-pack source.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    /// Original reference and baseline.
    pub source: LibrarySource,
    /// Current hash, absent if the source cannot be read.
    pub current_hash: Option<String>,
    /// current, changed, missing or unavailable.
    pub status: String,
    /// Explanation when the source cannot be included.
    pub message: String,
}

/// Complete context assembled from saved files only.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackPreview {
    /// Ordered sources with freshness information.
    pub sources: Vec<SourceStatus>,
    /// Plain Markdown; empty if any source is unavailable.
    pub text: String,
    /// Approximation, not a model-specific tokenizer count.
    pub estimated_tokens: u32,
    /// False prevents copying or handing off a partial pack.
    pub complete: bool,
}
