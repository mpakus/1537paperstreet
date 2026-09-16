import { isHttpHref, parentDir, parseAssetHref } from './tree'

/** What a preview click on an `<a href>` should do. */
export type PreviewHrefAction =
  | { kind: 'hash'; id: string }
  | { kind: 'http'; url: string }
  | { kind: 'document'; relPath: string; hash: string }
  | { kind: 'ignore' }

/** Classifies a preview href as in-document, browser, or project file. */
export function classifyPreviewHref(
  href: string,
  options: {
    projectId: string | null
    projectPath?: string | null
    currentRelPath?: string | null
  },
): PreviewHrefAction {
  const trimmed = href.trim()
  if (!trimmed) {
    return { kind: 'ignore' }
  }
  if (trimmed.startsWith('#')) {
    return { kind: 'hash', id: trimmed.slice(1) }
  }
  if (isHttpHref(trimmed)) {
    return { kind: 'http', url: trimmed }
  }

  const asset = parseAssetHref(trimmed)
  if (asset) {
    if (!options.projectId || asset.projectId !== options.projectId) {
      return { kind: 'ignore' }
    }
    return {
      kind: 'document',
      relPath: asset.relPath,
      hash: asset.hash,
    }
  }

  const fromFile = relPathFromFileHref(trimmed, options.projectPath)
  if (fromFile) {
    return fromFile
  }

  if (hasScheme(trimmed)) {
    return { kind: 'ignore' }
  }

  const relative = resolveRelativeHref(trimmed, options.currentRelPath ?? '')
  if (!relative) {
    return { kind: 'ignore' }
  }
  return { kind: 'document', relPath: relative.relPath, hash: relative.hash }
}

function hasScheme(href: string): boolean {
  return /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(href)
}

function relPathFromFileHref(
  href: string,
  projectPath: string | null | undefined,
): Extract<PreviewHrefAction, { kind: 'document' }> | null {
  if (!projectPath || !/^file:/i.test(href)) {
    return null
  }
  let absolute: string
  try {
    const url = new URL(href)
    if (url.protocol !== 'file:') {
      return null
    }
    absolute = decodeURIComponent(url.pathname)
  } catch {
    return null
  }
  const hash = hashOf(href)
  const relPath = relInsideProject(absolute, projectPath)
  if (!relPath) {
    return null
  }
  return { kind: 'document', relPath, hash }
}

function relInsideProject(
  absolute: string,
  projectPath: string,
): string | null {
  const root = projectPath.replace(/[/\\]+$/, '')
  const prefix = `${root}/`
  if (!absolute.startsWith(prefix)) {
    return null
  }
  const relPath = absolute.slice(prefix.length).replaceAll('\\', '/')
  if (!relPath || relPath.split('/').includes('..')) {
    return null
  }
  return relPath
}

function resolveRelativeHref(
  href: string,
  currentRelPath: string,
): { relPath: string; hash: string } | null {
  const hash = hashOf(href)
  const query = href.indexOf('?')
  const hashAt = href.indexOf('#')
  let end = href.length
  if (query !== -1) {
    end = Math.min(end, query)
  }
  if (hashAt !== -1) {
    end = Math.min(end, hashAt)
  }
  let path = href.slice(0, end)
  if (!path || path.startsWith('/') || path.startsWith('\\')) {
    return null
  }
  try {
    path = decodeURIComponent(path)
  } catch {
    return null
  }
  const parts = parentDir(currentRelPath).split('/').filter(Boolean)
  for (const segment of path.split('/')) {
    if (!segment || segment === '.') {
      continue
    }
    if (segment === '..') {
      return null
    }
    parts.push(segment)
  }
  const relPath = parts.join('/')
  if (!relPath) {
    return null
  }
  return { relPath, hash }
}

function hashOf(href: string): string {
  const index = href.indexOf('#')
  return index === -1 ? '' : href.slice(index + 1)
}
