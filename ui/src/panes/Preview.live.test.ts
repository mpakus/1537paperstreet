// @vitest-environment happy-dom

import { flushSync, mount, unmount } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'

import PreviewLiveFixture from './PreviewLiveFixture.svelte'

function block(index: number, hash: string, inner: string): string {
  return `<section class="chunk"><section data-block="${index}" data-src-line="${index + 1}" data-hash="${hash}">${inner}</section></section>`
}

type LivePreview = {
  setHtml: (next: string) => void
  setToc: (next: { level: number; title: string; id: string }[]) => void
  line: () => number | null
}

describe('Preview live html', () => {
  let target: HTMLDivElement | undefined
  let fixture: LivePreview | undefined

  afterEach(async () => {
    if (fixture) {
      await unmount(fixture)
    }
    target?.remove()
  })

  it('updates the edited block and keeps the rest of the article', () => {
    target = document.createElement('div')
    document.body.append(target)
    fixture = mount(PreviewLiveFixture, { target }) as LivePreview
    fixture.setHtml(
      block(0, 'aaa', '<p>One</p>') + block(1, 'bbb', '<p>Two</p>'),
    )
    flushSync()

    const article = target.querySelector('article')
    const kept = article?.querySelector<HTMLElement>('[data-hash="bbb"]')
    const marker = document.createElement('span')
    marker.id = 'kept-diagram'
    kept?.append(marker)

    fixture.setHtml(
      block(0, 'ccc', '<p>One!</p>') + block(1, 'bbb', '<p>Two</p>'),
    )
    flushSync()

    expect(article?.querySelector('[data-hash="bbb"]')).toBe(kept)
    expect(article?.querySelector('#kept-diagram')).toBe(marker)
    expect(article?.textContent).toContain('One!')
    expect(article?.textContent).toContain('Two')
  })

  it('reports the source line when a contents heading is chosen', () => {
    target = document.createElement('div')
    document.body.append(target)
    fixture = mount(PreviewLiveFixture, { target }) as LivePreview
    fixture.setToc([{ level: 2, title: 'Topic', id: 'topic' }])
    fixture.setHtml(
      '<section data-block="1" data-src-line="12" data-hash="abc"><h2 id="topic">Topic</h2></section>',
    )
    flushSync()

    const link = target.querySelector('a[href="#topic"]')
    expect(link).toBeTruthy()
    link?.dispatchEvent(new MouseEvent('click', { bubbles: true }))

    expect(fixture.line()).toBe(12)
  })
})
