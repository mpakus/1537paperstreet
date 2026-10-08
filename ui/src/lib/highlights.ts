import type { HighlightRange, HighlightSource } from './generated/core'

/** Rendered offsets captured before focus moves into the color palette. */
export type HighlightSelection = {
  blocks: {
    index: string
    hash: string
    text: string
    from: number
    to: number
  }[]
}

function textWithoutControls(node: Node): string {
  const clone = node.cloneNode(true) as ParentNode & Node
  for (const control of clone.querySelectorAll('.copy-control'))
    control.remove()
  for (const generated of clone.querySelectorAll('.math, figure.mermaid'))
    generated.replaceWith('\uFFFC')
  return clone.textContent ?? ''
}

/** Captures DOM text positions without interpreting Markdown. */
export function captureHighlight(
  article: HTMLElement,
  range: Range,
): HighlightSelection | null {
  if (range.collapsed || !article.contains(range.commonAncestorContainer))
    return null
  for (const generated of article.querySelectorAll(
    'pre, code, .math, figure.mermaid, .front-matter, img',
  )) {
    if (range.intersectsNode(generated)) return null
  }
  const blocks: HighlightSelection['blocks'] = []
  for (const block of article.querySelectorAll<HTMLElement>(
    'section[data-block][data-hash]',
  )) {
    if (!range.intersectsNode(block)) continue
    const prefix = document.createRange()
    prefix.selectNodeContents(block)
    let from = 0
    const text = textWithoutControls(block)
    let to = text.length
    if (block.contains(range.startContainer)) {
      prefix.setEnd(range.startContainer, range.startOffset)
      from = textWithoutControls(prefix.cloneContents()).length
    }
    if (block.contains(range.endContainer)) {
      prefix.setEnd(range.endContainer, range.endOffset)
      to = textWithoutControls(prefix.cloneContents()).length
    }
    if (from < to)
      blocks.push({
        index: block.dataset.block!,
        hash: block.dataset.hash!,
        text,
        from,
        to,
      })
  }
  return blocks.length ? { blocks } : null
}

/** Matches a captured selection against a fresh, sanitized Rust source map. */
export function mapHighlightSelection(
  selection: HighlightSelection,
  source: HighlightSource,
): HighlightRange[] {
  const root = document.createElement('template')
  root.innerHTML = source.html
  const ranges: HighlightRange[] = []
  const fail = () =>
    new Error(
      'The preview changed or contains text that cannot be highlighted. Select the text again.',
    )
  for (const selected of selection.blocks) {
    const block = Array.from(
      root.content.querySelectorAll<HTMLElement>('section[data-block]'),
    ).find(
      (node) =>
        node.dataset.block === selected.index &&
        node.dataset.hash === selected.hash,
    )
    if (!block || textWithoutControls(block) !== selected.text) throw fail()
    let covered = ''
    let at = 0
    let skip: Element | null = null
    const walker = document.createTreeWalker(
      block,
      NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT,
    )
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (skip?.contains(node)) continue
      skip = null
      if (!(node instanceof HTMLElement)) {
        at += node.textContent?.length ?? 0
        continue
      }
      if (node.matches('.copy-control, .math, figure.mermaid')) {
        if (!node.matches('.copy-control')) at += 1
        skip = node
        continue
      }
      const [hash, start, end] = (node.dataset.psMap ?? '').split(':')
      if (hash !== source.hash) continue
      const text = node.textContent ?? ''
      const from = Math.max(0, selected.from - at)
      const to = Math.min(text.length, selected.to - at)
      if (from < to) {
        covered += text.slice(from, to)
        ranges.push({ start: Number(start), end: Number(end), from, to })
      }
      at += text.length
      skip = node
    }
    if (
      covered.replace(/\s/g, '') !==
      selected.text.slice(selected.from, selected.to).replace(/\s/g, '')
    )
      throw fail()
  }
  if (!ranges.length) throw fail()
  return ranges
}
