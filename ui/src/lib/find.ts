import { findMatchOffsets } from './text'

/** Cap so a common word in a large document stays interactive. */
export const MAX_FIND_HITS = 500

const SKIP_CLOSEST = 'script, style, textarea, template, .mermaid'

/** Unwraps previous find marks so a new query can walk original text nodes. */
export function clearFindHits(host: ParentNode): void {
  const marks = [...host.querySelectorAll('mark.find-hit')]
  const parents = new Set<Node>()
  for (const mark of marks) {
    const parent = mark.parentNode
    if (!parent) {
      continue
    }
    parents.add(parent)
    while (mark.firstChild) {
      parent.insertBefore(mark.firstChild, mark)
    }
    parent.removeChild(mark)
  }
  for (const parent of parents) {
    parent.normalize()
  }
}

/** Wraps case-insensitive matches in `mark.find-hit` and returns them in order. */
export function applyFindHits(
  host: HTMLElement,
  needle: string,
  limit = MAX_FIND_HITS,
): { marks: HTMLElement[]; capped: boolean } {
  clearFindHits(host)
  if (!needle || limit <= 0) {
    return { marks: [], capped: false }
  }

  const walker = document.createTreeWalker(host, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement
      if (
        !parent ||
        parent.closest(SKIP_CLOSEST) ||
        parent.closest('mark.find-hit')
      ) {
        return NodeFilter.FILTER_REJECT
      }
      return NodeFilter.FILTER_ACCEPT
    },
  })

  const texts: Text[] = []
  let node = walker.nextNode()
  while (node) {
    if (node.nodeType === Node.TEXT_NODE && node.textContent) {
      texts.push(node as Text)
    }
    node = walker.nextNode()
  }

  const marks: HTMLElement[] = []
  let capped = false
  for (const textNode of texts) {
    if (marks.length >= limit) {
      capped = true
      break
    }
    if (wrapTextNode(textNode, needle, marks, limit)) {
      capped = true
      break
    }
  }
  return { marks, capped }
}

function wrapTextNode(
  textNode: Text,
  needle: string,
  marks: HTMLElement[],
  limit: number,
): boolean {
  let current: Text | null = textNode
  while (current && marks.length < limit) {
    const text = current.textContent ?? ''
    const [at] = findMatchOffsets(text, needle)
    if (at === undefined) {
      return false
    }
    const end = at + needle.length
    if (end < current.length) {
      current.splitText(end)
    }
    const hit = at > 0 ? current.splitText(at) : current
    const mark = document.createElement('mark')
    mark.className = 'find-hit'
    hit.parentNode?.insertBefore(mark, hit)
    mark.appendChild(hit)
    marks.push(mark)
    const after = mark.nextSibling
    current =
      after && after.nodeType === Node.TEXT_NODE ? (after as Text) : null
  }
  return marks.length >= limit && remainingHit(current, needle)
}

function remainingHit(node: Text | null, needle: string): boolean {
  if (!node?.textContent) {
    return false
  }
  return findMatchOffsets(node.textContent, needle).length > 0
}
