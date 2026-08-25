import type { TreeNode, WatchUpdate } from './generated/core'

/** One visible row in the flattened, lazily expanded file tree. */
export type TreeRow = {
  node: TreeNode
  depth: number
}

const MARKDOWN_NAME = /\.(md|markdown|mdown|mdwn)$/i

/** Returns whether a file name uses a recognized Markdown extension. */
export function isMarkdownPath(name: string): boolean {
  return MARKDOWN_NAME.test(name)
}

/** Icon bucket for a tree row: folder, Markdown, or anything else. */
export type FileIconKind = 'directory' | 'markdown' | 'file'

/** Distinguishes folders and Markdown files from other entries. */
export function fileIconKind(
  node: Pick<TreeNode, 'kind' | 'name'>,
): FileIconKind {
  if (node.kind === 'directory') {
    return 'directory'
  }
  if (isMarkdownPath(node.name)) {
    return 'markdown'
  }
  return 'file'
}

/** Directory prefixes that must be expanded to reveal `relPath`. */
export function ancestorDirs(relPath: string): string[] {
  const parts = relPath.split(/[/\\]/).filter(Boolean)
  parts.pop()
  const dirs: string[] = []
  let current = ''
  for (const part of parts) {
    current = current ? `${current}/${part}` : part
    dirs.push(current)
  }
  return dirs
}

/** Parent directory of a project-relative path, or `''` at the project root. */
export function parentDir(relPath: string): string {
  const parts = relPath.split(/[/\\]/).filter(Boolean)
  parts.pop()
  return parts.join('/')
}

/** Joins a project-relative directory and a file name. */
export function joinRel(dir: string, name: string): string {
  return dir ? `${dir}/${name}` : name
}

/** Directory that should receive a new file created from `node`. */
export function targetDir(node: TreeNode | null): string {
  if (!node) {
    return ''
  }
  if (node.kind === 'directory') {
    return node.relPath
  }
  return parentDir(node.relPath)
}

/** Parents first so lazy tree loads can expand from the root downward. */
export function sortDirsByDepth(dirs: Iterable<string>): string[] {
  return [...new Set(dirs)].sort((left, right) => {
    const leftDepth = left.split('/').filter(Boolean).length
    const rightDepth = right.split('/').filter(Boolean).length
    return leftDepth - rightDepth || left.localeCompare(right)
  })
}

/** Flattens loaded tree nodes according to the expanded directory set. */
export function flattenTree(
  rootNodes: TreeNode[],
  children: Record<string, TreeNode[]>,
  expanded: ReadonlySet<string>,
): TreeRow[] {
  const rows: TreeRow[] = []
  const walk = (nodes: TreeNode[], depth: number) => {
    for (const node of nodes) {
      rows.push({ node, depth })
      if (node.kind === 'directory' && expanded.has(node.relPath)) {
        const nested = children[node.relPath]
        if (nested) {
          walk(nested, depth + 1)
        }
      }
    }
  }
  walk(rootNodes, 0)
  return rows
}

/** Visible slice of a virtualized list, including a leading/trailing buffer. */
export function visibleWindow(
  count: number,
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  buffer: number,
): { start: number; end: number } {
  if (viewportHeight <= 0 || count === 0) {
    return { start: 0, end: count }
  }
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - buffer)
  const end = Math.min(
    count,
    Math.ceil((scrollTop + viewportHeight) / rowHeight) + buffer,
  )
  return { start, end }
}

/** Project-relative asset URL produced by the Markdown renderer. */
export type AssetRef = {
  projectId: string
  relPath: string
  hash: string
}

