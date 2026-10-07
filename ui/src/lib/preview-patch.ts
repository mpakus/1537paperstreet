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

  const reused = reuseByHash(currentBlocks, nextBlocks)
  const reusedSet = new Set(
    reused.filter((node): node is HTMLElement => node !== null),
  )
  const anchor = captureAnchor(scroller, currentBlocks, reusedSet)
  const changed: HTMLElement[] = []
  const replacements = new Map<Node, Node>()
  for (let index = 0; index < nextBlocks.length; index += 1) {
    const next = nextBlocks[index]
    const kept = reused[index]
    if (!next) {
      continue
    }
    if (kept) {
      syncBlockIdentity(kept, next)
      replacements.set(next, kept)
    } else {
      changed.push(next)
    }
  }
  const removed = currentBlocks.filter((node) => !reusedSet.has(node))
  // Keep chunk containers: replacing them discards content-visibility's
  // remembered heights, even if the blocks inside are reused.
  const chunks = [...article.children].filter((node) => node.matches('.chunk'))
  let chunkIndex = 0
  const roots = [...holder.childNodes].map((node) => {
    if (node instanceof HTMLElement && node.matches('.chunk')) {
      const chunk = chunks[chunkIndex++] ?? node
      reconcileChildren(
        chunk,
        [...node.childNodes].map((child) => replacements.get(child) ?? child),
      )
      return chunk
    }
    return replacements.get(node) ?? node
  })
  reconcileChildren(article, roots)
  appliedHtml.set(article, nextHtml)
  pinReadingPosition(scroller, anchor, fallback)
  return { changed, removed }
}

function reconcileChildren(parent: Element, children: Node[]) {
  const wanted = new Set(children)
  for (const child of [...parent.childNodes]) {
    if (!wanted.has(child)) {
      child.remove()
    }
  }
  let cursor = parent.firstChild
  for (const child of children) {
    if (child === cursor) {
      cursor = cursor.nextSibling
    } else {
      parent.insertBefore(child, cursor)
    }
  }
}

/** Reveals the active source line without moving an already visible edit. */
export function revealPreviewLine(
  article: HTMLElement,
  source: { line: number; lines: number },
) {
  const scroller = article.parentElement
  const blocks = blocksIn(article)
  if (!scroller || blocks.length === 0) {
    return
  }
  let target = blocks[0]!
  let endLine = source.lines + 1
  for (const block of blocks) {
    const line = Number(block.dataset.srcLine)
    if (line > source.line) {
      endLine = line
      break
    }
    target = block
  }
  const startLine = Number(target.dataset.srcLine)
  const fraction = Math.max(
    0,
    Math.min(1, (source.line - startLine) / Math.max(1, endLine - startLine)),
  )
  const rect = target.getBoundingClientRect()
  const viewport = scroller.getBoundingClientRect()
  const top = rect.top + fraction * rect.height
  if (top < viewport.top - 0.5 || top >= viewport.bottom) {
    scroller.scrollTop = Math.max(
      0,
      scroller.scrollTop + top - (viewport.top + viewport.bottom) / 2,
    )
  }
}

function blocksIn(root: ParentNode): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>('section[data-block]')]
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
  // Heading IDs depend on earlier headings, not just this block's source hash.
  const selector = 'h1[id], h2[id], h3[id], h4[id], h5[id], h6[id]'
  const headings = next.querySelectorAll<HTMLElement>(selector)
  if (headings.length > 0) {
    const current = kept.querySelectorAll<HTMLElement>(selector)
    headings.forEach((heading, index) => {
      const node = current[index]
      if (node && node.id !== heading.id) {
        node.id = heading.id
      }
    })
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
