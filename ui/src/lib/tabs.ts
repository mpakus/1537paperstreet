import type { DocumentMeta, DocumentSource, ViewMode } from './generated/core'
import { followRenamedPath } from './tree'

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
  /** Temporary single-click tab; the next preview click replaces it. */
  preview: boolean
  /** Preview, Edit, or Split last used for this file. */
  viewMode: ViewMode
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

/** How a document should land in the tab strip. */
export type TabOpenMode = 'preview' | 'pin' | 'keep'

/** True when editor text diverges from the loaded source. */
export function tabHasUnsavedDraft(tab: DocTab): boolean {
  return Boolean(tab.docSourceMeta && tab.draftText !== tab.docSourceMeta.text)
}

/** Places a tab as a temporary preview, a pinned tab, or an in-place refresh. */
export function placeDocTab(
  tabs: DocTab[],
  tab: DocTab,
  mode: TabOpenMode,
): DocTab[] {
  const existing = tabs.find((item) => item.relPath === tab.relPath)
  if (mode === 'keep') {
    return upsertTab(tabs, {
      ...tab,
      preview: existing?.preview ?? false,
    })
  }
  if (mode === 'pin' || (existing && !existing.preview)) {
    return upsertTab(tabs, { ...tab, preview: false })
  }
  let next = tabs
  const dirtyPreview = next.find(
    (item) =>
      item.preview && item.relPath !== tab.relPath && tabHasUnsavedDraft(item),
  )
  if (dirtyPreview) {
    next = upsertTab(next, { ...dirtyPreview, preview: false })
  }
  const previewIndex = next.findIndex((item) => item.preview)
  if (previewIndex >= 0 && next[previewIndex]?.relPath !== tab.relPath) {
    const placed = next.slice()
    const same = placed.findIndex((item) => item.relPath === tab.relPath)
    if (same >= 0) {
      placed.splice(previewIndex, 1)
      const index = same > previewIndex ? same - 1 : same
      placed[index] = { ...tab, preview: true }
      return placed
    }
    placed[previewIndex] = { ...tab, preview: true }
    return placed
  }
  return upsertTab(next, { ...tab, preview: true })
}

/** Drops a tab. */
export function removeTab(tabs: DocTab[], relPath: string): DocTab[] {
  return tabs.filter((tab) => tab.relPath !== relPath)
}

/** Writes the tab we are leaving back into the strip, if it is still open. */
export function persistLeavingTab(
  tabs: DocTab[],
  leaving: DocTab | null,
): DocTab[] {
  if (!leaving || !tabs.some((tab) => tab.relPath === leaving.relPath)) {
    return tabs
  }
  return placeDocTab(tabs, leaving, 'keep')
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
  if (from === to) {
    return tabs
  }
  return tabs.map((tab) => {
    const relPath = followRenamedPath(tab.relPath, from, to)
    if (relPath === tab.relPath) {
      return tab
    }
    return {
      ...tab,
      relPath,
      title: tabTitle(relPath),
      docMeta: tab.docMeta ? { ...tab.docMeta, relPath } : null,
    }
  })
}

/**
 * Order for reopening saved tabs. Pinned files come first, then a preview,
 * and the active file last so it stays focused.
 */
export function tabsToReopen<T extends { rel_path: string; preview: boolean }>(
  tabs: readonly T[],
  activeRelPath: string | null,
): T[] {
  const rest = tabs.filter((tab) => tab.rel_path !== activeRelPath)
  return [
    ...rest.filter((tab) => !tab.preview),
    ...rest.filter((tab) => tab.preview),
    ...tabs.filter((tab) => tab.rel_path === activeRelPath),
  ]
}

/** Tab strip and open document after a project rename, without reloading disk. */
export function followOpenRename(
  tabs: DocTab[],
  openRelPath: string | null,
  docMeta: DocumentMeta | null,
  from: string,
  to: string,
): {
  tabs: DocTab[]
  openRelPath: string | null
  docMeta: DocumentMeta | null
} {
  return {
    tabs: retitleTab(tabs, from, to),
    openRelPath: openRelPath ? followRenamedPath(openRelPath, from, to) : null,
    docMeta: docMeta
      ? { ...docMeta, relPath: followRenamedPath(docMeta.relPath, from, to) }
      : null,
  }
}

/**
 * Drops an external-change prompt that fired because this app renamed the file.
 * A real delete of an unrelated path is left alone.
 */
export function promptAfterRename<T extends { relPath: string }>(
  prompt: T | null,
  from: string,
  to: string,
): T | null {
  if (!prompt) {
    return null
  }
  if (followRenamedPath(prompt.relPath, from, to) !== prompt.relPath) {
    return null
  }
  return prompt
}