const ASSET_HREF =
  /^asset:\/\/localhost\/([^/]+)\/([^?#]*)(?:\?[^#]*)?(?:#(.*))?$/i

/** Parses `asset://localhost/<projectId>/<rel>` including an optional hash. */
export function parseAssetHref(href: string): AssetRef | null {
  const match = href.trim().match(ASSET_HREF)
  if (!match) {
    return null
  }
  const projectId = match[1]
  const encodedPath = match[2]
  if (!projectId || encodedPath == null) {
    return null
  }
  try {
    const relPath = decodeURIComponent(encodedPath)
    if (!relPath || relPath.includes('..')) {
      return null
    }
    return { projectId, relPath, hash: match[3] ?? '' }
  } catch {
    return null
  }
}

/** Returns whether a link should open in the system browser. */
export function isHttpHref(href: string): boolean {
  return /^https?:\/\//i.test(href.trim())
}

/** Parent directories that should reload after watched paths change. */
export function dirsToReload(paths: Iterable<string>): string[] {
  const dirs = new Set<string>([''])
  for (const path of paths) {
    dirs.add(parentDir(path))
  }
  return sortDirsByDepth(dirs)
}

/** Inclusive range of visible tree paths between an anchor and the clicked row. */
export function rangeRelPaths(
  rows: TreeRow[],
  anchor: string | null,
  target: string,
): string[] {
  const targetIndex = rows.findIndex((row) => row.node.relPath === target)
  if (targetIndex < 0) {
    return [target]
  }
  const anchorIndex = anchor
    ? rows.findIndex((row) => row.node.relPath === anchor)
    : targetIndex
  if (anchorIndex < 0) {
    return [target]
  }
  const start = Math.min(anchorIndex, targetIndex)
  const end = Math.max(anchorIndex, targetIndex)
  return rows.slice(start, end + 1).map((row) => row.node.relPath)
}

/** Destination folder under a pointer, or `null` when the pointer is not over the tree. */
export function dropDirAtPoint(x: number, y: number): string | null {
  if (typeof document === 'undefined') {
    return null
  }
  const el = document.elementFromPoint(x, y)
  if (!(el instanceof Element)) {
    return null
  }
  const row = el.closest('[data-rel]')
  if (row instanceof HTMLElement && row.dataset.rel != null) {
    return row.dataset.kind === 'directory'
      ? row.dataset.rel
      : parentDir(row.dataset.rel)
  }
  return null
}

/** Project row under a pointer, or `null` when the pointer is not over Projects. */
export function projectIdAtPoint(x: number, y: number): string | null {
  if (typeof document === 'undefined') {
    return null
  }
  const el = document.elementFromPoint(x, y)
  if (!(el instanceof Element)) {
    return null
  }
  const row = el.closest('[data-project-id]')
  if (row instanceof HTMLElement && row.dataset.projectId) {
    return row.dataset.projectId
  }
  return null
}

/** Marker that prefixes an in-app tree drag payload. */
export const TREE_DRAG_PREFIX = '1537paperstreet-items'

/** In-app tree items being dragged, including their source project. */
export type TreeDragPayload = {
  projectId: string
  paths: string[]
}

let activeTreeDrag: TreeDragPayload | null = null
let treeDragCopy = false
let lastTreeDropKey = ''
let lastTreeDropAt = 0

/** Records a tree drag so another pane can accept it if `dataTransfer` is empty. */
export function beginTreeDrag(projectId: string, paths: string[]): void {
  activeTreeDrag = { projectId, paths }
  treeDragCopy = false
}

/** The in-flight tree drag, if any. */
export function peekTreeDrag(): TreeDragPayload | null {
  return activeTreeDrag
}

/** Whether the current tree drag should copy (⌥) instead of move. */
export function peekTreeDragCopy(): boolean {
  return treeDragCopy
}

/** Updates copy vs move from the latest dragover modifiers. */
export function setTreeDragCopy(copy: boolean): void {
  treeDragCopy = copy
}

/** Clears the in-flight tree drag. Call from `dragend`. */
export function clearTreeDrag(): void {
  activeTreeDrag = null
  treeDragCopy = false
}

/** True when the same tree drop was already applied in this event burst. */
export function claimTreeDrop(projectId: string, paths: string[]): boolean {
  const key = `${projectId}:${paths.join('\n')}`
  const now = Date.now()
  if (lastTreeDropKey === key && now - lastTreeDropAt < 800) {
    return false
  }
  lastTreeDropKey = key
  lastTreeDropAt = now
  return true
}

/** Serializes a tree drag so another project can accept the drop. */
export function encodeTreeDrag(projectId: string, paths: string[]): string {
  return [TREE_DRAG_PREFIX, projectId, ...paths].join('\n')
}

/** Parses a tree drag payload, including legacy newline-separated paths. */
export function decodeTreeDrag(
  raw: string,
): { projectId: string | null; paths: string[] } | null {
  const lines = raw
    .replace(/\r\n/g, '\n')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
  if (lines.length === 0) {
    return null
  }
  if (lines[0] === TREE_DRAG_PREFIX) {
    const projectId = lines[1]
    const paths = lines.slice(2)
    if (!projectId || paths.length === 0) {
      return null
    }
    return { projectId, paths }
  }
  return { projectId: null, paths: lines }
}

/** Whether `transfer.types` lists `type` (DOMStringList or string array). */
export function dataTransferHasType(
  transfer: DataTransfer | null,
  type: string,
): boolean {
  if (!transfer) {
    return false
  }
  const types = transfer.types as unknown as {
    contains?: (name: string) => boolean
    includes?: (name: string) => boolean
    length: number
    [index: number]: string
  }
  if (typeof types.contains === 'function') {
    return types.contains(type)
  }
  if (typeof types.includes === 'function') {
    return types.includes(type)
  }
  return Array.from(
    { length: types.length },
    (_, index) => types[index],
  ).includes(type)
}

/** True when this drag is an in-app tree item. */
export function isTreeDrag(transfer: DataTransfer | null): boolean {
  return activeTreeDrag !== null || dataTransferHasType(transfer, 'text/plain')
}

/** In-memory payload first, then `text/plain`, so WKWebView drops still work. */
export function resolveTreeDrag(
  transfer: DataTransfer | null,
): TreeDragPayload | null {
  if (activeTreeDrag && activeTreeDrag.paths.length > 0) {
    return activeTreeDrag
  }
  const parsed = decodeTreeDrag(transfer?.getData('text/plain') ?? '')
  if (!parsed?.projectId || parsed.paths.length === 0) {
    return null
  }
  return { projectId: parsed.projectId, paths: parsed.paths }
}

/** Whether a coalesced watch update may have changed `relPath`. */
export function watchTouchesOpenFile(
  update: WatchUpdate,
  relPath: string,
): boolean {
  if ('rescanExpanded' in update) {
    return true
  }
  return update.pathsChanged.paths.some(
    (changed) =>
      changed === relPath ||
      relPath.startsWith(`${changed}/`) ||
      changed.startsWith(`${relPath}/`),
  )
}

/** Unsaved editor text that differs from the last loaded source. */
export function isDraftDirty(
  editorOpened: boolean,
  source: { text: string } | null,
  draftText: string,
): boolean {
  return Boolean(editorOpened && source && draftText !== source.text)
}
