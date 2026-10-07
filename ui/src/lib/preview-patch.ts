/** Blocks whose markup was replaced, and blocks that left the article. */
export type PreviewPatch = {
  changed: HTMLElement[]
  removed: HTMLElement[]
}

const appliedHtml = new WeakMap<HTMLElement, string>()

/**
 * Updates `article` to `nextHtml` without replacing blocks whose source hash
 * is unchanged. The reading pane keeps those nodes, so scroll, diagrams, and
 * copy buttons stay where they are.
 */
export function patchPreviewHtml(
  article: HTMLElement,
  nextHtml: string,
): PreviewPatch {
  if (appliedHtml.get(article) === nextHtml) {
    return { changed: [], removed: [] }
  }

  const scroller = article.parentElement
  const fallback = scroller?.scrollTop ?? 0
  if (!nextHtml) {
    const removed = blocksIn(article)
    article.replaceChildren()
    appliedHtml.set(article, nextHtml)
    pinReadingPosition(scroller, null, fallback)
    return { changed: [], removed }
  }

  const holder = document.createElement('div')
  holder.innerHTML = nextHtml
  const nextBlocks = blocksIn(holder)
  const currentBlocks = blocksIn(article)
  if (nextBlocks.length === 0 || currentBlocks.length === 0) {
    const removed = currentBlocks
    article.innerHTML = nextHtml
    appliedHtml.set(article, nextHtml)
    pinReadingPosition(scroller, null, fallback)
    const fresh = blocksIn(article)
    return {
      changed: fresh.length > 0 ? fresh : [article],
      removed,
    }
  }

  if (sameBlocks(currentBlocks, nextBlocks)) {
    appliedHtml.set(article, nextHtml)
    return { changed: [], removed: [] }
  }

  const reused = reuseByHash(currentBlocks, nextBlocks)
  const reusedSet = new Set(
    reused.filter((node): node is HTMLElement => node !== null),
  )
  const anchor = captureAnchor(scroller, currentBlocks, reusedSet)
  const changed: HTMLElement[] = []
  for (let index = 0; index < nextBlocks.length; index += 1) {
    const next = nextBlocks[index]
    const kept = reused[index]
    if (!next) {
      continue
    }
    if (kept) {
      syncBlockIdentity(kept, next)
      next.replaceWith(kept)
    } else {
      changed.push(next)
    }
  }
  const removed = currentBlocks.filter((node) => !reusedSet.has(node))
  article.replaceChildren(...holder.childNodes)
  appliedHtml.set(article, nextHtml)
  pinReadingPosition(scroller, anchor, fallback)
  return { changed, removed }
}

function blocksIn(root: ParentNode): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>('section[data-block]')]
}

function sameBlocks(current: HTMLElement[], next: HTMLElement[]): boolean {
  if (current.length !== next.length) {
    return false
  }
  for (let index = 0; index < current.length; index += 1) {
    const left = current[index]
    const right = next[index]
    if (
      !left ||
      !right ||
      left.getAttribute('data-hash') !== right.getAttribute('data-hash') ||
      left.getAttribute('data-block') !== right.getAttribute('data-block') ||
      left.getAttribute('data-src-line') !== right.getAttribute('data-src-line')
    ) {
      return false
    }
  }
  return true
}

/** Reuses current blocks in source order when the same hash appears twice. */
function reuseByHash(
  current: HTMLElement[],
  next: HTMLElement[],
): Array<HTMLElement | null> {
  const pools = new Map<string, HTMLElement[]>()
  for (const node of current) {
    const hash = node.getAttribute('data-hash') ?? ''
    const pool = pools.get(hash)
    if (pool) {
      pool.push(node)
    } else {
      pools.set(hash, [node])
    }
  }
  return next.map((node) => {
    const pool = pools.get(node.getAttribute('data-hash') ?? '')
    return pool?.shift() ?? null
  })
}

function syncBlockIdentity(kept: HTMLElement, next: HTMLElement) {
  for (const name of ['data-block', 'data-src-line'] as const) {
    const value = next.getAttribute(name)
    if (value != null && kept.getAttribute(name) !== value) {
      kept.setAttribute(name, value)
    }
  }
}

function captureAnchor(
  scroller: HTMLElement | null,
  blocks: readonly HTMLElement[],
  reused: ReadonlySet<HTMLElement>,
): { node: HTMLElement; offset: number } | null {
  if (!scroller || reused.size === 0) {
    return null
  }
  const edge = scroller.getBoundingClientRect().top
  for (const block of blocks) {
    if (!reused.has(block)) {
      continue
    }
    const rect = block.getBoundingClientRect()
    if (rect.bottom > edge + 1) {
      return { node: block, offset: rect.top - edge }
    }
  }
  for (let index = blocks.length - 1; index >= 0; index -= 1) {
    const block = blocks[index]
    if (block && reused.has(block)) {
      const rect = block.getBoundingClientRect()
      return { node: block, offset: rect.top - edge }
    }
  }
  return null
}

function pinReadingPosition(
  scroller: HTMLElement | null,
  anchor: { node: HTMLElement; offset: number } | null,
  fallback: number,
) {
  if (!scroller) {
    return
  }
  if (anchor?.node.isConnected) {
    const next =
      anchor.node.getBoundingClientRect().top -
      scroller.getBoundingClientRect().top
    const delta = next - anchor.offset
    if (Math.abs(delta) > 0.5) {
      scroller.scrollTop += delta
      return
    }
  }
  if (scroller.scrollTop !== fallback) {
    scroller.scrollTop = fallback
  }
}
