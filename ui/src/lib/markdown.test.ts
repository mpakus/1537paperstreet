import { describe, expect, it } from 'vitest'

import {
  applyMarkdownCommand,
  toggleHeading,
  toggleLinePrefix,
  toggleTaskAt,
  wrapInline,
} from './markdown'

describe('wrapInline', () => {
  it('wraps the selection and places the caret on the inner text', () => {
    expect(wrapInline('hello', 0, 5, '**', '**')).toEqual({
      text: '**hello**',
      start: 2,
      end: 7,
    })
  })

  it('unwraps marks that already surround the selection', () => {
    expect(wrapInline('**hello**', 2, 7, '**', '**')).toEqual({
      text: 'hello',
      start: 0,
      end: 5,
    })
  })
})

describe('toggleLinePrefix', () => {
  it('prefixes selected lines and strips the same prefix on a second call', () => {
    const listed = toggleLinePrefix('one\ntwo', 0, 7, '- ')
    expect(listed.text).toBe('- one\n- two')
    expect(
      toggleLinePrefix(listed.text, listed.start, listed.end, '- ').text,
    ).toBe('one\ntwo')
  })
})

describe('toggleHeading', () => {
  it('sets and clears an ATX heading', () => {
    const headed = toggleHeading('Title', 0, 5, 2)
    expect(headed.text).toBe('## Title')
    expect(toggleHeading(headed.text, 0, headed.text.length, 2).text).toBe(
      'Title',
    )
  })
})

describe('applyMarkdownCommand', () => {
  it('inserts a Markdown image snippet', () => {
    expect(applyMarkdownCommand('cat', 0, 3, 'edit-image')).toEqual({
      text: '![cat]()',
      start: 2,
      end: 5,
    })
  })

  it('cycles a GFM task list prefix', () => {
    const open = applyMarkdownCommand('ship', 0, 4, 'edit-task')
    expect(open.text).toBe('- [ ] ship')
    const checked = applyMarkdownCommand(
      open.text,
      0,
      open.text.length,
      'edit-task',
    )
    expect(checked.text).toBe('- [x] ship')
    const cleared = applyMarkdownCommand(
      checked.text,
      0,
      checked.text.length,
      'edit-task',
    )
    expect(cleared.text).toBe('ship')
  })

  it('toggles a task marker at a UTF-8 byte offset', () => {
    const open = '- [ ] Open'
    const checked = toggleTaskAt(open, open.indexOf('['))
    expect(checked).toBe('- [x] Open')
    expect(toggleTaskAt(checked ?? '', 2)).toBe('- [ ] Open')
    expect(toggleTaskAt('- [X] Done', 2)).toBe('- [ ] Done')
    const prefixed = 'café\n- [ ] item'
    const bytes = new TextEncoder().encode(prefixed)
    const marker = new TextEncoder().encode('[ ]')
    const offset = bytes.findIndex((_, index) =>
      marker.every((byte, markerIndex) => bytes[index + markerIndex] === byte),
    )
    expect(toggleTaskAt(prefixed, offset)).toBe('café\n- [x] item')
    expect(toggleTaskAt(open, 0)).toBeNull()
  })

  it('wraps a wiki link', () => {
    expect(applyMarkdownCommand('Guide', 0, 5, 'edit-wiki-link')).toEqual({
      text: '[[Guide]]',
      start: 2,
      end: 7,
    })
  })

  it('leaves unknown commands unchanged', () => {
    expect(applyMarkdownCommand('note', 0, 4, 'nope')).toEqual({
      text: 'note',
      start: 0,
      end: 4,
    })
  })
})
