//! Persisted file-tree expansion and the last open session.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::store::VersionedDocument;
use crate::{Error, Result};

/// Maximum number of document tabs kept in the last session.
const MAX_SESSION_TABS: usize = 64;

/// On-disk map of expanded directories and the last open workspace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    /// Storage schema version.
    pub schema_version: u32,
    /// Project-relative directory paths that were expanded in the tree.
    #[serde(default)]
    pub expanded: std::collections::BTreeMap<String, Vec<String>>,
    /// Files and tabs that were open the last time the app quit.
    #[serde(default)]
    pub session: Option<OpenSession>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            schema_version: <Self as VersionedDocument>::SCHEMA_VERSION,
            expanded: std::collections::BTreeMap::new(),
            session: None,
        }
    }
}

impl VersionedDocument for UiState {
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
        for paths in self.expanded.values() {
            for path in paths {
                validate_rel(Path::new(path))?;
            }
        }
        if let Some(session) = &self.session {
            session.validate()?;
        }
        Ok(())
    }
}

/// One document tab remembered for the next launch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct SessionTab {
    /// Project for this tab. An older session inherits the workspace's project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    /// Project-relative path of the open document.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// Whether the tab is a temporary preview tab.
    pub preview: bool,
    /// `preview`, `editor`, or `split` for this tab. Empty uses the session mode.
    #[serde(default)]
    pub view_mode: String,
}

/// Workspace the user left open: project, document tabs, and extra tabs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct OpenSession {
    /// Project selected in the sidebar and owner of the active document.
    pub project_id: String,
    /// Document tabs in strip order.
    pub tabs: Vec<SessionTab>,
    /// Document that was focused, when one was.
    #[ts(type = "string | null")]
    pub active_rel_path: Option<PathBuf>,
    /// Open Tags Cloud, Library, Graph, Dashboard and Assistant tabs, in strip order.
    pub workspace_tabs: Vec<String>,
    /// Tab that was showing: `document`, `assistant`, `dashboard`, `graph`, `library`, or `tags`.
    pub page: String,
    /// `preview`, `editor`, or `split`. Empty keeps the configured default.
    pub view_mode: String,
}

/// What to open on launch, plus notes for paths that are gone.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct RestoredSession {
    /// Project to activate. Empty when nothing was saved or the folder is gone.
    pub project_id: Option<String>,
    /// Document tabs that still exist on disk.
    pub tabs: Vec<SessionTab>,
    /// Document to focus. Empty when no saved document remains.
    #[ts(type = "string | null")]
    pub active_rel_path: Option<PathBuf>,
    /// Tags Cloud, Library, Graph, Dashboard and Assistant tabs to show again.
    pub workspace_tabs: Vec<String>,
    /// Tab to show after the documents are open.
    pub page: String,
    /// View mode to apply. Empty keeps the configured default.
    pub view_mode: String,
    /// User-facing notes for files and folders that could not be reopened.
    pub notices: Vec<String>,
}

/// Whether a saved project folder is still registered and on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionProbe {
    /// The project id is still in the registry.
    pub project_listed: bool,
    /// User-visible project name, when the project is still listed.
    pub project_name: String,
    /// The project path is still a directory.
    pub root_is_dir: bool,
}

/// What a saved relative path points at now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiskKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// Nothing at that path.
    Missing,
}

impl RestoredSession {
    /// Nothing to reopen.
    pub fn empty() -> Self {
        Self {
            project_id: None,
            tabs: Vec::new(),
            active_rel_path: None,
            workspace_tabs: Vec::new(),
            page: "document".into(),
            view_mode: String::new(),
            notices: Vec::new(),
        }
    }
}

impl SessionTab {
    fn project<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.project_id.as_deref().unwrap_or(fallback)
    }
}

impl OpenSession {
    fn validate(&self) -> Result<()> {
        for tab in &self.tabs {
            validate_rel(&tab.rel_path)?;
        }
        if let Some(active) = &self.active_rel_path {
            validate_rel(active)?;
        }
        Ok(())
    }
}

impl UiState {
    /// Returns the expanded directories for `project_id`.
    pub fn expanded_for(&self, project_id: &str) -> Vec<String> {
        self.expanded.get(project_id).cloned().unwrap_or_default()
    }

    /// Replaces the expanded directories for `project_id`.
    pub fn set_expanded(&mut self, project_id: String, paths: Vec<PathBuf>) -> Result<()> {
        let mut cleaned = Vec::new();
        for path in paths {
            validate_rel(&path)?;
            if path.as_os_str().is_empty() {
                continue;
            }
            cleaned.push(display_rel(&path));
        }
        cleaned.sort();
        cleaned.dedup();
        if cleaned.is_empty() {
            self.expanded.remove(&project_id);
        } else {
            self.expanded.insert(project_id, cleaned);
        }
        Ok(())
    }

