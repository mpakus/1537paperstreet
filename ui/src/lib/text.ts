/** One run of highlighted or plain text for a search query. */
export type HighlightPart = {
  text: string
  hit: boolean
}

/** Splits `text` so query matches can be wrapped without using HTML. */
export function highlightQuery(text: string, query: string): HighlightPart[] {
  const needle = query.trim()
  if (!needle) {
    return [{ text, hit: false }]
  }
  const haystack = text.toLowerCase()
  const match = needle.toLowerCase()
  const parts: HighlightPart[] = []
  let from = 0
  while (from <= text.length) {
    const at = haystack.indexOf(match, from)
    if (at < 0) {
      if (from < text.length) {
        parts.push({ text: text.slice(from), hit: false })
      }
      break
    }
    if (at > from) {
      parts.push({ text: text.slice(from, at), hit: false })
    }
    parts.push({ text: text.slice(at, at + needle.length), hit: true })
    from = at + needle.length
    if (needle.length === 0) {
      break
    }
  }
  return parts
}

/** Start offsets of case-insensitive matches of `needle` inside `haystack`. */
export function findMatchOffsets(haystack: string, needle: string): number[] {
  if (!needle) {
    return []
  }
  const text = haystack.toLowerCase()
  const match = needle.toLowerCase()
  const found: number[] = []
  let from = 0
  while (from < text.length) {
    const at = text.indexOf(match, from)
    if (at < 0) {
      break
    }
    found.push(at)
    from = at + needle.length
  }
  return found
}

/** Native window title (menu bar / Mission Control): app name plus the open path. */
export function windowTitle(
  projectPath?: string | null,
  relPath?: string | null,
): string {
  const app = '1537paperstreet'
  const root = projectPath?.replace(/[/\\]+$/, '') ?? ''
  const rel = relPath?.replace(/^[/\\]+/, '') ?? ''
  if (!rel) {
    return app
  }
  const path = root ? `${root}/${rel}` : rel
  return `${app} - ${path}`
}
