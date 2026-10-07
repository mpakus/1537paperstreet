// @vitest-environment happy-dom

import { describe, expect, it } from 'vitest'

import { sourceLineForHeading } from './toc'

describe('sourceLineForHeading', () => {
  it('reads the source line from the heading block', () => {
    const root = document.createElement('div')
    root.innerHTML =
      '<section data-block="1" data-src-line="12" data-hash="abc"><h2 id="topic">Topic</h2></section>'
    const heading = root.querySelector('#topic')
    expect(heading).toBeTruthy()
    expect(sourceLineForHeading(heading as Element)).toBe(12)
  })

  it('ignores a heading with no source line', () => {
    const root = document.createElement('div')
    root.innerHTML = '<h2 id="topic">Topic</h2>'
    const heading = root.querySelector('#topic')
    expect(sourceLineForHeading(heading as Element)).toBeNull()
  })
})
