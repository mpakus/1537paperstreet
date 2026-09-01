// @vitest-environment happy-dom

import { describe, expect, it } from 'vitest'

import { MAX_FIND_HITS, applyFindHits, clearFindHits } from './find'

describe('applyFindHits', () => {
  it('wraps matches as you would type a longer query', () => {
    const host = document.createElement('article')
    host.textContent = 'Fight Club Notes'
    const first = applyFindHits(host, 'c')
    expect(first.marks).toHaveLength(1)
    expect(first.marks[0]?.textContent).toBe('C')
    expect(host.querySelectorAll('mark.find-hit')).toHaveLength(1)

    const second = applyFindHits(host, 'club')
    expect(second.marks).toHaveLength(1)
    expect(second.marks[0]?.textContent).toBe('Club')
    expect(host.textContent).toBe('Fight Club Notes')
  })

  it('clears marks when the query is empty', () => {
    const host = document.createElement('article')
    host.textContent = 'alpha'
    applyFindHits(host, 'a')
    expect(host.querySelectorAll('mark.find-hit').length).toBeGreaterThan(0)
    applyFindHits(host, '')
    expect(host.querySelectorAll('mark.find-hit')).toHaveLength(0)
    clearFindHits(host)
    expect(host.textContent).toBe('alpha')
  })

  it('skips mermaid figures and caps the hit list', () => {
    const host = document.createElement('article')
    host.innerHTML = '<p>hit</p><figure class="mermaid">hit</figure><p>hit</p>'
    const { marks } = applyFindHits(host, 'hit')
    expect(marks).toHaveLength(2)
    expect(MAX_FIND_HITS).toBe(500)
    const many = document.createElement('article')
    many.textContent = 'x '.repeat(600)
    const capped = applyFindHits(many, 'x', 3)
    expect(capped.marks).toHaveLength(3)
    expect(capped.capped).toBe(true)
  })
})