    /// Remembers the open project, document tabs, and workspace tabs.
    pub fn set_session(&mut self, session: OpenSession) -> Result<()> {
        let project_id = session.project_id.trim().to_owned();
        if project_id.is_empty() {
            self.session = None;
            return Ok(());
        }
        let mut tabs: Vec<SessionTab> = Vec::new();
        for tab in session.tabs {
            validate_rel(&tab.rel_path)?;
            if tab.rel_path.as_os_str().is_empty() {
                continue;
            }
            let rel_path = PathBuf::from(display_rel(&tab.rel_path));
            let view_mode = normalize_view_mode(&tab.view_mode);
            let owner = tab
                .project_id
                .as_deref()
                .map(str::trim)
                .filter(|owner| !owner.is_empty() && *owner != project_id)
                .map(str::to_owned);
            if let Some(existing) = tabs
                .iter_mut()
                .find(|item| item.rel_path == rel_path && item.project_id == owner)
            {
                if !tab.preview {
                    existing.preview = false;
                }
                if !view_mode.is_empty() {
                    existing.view_mode = view_mode;
                }
                continue;
            }
            tabs.push(SessionTab {
                project_id: owner,
                rel_path,
                preview: tab.preview,
                view_mode,
            });
        }
        if tabs.len() > MAX_SESSION_TABS {
            tabs = tabs.split_off(tabs.len() - MAX_SESSION_TABS);
        }
        keep_one_preview(&mut tabs);
        let active_rel_path = normalize_active(session.active_rel_path, &tabs, &project_id)?;
        let workspace_tabs = normalize_workspace(session.workspace_tabs, &session.page);
        let page = normalize_page(&session.page, &workspace_tabs);
        self.session = Some(OpenSession {
            project_id,
            tabs,
            active_rel_path,
            workspace_tabs,
            page,
            view_mode: normalize_view_mode(&session.view_mode),
        });
        Ok(())
    }

    /// Drops tabs and folders that are gone and returns what can still be opened.
    ///
    /// Missing paths are removed from the saved session so the same note is not
    /// added on every later launch. `kind_at` receives the owning project id and path.
    pub fn restore_session(
        &mut self,
        probe: &SessionProbe,
        kind_at: impl Fn(&str, &str) -> DiskKind,
    ) -> RestoredSession {
        let Some(session) = self.session.clone() else {
            return RestoredSession::empty();
        };
        let mut notices = Vec::new();
        let primary_available = probe.project_listed && probe.root_is_dir;
        if !probe.project_listed {
            notices.push(missing_project_notice());
        } else if !probe.root_is_dir {
            notices.push(missing_folder_notice(&probe.project_name));
        }

        let mut kept = Vec::new();
        for tab in &session.tabs {
            let project = tab.project(&session.project_id);
            if project == session.project_id && !primary_available {
                continue;
            }
            let rel = display_rel(&tab.rel_path);
            if kind_at(project, &rel) == DiskKind::File {
                kept.push(tab.clone());
            } else {
                notices.push(missing_file_notice(&rel));
            }
        }
        keep_one_preview(&mut kept);
        let mut project_id = session.project_id.clone();
        let active_rel_path = if primary_available {
            choose_active(
                &session.tabs,
                &kept,
                session.active_rel_path.as_deref(),
                &project_id,
            )
            .map(PathBuf::from)
        } else if let Some(last) = kept.last() {
            project_id = last.project(&session.project_id).to_owned();
            Some(last.rel_path.clone())
        } else {
            self.session = None;
            let mut restored = RestoredSession::empty();
            restored.notices = notices;
            return restored;
        };

        let mut drop_expanded = Vec::new();
        if let Some(paths) = self
            .expanded
            .get(&session.project_id)
            .filter(|_| primary_available)
        {
            for path in paths {
                if kind_at(&session.project_id, path) != DiskKind::Directory {
                    drop_expanded.push(path.clone());
                }
            }
        }
        if !drop_expanded.is_empty() {
            if let Some(paths) = self.expanded.get_mut(&session.project_id) {
                paths.retain(|path| !drop_expanded.iter().any(|gone| gone == path));
                if paths.is_empty() {
                    self.expanded.remove(&session.project_id);
                }
            }
            for path in &drop_expanded {
                notices.push(missing_dir_notice(path));
            }
        }

        let workspace_tabs = session.workspace_tabs.clone();
        let page = session.page.clone();
        let view_mode = session.view_mode.clone();
        self.session = Some(OpenSession {
            project_id: project_id.clone(),
            tabs: kept.clone(),
            active_rel_path: active_rel_path.clone(),
            workspace_tabs: workspace_tabs.clone(),
            page: page.clone(),
            view_mode: view_mode.clone(),
        });

        RestoredSession {
            project_id: Some(project_id),
            tabs: kept,
            active_rel_path,
            workspace_tabs,
            page,
            view_mode,
            notices,
        }
    }

