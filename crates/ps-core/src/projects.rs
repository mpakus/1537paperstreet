//! Persistent project registry.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;
use ulid::Ulid;

use crate::store::{JsonStore, StoreWarning, VersionedDocument};
use crate::tree;
use crate::{Error, Result};

/// A registered folder containing Markdown documents.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct Project {
    /// Stable, lexicographically sortable ULID.
    pub id: String,
    /// User-visible project name.
    pub name: String,
    /// Absolute path to the project folder.
    #[ts(type = "string")]
    pub path: PathBuf,
    /// RFC 3339 timestamp at which the project was registered.
    pub added_at: String,
    /// RFC 3339 timestamp of the most recent open operation.
    pub last_opened_at: Option<String>,
    /// Whether the project stays above unpinned projects.
    pub pinned: bool,
    /// Optional theme accent override.
    pub accent: Option<String>,
    /// Last opened project-relative file path.
    pub last_file: Option<String>,
    /// Cached path availability, omitted from persistent storage.
    #[serde(skip_serializing, default)]
    pub available: Option<bool>,
}

/// Arguments for `projects_list`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsListQuery {
    /// Optional fuzzy query over project names and paths.
    pub query: Option<String>,
    /// Maximum number of projects to return.
    pub limit: u32,
    /// Number of ranked matches to skip.
    pub offset: u32,
}

/// A page of registered projects.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsListResult {
    /// Projects in the requested page.
    pub items: Vec<Project>,
    /// Number of projects matching the query before paging.
    pub total: u32,
}

/// Result of opening a folder or Markdown file dropped onto the window.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OpenDropResult {
    /// Project that owns the dropped path, created if it was not registered.
    pub project: Project,
    /// Project-relative path of a dropped Markdown file, when one was opened.
    #[ts(type = "string | null")]
    pub open_rel_path: Option<PathBuf>,
}

impl Project {
    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::InvalidProject {
                reason: "the name is empty",
            });
        }
        if !self.path.is_absolute() {
            return Err(Error::InvalidProject {
                reason: "the folder path is not absolute",
            });
        }
        if Ulid::from_string(&self.id).is_err() {
            return Err(Error::InvalidProject {
                reason: "the identifier is not a ULID",
            });
        }
        Ok(())
    }
}

/// The versioned document stored in `projects.json`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectsDocument {
    /// Storage schema version.
    pub schema_version: u32,
    /// Registered projects in display order.
    pub projects: Vec<Project>,
}

impl Default for ProjectsDocument {
    fn default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            projects: Vec::new(),
        }
    }
}

impl VersionedDocument for ProjectsDocument {
    const SCHEMA_VERSION: u32 = 1;

    fn migrate(value: Value, from: u32) -> Result<Value> {
        let _ = value;
        Err(Error::UnsupportedSchema {
            found: from,
            supported: Self::SCHEMA_VERSION,
        })
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(Error::UnsupportedSchema {
                found: self.schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        for project in &self.projects {
            project.validate()?;
        }
        Ok(())
    }
}

/// CRUD service for the persistent project registry.
pub struct ProjectStore {
    store: JsonStore<ProjectsDocument>,
}

impl ProjectStore {
    /// Opens the project registry at `path`.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        JsonStore::open(path).map(|store| Self { store })
    }

    /// Returns projects in their stored display order.
    pub fn list(&self) -> &[Project] {
        &self.store.value().projects
    }

    /// Returns a registered project by identifier.
    pub fn get(&self, id: &str) -> Result<&Project> {
        self.store
            .value()
            .projects
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(|| Error::ProjectNotFound { id: id.to_owned() })
    }

    /// Registers an absolute project folder path.
    pub fn add(&mut self, name: impl Into<String>, path: PathBuf) -> Result<Project> {
        let project = Project {
            id: Ulid::generate().to_string(),
            name: name.into(),
            path,
            added_at: now_rfc3339()?,
            last_opened_at: None,
            pinned: false,
            accent: None,
            last_file: None,
            available: None,
        };
        project.validate()?;
        let returned = project.clone();
        self.store
            .update(|document| document.projects.push(project));
        Ok(returned)
    }

    /// Changes the user-visible name of a registered project.
    pub fn rename(&mut self, id: &str, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(Error::InvalidProject {
                reason: "the name is empty",
            });
        }
        let project = self.find_mut(id)?;
        project.name = name;
        self.mark_dirty();
        Ok(())
    }

