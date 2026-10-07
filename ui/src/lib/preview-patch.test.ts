// @vitest-environment happy-dom

import { describe, expect, it } from 'vitest'

import { patchPreviewHtml, revealPreviewLine } from './preview-patch'

function block(
  index: number,
  hash: string,
  inner: string,
  line = index + 1,
): string {
  return `<section data-block="${index}" data-src-line="${line}" data-hash="${hash}">${inner}</section>`
}

function doc(blocks: string): string {
  return `<section class="chunk">${blocks}</section>`
}

function mount(): { scroller: HTMLElement; article: HTMLElement } {
  const scroller = document.createElement('div')
  const article = document.createElement('article')
  scroller.append(article)
  document.body.append(scroller)
  return { scroller, article }
}

describe('patchPreviewHtml', () => {
  it('replaces the edited block and keeps the others', () => {
    const { article } = mount()
    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>One</p>') + block(1, 'bbb', '<p>Two</p>')),
    )
    const kept = article.querySelector<HTMLElement>('[data-hash="bbb"]')
    const marker = document.createElement('span')
    marker.id = 'keep'
    kept?.append(marker)

    const patch = patchPreviewHtml(
      article,
      doc(block(0, 'ccc', '<p>One!</p>') + block(1, 'bbb', '<p>Two</p>')),
    )

    expect(article.querySelector('[data-hash="bbb"]')).toBe(kept)
    expect(document.getElementById('keep')).toBe(marker)
    expect(article.querySelector('[data-hash="ccc"]')?.textContent).toBe('One!')
    expect(patch.changed).toHaveLength(1)
    expect(patch.removed).toHaveLength(1)
    expect(patch.changed[0]?.getAttribute('data-hash')).toBe('ccc')
  })

  it('keeps the first of two identical blocks when the second changes', () => {
    const { article } = mount()
    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>Hi</p>') + block(1, 'aaa', '<p>Hi</p>')),
    )
    const first = article.querySelector('section[data-block]')

    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>Hi</p>') + block(1, 'bbb', '<p>Hi!</p>')),
    )

    expect(article.querySelector('section[data-block]')).toBe(first)
    expect(article.querySelector('[data-hash="bbb"]')?.textContent).toBe('Hi!')
  })

  it('keeps chunk containers and never detaches unchanged blocks while typing', () => {
    const { article } = mount()
    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>One</p>') + block(1, 'bbb', '<p>Two</p>')) +
        doc(block(2, 'ddd', '<p>Three</p>')),
    )
    const chunks = [...article.children]
    const kept = article.querySelector('[data-hash="bbb"]')!
    const observer = new MutationObserver(() => {})
    observer.observe(article, { childList: true, subtree: true })

    patchPreviewHtml(
      article,
      doc(block(0, 'ccc', '<p>One!</p>') + block(1, 'bbb', '<p>Two</p>')) +
        doc(block(2, 'ddd', '<p>Three</p>')),
    )

    const removed = observer
      .takeRecords()
      .flatMap((record) => [...record.removedNodes])
    observer.disconnect()
    expect([...article.children]).toEqual(chunks)
    expect(removed).not.toContain(kept)
    expect(removed).not.toContain(chunks[1])
  })

  it('renumbers a reused block when a block is inserted above it', () => {
    const { article } = mount()
    patchPreviewHtml(article, doc(block(0, 'aaa', '<p>Tail</p>', 1)))
    const tail = article.querySelector<HTMLElement>('[data-hash="aaa"]')

    patchPreviewHtml(
      article,
      doc(
        block(0, 'bbb', '<p>Head</p>', 1) + block(1, 'aaa', '<p>Tail</p>', 3),
      ),
    )

    expect(article.querySelector('[data-hash="aaa"]')).toBe(tail)
    expect(tail?.getAttribute('data-block')).toBe('1')
    expect(tail?.getAttribute('data-src-line')).toBe('3')
  })

  it('handles blocks moving across chunk boundaries and removal of the last chunk', () => {
    const { article } = mount()
    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>One</p>') + block(1, 'bbb', '<p>Two</p>')) +
        doc(block(2, 'ccc', '<p>Three</p>')),
    )
    const kept = article.querySelector('[data-hash="bbb"]')
    const reflowed =
      doc(block(0, 'aaa', '<p>One</p>')) +
      doc(block(1, 'bbb', '<p>Two</p>') + block(2, 'ccc', '<p>Three</p>'))

    patchPreviewHtml(article, reflowed)

    expect(article.innerHTML).toBe(reflowed)
    expect(article.querySelector('[data-hash="bbb"]')).toBe(kept)
    const shortened = doc(block(0, 'bbb', '<p>Two</p>'))
    patchPreviewHtml(article, shortened)
    expect(article.innerHTML).toBe(shortened)
    expect(article.querySelector('[data-hash="bbb"]')).toBe(kept)
  })

  it('leaves the reading position in place', () => {
    const { scroller, article } = mount()
    patchPreviewHtml(
      article,
      doc(block(0, 'aaa', '<p>One</p>') + block(1, 'bbb', '<p>Two</p>')),
    )
    scroller.scrollTop = 80

    patchPreviewHtml(
      article,
      doc(block(0, 'ccc', '<p>One!</p>') + block(1, 'bbb', '<p>Two</p>')),
    )

    expect(scroller.scrollTop).toBe(80)
  })

  it('does not rebuild the article when the markup is unchanged', () => {
    const { article } = mount()
    const html = doc(block(0, 'aaa', '<p>One</p>'))
    patchPreviewHtml(article, html)
    const section = article.querySelector('section[data-block]')
    const marker = document.createElement('i')
    section?.append(marker)

    const patch = patchPreviewHtml(article, html)

    expect(patch).toEqual({ changed: [], removed: [] })
    expect(article.querySelector('i')).toBe(marker)
  })

  it('replaces markup that has no blocks', () => {
    const { article } = mount()
    patchPreviewHtml(article, '<p>Hi</p>')
    expect(article.textContent).toBe('Hi')
    patchPreviewHtml(article, '<p>Yo</p>')
    expect(article.textContent).toBe('Yo')
    patchPreviewHtml(article, '')
    expect(article.childNodes).toHaveLength(0)
  })
})

describe('revealPreviewLine', () => {
  it('reveals the edited line inside a tall block and leaves visible text still', () => {
    const { scroller, article } = mount()
    patchPreviewHtml(
      article,
      doc(
        block(0, 'aaa', '<pre>Long code</pre>', 1) +
          block(1, 'bbb', '<p>Tail</p>', 101),
      ),
    )
    scroller.getBoundingClientRect = () => ({ top: 0, bottom: 400 }) as DOMRect
    const code = article.querySelector<HTMLElement>('[data-block="0"]')!
    code.getBoundingClientRect = () =>
      ({ top: -scroller.scrollTop, height: 2000 }) as DOMRect

    revealPreviewLine(article, { line: 75, lines: 101 })
    expect(scroller.scrollTop).toBeGreaterThan(1000)
    const top = scroller.scrollTop
    revealPreviewLine(article, { line: 75, lines: 101 })
    expect(scroller.scrollTop).toBe(top)
    revealPreviewLine(article, { line: 2, lines: 101 })
    expect(scroller.scrollTop).toBe(0)

    scroller.scrollTop = 200
    code.getBoundingClientRect = () =>
      ({ top: 200 - scroller.scrollTop, height: 2000 }) as DOMRect
    revealPreviewLine(article, { line: 1, lines: 101 })
    expect(scroller.scrollTop).toBe(200)
  })
})