    /// Drops expansion and session state for a project removed from the registry.
    pub fn remove_project(&mut self, project_id: &str) {
        self.expanded.remove(project_id);
        let Some(session) = &mut self.session else {
            return;
        };
        session
            .tabs
            .retain(|tab| tab.project(&session.project_id) != project_id);
        if session.project_id == project_id {
            if let Some(tab) = session.tabs.last() {
                session.project_id = tab.project(&session.project_id).to_owned();
                session.active_rel_path = Some(tab.rel_path.clone());
            } else {
                self.session = None;
            }
        }
    }
}

fn validate_rel(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(Error::UnsafePath {
            path: path.to_path_buf(),
            reason: "saved paths must be project-relative",
        });
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(Error::UnsafePath {
                path: path.to_path_buf(),
                reason: "saved paths cannot contain '..'",
            });
        }
    }
    Ok(())
}

fn display_rel(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_active(
    active: Option<PathBuf>,
    tabs: &[SessionTab],
    project_id: &str,
) -> Result<Option<PathBuf>> {
    let Some(active) = active else {
        return Ok(None);
    };
    validate_rel(&active)?;
    if active.as_os_str().is_empty() {
        return Ok(None);
    }
    let rel = PathBuf::from(display_rel(&active));
    if tabs
        .iter()
        .any(|tab| tab.project(project_id) == project_id && tab.rel_path == rel)
    {
        Ok(Some(rel))
    } else {
        Ok(tabs
            .iter()
            .rfind(|tab| tab.project(project_id) == project_id)
            .map(|tab| tab.rel_path.clone()))
    }
}

fn normalize_workspace(tabs: Vec<String>, page: &str) -> Vec<String> {
    let mut incoming = tabs;
    if matches!(
        page,
        "assistant" | "dashboard" | "graph" | "library" | "tags"
    ) {
        incoming.push(page.to_owned());
    }
    let mut out = Vec::new();
    for tab in incoming {
        if matches!(
            tab.as_str(),
            "assistant" | "dashboard" | "graph" | "library" | "tags"
        ) && !out.contains(&tab)
        {
            out.push(tab);
        }
    }
    out
}

fn normalize_page(page: &str, workspace: &[String]) -> String {
    if (matches!(
        page,
        "assistant" | "dashboard" | "graph" | "library" | "tags"
    )) && workspace.iter().any(|tab| tab == page)
    {
        page.to_owned()
    } else {
        "document".into()
    }
}

fn normalize_view_mode(value: &str) -> String {
    match value {
        "preview" | "editor" | "split" => value.to_owned(),
        _ => String::new(),
    }
}

fn keep_one_preview(tabs: &mut [SessionTab]) {
    let Some(last) = tabs.iter().rposition(|tab| tab.preview) else {
        return;
    };
    for (index, tab) in tabs.iter_mut().enumerate() {
        if index != last {
            tab.preview = false;
        }
    }
}

fn choose_active(
    original: &[SessionTab],
    kept: &[SessionTab],
    active: Option<&Path>,
    project_id: &str,
) -> Option<String> {
    let active = active?;
    let remains = |path: &Path| {
        kept.iter()
            .any(|tab| tab.project(project_id) == project_id && tab.rel_path == path)
    };
    if remains(active) {
        return Some(display_rel(active));
    }
    let index = original
        .iter()
        .position(|tab| tab.project(project_id) == project_id && tab.rel_path == active);
    if let Some(index) = index {
        let (before, after) = original.split_at(index);
        return after
            .iter()
            .skip(1)
            .chain(before.iter().rev())
            .find(|tab| tab.project(project_id) == project_id && remains(&tab.rel_path))
            .map(|tab| display_rel(&tab.rel_path));
    }
    kept.iter()
        .rfind(|tab| tab.project(project_id) == project_id)
        .map(|tab| display_rel(&tab.rel_path))
}

fn missing_file_notice(rel: &str) -> String {
    format!("Couldn't reopen \"{rel}\". That file is no longer in the project.")
}

fn missing_folder_notice(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        missing_project_notice()
    } else {
        format!("Couldn't reopen \"{name}\". That folder is no longer on disk.")
    }
}

fn missing_dir_notice(rel: &str) -> String {
    format!("Couldn't reopen \"{rel}\". That folder is no longer in the project.")
}

fn missing_project_notice() -> String {
    "Couldn't reopen the last folder. It is no longer in the project list.".into()
}