    /// Removes only a registry record and never touches its project folder.
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let Some(index) = self
            .store
            .value()
            .projects
            .iter()
            .position(|project| project.id == id)
        else {
            return Err(Error::ProjectNotFound { id: id.to_owned() });
        };
        self.store.update(|document| {
            document.projects.remove(index);
        });
        Ok(())
    }

    /// Checks a project folder on demand and caches the result in memory.
    pub fn refresh_availability(&mut self, id: &str) -> Result<bool> {
        let project = self.find_mut(id)?;
        let available = project.path.is_dir();
        project.available = Some(available);
        self.mark_dirty();
        Ok(available)
    }

    /// Immediately persists pending registry changes.
    pub fn flush(&mut self) -> Result<bool> {
        self.store.flush()
    }

    /// Persists pending changes and closes the registry.
    pub fn close(self) -> Result<()> {
        self.store.close()
    }

    /// Returns and clears a recoverable warning from opening the registry.
    pub fn take_warning(&mut self) -> Option<StoreWarning> {
        self.store.take_warning()
    }

    /// Returns the registered project whose folder is `path`, if any.
    pub fn find_by_path(&self, path: &Path) -> Option<&Project> {
        let canonical = path.canonicalize().ok();
        self.list()
            .iter()
            .find(|project| paths_refer_to_same_folder(&project.path, path, canonical.as_deref()))
    }

    /// Returns the registered project that contains `absolute`, preferring the deepest root.
    pub fn find_containing(&self, absolute: &Path) -> Option<&Project> {
        let canonical = absolute.canonicalize().ok()?;
        self.list()
            .iter()
            .filter_map(|project| {
                let root = project.path.canonicalize().ok()?;
                if canonical == root || canonical.starts_with(&root) {
                    Some((root.as_os_str().len(), project))
                } else {
                    None
                }
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, project)| project)
    }

    /// Registers `path` as a project, or returns the existing record for that folder.
    pub fn ensure_folder(&mut self, path: PathBuf) -> Result<Project> {
        let canonical = path
            .canonicalize()
            .map_err(|source| Error::io("open the folder", &path, source))?;
        if !canonical.is_dir() {
            return Err(Error::UnsafePath {
                path,
                reason: "the dropped path is not a folder",
            });
        }
        if let Some(existing) = self.find_by_path(&canonical) {
            return Ok(existing.clone());
        }
        let name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Project")
            .to_owned();
        self.add(name, canonical)
    }

    /// Points an existing project record at a different folder on disk.
    ///
    /// The previous folder is not modified. Duplicate registrations are rejected.
    pub fn relocate(&mut self, id: &str, path: PathBuf) -> Result<Project> {
        let canonical = path
            .canonicalize()
            .map_err(|source| Error::io("open the folder", &path, source))?;
        if !canonical.is_dir() {
            return Err(Error::UnsafePath {
                path,
                reason: "the replacement path is not a folder",
            });
        }
        if let Some(existing) = self.find_by_path(&canonical) {
            if existing.id != id {
                return Err(Error::InvalidProject {
                    reason: "that folder is already registered as another project",
                });
            }
            return Ok(existing.clone());
        }
        let project = {
            let project = self.find_mut(id)?;
            project.path = canonical;
            project.available = Some(true);
            project.clone()
        };
        self.mark_dirty();
        Ok(project)
    }

    /// Records that a project was opened, optionally updating the last file.
    pub fn touch_opened(&mut self, id: &str, last_file: Option<String>) -> Result<()> {
        let project = self.find_mut(id)?;
        project.last_opened_at = Some(now_rfc3339()?);
        if let Some(last_file) = last_file {
            project.last_file = Some(last_file);
        }
        self.mark_dirty();
        Ok(())
    }

    fn find_mut(&mut self, id: &str) -> Result<&mut Project> {
        self.store
            .value_mut()
            .projects
            .iter_mut()
            .find(|project| project.id == id)
            .ok_or_else(|| Error::ProjectNotFound { id: id.to_owned() })
    }

    fn mark_dirty(&mut self) {
        self.store.mark_dirty();
    }
}

fn now_rfc3339() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|source| Error::TimeFormat { source })
}

