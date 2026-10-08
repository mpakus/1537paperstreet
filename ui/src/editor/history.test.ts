// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { createMarkdownEditor } from './setup'

describe('preview marker history', () => {
  it('undoes each highlight separately without losing preceding unsaved text', () => {
    const host = document.createElement('div')
    document.body.append(host)
    let text = 'saved'
    const editor = createMarkdownEditor(host, {
      doc: text,
      fileName: 'note.md',
      writable: true,
      spellcheck: false,
      lineNumbers: false,
      softWrap: true,
      indentUnit: 2,
      onChange: (next) => {
        text = next
      },
    })
    editor.setDoc('saved and unsaved')
    editor.setDoc('saved and ==unsaved==', true)
    editor.setDoc('==saved== and ==unsaved==', true)
    expect(editor.undo()).toBe(true)
    expect(text).toBe('saved and ==unsaved==')
    expect(editor.undo()).toBe(true)
    expect(text).toBe('saved and unsaved')
    expect(editor.redo()).toBe(true)
    expect(text).toBe('saved and ==unsaved==')
    editor.destroy()
    host.remove()
  })
})
