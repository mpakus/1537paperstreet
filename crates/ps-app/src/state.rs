use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ps_core::agents::{
    AgentPresetInfo, AgentSessionStats, PromptHistory, PromptHistoryEntry, detect_presets,
    open_prompt_history,
};
use ps_core::config::Config;
use ps_core::dashboard::DashboardSnapshot;
use ps_core::docio::{
    self, DocOpenResult, DocPreview, DocumentMeta, DocumentSource, DocumentStat, LoadedDocument,
    RestoreTraits, TocEntry, WrittenDocument,
};
use ps_core::encrypted::{self, DocumentSecurity, EncryptedDocuments};
use ps_core::fsops::{self, ConflictStrategy, CopyOutcome, MoveOutcome, UntitledKind};
use ps_core::log::FileLog;
use ps_core::mermaid_cache::MermaidSvgCache;
use ps_core::paths::AppPaths;
use ps_core::projects::{
    self, OpenDropResult, Project, ProjectStore, ProjectsListQuery, ProjectsListResult,
};
use ps_core::search::ProjectSearch;
use ps_core::store::JsonStore;
use ps_core::themes::{ThemeCatalog, ThemeInfo};
use ps_core::tree::{self, TreeNode};
use ps_core::ui_state::{DiskKind, OpenSession, RestoredSession, SessionProbe, UiState};
use ps_core::{Error, Result};

#[derive(Clone)]
pub(crate) struct AppState {
    #[allow(dead_code)]
    paths: AppPaths,
    graph_lock: Arc<Mutex<()>>,
    library_lock: Arc<Mutex<()>>,
    tags: Arc<Mutex<ps_core::tags::TagIndex>>,
    encrypted: Arc<EncryptedDocuments>,
    pub(crate) exit_confirmed: Arc<std::sync::atomic::AtomicBool>,
    config: Arc<Mutex<JsonStore<Config>>>,
    projects: Arc<Mutex<ProjectStore>>,
    search: Arc<Mutex<ProjectSearch>>,
    ui_state: Arc<Mutex<JsonStore<UiState>>>,
    themes: Arc<ThemeCatalog>,
    log: Arc<FileLog>,
    mermaid_cache: Arc<MermaidSvgCache>,
    prompt_history: Arc<Mutex<JsonStore<PromptHistory>>>,
}

impl AppState {
    pub(crate) fn open(paths: AppPaths) -> Result<Self> {
        paths.ensure()?;
        let config = JsonStore::open(paths.config_file())?;
        let projects = ProjectStore::open(paths.projects_file())?;
        let ui_state = JsonStore::open(paths.ui_state_file())?;
        let mut search = ProjectSearch::new();
        for project in projects.list() {
            search.upsert(project.clone());
        }

        let log = FileLog::open(paths.log_file())?;
        let _ = log.info("application data opened");
        let themes = ThemeCatalog::load(&paths.themes());
        for warning in &themes.warnings {
            let _ = log.warn(warning);
        }

        let mermaid_cache = MermaidSvgCache::open(paths.mermaid_cache())?;
        let prompt_history = open_prompt_history(paths.agent_prompts_file())?;

        Ok(Self {
            graph_lock: Arc::new(Mutex::new(())),
            library_lock: Arc::new(Mutex::new(())),
            tags: Arc::new(Mutex::new(ps_core::tags::TagIndex::default())),
            paths,
            encrypted: Arc::new(EncryptedDocuments::default()),
            exit_confirmed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            config: Arc::new(Mutex::new(config)),
            projects: Arc::new(Mutex::new(projects)),
            search: Arc::new(Mutex::new(search)),
            ui_state: Arc::new(Mutex::new(ui_state)),
            themes: Arc::new(themes),
            log: Arc::new(log),
            mermaid_cache: Arc::new(mermaid_cache),
            prompt_history: Arc::new(Mutex::new(prompt_history)),
        })
    }

    /// Appends a warning that does not include document text.
    pub(crate) fn log_warn(&self, message: &str) {
        let _ = self.log.warn(message);
    }

    pub(crate) fn graph_get(
        &self,
        project_id: &str,
        query: &ps_core::graph::GraphQuery,
    ) -> Result<ps_core::graph::GraphSnapshot> {
        let root = self.project_root(project_id)?;
        let _guard = self
            .graph_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ps_render::project_graph(&self.paths.cache().join("graph"), &root, query)
    }

    pub(crate) fn config_get(&self) -> Config {
        self.config
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .value()
            .clone()
    }

    fn library_projects(&self) -> Vec<ps_core::library::LibraryProject> {
        self.projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .list()
            .iter()
            .map(|project| ps_core::library::LibraryProject {
                id: project.id.clone(),
                name: project.name.clone(),
                root: project.path.clone(),
            })
            .collect()
    }

    pub(crate) fn tags_get(
        &self,
        query: ps_core::tags::TagQuery,
        refresh: bool,
    ) -> Result<ps_core::tags::TagSnapshot> {
        self.tags.lock().unwrap_or_else(|e| e.into_inner()).query(
            &self.library_projects(),
            &query,
            refresh,
            ps_render::inline_tags,
        )
    }

    pub(crate) fn library_get(
        &self,
        query: ps_core::library::LibraryQuery,
    ) -> Result<ps_core::library::LibrarySnapshot> {
        let _guard = self.library_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut store = JsonStore::<ps_core::library::LibraryData>::open(
            self.paths.root().join("library.json"),
        )?;
        let mut snapshot = ps_core::library::scan(
            &self.library_projects(),
            &query,
            store.value(),
            ps_render::library_sections,
        )?;
        if let Some(ps_core::store::StoreWarning::CorruptFileMoved { preserved_at }) =
            store.take_warning()
        {
            snapshot.notices.push(format!("Damaged Library metadata was preserved at {}. Collections and reviews started empty.", preserved_at.display()));
        }
        Ok(snapshot)
    }

    pub(crate) fn library_update(
        &self,
        revision: String,
        change: ps_core::library::LibraryMutation,
    ) -> Result<String> {
        let _guard = self.library_lock.lock().unwrap_or_else(|e| e.into_inner());
        ps_core::library::mutate(
            &self.paths.root().join("library.json"),
            &revision,
            change,
            &self.library_projects(),
        )
        .map(|data| data.revision)
    }

    pub(crate) fn library_preview(
        &self,
        sources: Vec<ps_core::library::LibrarySource>,
    ) -> Result<ps_core::library::PackPreview> {
        ps_core::library::preview(
            &self.library_projects(),
            &sources,
            ps_render::library_sections,
        )
    }

    pub(crate) fn library_note_create(
        &self,
        project_id: String,
        input: ps_core::library::NoteInput,
    ) -> Result<String> {
        ps_core::library::create_note(&self.project_root(&project_id)?, &input)?;
        Ok(input.path)
    }

    pub(crate) fn config_set(&self, config: Config) -> Result<()> {
        config.validate()?;
        let mut store = self
            .config
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        store.replace(config);
        store.flush().map(|_| ())
    }

    pub(crate) fn projects_list(&self, query: ProjectsListQuery) -> ProjectsListResult {
        let limit = usize::try_from(query.limit).unwrap_or(usize::MAX);
        let offset = usize::try_from(query.offset).unwrap_or(usize::MAX);
        let trimmed = query
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let (mut items, total) = if let Some(needle) = trimmed {
            self.search
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .page(needle, limit, offset)
        } else {
            let projects = self
                .projects
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let all = projects.list();
            let total = all.len();
            (
                all.iter().skip(offset).take(limit).cloned().collect(),
                total,
            )
        };

        for item in &mut items {
            item.available = Some(item.path.is_dir());
        }

        ProjectsListResult {
            items,
            total: u32::try_from(total).unwrap_or(u32::MAX),
        }
    }

    pub(crate) fn themes_list(&self) -> Vec<ThemeInfo> {
        self.themes.infos()
    }

    pub(crate) fn themes_css(&self) -> String {
        self.themes.css()
    }

    pub(crate) fn mermaid_cache_get(
        &self,
        source_hash: String,
        theme_id: String,
    ) -> Result<Option<String>> {
        self.mermaid_cache.get(&source_hash, &theme_id)
    }

    pub(crate) fn mermaid_cache_put(
        &self,
        source_hash: String,
        theme_id: String,
        svg: String,
    ) -> Result<()> {
        self.mermaid_cache.put(&source_hash, &theme_id, &svg)
    }

    pub(crate) fn save_user_file(&self, path: PathBuf, bytes: Vec<u8>) -> Result<()> {
        save_user_picked_file(&path, &bytes)
    }

    pub(crate) fn projects_relocate(&self, id: String, path: PathBuf) -> Result<Project> {
        self.encrypted.ensure_all_locked()?;
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let project = projects.relocate(&id, path)?;
        self.search
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .upsert(project.clone());
        projects.flush()?;
        Ok(project)
    }

    pub(crate) fn projects_get(&self, id: String) -> Result<Project> {
        self.projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
    }

    pub(crate) fn projects_add(&self, name: String, path: PathBuf) -> Result<Project> {
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let project = projects.add(name, path)?;
        self.search
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .upsert(project.clone());
        projects.flush()?;
        Ok(project)
    }

    pub(crate) fn open_dropped_paths(&self, paths: Vec<PathBuf>) -> Result<OpenDropResult> {
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let result = projects::open_dropped_paths(&mut projects, &paths)?;
        let mut search = self
            .search
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for project in projects.list() {
            search.upsert(project.clone());
        }
        projects.flush()?;
        Ok(result)
    }