/// Opens dropped filesystem paths as a project, optionally focusing a Markdown file.
///
/// Folders are registered (or reused). Markdown files reuse a containing project when
/// one exists; otherwise the file's parent folder is registered and the file is opened.
pub fn open_dropped_paths(store: &mut ProjectStore, paths: &[PathBuf]) -> Result<OpenDropResult> {
    let mut opened = None;
    for path in paths {
        opened = Some(open_dropped_path(store, path)?);
    }
    opened.ok_or(Error::EmptyDrop)
}

fn open_dropped_path(store: &mut ProjectStore, path: &Path) -> Result<OpenDropResult> {
    let metadata =
        fs::metadata(path).map_err(|source| Error::io("open the dropped path", path, source))?;
    if metadata.is_dir() {
        let project = store.ensure_folder(path.to_path_buf())?;
        store.touch_opened(&project.id, None)?;
        return Ok(OpenDropResult {
            project: store.get(&project.id)?.clone(),
            open_rel_path: None,
        });
    }
    if !metadata.is_file() || !is_markdown_path(path) {
        return Err(Error::UnsupportedDrop {
            path: path.to_path_buf(),
        });
    }

    let canonical_file = path
        .canonicalize()
        .map_err(|source| Error::io("open the dropped file", path, source))?;
    let project = if let Some(existing) = store.find_containing(&canonical_file) {
        existing.clone()
    } else {
        let parent = canonical_file.parent().ok_or_else(|| Error::UnsafePath {
            path: canonical_file.clone(),
            reason: "the file does not have a parent folder",
        })?;
        store.ensure_folder(parent.to_path_buf())?
    };
    let node = tree::node_at(&project.path, &canonical_file)?;
    let last_file = node.rel_path.to_string_lossy().into_owned();
    store.touch_opened(&project.id, Some(last_file))?;
    Ok(OpenDropResult {
        project: store.get(&project.id)?.clone(),
        open_rel_path: Some(node.rel_path),
    })
}

/// Returns whether `path` uses a recognized Markdown file extension.
///
/// ```
/// use std::path::Path;
/// use ps_core::projects::is_markdown_path;
///
/// assert!(is_markdown_path(Path::new("notes.md")));
/// assert!(is_markdown_path(Path::new("Note.MARKDOWN")));
/// assert!(!is_markdown_path(Path::new("cover.png")));
/// ```
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mdwn"
            )
        })
}

/// Language hint used to highlight a non-Markdown file in the preview.
///
/// Markdown paths return [`None`] so they go through the document renderer.
/// Every other path returns the extension, or the lowercased file name when
/// there is no extension (`Makefile`, `Dockerfile`). Unknown languages are
/// still shown as escaped plain text.
///
/// ```
/// use std::path::Path;
/// use ps_core::projects::{is_editable_path, is_source_path, source_language};
///
/// assert_eq!(source_language(Path::new("src/main.rs")).as_deref(), Some("rs"));
/// assert_eq!(source_language(Path::new("notes.py")).as_deref(), Some("py"));
/// assert_eq!(source_language(Path::new("Makefile")).as_deref(), Some("makefile"));
/// assert_eq!(source_language(Path::new("notes.md")), None);
/// assert!(is_source_path(Path::new("notes.txt")));
/// assert!(is_editable_path(Path::new("LICENSE")));
/// assert!(is_editable_path(Path::new("cover.png")));
/// assert!(!is_source_path(Path::new("notes.md")));
/// ```
pub fn source_language(path: &Path) -> Option<String> {
    if is_markdown_path(path) {
        return None;
    }
    Some(language_hint(path))
}

fn language_hint(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase())
        })
        .unwrap_or_else(|| String::from("txt"))
}

/// Returns whether `path` is a non-Markdown file shown as source.
pub fn is_source_path(path: &Path) -> bool {
    source_language(path).is_some()
}

/// Returns whether the app will try to open `path` as a document.
pub fn is_editable_path(path: &Path) -> bool {
    path.file_name().is_some()
}

fn paths_refer_to_same_folder(
    stored: &Path,
    requested: &Path,
    requested_canonical: Option<&Path>,
) -> bool {
    if let Ok(stored_canonical) = stored.canonicalize() {
        if let Some(requested_canonical) = requested_canonical {
            return stored_canonical == requested_canonical;
        }
        return stored_canonical == requested;
    }
    stored == requested
}
