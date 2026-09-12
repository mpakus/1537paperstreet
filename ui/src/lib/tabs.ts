import type { DocumentMeta, DocumentSource } from './generated/core'

/** Workspace page shown in the main tab strip. */
export type WorkspacePage = 'document' | 'assistant' | 'dashboard'

/** Optional non-document tab in the workspace strip. */
export type WorkspaceTab = Exclude<WorkspacePage, 'document'>

/** Opens one workspace tab without duplicating it. */
export function openWorkspaceTab(
  tabs: WorkspaceTab[],
  tab: WorkspaceTab,
): WorkspaceTab[] {
  return tabs.includes(tab) ? tabs : [...tabs, tab]
}

/** Closes one workspace tab. */
export function closeWorkspaceTab(
  tabs: WorkspaceTab[],
  tab: WorkspaceTab,
): WorkspaceTab[] {
  return tabs.filter((item) => item !== tab)
}

/** One open document in the tab strip. */
export type DocTab = {
  relPath: string
  title: string
  html: string
  docMeta: DocumentMeta | null
  docSourceMeta: DocumentSource | null
  draftText: string
}

/** File name used as the tab label. */
export function tabTitle(relPath: string): string {
  return relPath.split(/[/\\]/).filter(Boolean).at(-1) ?? relPath
}

/** Inserts or replaces a tab for the same relative path. */
export function upsertTab(tabs: DocTab[], tab: DocTab): DocTab[] {
  const index = tabs.findIndex((item) => item.relPath === tab.relPath)
  if (index < 0) {
    return [...tabs, tab]
  }
  const next = tabs.slice()
  next[index] = tab
  return next
}

/** Drops a tab. */
export function removeTab(tabs: DocTab[], relPath: string): DocTab[] {
  return tabs.filter((tab) => tab.relPath !== relPath)
}

/** What ⌘W should close: a workspace page, the open document, or nothing. */
export type CloseActiveTarget =
  | { kind: 'workspace'; page: WorkspaceTab }
  | { kind: 'document'; relPath: string }
  | { kind: 'none' }

/** Active tab closed by Close Tab (⌘W). */
export function closeActiveTarget(
  page: WorkspacePage,
  openRelPath: string | null,
): CloseActiveTarget {
  if (page === 'assistant' || page === 'dashboard') {
    return { kind: 'workspace', page }
  }
  if (openRelPath) {
    return { kind: 'document', relPath: openRelPath }
  }
  return { kind: 'none' }
}

/** Tab to activate after `closed` is removed. */
export function nextAfterClose(tabs: DocTab[], closed: string): string | null {
  const index = tabs.findIndex((tab) => tab.relPath === closed)
  if (index < 0) {
    return tabs.at(-1)?.relPath ?? null
  }
  return tabs[index + 1]?.relPath ?? tabs[index - 1]?.relPath ?? null
}

/** Keeps a tab when its file is renamed. */
export function retitleTab(tabs: DocTab[], from: string, to: string): DocTab[] {
  return tabs.map((tab) =>
    tab.relPath === from ? { ...tab, relPath: to, title: tabTitle(to) } : tab,
  )
}
