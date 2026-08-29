//! Thin Tauri command wrappers around `ps-core`.
//!
//! Tauri 2 maps command argument keys to camelCase unless told otherwise.
//! PLAN § 3.3 and `ui/src/lib/ipc.ts` use snake_case (`project_id`, `rel_path`),
//! so every command sets `rename_all = "snake_case"`.

use std::path::PathBuf;

use ps_core::config::Config;
use ps_core::docio::{
    DocChunkEvent, DocDoneEvent, DocOpenResult, DocumentSource, DocumentStat, RestoreTraits,
    WrittenDocument,
};
use ps_core::fsops::{ConflictStrategy, UntitledKind};
use ps_core::projects::{OpenDropResult, Project, ProjectsListQuery, ProjectsListResult};
use ps_core::themes::ThemeInfo;
use ps_core::tree::TreeNode;
use ps_core::updates::UpdateCheck;
use tauri::{Emitter, State};

use crate::fs_watch::{self, WatchHub};
use crate::state::AppState;

fn to_command_error(error: ps_core::Error) -> String {
    error.to_string()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn config_get(state: State<'_, AppState>) -> Config {
    state.config_get()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn config_set(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.config_set(config))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))?;
    crate::tray::apply_dock_visibility(&app);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn projects_list(
    state: State<'_, AppState>,
    query: ProjectsListQuery,
) -> ProjectsListResult {
    state.projects_list(query)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn projects_add(
    state: State<'_, AppState>,
    name: String,
    path: PathBuf,
) -> Result<Project, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.projects_add(name, path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn open_dropped_paths(
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<OpenDropResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.open_dropped_paths(paths))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn projects_rename(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.projects_rename(id, name))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn projects_remove(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.projects_remove(id))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn projects_relocate(
    state: State<'_, AppState>,
    id: String,
    path: PathBuf,
) -> Result<Project, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.projects_relocate(id, path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn themes_list(state: State<'_, AppState>) -> Vec<ThemeInfo> {
    state.themes_list()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn themes_css(state: State<'_, AppState>) -> String {
    state.themes_css()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn mermaid_cache_get(
    state: State<'_, AppState>,
    source_hash: String,
    theme_id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.mermaid_cache_get(source_hash, theme_id))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn mermaid_cache_put(
    state: State<'_, AppState>,
    source_hash: String,
    theme_id: String,
    svg: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.mermaid_cache_put(source_hash, theme_id, svg)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn save_user_file(
    state: State<'_, AppState>,
    path: PathBuf,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.save_user_file(path, bytes))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn tree_read_dir(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.tree_read_dir(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn tree_expanded_get(state: State<'_, AppState>, project_id: String) -> Vec<String> {
    state.tree_expanded_get(project_id)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn tree_expanded_set(
    state: State<'_, AppState>,
    hub: State<'_, WatchHub>,
    project_id: String,
    rel_paths: Vec<PathBuf>,
) -> Result<(), String> {
    let hub = hub.inner().clone();
    let expanded = rel_paths.clone();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.tree_expanded_set(project_id, rel_paths))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))?;
    hub.set_expanded(expanded)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_mkdir(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<TreeNode, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_mkdir(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_create_file(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<TreeNode, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_create_file(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_create_untitled(
    state: State<'_, AppState>,
    project_id: String,
    parent_rel: PathBuf,
    kind: UntitledKind,
) -> Result<TreeNode, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.fs_create_untitled(project_id, parent_rel, kind)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn reveal_in_finder(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.reveal_in_finder(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn open_external(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.open_external(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_rename(
    state: State<'_, AppState>,
    project_id: String,
    from: PathBuf,
    to: PathBuf,
) -> Result<TreeNode, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_rename(project_id, from, to))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_copy(
    state: State<'_, AppState>,
    project_id: String,
    from: Vec<PathBuf>,
    to_dir: PathBuf,
    conflict: ConflictStrategy,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_copy(project_id, from, to_dir, conflict))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_transfer(
    state: State<'_, AppState>,
    from_project_id: String,
    from: Vec<PathBuf>,
    to_project_id: String,
    to_dir: PathBuf,
    copy: bool,
    conflict: ConflictStrategy,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.fs_transfer(from_project_id, from, to_project_id, to_dir, copy, conflict)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_import(
    state: State<'_, AppState>,
    project_id: String,
    sources: Vec<PathBuf>,
    to_dir: PathBuf,
    conflict: ConflictStrategy,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.fs_import(project_id, sources, to_dir, conflict)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_move(
    state: State<'_, AppState>,
    project_id: String,
    from: Vec<PathBuf>,
    to_dir: PathBuf,
    conflict: ConflictStrategy,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_move(project_id, from, to_dir, conflict))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn fs_trash(
    state: State<'_, AppState>,
    project_id: String,
    rel_paths: Vec<PathBuf>,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.fs_trash(project_id, rel_paths))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn doc_source(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<DocumentSource, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.doc_source(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn doc_stat(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<DocumentStat, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.doc_stat(project_id, rel_path))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn doc_save(
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
    text: String,
    base_hash: String,
    traits: RestoreTraits,
) -> Result<WrittenDocument, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.doc_save(project_id, rel_path, text, base_hash, traits)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn doc_open(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    rel_path: PathBuf,
) -> Result<DocOpenResult, String> {
    let state = state.inner().clone();
    let prepared =
        tauri::async_runtime::spawn_blocking(move || state.doc_open(project_id, rel_path))
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result.map_err(to_command_error))?;

    let result = prepared.result;
    let remaining_chunks = prepared.remaining_chunks;
    let project_id = result.meta.project_id.clone();
    let rel_path = result.meta.rel_path.clone();
    let chunk_count = result.meta.chunk_count;
    tauri::async_runtime::spawn(async move {
        for (offset, html) in remaining_chunks.into_iter().enumerate() {
            let index = u32::try_from(offset.saturating_add(1)).unwrap_or(u32::MAX);
            let _ = app.emit(
                "doc://chunk",
                DocChunkEvent {
                    project_id: project_id.clone(),
                    rel_path: rel_path.clone(),
                    index,
                    html,
                },
            );
        }
        let _ = app.emit(
            "doc://done",
            DocDoneEvent {
                project_id,
                rel_path,
                chunk_count,
            },
        );
    });
    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn files_search(
    state: State<'_, AppState>,
    project_id: String,
    query: String,
    limit: u32,
) -> Result<Vec<TreeNode>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.files_search(project_id, query, limit))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn copy_conflicts(
    state: State<'_, AppState>,
    project_id: String,
    from: Vec<PathBuf>,
    to_dir: PathBuf,
) -> Result<Vec<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.copy_conflicts(project_id, from, to_dir))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn open_url(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || AppState::open_url(&url))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn updates_check() -> Result<UpdateCheck, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let body = crate::updates::fetch_latest_release_json()?;
        ps_core::updates::from_github_json(env!("CARGO_PKG_VERSION"), &body)
            .map_err(to_command_error)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn watch_start(
    state: State<'_, AppState>,
    hub: State<'_, WatchHub>,
    project_id: String,
) -> Result<(), String> {
    let hub = hub.inner().clone();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        fs_watch::start_for_project(&hub, &state, project_id)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn watch_set_expanded(
    hub: State<'_, WatchHub>,
    rel_paths: Vec<PathBuf>,
) -> Result<(), String> {
    hub.set_expanded(rel_paths)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn watch_stop(hub: State<'_, WatchHub>) -> Result<(), String> {
    hub.stop()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn export_pdf(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    html: String,
    path: PathBuf,
) -> Result<(), String> {
    if html.len() > 32 * 1024 * 1024 {
        return Err("This document is too large to export as PDF.".into());
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(crate::pdf::html_to_pdf(&html));
    })
    .map_err(|error| error.to_string())?;
    let bytes = rx.recv().map_err(|error| error.to_string())??;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.save_user_file(path, bytes))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_presets() -> Vec<ps_core::agents::AgentPresetInfo> {
    AppState::agent_presets()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_make_server(
    preset: ps_core::agents::AgentPreset,
    name: String,
    command: String,
    args: Vec<String>,
) -> ps_core::agents::AgentServer {
    match preset {
        ps_core::agents::AgentPreset::Custom => {
            ps_core::agents::AgentServer::custom(name, command, args)
        }
        named => ps_core::agents::AgentServer::from_preset(named),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_prompt_history(
    state: State<'_, AppState>,
) -> Vec<ps_core::agents::PromptHistoryEntry> {
    state.prompt_history_list()
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_prompt_history_remove(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.prompt_history_remove(&id).map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_prompt_history_clear(state: State<'_, AppState>) -> Result<(), String> {
    state.prompt_history_clear().map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn agent_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    hub: State<'_, crate::agent::AgentHub>,
    server_id: String,
    project_id: String,
    permission: ps_core::agents::AgentPermission,
) -> Result<ps_core::agents::AgentClientEvent, String> {
    let state = state.inner().clone();
    let server = state.agent_server(&server_id).map_err(to_command_error)?;
    let cwd = state.project_root(&project_id).map_err(to_command_error)?;
    let hub = hub.inner().clone();
    tauri::async_runtime::spawn_blocking(move || hub.start(app, &server, &cwd, permission))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(to_command_error))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_stop(hub: State<'_, crate::agent::AgentHub>) {
    hub.stop();
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_prompt(
    state: State<'_, AppState>,
    hub: State<'_, crate::agent::AgentHub>,
    server_id: String,
    text: String,
) -> Result<(), String> {
    state
        .prompt_history_push(server_id, &text)
        .map_err(to_command_error)?;
    hub.prompt(&text).map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_cancel(hub: State<'_, crate::agent::AgentHub>) -> Result<(), String> {
    hub.cancel().map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_set_model(
    hub: State<'_, crate::agent::AgentHub>,
    model_id: String,
) -> Result<(), String> {
    hub.set_model(model_id).map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) fn agent_permission_reply(
    hub: State<'_, crate::agent::AgentHub>,
    id: u64,
    option_id: String,
) -> Result<(), String> {
    hub.permission_reply(id, option_id)
        .map_err(to_command_error)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn dashboard_get(
    state: State<'_, AppState>,
    hub: State<'_, crate::agent::AgentHub>,
    project_id: Option<String>,
) -> Result<ps_core::dashboard::DashboardSnapshot, String> {
    let state = state.inner().clone();
    let session = hub.session_stats();
    tauri::async_runtime::spawn_blocking(move || state.dashboard(project_id.as_deref(), session))
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn ipc_commands_use_snake_case_argument_names() {
        let source = include_str!("commands.rs");
        let mut command_attrs = 0usize;
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("#[tauri::command") {
                continue;
            }
            command_attrs += 1;
            assert!(
                trimmed.contains("rename_all = \"snake_case\""),
                "command attribute on line {} must use snake_case args so they match PLAN § 3.3",
                index + 1
            );
        }
        assert!(
            command_attrs > 0,
            "expected at least one #[tauri::command] attribute"
        );
    }
}
