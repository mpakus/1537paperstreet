// @vitest-environment happy-dom

import { describe, expect, it } from 'vitest'

import { createMarkdownEditor } from './setup'

describe('scrollToLine', () => {
  it('places the cursor at the start of the requested line', () => {
    const host = document.createElement('div')
    document.body.append(host)
    const doc = '# One\n\n## Two\n'
    const editor = createMarkdownEditor(host, {
      doc,
      fileName: 'note.md',
      writable: true,
      spellcheck: false,
      lineNumbers: false,
      softWrap: true,
      indentUnit: 2,
      onChange() {},
    })

    editor.scrollToLine(3)

    expect(editor.selection()).toEqual({
      start: doc.indexOf('##'),
      end: doc.indexOf('##'),
    })
    editor.scrollToLine(99)
    expect(editor.selection().start).toBe(doc.length)
    editor.destroy()
    host.remove()
  })
})
