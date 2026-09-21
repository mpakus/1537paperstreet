import { describe, expect, it } from 'vitest'

import { highlightQuery, findMatchOffsets, windowTitle } from './text'

describe('highlightQuery', () => {
  it('returns the whole string when the query is empty', () => {
    expect(highlightQuery('Notes', '')).toEqual([{ text: 'Notes', hit: false }])
  })

  it('marks case-insensitive matches', () => {
    expect(highlightQuery('Fight Club Notes', 'club')).toEqual([
      { text: 'Fight ', hit: false },
      { text: 'Club', hit: true },
      { text: ' Notes', hit: false },
    ])
  })

  it('lists case-insensitive match offsets', () => {
    expect(findMatchOffsets('Fight Club Notes', '')).toEqual([])
    expect(findMatchOffsets('Fight Club Notes', 'club')).toEqual([6])
    expect(findMatchOffsets('aba ba', 'ba')).toEqual([1, 4])
  })
})

describe('windowTitle', () => {
  it('is the app name when no document is open', () => {
    expect(windowTitle()).toBe('1537paperstreet')
    expect(windowTitle('/Users/me/notes', null)).toBe('1537paperstreet')
  })

  it('appends the document path under the project root', () => {
    expect(windowTitle('/Users/me/notes/', 'guide.md')).toBe(
      '1537paperstreet - /Users/me/notes/guide.md',
    )
  })
})
