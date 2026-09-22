import type { OpenSession, ViewMode } from './generated/core'
import type { WorkspacePage, WorkspaceTab } from './tabs'

/** `preview`, `editor`, or `split` stored for one tab. Anything else is unset. */
export function savedViewMode(value: string): ViewMode | undefined {
  if (value === 'preview' || value === 'editor' || value === 'split') {
    return value
  }
  return undefined
}

/**
 * Mode to show for a document. An explicit request wins, then the mode that
 * tab already remembered, then the mode on screen.
 */
export function modeForTab(
  cached: ViewMode | undefined,
  requested: ViewMode | undefined,
  current: ViewMode,
): ViewMode {
  return requested ?? cached ?? current
}

/** Snapshot of the open project, document tabs, and workspace tabs. */
export function openSession(input: {
  projectId: string
  tabs: { relPath: string; preview: boolean; viewMode: ViewMode }[]
  activeRelPath: string | null
  workspaceTabs: readonly WorkspaceTab[]
  page: WorkspacePage
  viewMode: ViewMode
}): OpenSession {
  const tabs = input.tabs
    .filter((tab) => tab.relPath.length > 0)
    .map((tab) => ({
      rel_path: tab.relPath,
      preview: tab.preview,
      view_mode: tab.viewMode,
    }))
  const active = input.activeRelPath
  if (active && !tabs.some((tab) => tab.rel_path === active)) {
    tabs.push({ rel_path: active, preview: false, view_mode: input.viewMode })
  }
  return {
    project_id: input.projectId,
    tabs,
    active_rel_path: active,
    workspace_tabs: [...input.workspaceTabs],
    page: input.page,
    view_mode: input.viewMode,
  }
}
