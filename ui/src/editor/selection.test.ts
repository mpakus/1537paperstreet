// @vitest-environment happy-dom
import { expect, it } from 'vitest'
import { createMarkdownEditor } from './setup'

it('signals forward and backward selections, and clears that state for a cursor', () => {
  const host = document.createElement('div')
  document.body.append(host)
  const doc = 'first line\nsecond line'
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
  try {
    for (const [from, to] of [
      [0, doc.length],
      [doc.length, 0],
    ]) {
      editor.setTextAndSelection(doc, from, to)
      expect(editor.selection()).toEqual({ start: 0, end: doc.length })
      expect(
        host.querySelector('.cm-editor')?.hasAttribute('data-selecting'),
      ).toBe(true)
    }
    editor.setTextAndSelection(doc, 0, 0)
    expect(
      host.querySelector('.cm-editor')?.hasAttribute('data-selecting'),
    ).toBe(false)
  } finally {
    editor.destroy()
    host.remove()
  }
})
