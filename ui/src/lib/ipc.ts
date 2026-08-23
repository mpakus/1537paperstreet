import type {
  Config,
  ConflictStrategy,
  DocOpenResult,
  DocumentMeta,
  DocumentSource,
  DocumentStat,
  RestoreTraits,
  OpenDropResult,
  Project,
  ProjectsListQuery,
  ProjectsListResult,
  TocEntry,
  TreeNode,
  UntitledKind,
  ViewMode,
  FsChangedEvent,
  WatchUpdate,
  ThemeInfo,
  ThemeAppearance,
  WrittenDocument,
  AgentChoice,
  AgentClientEvent,
  AgentPermission,
  AgentPreset,
  AgentPresetInfo,
  AgentServer,
  PromptHistoryEntry,
  UpdateCheck,
} from './generated/core'

type InvokeFn = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>

async function tauriInvoke(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke(cmd, args)
}

let invokeImpl: InvokeFn = tauriInvoke

/** Replaces the Tauri invoke implementation. Tests pass `null` to restore it. */
export function setInvokeForTests(impl: InvokeFn | null): void {
  invokeImpl = impl ?? tauriInvoke
}

async function invokeIpc<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return (await invokeImpl(cmd, args)) as T
}

/** Turns an IPC failure into a user-visible message. */
export function errorMessage(cause: unknown): string {
  if (typeof cause === 'string') {
    return cause
  }
  if (cause instanceof Error) {
    return cause.message
  }
  return String(cause)
}

/** Reads the persisted application configuration. */
export function configGet(): Promise<Config> {
  return invokeIpc('config_get')
}

/** Replaces the persisted application configuration. */
export function configSet(config: Config): Promise<void> {
  return invokeIpc('config_set', { config })
}

/** Returns a page of registered projects. */
export function projectsList(
  query: ProjectsListQuery,
): Promise<ProjectsListResult> {
  return invokeIpc('projects_list', { query })
}

/** Renames a project list entry without touching files on disk. */
export function projectsRename(id: string, name: string): Promise<void> {
  return invokeIpc('projects_rename', { id, name })
}

/** Removes a project list entry without touching files on disk. */
export function projectsRemove(id: string): Promise<void> {
  return invokeIpc('projects_remove', { id })
}

/** Points a project record at a different folder. */
export function projectsRelocate(id: string, path: string): Promise<Project> {
  return invokeIpc('projects_relocate', { id, path })
}

/** Built-in and user themes. */
export function themesList(): Promise<ThemeInfo[]> {
  return invokeIpc('themes_list')
}

/** CSS variable blocks for every loaded theme. */
export function themesCss(): Promise<string> {
  return invokeIpc('themes_css')
}

/** Cached Mermaid SVG for a source hash and theme. */
export function mermaidCacheGet(
  sourceHash: string,
  themeId: string,
): Promise<string | null> {
  return invokeIpc('mermaid_cache_get', {
    source_hash: sourceHash,
    theme_id: themeId,
  })
}

/** Stores a rendered Mermaid SVG. */
export function mermaidCachePut(
  sourceHash: string,
  themeId: string,
  svg: string,
): Promise<void> {
  return invokeIpc('mermaid_cache_put', {
    source_hash: sourceHash,
    theme_id: themeId,
    svg,
  })
}

/** Writes bytes to a path chosen in a native Save dialog. */
export function saveUserFile(path: string, bytes: number[]): Promise<void> {
  return invokeIpc('save_user_file', { path, bytes })
}

/** Writes a PDF of the current document to a path from the Save dialog. */
export function exportPdf(path: string, html: string): Promise<void> {
  return invokeIpc('export_pdf', { path, html })
}

/** Native folder picker. */
export async function pickFolder(): Promise<string | null> {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selected = await open({ directory: true, multiple: false })
  if (selected == null) {
    return null
  }
  const path = Array.isArray(selected) ? selected[0] : selected
  return path ?? null
}

/** Reads one directory level inside a project. */
export function treeReadDir(
  projectId: string,
  relPath: string,
): Promise<TreeNode[]> {
  return invokeIpc('tree_read_dir', {
    project_id: projectId,
    rel_path: relPath,
  })
}

/** Returns directories that were expanded the last time this project was shown. */
export function treeExpandedGet(projectId: string): Promise<string[]> {
  return invokeIpc('tree_expanded_get', { project_id: projectId })
}

