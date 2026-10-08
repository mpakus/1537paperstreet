// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { captureHighlight, mapHighlightSelection } from './highlights'

function article(html: string) {
  const root = document.createElement('article')
  root.innerHTML = `<section data-block="0" data-hash="abc">${html}</section>`
  return root
}

describe('preview highlight source mapping', () => {
  it('ignores presentation-only Copy buttons in quotes', () => {
    const root = article(
      '<div class="copy-block"><blockquote><p>quote</p></blockquote><button class="copy-control">Copy</button></div>',
    )
    const range = document.createRange()
    range.selectNodeContents(root.querySelector('p')!)
    const selected = captureHighlight(root, range)!
    expect(selected.blocks[0]?.text).toBe('quote')
    expect(
      mapHighlightSelection(selected, {
        hash: 'nonce',
        html: article(
          '<blockquote><p><span data-ps-map="nonce:2:7">quote</span></p></blockquote>',
        ).innerHTML,
      }),
    ).toEqual([{ start: 2, end: 7, from: 0, to: 5 }])
  })
  it('maps the second occurrence, Unicode and nested formatting exactly', () => {
    const root = article('<p>same <strong>same 🐈</strong></p>')
    const range = document.createRange()
    const text = root.querySelector('strong')!.firstChild!
    range.setStart(text, 0)
    range.setEnd(text, 7)
    const selected = captureHighlight(root, range)!
    const mapped = {
      hash: 'nonce',
      html: article(
        '<p><span data-ps-map="nonce:0:5">same </span><strong><span data-ps-map="nonce:7:16">same 🐈</span></strong></p>',
      ).innerHTML,
    }
    expect(mapHighlightSelection(selected, mapped)).toEqual([
      { start: 7, end: 16, from: 0, to: 7 },
    ])
  })

  it('rejects stale blocks, unsupported content and forged source attributes', () => {
    const root = article('<p>word code</p>')
    const range = document.createRange()
    range.selectNodeContents(root.querySelector('p')!)
    const selected = captureHighlight(root, range)!
    expect(selected).not.toBeNull()
    expect(() =>
      mapHighlightSelection(selected, {
        hash: 'nonce',
        html: article(
          '<p><span data-ps-map="nonce:0:5">word </span><code>code</code></p>',
        ).innerHTML,
      }),
    ).toThrow()
    expect(() =>
      mapHighlightSelection(selected, {
        hash: 'nonce',
        html: article('<p><span data-ps-map="forged:0:11">word code</span></p>')
          .innerHTML,
      }),
    ).toThrow()
    expect(() =>
      mapHighlightSelection(selected, {
        hash: 'nonce',
        html: '<section data-block="0" data-hash="different">word code</section>',
      }),
    ).toThrow()
  })

  it('allows text beside rendered math, but rejects selecting math or code', () => {
    const root = article(
      '<p><span class="math"><span>rendered equation</span></span> after</p>',
    )
    const range = document.createRange()
    range.selectNodeContents(root.querySelector('p')!.lastChild!)
    const selected = captureHighlight(root, range)!
    expect(
      mapHighlightSelection(selected, {
        hash: 'nonce',
        html: article(
          '<p><span class="math">x</span><span data-ps-map="nonce:3:9"> after</span></p>',
        ).innerHTML,
      }),
    ).toEqual([{ start: 3, end: 9, from: 0, to: 6 }])
    range.selectNodeContents(root.querySelector('p')!)
    expect(captureHighlight(root, range)).toBeNull()
  })

  it('captures multiple blocks while keeping rendered whitespace offsets', () => {
    const root = article('<p>one</p>\n')
    root.innerHTML +=
      '<section data-block="1" data-hash="def"><p>two</p>\n</section>'
    const range = document.createRange()
    range.setStart(root.querySelector('p')!.firstChild!, 1)
    range.setEnd(root.querySelectorAll('p')[1]!.firstChild!, 2)
    expect(captureHighlight(root, range)?.blocks).toEqual([
      { index: '0', hash: 'abc', text: 'one\n', from: 1, to: 4 },
      { index: '1', hash: 'def', text: 'two\n', from: 0, to: 2 },
    ])
  })
})