    pub(crate) fn projects_rename(&self, id: String, name: String) -> Result<()> {
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        projects.rename(&id, name)?;
        if let Some(project) = projects
            .list()
            .iter()
            .find(|project| project.id == id)
            .cloned()
        {
            self.search
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .upsert(project);
        }
        projects.flush().map(|_| ())
    }

    pub(crate) fn projects_remove(&self, id: String) -> Result<()> {
        self.encrypted
            .ensure_locked(&self.project_root(&id)?, Path::new(""))?;
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        projects.remove(&id)?;
        self.search
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&id);
        let mut ui_state = self
            .ui_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ui_state.update(|state| state.remove_project(&id));
        ui_state.flush()?;
        let _ = self
            .log
            .info(&format!("removed project {id} from the list"));
        projects.flush().map(|_| ())
    }

    pub(crate) fn tree_read_dir(
        &self,
        project_id: String,
        rel_path: PathBuf,
    ) -> Result<Vec<TreeNode>> {
        let root = self.project_root(&project_id)?;
        let show_hidden = self.config_get().files.show_hidden;
        tree::read_dir(&root, &rel_path, show_hidden)
    }

    pub(crate) fn tree_expanded_get(&self, project_id: String) -> Vec<String> {
        self.ui_state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .value()
            .expanded_for(&project_id)
    }

    pub(crate) fn tree_expanded_set(
        &self,
        project_id: String,
        rel_paths: Vec<PathBuf>,
    ) -> Result<()> {
        let mut store = self
            .ui_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut state = store.value().clone();
        state.set_expanded(project_id, rel_paths)?;
        store.replace(state);
        store.flush().map(|_| ())
    }

    pub(crate) fn session_set(&self, session: OpenSession) -> Result<()> {
        let mut store = self
            .ui_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut state = store.value().clone();
        state.set_session(session)?;
        store.replace(state);
        store.flush().map(|_| ())
    }

    pub(crate) fn session_restore(&self) -> Result<RestoredSession> {
        let project_id = {
            let store = self
                .ui_state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match store.value().session.as_ref() {
                Some(session) => session.project_id.clone(),
                None => return Ok(RestoredSession::empty()),
            }
        };
        let (probe, root) = self.session_probe(&project_id);
        let mut store = self
            .ui_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut state = store.value().clone();
        if state
            .session
            .as_ref()
            .is_none_or(|session| session.project_id != project_id)
        {
            return Ok(RestoredSession::empty());
        }
        let restored = state.restore_session(&probe, |rel| disk_kind(root.as_deref(), rel));
        if state != *store.value() {
            store.replace(state);
            store.flush()?;
        }
        Ok(restored)
    }

    fn session_probe(&self, project_id: &str) -> (SessionProbe, Option<PathBuf>) {
        let projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match projects.get(project_id).ok() {
            Some(project) => {
                let probe = SessionProbe {
                    project_listed: true,
                    project_name: project.name.clone(),
                    root_is_dir: project.path.is_dir(),
                };
                (probe, Some(project.path.clone()))
            }
            None => (
                SessionProbe {
                    project_listed: false,
                    project_name: String::new(),
                    root_is_dir: false,
                },
                None,
            ),
        }
    }

    pub(crate) fn fs_mkdir(&self, project_id: String, rel_path: PathBuf) -> Result<TreeNode> {
        let root = self.project_root(&project_id)?;
        let absolute = fsops::mkdir(&root, &rel_path)?;
        tree::node_at(&root, &absolute)
    }

    pub(crate) fn fs_create_file(&self, project_id: String, rel_path: PathBuf) -> Result<TreeNode> {
        let root = self.project_root(&project_id)?;
        let absolute = fsops::create_file(&root, &rel_path)?;
        tree::node_at(&root, &absolute)
    }

    pub(crate) fn fs_create_untitled(
        &self,
        project_id: String,
        parent_rel: PathBuf,
        kind: UntitledKind,
    ) -> Result<TreeNode> {
        let root = self.project_root(&project_id)?;
        let absolute = fsops::create_untitled(&root, &parent_rel, kind)?;
        tree::node_at(&root, &absolute)
    }

    pub(crate) fn reveal_in_finder(&self, project_id: String, rel_path: PathBuf) -> Result<()> {
        let path = self.absolute_in_project(&project_id, &rel_path)?;
        crate::open_path::reveal(&path)
    }

    pub(crate) fn open_external(&self, project_id: String, rel_path: PathBuf) -> Result<()> {
        let path = self.absolute_in_project(&project_id, &rel_path)?;
        crate::open_path::launch(&path)
    }

    pub(crate) fn fs_rename(
        &self,
        project_id: String,
        from: PathBuf,
        to: PathBuf,
    ) -> Result<TreeNode> {
        let root = self.project_root(&project_id)?;
        self.encrypted.ensure_locked(&root, &from)?;
        self.encrypted.ensure_locked(&root, &to)?;
        let absolute = fsops::rename(&root, &from, &to)?;
        tree::node_at(&root, &absolute)
    }

    pub(crate) fn fs_copy(
        &self,
        project_id: String,
        from: Vec<PathBuf>,
        to_dir: PathBuf,
        conflict: ConflictStrategy,
    ) -> Result<Vec<TreeNode>> {
        let root = self.project_root(&project_id)?;
        let mut nodes = Vec::with_capacity(from.len());
        for rel_path in from {
            let file_name = rel_path.file_name().ok_or_else(|| Error::UnsafePath {
                path: rel_path.clone(),
                reason: "the copy source does not have a file name",
            })?;
            let destination = to_dir.join(file_name);
            if conflict == ConflictStrategy::Replace {
                self.encrypted.ensure_locked(&root, &destination)?;
            }
            let outcome = fsops::copy(
                &root,
                &rel_path,
                &destination,
                conflict,
                pending_history_snapshot,
                |_| {},
            )?;
            let absolute = match outcome {
                CopyOutcome::Copied { path } | CopyOutcome::Skipped { path } => path,
            };
            nodes.push(tree::node_at(&root, &absolute)?);
        }
        Ok(nodes)
    }

    pub(crate) fn fs_transfer(
        &self,
        from_project_id: String,
        from: Vec<PathBuf>,
        to_project_id: String,
        to_dir: PathBuf,
        copy: bool,
        conflict: ConflictStrategy,
    ) -> Result<Vec<TreeNode>> {
        self.fs_transfer_with(
            from_project_id,
            from,
            to_project_id,
            to_dir,
            copy,
            conflict,
            |root, paths| fsops::trash(root, paths, pending_history_snapshot),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn fs_transfer_with<D>(
        &self,
        from_project_id: String,
        from: Vec<PathBuf>,
        to_project_id: String,
        to_dir: PathBuf,
        copy: bool,
        conflict: ConflictStrategy,
        delete_source: D,
    ) -> Result<Vec<TreeNode>>
    where
        D: FnOnce(&Path, &[PathBuf]) -> Result<()>,
    {
        let from_root = self.project_root(&from_project_id)?;
        let to_root = self.project_root(&to_project_id)?;
        if !copy {
            for rel in &from {
                self.encrypted.ensure_locked(&from_root, rel)?;
            }
        }
        if conflict == ConflictStrategy::Replace {
            self.encrypted.ensure_locked(&to_root, &to_dir)?;
        }
        let from_canonical = from_root.canonicalize().map_err(|source| Error::Io {
            action: "open the source project",
            path: from_root.clone(),
            source,
        })?;
        let to_canonical = to_root.canonicalize().map_err(|source| Error::Io {
            action: "open the destination project",
            path: to_root.clone(),
            source,
        })?;
        if from_canonical == to_canonical {
            return if copy {
                self.fs_copy(to_project_id, from, to_dir, conflict)
            } else {
                self.fs_move(to_project_id, from, to_dir, conflict)
            };
        }

        let mut nodes = Vec::with_capacity(from.len());
        let mut copied = Vec::new();
        for rel_path in from {
            let source = fsops::resolve(&from_root, &rel_path)?;
            let outcome = fsops::import_into(
                &to_root,
                &to_dir,
                std::slice::from_ref(&source),
                conflict,
                pending_history_snapshot,
                |_| {},
            )?
            .into_iter()
            .next()
            .ok_or_else(|| Error::UnsafePath {
                path: rel_path.clone(),
                reason: "the item could not be copied into the other project",
            })?;
            let absolute = match &outcome {
                CopyOutcome::Copied { path } | CopyOutcome::Skipped { path } => path.clone(),
            };
            if matches!(outcome, CopyOutcome::Copied { .. }) {
                copied.push(rel_path);
            }
            nodes.push(tree::node_at(&to_root, &absolute)?);
        }
        if !copy && !copied.is_empty() {
            delete_source(&from_root, &copied)?;
        }
        Ok(nodes)
    }

    pub(crate) fn fs_import(
        &self,
        project_id: String,
        sources: Vec<PathBuf>,
        to_dir: PathBuf,
        conflict: ConflictStrategy,
    ) -> Result<Vec<TreeNode>> {
        let root = self.project_root(&project_id)?;
        if conflict == ConflictStrategy::Replace {
            self.encrypted.ensure_locked(&root, &to_dir)?;
        }
        fsops::import_into(
            &root,
            &to_dir,
            &sources,
            conflict,
            pending_history_snapshot,
            |_| {},
        )?
        .into_iter()
        .map(|outcome| {
            let absolute = match outcome {
                CopyOutcome::Copied { path } | CopyOutcome::Skipped { path } => path,
            };
            tree::node_at(&root, &absolute)
        })
        .collect()
    }

    pub(crate) fn fs_move(
        &self,
        project_id: String,
        from: Vec<PathBuf>,
        to_dir: PathBuf,
        conflict: ConflictStrategy,
    ) -> Result<Vec<TreeNode>> {
        let root = self.project_root(&project_id)?;
        for rel in &from {
            self.encrypted.ensure_locked(&root, rel)?;
        }
        if conflict == ConflictStrategy::Replace {
            self.encrypted.ensure_locked(&root, &to_dir)?;
        }
        fsops::move_items(
            &root,
            &from,
            &to_dir,
            conflict,
            pending_history_snapshot,
            |_| {},
        )?
        .into_iter()
        .map(|outcome| {
            let absolute = match outcome {
                MoveOutcome::Moved { path, .. } | MoveOutcome::Skipped { path, .. } => path,
            };
            tree::node_at(&root, &absolute)
        })
        .collect()
    }

    pub(crate) fn fs_trash(&self, project_id: String, rel_paths: Vec<PathBuf>) -> Result<()> {
        self.fs_trash_with(project_id, rel_paths, |root, paths| {
            fsops::trash(root, paths, pending_history_snapshot)
        })
    }

    fn fs_trash_with<D>(&self, project_id: String, rel_paths: Vec<PathBuf>, delete: D) -> Result<()>
    where
        D: FnOnce(&Path, &[PathBuf]) -> Result<()>,
    {
        let root = self.project_root(&project_id)?;
        for rel in &rel_paths {
            self.encrypted.ensure_locked(&root, rel)?;
        }
        delete(&root, &rel_paths)
    }

    pub(crate) fn files_search(
        &self,
        project_id: String,
        query: String,
        limit: u32,
    ) -> Result<Vec<TreeNode>> {
        let root = self.project_root(&project_id)?;
        let show_hidden = self.config_get().files.show_hidden;
        tree::search_markdown(
            &root,
            &query,
            show_hidden,
            usize::try_from(limit).unwrap_or(usize::MAX),
        )
    }

    pub(crate) fn content_search(
        &self,
        project_id: String,
        scope: PathBuf,
        query: String,
        limit: u32,
    ) -> Result<ps_core::content_search::TextSearch> {
        let root = self.project_root(&project_id)?;
        let show_hidden = self.config_get().files.show_hidden;
        let limit = usize::try_from(limit).unwrap_or(40).min(100);
        ps_core::content_search::search_documents(&root, &scope, &query, show_hidden, limit)
    }

    pub(crate) fn copy_conflicts(
        &self,
        project_id: String,
        from: Vec<PathBuf>,
        to_dir: PathBuf,
    ) -> Result<Vec<String>> {
        let root = self.project_root(&project_id)?;
        let mut names = Vec::new();
        for rel_path in from {
            let file_name = rel_path.file_name().ok_or_else(|| Error::UnsafePath {
                path: rel_path.clone(),
                reason: "the copy source does not have a file name",
            })?;
            let destination = to_dir.join(file_name);
            let absolute = fsops::resolve(&root, &destination)?;
            if std::fs::symlink_metadata(&absolute).is_ok() {
                names.push(file_name.to_string_lossy().into_owned());
            }
        }
        Ok(names)
    }

    pub(crate) fn resolve_asset(&self, project_id: &str, rel_path: &Path) -> Result<PathBuf> {
        let path = self.absolute_in_project(project_id, rel_path)?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|source| Error::Io {
            action: "open the asset",
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() {
            return Err(Error::UnsafePath {
                path: path.clone(),
                reason: "folders cannot be served as assets",
            });
        }
        Ok(path)
    }

    pub(crate) fn open_url(url: &str) -> Result<()> {
        if !is_http_url(url) {
            return Err(Error::UnsupportedUrl);
        }
        crate::open_path::open_url(url)
    }

    pub(crate) fn doc_unlock(
        &self,
        project_id: String,
        rel_path: PathBuf,
        passphrase: String,
    ) -> Result<()> {
        self.encrypted.unlock(
            &self.project_root(&project_id)?,
            &rel_path,
            passphrase.into(),
        )
    }

    pub(crate) fn doc_lock(&self, project_id: String, rel_path: PathBuf) -> Result<()> {
        self.encrypted
            .lock(&self.project_root(&project_id)?, &rel_path)
    }

    pub(crate) fn doc_lock_all(&self) -> Result<()> {
        self.encrypted.lock_all()
    }

    pub(crate) fn doc_create_encrypted(
        &self,
        project_id: String,
        rel_path: PathBuf,
        text: String,
        passphrase: String,
    ) -> Result<()> {
        self.encrypted.create(
            &self.project_root(&project_id)?,
            &rel_path,
            &text,
            passphrase.into(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn doc_encrypt_existing(
        &self,
        project_id: String,
        rel_path: PathBuf,
        destination: PathBuf,
        text: String,
        base_hash: String,
        traits: RestoreTraits,
        passphrase: String,
    ) -> Result<()> {
        self.encrypted.encrypt_existing(
            &self.project_root(&project_id)?,
            &rel_path,
            &destination,
            &text,
            &base_hash,
            traits,
            passphrase.into(),
        )
    }

    pub(crate) fn doc_delete_original(&self, project_id: String, rel_path: PathBuf) -> Result<()> {
        self.encrypted
            .delete_original(&self.project_root(&project_id)?, &rel_path)
    }

    pub(crate) fn doc_snapshot_encrypted(
        &self,
        project_id: String,
        rel_path: PathBuf,
        text: String,
        traits: RestoreTraits,
    ) -> Result<()> {
        self.encrypted.snapshot_before_discard(
            &self.project_root(&project_id)?,
            &rel_path,
            &text,
            traits,
        )
    }

    pub(crate) fn doc_change_password(
        &self,
        project_id: String,
        rel_path: PathBuf,
        base_hash: String,
        passphrase: String,
    ) -> Result<WrittenDocument> {
        let root = self.project_root(&project_id)?;
        let loaded = self.encrypted.read(&root, &rel_path)?;
        self.encrypted.save(
            &root,
            &rel_path,
            &loaded.source.text,
            &base_hash,
            RestoreTraits::from_source(&loaded.source),
            Some(passphrase.into()),
        )
    }

    pub(crate) fn doc_export_decrypted(
        &self,
        project_id: String,
        rel_path: PathBuf,
        destination: PathBuf,
        base_hash: String,
    ) -> Result<()> {
        self.encrypted.export(
            &self.project_root(&project_id)?,
            &rel_path,
            &destination,
            &base_hash,
        )
    }

    pub(crate) fn doc_source(
        &self,
        project_id: String,
        rel_path: PathBuf,
    ) -> Result<DocumentSource> {
        Ok(self.load_document(&project_id, &rel_path)?.source)
    }

    pub(crate) fn doc_stat(&self, project_id: String, rel_path: PathBuf) -> Result<DocumentStat> {
        let root = self.project_root(&project_id)?;
        if encrypted::is_age_path(&rel_path) {
            self.encrypted.stat(&root, &rel_path)
        } else {
            docio::stat_doc(&root, &rel_path)
        }
    }

    pub(crate) fn doc_save(
        &self,
        project_id: String,
        rel_path: PathBuf,
        text: String,
        base_hash: String,
        traits: RestoreTraits,
    ) -> Result<WrittenDocument> {
        let root = self.project_root(&project_id)?;
        if encrypted::is_age_path(&rel_path) {
            self.encrypted
                .save(&root, &rel_path, &text, &base_hash, traits, None)
        } else {
            docio::write_doc(&root, &rel_path, &text, &base_hash, traits)
        }
    }

    pub(crate) fn doc_open(
        &self,
        project_id: String,
        rel_path: PathBuf,
    ) -> Result<PreparedDocument> {
        let root = self.project_root(&project_id)?;
        let security = if encrypted::is_age_path(&rel_path) {
            self.encrypted.status(&root, &rel_path)?
        } else {
            DocumentSecurity::Plain
        };
        if security == DocumentSecurity::Locked {
            let stat = self.encrypted.stat(&root, &rel_path)?;
            return Ok(PreparedDocument {
                result: DocOpenResult {
                    meta: DocumentMeta {
                        security,
                        project_id,
                        title: document_title(&rel_path, &[]),
                        rel_path,
                        hash: stat.hash,
                        size: stat.size,
                        writable: false,
                        readonly_reason: Some(
                            "Unlock this encrypted note to read or edit it.".into(),
                        ),
                        source_only: true,
                        chunk_count: 0,
                        toc: Vec::new(),
                    },
                    first_chunk: None,
                },
                remaining_chunks: Vec::new(),
            });
        }
        let loaded = self.load_document(&project_id, &rel_path)?;
        let allow_raw_html = self.config_get().viewer.allow_raw_html;

        let (toc, mut chunks) = if projects::is_image_path(&rel_path) {
            (
                Vec::new(),
                vec![ps_render::render_image(&project_id, &rel_path)],
            )
        } else if loaded.source_only {
            (Vec::new(), Vec::new())
        } else if let Some(language) = projects::source_language(&rel_path) {
            let html = ps_render::render_source(&loaded.source.text, &language);
            (
                Vec::new(),
                ps_render::html_chunks(&html)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
        } else {
            let rendered = ps_render::render_project_with_options(
                &loaded.source.text,
                &root,
                &rel_path,
                &project_id,
                ps_render::RenderOptions { allow_raw_html },
            );
            let toc = rendered
                .toc
                .into_iter()
                .map(|item| TocEntry {
                    level: item.level,
                    title: item.title,
                    id: item.id,
                })
                .collect();
            let chunks = ps_render::html_chunks(&rendered.html)
                .into_iter()
                .map(str::to_owned)
                .collect();
            (toc, chunks)
        };

        // Protected HTML is returned directly, never broadcast to other windows.
        if security == DocumentSecurity::Unlocked && !chunks.is_empty() {
            chunks = vec![chunks.concat()];
        }
        let chunk_count = u32::try_from(chunks.len()).unwrap_or(u32::MAX);
        let first_chunk = if chunks.is_empty() {
            None
        } else {
            Some(chunks.remove(0))
        };
        // Keep headings out of OS titles and saved UI state for protected notes.
        let title = document_title(
            &rel_path,
            if security == DocumentSecurity::Plain {
                &toc
            } else {
                &[]
            },
        );

        Ok(PreparedDocument {
            result: DocOpenResult {
                meta: DocumentMeta {
                    security,
                    project_id,
                    rel_path,
                    title,
                    hash: loaded.hash,
                    size: loaded.size,
                    writable: loaded.source.writable,
                    readonly_reason: loaded.source.readonly_reason,
                    source_only: loaded.source_only,
                    chunk_count,
                    toc,
                },
                first_chunk,
            },
            remaining_chunks: chunks,
        })
    }

    /// Renders editor text for Split, without writing the file.
    pub(crate) fn doc_preview(
        &self,
        project_id: String,
        rel_path: PathBuf,
        text: String,
    ) -> Result<DocPreview> {
        let root = self.project_root(&project_id)?;
        let _resolved = fsops::resolve(&root, &rel_path)?;
        let allow_raw_html = self.config_get().viewer.allow_raw_html;
        let rendered = ps_render::render_project_with_options(
            &text,
            &root,
            &rel_path,
            &project_id,
            ps_render::RenderOptions { allow_raw_html },
        );
        let toc = rendered
            .toc
            .into_iter()
            .map(|item| TocEntry {
                level: item.level,
                title: item.title,
                id: item.id,
            })
            .collect();
        Ok(DocPreview {
            html: rendered.html,
            toc,
        })
    }

    fn load_document(&self, project_id: &str, rel_path: &Path) -> Result<LoadedDocument> {
        let root = self.project_root(project_id)?;
        if encrypted::is_age_path(rel_path) {
            self.encrypted.read(&root, rel_path)
        } else {
            docio::read_doc(&root, rel_path)
        }
    }

    pub(crate) fn project_root(&self, id: &str) -> Result<PathBuf> {
        let projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Ok(projects.get(id)?.path.clone())
    }

    pub(crate) fn agent_presets() -> Vec<AgentPresetInfo> {
        detect_presets(None)
    }

    pub(crate) fn agent_server(&self, id: &str) -> Result<ps_core::agents::AgentServer> {
        self.config_get()
            .agents
            .servers
            .into_iter()
            .find(|server| server.id == id)
            .ok_or_else(|| Error::Agent {
                message: "That agent is not in Settings.".into(),
            })
    }

    pub(crate) fn prompt_history_list(&self) -> Vec<PromptHistoryEntry> {
        self.prompt_history
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .value()
            .entries
            .clone()
    }

    pub(crate) fn prompt_history_push(
        &self,
        server_id: String,
        text: &str,
    ) -> Result<PromptHistoryEntry> {
        let mut store = self
            .prompt_history
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut history = store.value().clone();
        let entry = history.push(server_id, text)?;
        store.replace(history);
        store.flush()?;
        Ok(entry)
    }

    pub(crate) fn prompt_history_remove(&self, id: &str) -> Result<()> {
        let mut store = self
            .prompt_history
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut history = store.value().clone();
        history.remove(id)?;
        store.replace(history);
        store.flush()?;
        Ok(())
    }

    pub(crate) fn prompt_history_clear(&self) -> Result<()> {
        let mut store = self
            .prompt_history
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut history = store.value().clone();
        history.clear();
        store.replace(history);
        store.flush()?;
        Ok(())
    }

    pub(crate) fn dashboard(
        &self,
        project_id: Option<&str>,
        session: AgentSessionStats,
    ) -> DashboardSnapshot {
        let config = self.config_get();
        let projects = {
            let store = self
                .projects
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            store.list().to_vec()
        };
        let active =
            project_id.and_then(|id| projects.iter().find(|project| project.id == id).cloned());
        let markdown = active
            .as_ref()
            .and_then(|project| tree::count_markdown(&project.path, config.files.show_hidden).ok())
            .unwrap_or_default();
        let history = self.prompt_history_list();
        ps_core::dashboard::snapshot(
            &projects,
            &config.agents,
            &history,
            active.as_ref(),
            markdown,
            &session,
        )
    }

    fn absolute_in_project(&self, project_id: &str, rel_path: &Path) -> Result<PathBuf> {
        let root = self.project_root(project_id)?;
        fsops::resolve(&root, rel_path)
    }

    #[cfg(test)]
    pub(crate) fn paths(&self) -> &AppPaths {
        &self.paths
    }
}

/// History snapshots for destructive filesystem operations land in T-200.
fn pending_history_snapshot(_: &Path) -> Result<()> {
    Ok(())
}

fn is_http_url(url: &str) -> bool {
    if url.is_empty()
        || url
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == b'\0')
    {
        return false;
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty()
        && !host.contains('@')
        && host.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | ':')
        })
}

/// Writes bytes chosen through a native Save dialog. This is not a project path.
fn save_user_picked_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0) {
        return Err(Error::UnsafePath {
            path: path.to_path_buf(),
            reason: "the save location must be an absolute path",
        });
    }
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if !matches!(extension, "png" | "svg" | "pdf") {
        return Err(Error::UnsafePath {
            path: path.to_path_buf(),
            reason: "the file can be saved as PNG, SVG, or PDF",
        });
    }
    let Some(parent) = path.parent() else {
        return Err(Error::UnsafePath {
            path: path.to_path_buf(),
            reason: "the save location has no parent folder",
        });
    };
    if !parent.is_dir() {
        return Err(Error::Io {
            action: "open the save folder",
            path: parent.to_path_buf(),
            source: io::Error::from(io::ErrorKind::NotFound),
        });
    }
    let temporary = parent.join(format!(
        ".{}.part",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("diagram")
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|source| Error::Io {
            action: "create the saved file",
            path: temporary.clone(),
            source,
        })?;
    std::io::Write::write_all(&mut file, bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| Error::Io {
            action: "write the saved file",
            path: temporary.clone(),
            source,
        })?;
    drop(file);
    std::fs::rename(&temporary, path).map_err(|source| Error::Io {
        action: "replace the saved file",
        path: path.to_path_buf(),
        source,
    })
}

/// Rendered document ready for the `doc_open` command to return and stream.
pub(crate) struct PreparedDocument {
    /// Metadata and the first HTML chunk.
    pub result: DocOpenResult,
    /// Chunks 1..n emitted on `doc://chunk` after the command returns.
    pub remaining_chunks: Vec<String>,
}

fn document_title(rel_path: &Path, toc: &[TocEntry]) -> String {
    toc.first()
        .map(|entry| entry.title.as_str())
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            rel_path
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| String::from("Untitled"))
}

fn disk_kind(root: Option<&Path>, rel: &str) -> DiskKind {
    let Some(root) = root else {
        return DiskKind::Missing;
    };
    match fsops::resolve(root, Path::new(rel)) {
        Ok(absolute) if absolute.is_file() => DiskKind::File,
        Ok(absolute) if absolute.is_dir() => DiskKind::Directory,
        _ => DiskKind::Missing,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use ps_core::agents::{AgentServer, AgentSessionStats};
    use ps_core::config::Config;
    use ps_core::docio::RestoreTraits;
    use ps_core::fsops::ConflictStrategy;
    use ps_core::paths::AppPaths;
    use ps_core::projects::ProjectsListQuery;

    use super::AppState;

    #[test]
    fn tags_project_lookup_reaches_beyond_the_first_registry_page() {
        let (temporary, state) = open_state();
        let mut last = None;
        for n in 0..205 {
            let root = temporary.path().join(format!("folder-{n}"));
            fs::create_dir(&root).unwrap();
            last = Some(state.projects_add(format!("Folder {n:03}"), root).unwrap());
        }
        let project = last.unwrap();
        assert_eq!(
            state.projects_get(project.id.clone()).unwrap().path,
            project.path
        );
        assert!(state.projects_get("missing".into()).is_err());
        fs::write(project.path.join("note.md"), "#connected").unwrap();
        let view = state
            .tags_get(
                ps_core::tags::TagQuery {
                    project_id: project.id.clone(),
                    ..Default::default()
                },
                true,
            )
            .unwrap();
        assert_eq!(view.notes[0].project_id, project.id);
    }

    fn open_state() -> (tempfile::TempDir, AppState) {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let state = AppState::open(AppPaths::from_root(temporary.path()))
            .expect("application state should open");
        (temporary, state)
    }

    #[test]
    fn library_reopens_saved_sources_and_excludes_missing_projects() {
        use ps_core::library::{LibraryMutation, LibrarySource, NoteInput, SavedSet, SavedSetKind};
        let (temporary, state) = open_state();
        let root = temporary.path().join("notes");
        fs::create_dir(&root).unwrap();
        let project = state.projects_add("Notes".into(), root.clone()).unwrap();
        state
            .library_note_create(
                project.id.clone(),
                NoteInput {
                    path: "study.md".into(),
                    title: "Study".into(),
                    note_type: "paper".into(),
                    status: "unread".into(),
                    tags: "agents".into(),
                    source: "doi:example".into(),
                },
            )
            .unwrap();
        let snapshot = state.library_get(Default::default()).unwrap();
        assert_eq!(snapshot.notes.len(), 1);
        assert!(
            snapshot.notes[0]
                .sections
                .iter()
                .any(|section| section.id == "findings")
        );
        let source = LibrarySource {
            project_id: project.id,
            path: "study.md".into(),
            section: Some("findings".into()),
            hash: snapshot.notes[0].hash.clone(),
        };
        state
            .library_update(
                snapshot.revision,
                LibraryMutation::Save {
                    set: SavedSet {
                        id: String::new(),
                        name: "Study pack".into(),
                        kind: SavedSetKind::Pack,
                        sources: vec![source],
                    },
                },
            )
            .unwrap();
        drop(state);
        let state = AppState::open(AppPaths::from_root(temporary.path())).unwrap();
        let snapshot = state.library_get(Default::default()).unwrap();
        let pack = state
            .library_preview(snapshot.sets[0].sources.clone())
            .unwrap();
        assert!(pack.complete);
        assert!(pack.text.contains("## Findings"));
        assert!(!pack.text.contains("## Methods"));
        let mut missing = snapshot.sets[0].sources.clone();
        missing[0].project_id = "unregistered".into();
        assert!(!state.library_preview(missing).unwrap().complete);
        assert!(
            !fs::read_to_string(temporary.path().join("library.json"))
                .unwrap()
                .contains("## Findings")
        );
    }

    #[test]
    fn opens_default_stores_under_the_application_root() {
        let (temporary, state) = open_state();

        assert_eq!(state.paths().root(), temporary.path());
        assert_eq!(state.config_get(), Config::default());
        assert_eq!(
            state
                .projects_list(ProjectsListQuery {
                    limit: 50,
                    ..ProjectsListQuery::default()
                })
                .total,
            0
        );
        assert!(state.paths().mermaid_cache().is_dir());
    }

    #[test]
    fn records_warnings_in_the_application_log() {
        let (_temporary, state) = open_state();
        state.log_warn("sidebar vibrancy could not be applied");
        let text = fs::read_to_string(state.paths().log_file()).expect("log");
        assert!(text.contains("warn sidebar vibrancy could not be applied"));
        assert!(!text.contains("# Heading"));
    }

    #[test]
    fn agent_history_and_dashboard_share_persisted_application_state() {
        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir(&notes).expect("project directory");
        fs::write(notes.join("readme.md"), b"# Notes\n").expect("document");
        let project = state
            .projects_add("Notes".into(), notes)
            .expect("add project");

        let server = AgentServer::custom("Local", "/usr/bin/true", Vec::new());
        let mut config = state.config_get();
        config.agents.default_server_id = Some(server.id.clone());
        config.agents.servers.push(server.clone());
        state.config_set(config).expect("save agent");

        assert_eq!(state.agent_server(&server.id).expect("server"), server);
        assert!(state.agent_server("missing").is_err());
        assert_eq!(AppState::agent_presets().len(), 3);

        let entry = state
            .prompt_history_push(server.id.clone(), "Summarize these notes")
            .expect("save prompt");
        assert_eq!(entry.text, "Summarize these notes");
        assert_eq!(state.prompt_history_list(), vec![entry.clone()]);

        let session = AgentSessionStats::started(&server.name);
        let dashboard = state.dashboard(Some(&project.id), session);
        assert_eq!(dashboard.title, "Dashboard");
        assert!(dashboard.sections.iter().any(|section| {
            section.title == "Recent prompts"
                && section.rows.iter().any(|row| row.title == entry.text)
        }));

        let reopened = AppState::open(AppPaths::from_root(temporary.path())).expect("reopen");
        assert_eq!(reopened.prompt_history_list(), vec![entry]);
        let empty = reopened.dashboard(Some("missing"), AgentSessionStats::default());
        assert!(empty.sections.iter().any(|section| {
            section.title == "Open project"
                && section
                    .metrics
                    .iter()
                    .any(|metric| metric.label == "Folder" && metric.value == "None")
        }));
    }

    #[test]
    fn prompt_history_remove_and_clear_persist() {
        let (temporary, state) = open_state();
        let older = state
            .prompt_history_push("01SERVER".into(), "older prompt")
            .expect("save older");
        let newer = state
            .prompt_history_push("01SERVER".into(), "newer prompt")
            .expect("save newer");
        assert_eq!(
            state
                .prompt_history_list()
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            vec!["newer prompt", "older prompt"]
        );

        state
            .prompt_history_remove(&older.id)
            .expect("remove older");
        assert_eq!(state.prompt_history_list(), vec![newer.clone()]);
        assert!(state.prompt_history_remove("missing").is_err());

        state.prompt_history_clear().expect("clear");
        assert!(state.prompt_history_list().is_empty());

        let reopened = AppState::open(AppPaths::from_root(temporary.path())).expect("reopen");
        assert!(reopened.prompt_history_list().is_empty());
        assert_eq!(newer.text, "newer prompt");
    }

    #[test]
    fn mermaid_cache_round_trips_svg_per_theme() {
        let (_temporary, state) = open_state();
        let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        assert!(
            state
                .mermaid_cache_get(hash.into(), "paper-light".into())
                .expect("miss")
                .is_none()
        );
        state
            .mermaid_cache_put(hash.into(), "paper-light".into(), svg.into())
            .expect("put");
        assert_eq!(
            state
                .mermaid_cache_get(hash.into(), "paper-light".into())
                .expect("hit")
                .as_deref(),
            Some(svg)
        );
        assert!(
            state
                .save_user_file(PathBuf::from("diagram.png"), b"nope".to_vec())
                .is_err()
        );
    }

    #[test]
    fn save_user_file_writes_png_and_svg_atomically() {
        let (temporary, state) = open_state();
        let png = temporary.path().join("diagram.png");
        let svg = temporary.path().join("diagram.svg");
        state
            .save_user_file(png.clone(), b"\x89PNG".to_vec())
            .expect("png");
        state
            .save_user_file(svg.clone(), b"<svg></svg>".to_vec())
            .expect("svg");
        let pdf = temporary.path().join("note.pdf");
        state
            .save_user_file(pdf.clone(), b"%PDF-1.4".to_vec())
            .expect("pdf");
        assert_eq!(fs::read(&png).expect("read png"), b"\x89PNG");
        assert_eq!(fs::read(&svg).expect("read svg"), b"<svg></svg>");
        assert_eq!(fs::read(&pdf).expect("read pdf"), b"%PDF-1.4");
        assert!(
            state
                .save_user_file(temporary.path().join("diagram.txt"), b"nope".to_vec())
                .is_err()
        );
        assert!(
            state
                .save_user_file(temporary.path().join("missing/out.png"), b"x".to_vec())
                .is_err()
        );
    }

    #[test]
    fn config_set_validates_and_persists() {
        let (temporary, state) = open_state();
        let mut config = Config::default();
        config.typography.font_size = 18;
        state.config_set(config.clone()).expect("config saved");
        assert_eq!(state.config_get().typography.font_size, 18);

        let reopened = AppState::open(AppPaths::from_root(temporary.path())).expect("reopen");
        assert_eq!(reopened.config_get().typography.font_size, 18);

        config.typography.font_size = 9;
        assert!(state.config_set(config).is_err());
        assert_eq!(state.config_get().typography.font_size, 18);
    }

    #[test]
    fn project_commands_never_touch_the_folder() {
        let (temporary, state) = open_state();
        let project_root = temporary.path().join("notes");
        fs::create_dir(&project_root).expect("project directory");
        let document = project_root.join("keep.md");
        fs::write(&document, b"keep").expect("document");

        let added = state
            .projects_add("Notes".into(), project_root.clone())
            .expect("add");
        let listed = state.projects_list(ProjectsListQuery {
            limit: 10,
            ..ProjectsListQuery::default()
        });
        assert_eq!(listed.total, 1);
        assert_eq!(listed.items[0].id, added.id);

        let searched = state.projects_list(ProjectsListQuery {
            query: Some("nts".into()),
            limit: 10,
            offset: 0,
        });
        assert_eq!(searched.total, 1);

        state
            .projects_rename(added.id.clone(), "Writing".into())
            .expect("rename");
        state.projects_remove(added.id).expect("remove");

        assert_eq!(
            state
                .projects_list(ProjectsListQuery {
                    limit: 10,
                    ..ProjectsListQuery::default()
                })
                .total,
            0
        );
        assert_eq!(fs::read(&document).expect("preserved"), b"keep");
    }

    #[test]
    fn dropped_paths_register_a_project_and_open_markdown() {
        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir(&notes).expect("project directory");
        let document = notes.join("readme.md");
        fs::write(&document, b"# Hello\n").expect("document");

        let folder = state
            .open_dropped_paths(vec![notes.clone()])
            .expect("open folder");
        assert_eq!(folder.project.name, "notes");
        assert!(folder.open_rel_path.is_none());

        let file = state.open_dropped_paths(vec![document]).expect("open file");
        assert_eq!(file.project.id, folder.project.id);
        assert_eq!(file.open_rel_path, Some(PathBuf::from("readme.md")));
        assert_eq!(
            state
                .projects_list(ProjectsListQuery {
                    limit: 10,
                    ..ProjectsListQuery::default()
                })
                .total,
            1
        );

        let image = notes.join("cover.png");
        fs::write(&image, b"png").expect("image");
        assert!(state.open_dropped_paths(vec![image]).is_err());
    }

    #[test]
    fn tree_expansion_is_persisted_per_project_and_cleared_on_remove() {
        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir(&notes).expect("project directory");
        let project = state.projects_add("Notes".into(), notes).expect("add");

        state
            .tree_expanded_set(project.id.clone(), vec![PathBuf::from("chapters")])
            .expect("save expanded");
        assert_eq!(
            state.tree_expanded_get(project.id.clone()),
            vec!["chapters".to_owned()]
        );
        assert!(
            state
                .tree_expanded_set(project.id.clone(), vec![PathBuf::from("..")])
                .is_err()
        );

        let reopened = AppState::open(AppPaths::from_root(temporary.path())).expect("reopen");
        assert_eq!(
            reopened.tree_expanded_get(project.id.clone()),
            vec!["chapters".to_owned()]
        );
        reopened
            .projects_remove(project.id.clone())
            .expect("remove");
        assert!(reopened.tree_expanded_get(project.id).is_empty());
    }

    #[test]
    fn last_session_reopens_existing_files_and_notes_the_rest_once() {
        use ps_core::ui_state::{OpenSession, SessionTab};

        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir_all(notes.join("chapters")).expect("project directory");
        fs::write(notes.join("keep.md"), b"# Keep\n").expect("file");
        fs::write(notes.join("gone.md"), b"# Gone\n").expect("file");
        let project = state
            .projects_add("Notes".into(), notes.clone())
            .expect("add");
        state
            .tree_expanded_set(
                project.id.clone(),
                vec![PathBuf::from("chapters"), PathBuf::from("missing")],
            )
            .expect("expanded");
        state
            .session_set(OpenSession {
                project_id: project.id.clone(),
                tabs: vec![
                    SessionTab {
                        rel_path: PathBuf::from("keep.md"),
                        preview: false,
                        view_mode: "preview".into(),
                    },
                    SessionTab {
                        rel_path: PathBuf::from("gone.md"),
                        preview: true,
                        view_mode: "editor".into(),
                    },
                ],
                active_rel_path: Some(PathBuf::from("gone.md")),
                workspace_tabs: vec!["dashboard".into()],
                page: "dashboard".into(),
                view_mode: "editor".into(),
            })
            .expect("save session");

        fs::remove_file(notes.join("gone.md")).expect("delete file");
        fs::create_dir(notes.join("missing")).expect("folder exists for a moment");
        fs::remove_dir(notes.join("missing")).expect("delete folder");

        let restored = state.session_restore().expect("restore");
        assert_eq!(restored.project_id.as_deref(), Some(project.id.as_str()));
        assert_eq!(restored.tabs.len(), 1);
        assert_eq!(restored.tabs[0].rel_path, PathBuf::from("keep.md"));
        assert_eq!(restored.tabs[0].view_mode, "preview");
        assert_eq!(
            restored.active_rel_path.as_deref(),
            Some(Path::new("keep.md"))
        );
        assert_eq!(restored.page, "dashboard");
        assert_eq!(restored.view_mode, "editor");
        assert_eq!(
            restored.notices,
            vec![
                "Couldn't reopen \"gone.md\". That file is no longer in the project.".to_owned(),
                "Couldn't reopen \"missing\". That folder is no longer in the project.".to_owned(),
            ]
        );

        let again = state.session_restore().expect("restore again");
        assert!(again.notices.is_empty());
        assert_eq!(again.tabs.len(), 1);

        fs::remove_dir_all(&notes).expect("delete project");
        let missing = state.session_restore().expect("missing folder");
        assert!(missing.project_id.is_none());
        assert_eq!(
            missing.notices,
            vec!["Couldn't reopen \"Notes\". That folder is no longer on disk.".to_owned()]
        );
        let quiet = state.session_restore().expect("already cleared");
        assert!(quiet.notices.is_empty());
    }

    #[test]
    fn untitled_items_and_finder_paths_stay_inside_the_project() {
        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir(&notes).expect("project directory");
        let project = state.projects_add("Notes".into(), notes).expect("add");

        let file = state
            .fs_create_untitled(
                project.id.clone(),
                PathBuf::new(),
                ps_core::fsops::UntitledKind::File,
            )
            .expect("untitled file");
        assert_eq!(file.name, "untitled.md");
        let folder = state
            .fs_create_untitled(
                project.id.clone(),
                PathBuf::new(),
                ps_core::fsops::UntitledKind::Folder,
            )
            .expect("untitled folder");
        assert_eq!(folder.name, "untitled");

        assert!(
            state
                .fs_create_untitled(
                    project.id.clone(),
                    PathBuf::from(".."),
                    ps_core::fsops::UntitledKind::File,
                )
                .is_err()
        );
        assert!(
            state
                .reveal_in_finder(project.id.clone(), PathBuf::from(".."))
                .is_err()
        );
        assert!(
            state
                .open_external(project.id, PathBuf::from("../secret.md"))
                .is_err()
        );
    }

    #[test]
    fn files_search_and_assets_stay_inside_the_project() {
        let (temporary, state) = open_state();
        let notes = temporary.path().join("notes");
        fs::create_dir_all(notes.join("chapters")).expect("project directory");
        fs::write(notes.join("readme.md"), b"# Hi").expect("readme");
        fs::write(notes.join("chapters/intro.md"), b"# Intro").expect("intro");
        fs::write(notes.join("cover.png"), b"png").expect("image");
        let project = state
            .projects_add("Notes".into(), notes.clone())
            .expect("add");

        let found = state
            .files_search(project.id.clone(), "intro".into(), 20)
            .expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "intro.md");

        let text = state
            .content_search(project.id.clone(), PathBuf::new(), "Intro".into(), 20)
            .expect("content search");
        assert_eq!(text.hits.len(), 1);
        assert_eq!(text.hits[0].rel_path, Path::new("chapters/intro.md"));
        assert!(
            state
                .content_search(project.id.clone(), PathBuf::from(".."), "Intro".into(), 20)
                .is_err()
        );

        let asset = state
            .resolve_asset(&project.id, Path::new("cover.png"))
            .expect("asset");
        assert_eq!(
            asset,
            notes.join("cover.png").canonicalize().expect("canonical")
        );
        assert!(
            state
                .resolve_asset(&project.id, Path::new("../cover.png"))
                .is_err()
        );
        assert!(
            state
                .resolve_asset(&project.id, Path::new("chapters"))
                .is_err()
        );

        let outside = temporary.path().join("outside.png");
        fs::write(&outside, b"png").expect("outside image");
        let imported = state
            .fs_import(
                project.id.clone(),
                vec![outside.clone()],
                PathBuf::from("chapters"),
                ps_core::fsops::ConflictStrategy::KeepBoth,
            )
            .expect("import");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "outside.png");
        assert!(notes.join("chapters/outside.png").exists());
        assert!(
            state
                .fs_import(
                    project.id.clone(),
                    vec![temporary.path().join("../nope.txt")],
                    PathBuf::from(".."),
                    ps_core::fsops::ConflictStrategy::KeepBoth,
                )
                .is_err()
        );

        let conflicts = state
            .copy_conflicts(project.id, vec![PathBuf::from("readme.md")], PathBuf::new())
            .expect("conflicts");
        assert_eq!(conflicts, ["readme.md"]);
    }

    #[test]
    fn open_url_rejects_non_http_schemes() {
        assert!(AppState::open_url("javascript:alert(1)").is_err());
        assert!(AppState::open_url("file:///etc/passwd").is_err());
        assert!(AppState::open_url("https://example.com\n").is_err());
        assert!(AppState::open_url("http://user@host/").is_err());
    }

    #[test]
    fn filesystem_commands_stay_inside_the_project() {
        let (temporary, state) = open_state();
        let project_root = temporary.path().join("notes");
        fs::create_dir(&project_root).expect("project directory");
        let project = state
            .projects_add("Notes".into(), project_root.clone())
            .expect("add");

        let folder = state
            .fs_mkdir(project.id.clone(), PathBuf::from("inbox"))
            .expect("mkdir");
        assert_eq!(folder.name, "inbox");
        assert_eq!(folder.kind, ps_core::tree::TreeNodeKind::Directory);

        let created = state
            .fs_create_file(project.id.clone(), PathBuf::from("inbox/draft.md"))
            .expect("create");
        assert_eq!(created.name, "draft.md");
        assert_eq!(created.kind, ps_core::tree::TreeNodeKind::File);

        let renamed = state
            .fs_rename(
                project.id.clone(),
                PathBuf::from("inbox/draft.md"),
                PathBuf::from("inbox/note.md"),
            )
            .expect("rename");
        assert_eq!(renamed.rel_path, PathBuf::from("inbox/note.md"));

        let copied = state
            .fs_copy(
                project.id.clone(),
                vec![PathBuf::from("inbox/note.md")],
                PathBuf::new(),
                ConflictStrategy::KeepBoth,
            )
            .expect("copy");
        assert_eq!(copied.len(), 1);
        assert_eq!(copied[0].name, "note.md");
        assert_eq!(fs::read(project_root.join("note.md")).expect("copied"), b"");

        let moved = state
            .fs_move(
                project.id.clone(),
                vec![PathBuf::from("note.md")],
                PathBuf::from("inbox"),
                ConflictStrategy::KeepBoth,
            )
            .expect("move");
        assert_eq!(moved[0].name, "note 2.md");
        assert!(!project_root.join("note.md").exists());
        assert_eq!(
            fs::read(project_root.join("inbox/note 2.md")).expect("moved"),
            b""
        );

        let nodes = state
            .tree_read_dir(project.id.clone(), PathBuf::from("inbox"))
            .expect("tree");
        let names = nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"note.md"));
        assert!(names.contains(&"note 2.md"));

        fs::write(project_root.join(".hidden.md"), b"secret").expect("hidden");
        fs::create_dir(project_root.join(".docs")).expect("dot folder");
        let shown = state
            .tree_read_dir(project.id.clone(), PathBuf::new())
            .expect("complete tree");
        assert!(shown.iter().any(|node| node.name == ".hidden.md"));
        assert!(shown.iter().any(|node| node.name == ".docs"));

        let mut config = state.config_get();
        config.files.show_hidden = false;
        state.config_set(config).expect("hide hidden");
        let filtered = state
            .tree_read_dir(project.id.clone(), PathBuf::new())
            .expect("filtered tree");
        assert!(filtered.iter().all(|node| node.name != ".hidden.md"));
        assert!(filtered.iter().any(|node| node.name == ".docs"));

        assert!(
            state
                .fs_mkdir(project.id.clone(), PathBuf::from("../escape"))
                .is_err()
        );
        assert!(!temporary.path().join("escape").exists());
        assert!(
            state
                .tree_read_dir(project.id.clone(), PathBuf::from(".."))
                .is_err()
        );
        assert!(
            state
                .tree_read_dir("missing".into(), PathBuf::new())
                .is_err()
        );

        state
            .fs_trash_with(
                project.id.clone(),
                vec![PathBuf::from("inbox/note.md")],
                |root, paths| ps_core::fsops::permanently_delete(root, paths, |_| Ok(())),
            )
            .expect("trash");
        assert!(!project_root.join("inbox/note.md").exists());
        assert!(project_root.join("inbox/note 2.md").exists());
    }

    #[test]
    fn transfers_copy_and_move_files_between_projects() {
        let (temporary, state) = open_state();
        let alpha_root = temporary.path().join("alpha");
        let beta_root = temporary.path().join("beta");
        fs::create_dir(&alpha_root).expect("alpha");
        fs::create_dir(&beta_root).expect("beta");
        fs::write(alpha_root.join("note.md"), b"from alpha").expect("note");
        fs::write(alpha_root.join("keep.md"), b"stay").expect("keep");
        fs::write(beta_root.join("note.md"), b"already here").expect("conflict");
        let alpha = state
            .projects_add("Alpha".into(), alpha_root.clone())
            .expect("alpha project");
        let beta = state
            .projects_add("Beta".into(), beta_root.clone())
            .expect("beta project");

        let copied = state
            .fs_transfer(
                alpha.id.clone(),
                vec![PathBuf::from("keep.md")],
                beta.id.clone(),
                PathBuf::new(),
                true,
                ConflictStrategy::KeepBoth,
            )
            .expect("copy across");
        assert_eq!(copied[0].name, "keep.md");
        assert_eq!(
            fs::read(alpha_root.join("keep.md")).expect("source kept"),
            b"stay"
        );
        assert_eq!(
            fs::read(beta_root.join("keep.md")).expect("copied"),
            b"stay"
        );

        let skipped = state
            .fs_transfer(
                alpha.id.clone(),
                vec![PathBuf::from("note.md")],
                beta.id.clone(),
                PathBuf::new(),
                false,
                ConflictStrategy::Skip,
            )
            .expect("skip move");
        assert_eq!(skipped[0].name, "note.md");
        assert_eq!(
            fs::read(alpha_root.join("note.md")).expect("source not trashed"),
            b"from alpha"
        );
        assert_eq!(
            fs::read(beta_root.join("note.md")).expect("destination unchanged"),
            b"already here"
        );

        let moved = state
            .fs_transfer_with(
                alpha.id.clone(),
                vec![PathBuf::from("note.md")],
                beta.id.clone(),
                PathBuf::new(),
                false,
                ConflictStrategy::KeepBoth,
                |root, paths| ps_core::fsops::permanently_delete(root, paths, |_| Ok(())),
            )
            .expect("move across");
        assert_eq!(moved[0].name, "note 2.md");
        assert!(!alpha_root.join("note.md").exists());
        assert_eq!(
            fs::read(beta_root.join("note 2.md")).expect("moved"),
            b"from alpha"
        );
    }

    #[test]
    fn conversion_opens_markdown_content_and_deletes_only_its_verified_original() {
        let (temporary, state) = open_state();
        let root = temporary.path().join("notes");
        fs::create_dir(&root).unwrap();
        let project = state.projects_add("Notes".into(), root.clone()).unwrap();
        let source = PathBuf::from("Original.MARKDOWN");
        let destination = PathBuf::from("Original.MARKDOWN.age");
        fs::write(root.join(&source), "# Converted heading\n\nConverted body").unwrap();
        let loaded = ps_core::docio::read_doc(&root, &source).unwrap();
        state
            .doc_encrypt_existing(
                project.id.clone(),
                source.clone(),
                destination.clone(),
                loaded.source.text.clone(),
                loaded.hash,
                RestoreTraits::from_source(&loaded.source),
                "test-only passphrase".into(),
            )
            .unwrap();
        let opened = state
            .doc_open(project.id.clone(), destination.clone())
            .unwrap();
        assert!(
            opened
                .result
                .first_chunk
                .unwrap()
                .contains("<p>Converted body</p>")
        );
        assert!(opened.remaining_chunks.is_empty());
        assert!(root.join(&source).exists());
        state
            .doc_delete_original(project.id.clone(), destination.clone())
            .unwrap();
        assert!(!root.join(&source).exists());
        state.doc_lock_all().unwrap();
        state
            .doc_unlock(
                project.id.clone(),
                destination.clone(),
                "test-only passphrase".into(),
            )
            .unwrap();
        let reopened = state.doc_open(project.id, destination).unwrap();
        assert!(
            reopened
                .result
                .first_chunk
                .unwrap()
                .contains("<p>Converted body</p>")
        );
    }

    #[test]
    fn encrypted_notes_require_unlock_and_never_stream_or_index_plaintext() {
        let (temporary, state) = open_state();
        let root = temporary.path().join("notes");
        fs::create_dir(&root).unwrap();
        let project = state.projects_add("Notes".into(), root.clone()).unwrap();
        let rel = PathBuf::from("secret.md.age");
        state
            .doc_create_encrypted(
                project.id.clone(),
                rel.clone(),
                "# Confidential title\n\nSecret body".into(),
                "test-only passphrase".into(),
            )
            .unwrap();
        let opened = state.doc_open(project.id.clone(), rel.clone()).unwrap();
        assert_eq!(
            opened.result.meta.security,
            ps_core::encrypted::DocumentSecurity::Unlocked
        );
        assert!(!opened.result.meta.title.contains("Confidential"));
        assert!(opened.result.first_chunk.unwrap().contains("Secret body"));
        assert!(opened.remaining_chunks.is_empty());
        let found = ps_core::content_search::search_documents(
            &root,
            Path::new(""),
            "Secret body",
            true,
            100,
        )
        .unwrap();
        assert!(found.hits.is_empty());
        state.doc_lock_all().unwrap();
        let locked = state.doc_open(project.id.clone(), rel.clone()).unwrap();
        assert_eq!(
            locked.result.meta.security,
            ps_core::encrypted::DocumentSecurity::Locked
        );
        assert!(locked.result.first_chunk.is_none());
        assert!(locked.result.meta.toc.is_empty());
        assert!(state.doc_source(project.id.clone(), rel.clone()).is_err());
        state
            .doc_unlock(
                project.id.clone(),
                rel.clone(),
                "test-only passphrase".into(),
            )
            .unwrap();
        assert!(
            state
                .doc_source(project.id, rel)
                .unwrap()
                .text
                .contains("Secret body")
        );
    }

    #[test]
    fn doc_preview_renders_unsaved_task_lines() {
        let (temporary, state) = open_state();
        let project_root = temporary.path().join("notes");
        fs::create_dir(&project_root).expect("project directory");
        fs::write(project_root.join("readme.md"), "# Hello\n\nSaved.\n").expect("document");
        let project = state
            .projects_add("Notes".into(), project_root)
            .expect("add");
        let draft = "# Hello\n\n[ ] one\n[x] two\n";
        let preview = state
            .doc_preview(project.id, PathBuf::from("readme.md"), draft.into())
            .expect("preview");
        assert!(
            preview.html.contains("type=\"checkbox\""),
            "{}",
            preview.html
        );
        assert!(preview.html.contains("one"), "{}", preview.html);
        assert!(preview.html.contains("checked"), "{}", preview.html);
        assert!(preview.toc.iter().any(|entry| entry.title == "Hello"));
    }

    #[test]
    fn highlights_preserve_file_traits_and_never_overwrite_a_conflict() {
        let (temporary, state) = open_state();
        let root = temporary.path().join("notes");
        fs::create_dir(&root).expect("directory");
        let path = root.join("note.md");
        let original = b"\xef\xbb\xbfSaved.\r\n";
        fs::write(&path, original).expect("fixture");
        let project = state.projects_add("Notes".into(), root).expect("project");
        let rel = PathBuf::from("note.md");
        let source = state
            .doc_source(project.id.clone(), rel.clone())
            .expect("source");
        let opened = state
            .doc_open(project.id.clone(), rel.clone())
            .expect("open");
        let unchanged = state
            .doc_save(
                project.id.clone(),
                rel.clone(),
                source.text.clone(),
                opened.result.meta.hash.clone(),
                RestoreTraits::from_source(&source),
            )
            .expect("round trip");
        assert!(unchanged.skipped);
        assert_eq!(fs::read(&path).expect("bytes"), original);
        let draft = "Saved. Unsaved.\n";
        let next = ps_render::apply_highlight(
            draft,
            &[ps_core::edit::HighlightRange {
                start: 0,
                end: 15,
                from: 7,
                to: 14,
            }],
            Some("#fc0"),
        )
        .expect("marker");
        let preview = state
            .doc_preview(project.id.clone(), rel.clone(), next.clone())
            .expect("preview");
        assert!(preview.html.contains("background-color:#ffcc00"));
        assert_eq!(fs::read(&path).expect("no implicit write"), original);
        let saved = state
            .doc_save(
                project.id.clone(),
                rel.clone(),
                next.clone(),
                opened.result.meta.hash,
                RestoreTraits::from_source(&source),
            )
            .expect("save");
        assert_eq!(
            fs::read(&path).expect("saved bytes"),
            "\u{feff}Saved. =={#ffcc00}Unsaved==.\r\n".as_bytes()
        );
        fs::write(&path, b"Changed elsewhere").expect("external change");
        assert!(
            state
                .doc_save(
                    project.id,
                    rel,
                    next,
                    saved.hash,
                    RestoreTraits::from_source(&source)
                )
                .is_err()
        );
        assert_eq!(
            fs::read(&path).expect("external bytes"),
            b"Changed elsewhere"
        );
    }

    #[test]
    fn document_commands_read_source_and_return_the_first_chunk() {
        let (temporary, state) = open_state();
        let project_root = temporary.path().join("notes");
        fs::create_dir(&project_root).expect("project directory");
        fs::write(project_root.join("readme.md"), "# Hello\n\nParagraph.\n").expect("document");
        let project = state
            .projects_add("Notes".into(), project_root.clone())
            .expect("add");

        let source = state
            .doc_source(project.id.clone(), PathBuf::from("readme.md"))
            .expect("source");
        assert_eq!(source.text, "# Hello\n\nParagraph.\n");
        assert!(source.writable);

        let stat = state
            .doc_stat(project.id.clone(), PathBuf::from("readme.md"))
            .expect("stat");
        assert_eq!(
            stat.size,
            u64::try_from(b"# Hello\n\nParagraph.\n".len()).unwrap()
        );
        assert_eq!(stat.hash.len(), 64);
        assert!(source.trailing_newline);

        let opened = state
            .doc_open(project.id.clone(), PathBuf::from("readme.md"))
            .expect("open");
        assert_eq!(opened.result.meta.title, "Hello");
        assert_eq!(opened.result.meta.chunk_count, 1);
        assert!(opened.remaining_chunks.is_empty());
        let first = opened.result.first_chunk.expect("first chunk");
        assert!(first.contains("<h1"));
        assert!(first.contains("Hello"));
        assert!(first.starts_with("<section class=\"chunk\">"));

        let saved = state
            .doc_save(
                project.id.clone(),
                PathBuf::from("readme.md"),
                source.text.clone(),
                opened.result.meta.hash.clone(),
                RestoreTraits::from_source(&source),
            )
            .expect("save");
        assert!(saved.skipped);

        let first_block = "a".repeat(40 * 1024);
        let second_block = "b".repeat(30 * 1024);
        let third_block = "c".repeat(1024);
        fs::write(
            project_root.join("wide.md"),
            format!("{first_block}\n\n{second_block}\n\n{third_block}\n"),
        )
        .expect("wide document");
        let wide = state
            .doc_open(project.id.clone(), PathBuf::from("wide.md"))
            .expect("wide open");
        assert_eq!(wide.result.meta.chunk_count, 2);
        assert_eq!(wide.remaining_chunks.len(), 1);
        assert!(
            wide.result
                .first_chunk
                .as_deref()
                .is_some_and(|chunk| chunk.contains(&first_block))
        );
        assert!(wide.remaining_chunks[0].contains(&third_block));

        fs::write(project_root.join("config.json"), b"{\"ok\": true}\n").expect("json");
        let json = state
            .doc_open(project.id.clone(), PathBuf::from("config.json"))
            .expect("json open");
        assert!(!json.result.meta.source_only);
        let json_html = json.result.first_chunk.expect("json chunk");
        assert!(json_html.contains("<pre class=\"code\">"));
        assert!(json_html.contains("syntax-"));
        assert!(!json_html.contains("<p>"));

        fs::write(
            project_root.join("lib.rs"),
            b"fn main() {\n    println!(\"hi\");\n}\n",
        )
        .expect("rust");
        let rust = state
            .doc_open(project.id.clone(), PathBuf::from("lib.rs"))
            .expect("rust open");
        let rust_html = rust.result.first_chunk.expect("rust chunk");
        assert!(rust_html.contains("syntax-source syntax-rust"));

        fs::write(project_root.join("notes.txt"), b"hello <plain>\n").expect("txt");
        let txt = state
            .doc_open(project.id.clone(), PathBuf::from("notes.txt"))
            .expect("txt open");
        let txt_html = txt.result.first_chunk.expect("txt chunk");
        assert!(txt_html.contains("<pre class=\"code\">"));
        assert!(txt_html.contains("&lt;plain&gt;"));
        assert!(!txt_html.contains("<p>"));

        fs::write(project_root.join("hi.py"), b"def hi():\n    return 1\n").expect("python");
        let python = state
            .doc_open(project.id.clone(), PathBuf::from("hi.py"))
            .expect("python open");
        let python_html = python.result.first_chunk.expect("python chunk");
        assert!(python_html.contains("syntax-source syntax-python"));

        fs::write(project_root.join("binary.md"), [0xff, 0xfe]).expect("binary");
        let binary = state
            .doc_open(project.id.clone(), PathBuf::from("binary.md"))
            .expect("binary open");
        assert!(binary.result.meta.source_only);
        assert!(binary.result.first_chunk.is_none());
        assert_eq!(binary.result.meta.chunk_count, 0);

        fs::write(project_root.join("cover.png"), b"\x89PNG\r\n\x1a\n").expect("png");
        let image = state
            .doc_open(project.id.clone(), PathBuf::from("cover.png"))
            .expect("png open");
        assert!(!image.result.meta.source_only);
        assert!(!image.result.meta.writable);
        assert!(image.result.meta.readonly_reason.is_none());
        let image_html = image.result.first_chunk.expect("png chunk");
        assert!(image_html.contains(&format!(
            "src=\"asset://localhost/{}/cover.png\"",
            project.id
        )));
        assert!(image_html.contains("class=\"image-file\""));
        assert!(!image_html.contains("PNG"));

        fs::create_dir(project_root.join("shots")).expect("shots");
        fs::write(project_root.join("shots/my photo.jpeg"), b"\xFF\xD8\xFF").expect("jpeg");
        let jpeg = state
            .doc_open(project.id.clone(), PathBuf::from("shots/my photo.jpeg"))
            .expect("jpeg open");
        let jpeg_html = jpeg.result.first_chunk.expect("jpeg chunk");
        assert!(jpeg_html.contains(&format!(
            "src=\"asset://localhost/{}/shots/my%20photo.jpeg\"",
            project.id
        )));

        fs::write(
            project_root.join("icon.svg"),
            b"<svg><script>alert(1)</script></svg>",
        )
        .expect("svg");
        let svg = state
            .doc_open(project.id.clone(), PathBuf::from("icon.svg"))
            .expect("svg open");
        assert!(!svg.result.meta.writable);
        let svg_html = svg.result.first_chunk.expect("svg chunk");
        assert!(svg_html.contains("<img "));
        assert!(!svg_html.contains("<script"));
        assert!(!svg_html.contains("alert"));

        assert!(
            state
                .doc_open(project.id.clone(), PathBuf::from(".."))
                .is_err()
        );
        assert!(
            state
                .doc_source(project.id, PathBuf::from("../secret.md"))
                .is_err()
        );
    }

    #[test]
    fn document_open_respects_allow_raw_html() {
        let (temporary, state) = open_state();
        let project_root = temporary.path().join("notes");
        fs::create_dir(&project_root).expect("project directory");
        fs::write(
            project_root.join("raw.md"),
            "<script>nope</script>\n<mark data-reader=\"yes\">raw</mark>\n",
        )
        .expect("document");
        let project = state
            .projects_add("Notes".into(), project_root)
            .expect("add");

        let sanitized = state
            .doc_open(project.id.clone(), PathBuf::from("raw.md"))
            .expect("sanitized");
        let html = sanitized.result.first_chunk.expect("chunk");
        assert!(!html.to_ascii_lowercase().contains("<script"));

        let mut config = state.config_get();
        config.viewer.allow_raw_html = true;
        state.config_set(config).expect("allow raw html");
        let raw = state
            .doc_open(project.id, PathBuf::from("raw.md"))
            .expect("raw");
        assert!(
            raw.result
                .first_chunk
                .as_deref()
                .is_some_and(|chunk| chunk.contains("<mark data-reader=\"yes\">raw</mark>"))
        );
    }
}