/** Persists the expanded directories for a project. */
export function treeExpandedSet(
  projectId: string,
  relPaths: string[],
): Promise<void> {
  return invokeIpc('tree_expanded_set', {
    project_id: projectId,
    rel_paths: relPaths,
  })
}

/** Opens a document and returns the first HTML chunk. */
export function docOpen(
  projectId: string,
  relPath: string,
): Promise<DocOpenResult> {
  return invokeIpc('doc_open', { project_id: projectId, rel_path: relPath })
}

/** Reads a document's source text and on-disk traits. */
export function docSource(
  projectId: string,
  relPath: string,
): Promise<DocumentSource> {
  return invokeIpc('doc_source', { project_id: projectId, rel_path: relPath })
}

/** Hashes a document on disk without loading its text. */
export function docStat(
  projectId: string,
  relPath: string,
): Promise<DocumentStat> {
  return invokeIpc('doc_stat', { project_id: projectId, rel_path: relPath })
}

/** Writes a document, restoring BOM, EOL, and a trailing newline. */
export function docSave(
  projectId: string,
  relPath: string,
  text: string,
  baseHash: string,
  traits: RestoreTraits,
): Promise<WrittenDocument> {
  return invokeIpc('doc_save', {
    project_id: projectId,
    rel_path: relPath,
    text,
    base_hash: baseHash,
    traits,
  })
}

/** Registers dropped folders or opens a dropped Markdown file. */
export function openDroppedPaths(paths: string[]): Promise<OpenDropResult> {
  return invokeIpc('open_dropped_paths', { paths })
}

/** Shows a native picker, then opens the chosen Markdown file or folder. */
export async function pickAndOpen(
  kind: 'file' | 'folder',
): Promise<OpenDropResult | null> {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selected = await open({
    directory: kind === 'folder',
    multiple: false,
    filters:
      kind === 'file'
        ? [
            {
              name: 'Markdown',
              extensions: ['md', 'markdown', 'mdown', 'mdwn'],
            },
          ]
        : undefined,
  })
  if (selected == null) {
    return null
  }
  const path = Array.isArray(selected) ? selected[0] : selected
  if (!path) {
    return null
  }
  return openDroppedPaths([path])
}

/** Creates `untitled.md` or an `untitled` folder, using the next free name. */
export function fsCreateUntitled(
  projectId: string,
  parentRel: string,
  kind: UntitledKind,
): Promise<TreeNode> {
  return invokeIpc('fs_create_untitled', {
    project_id: projectId,
    parent_rel: parentRel,
    kind,
  })
}

/** Renames one project item. */
export function fsRename(
  projectId: string,
  from: string,
  to: string,
): Promise<TreeNode> {
  return invokeIpc('fs_rename', { project_id: projectId, from, to })
}

/** Copies project items into a directory. */
export function fsCopy(
  projectId: string,
  from: string[],
  toDir: string,
  conflict: ConflictStrategy,
): Promise<TreeNode[]> {
  return invokeIpc('fs_copy', {
    project_id: projectId,
    from,
    to_dir: toDir,
    conflict,
  })
}

/** Moves project items into a directory. */
export function fsMove(
  projectId: string,
  from: string[],
  toDir: string,
  conflict: ConflictStrategy,
): Promise<TreeNode[]> {
  return invokeIpc('fs_move', {
    project_id: projectId,
    from,
    to_dir: toDir,
    conflict,
  })
}

/** Copies or moves items from one project into another. */
export function fsTransfer(
  fromProjectId: string,
  from: string[],
  toProjectId: string,
  toDir: string,
  copy: boolean,
  conflict: ConflictStrategy,
): Promise<TreeNode[]> {
  return invokeIpc('fs_transfer', {
    from_project_id: fromProjectId,
    from,
    to_project_id: toProjectId,
    to_dir: toDir,
    copy,
    conflict,
  })
}

/** Names that already exist at the copy/move destination. */
export function copyConflicts(
  projectId: string,
  from: string[],
  toDir: string,
): Promise<string[]> {
  return invokeIpc('copy_conflicts', {
    project_id: projectId,
    from,
    to_dir: toDir,
  })
}

/** Copies files from absolute Finder paths into a project folder. */
export function fsImport(
  projectId: string,
  sources: string[],
  toDir: string,
  conflict: ConflictStrategy,
): Promise<TreeNode[]> {
  return invokeIpc('fs_import', {
    project_id: projectId,
    sources,
    to_dir: toDir,
    conflict,
  })
}

/** Fuzzy-searches Markdown files in a project. */
export function filesSearch(
  projectId: string,
  query: string,
  limit = 40,
): Promise<TreeNode[]> {
  return invokeIpc('files_search', { project_id: projectId, query, limit })
}

/** Opens an http(s) URL in the system browser. */
export function openUrl(url: string): Promise<void> {
  return invokeIpc('open_url', { url })
}

/** Compares the running app to the latest GitHub Release. */
export function updatesCheck(): Promise<UpdateCheck> {
  return invokeIpc('updates_check')
}

/** Starts watching the active project for tree updates. */
export function watchStart(projectId: string): Promise<void> {
  return invokeIpc('watch_start', { project_id: projectId })
}

/** Stops the active project watcher. */
export function watchStop(): Promise<void> {
  return invokeIpc('watch_stop')
}
/** Moves selected items to Trash. */
export function fsTrash(projectId: string, relPaths: string[]): Promise<void> {
  return invokeIpc('fs_trash', { project_id: projectId, rel_paths: relPaths })
}

/** Reveals a project path in Finder. */
export function revealInFinder(
  projectId: string,
  relPath: string,
): Promise<void> {
  return invokeIpc('reveal_in_finder', {
    project_id: projectId,
    rel_path: relPath,
  })
}

/** Opens a project file in the default external application. */
export function openExternal(
  projectId: string,
  relPath: string,
): Promise<void> {
  return invokeIpc('open_external', {
    project_id: projectId,
    rel_path: relPath,
  })
}

/** Starts a native window drag from the overlay titlebar. */
export async function startWindowDrag(): Promise<void> {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().startDragging()
}

/** Sets the native window title (Mission Control, Cmd-Tab). */
export async function setWindowTitle(title: string): Promise<void> {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().setTitle(title)
}

/** Reads the bundled app version from Tauri. */
export async function getAppVersion(): Promise<string> {
  const { getVersion } = await import('@tauri-apps/api/app')
  return getVersion()
}

/** PATH probes for OpenCode, Claude, and Codex. Never downloads. */
export function agentPresets(): Promise<AgentPresetInfo[]> {
  return invokeIpc('agent_presets')
}

/** Builds a server record with a ULID. */
export function agentMakeServer(
  preset: AgentPreset,
  name: string,
  command: string,
  args: string[],
): Promise<AgentServer> {
  return invokeIpc('agent_make_server', { preset, name, command, args })
}

/** Stored user prompts, newest first. */
export function agentPromptHistory(): Promise<PromptHistoryEntry[]> {
  return invokeIpc('agent_prompt_history')
}

/** Starts an ACP session for the open project. */
export function agentStart(
  serverId: string,
  projectId: string,
  permission: AgentPermission,
): Promise<AgentClientEvent> {
  return invokeIpc('agent_start', {
    server_id: serverId,
    project_id: projectId,
    permission,
  })
}

/** Stops the live ACP subprocess. */
export function agentStop(): Promise<void> {
  return invokeIpc('agent_stop')
}

/** Sends a prompt and stores it in local history. */
export function agentPrompt(serverId: string, text: string): Promise<void> {
  return invokeIpc('agent_prompt', { server_id: serverId, text })
}

/** Cancels the current prompt turn. */
export function agentCancel(): Promise<void> {
  return invokeIpc('agent_cancel')
}

/** Asks the agent to switch model when it advertises models. */
export function agentSetModel(modelId: string): Promise<void> {
  return invokeIpc('agent_set_model', { model_id: modelId })
}

/** Answers a tool permission request. */
export function agentPermissionReply(
  id: number,
  optionId: string,
): Promise<void> {
  return invokeIpc('agent_permission_reply', { id, option_id: optionId })
}

export type {
  AgentChoice,
  AgentClientEvent,
  AgentPermission,
  AgentPreset,
  AgentPresetInfo,
  AgentServer,
  PromptHistoryEntry,
  Config,
  ConflictStrategy,
  DocOpenResult,
  DocumentMeta,
  DocumentSource,
  DocumentStat,
  RestoreTraits,
  OpenDropResult,
  Project,
  ProjectsListQuery,
  ProjectsListResult,
  TocEntry,
  TreeNode,
  UntitledKind,
  ViewMode,
  FsChangedEvent,
  WatchUpdate,
  ThemeInfo,
  ThemeAppearance,
  WrittenDocument,
  UpdateCheck,
}
