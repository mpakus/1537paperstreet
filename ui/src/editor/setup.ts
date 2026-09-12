import { jsonParseLinter } from '@codemirror/lang-json'
import { indentUnit, syntaxHighlighting } from '@codemirror/language'
import { linter } from '@codemirror/lint'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { Compartment, EditorState, type Extension } from '@codemirror/state'
import {
  drawSelection,
  dropCursor,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from '@codemirror/view'

import { isMarkdownPath } from '../lib/tree'
import { languageSupport } from './catalog'
import { codeHighlightStyle, markdownHighlightStyle } from './highlight'
import { usesJsonLinter, usesSyntaxLinter } from './language'
import { syntaxErrors } from './lint'
import type { MarkdownEditor, MarkdownEditorOptions } from './types'

export type { MarkdownEditor, MarkdownEditorOptions }

const editorTheme = EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: 'var(--bg)',
    color: 'var(--fg)',
    fontFamily: 'var(--font-body)',
    fontSize: 'var(--font-size)',
    lineHeight: 'var(--line-height)',
  },
  '&.cm-focused': {
    outline: 'none',
  },
  '.cm-scroller': {
    fontFamily: 'var(--font-body)',
    lineHeight: 'inherit',
  },
  '.cm-content': {
    caretColor: 'var(--ed-cursor)',
    padding: 'var(--space-6) var(--space-4)',
  },
  '.cm-line': {
    padding: '0',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: 'var(--ed-cursor)',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection':
    {
      backgroundColor: 'var(--ed-sel)',
    },
  '.cm-activeLine': {
    backgroundColor: 'var(--ed-active-line)',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--bg)',
    color: 'var(--ed-syntax)',
    border: 'none',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'var(--ed-active-line)',
    color: 'var(--fg-muted)',
  },
})

function clamp(value: number, max: number): number {
  return Math.max(0, Math.min(max, value))
}

function indentExtension(spaces: number) {
  const count = Math.max(1, Math.min(8, Math.round(spaces) || 2))
  return [EditorState.tabSize.of(count), indentUnit.of(' '.repeat(count))]
}

function lineNumberExtension(on: boolean) {
  return on ? [lineNumbers(), highlightActiveLineGutter()] : []
}

function wrapExtension(on: boolean) {
  return on ? EditorView.lineWrapping : []
}

function spellcheckAttributes(on: boolean, fileName: string) {
  return EditorView.contentAttributes.of({
    'aria-label': isMarkdownPath(fileName) ? 'Markdown source' : 'Source',
    spellcheck: on ? 'true' : 'false',
  })
}

function highlightExtension(fileName: string): Extension {
  const style = isMarkdownPath(fileName)
    ? markdownHighlightStyle
    : codeHighlightStyle
  return syntaxHighlighting(style, { fallback: false })
}

function lintExtension(fileName: string): Extension {
  if (usesJsonLinter(fileName)) {
    return linter(jsonParseLinter())
  }
  if (usesSyntaxLinter(fileName)) {
    return linter((view) => syntaxErrors(view.state))
  }
  return []
}

/**
 * Creates a CodeMirror source editor. Call only after a dynamic import so
 * Preview-only sessions never load the editor chunk.
 */
export function createMarkdownEditor(
  parent: HTMLElement,
  options: MarkdownEditorOptions,
): MarkdownEditor {
  const writable = new Compartment()
  const numbers = new Compartment()
  const wrap = new Compartment()
  const indent = new Compartment()
  const attrs = new Compartment()
  const language = new Compartment()
  const highlighting = new Compartment()
  const lint = new Compartment()
  let currentFileName = options.fileName ?? ''
  let spellcheckOn = options.spellcheck
  let languageGen = 0

  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc: options.doc,
      extensions: [
        history(),
        drawSelection(),
        dropCursor(),
        highlightActiveLine(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        language.of([]),
        highlighting.of(highlightExtension(currentFileName)),
        lint.of(lintExtension(currentFileName)),
        editorTheme,
        writable.of(EditorState.readOnly.of(!options.writable)),
        numbers.of(lineNumberExtension(options.lineNumbers)),
        wrap.of(wrapExtension(options.softWrap)),
        indent.of(indentExtension(options.indentUnit)),
        attrs.of(spellcheckAttributes(spellcheckOn, currentFileName)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            options.onChange(update.state.doc.toString())
          }
        }),
      ],
    }),
  })

  function setDoc(text: string) {
    if (view.state.doc.toString() === text) {
      return
    }
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: text },
    })
  }

  function applyFileName(fileName: string) {
    currentFileName = fileName
    const gen = ++languageGen
    view.dispatch({
      effects: [
        highlighting.reconfigure(highlightExtension(fileName)),
        lint.reconfigure(lintExtension(fileName)),
        attrs.reconfigure(spellcheckAttributes(spellcheckOn, fileName)),
      ],
    })
    void languageSupport(fileName).then((support) => {
      if (gen !== languageGen) {
        return
      }
      view.dispatch({
        effects: language.reconfigure(support),
      })
    })
  }

  applyFileName(currentFileName)

  return {
    setDoc,
    setFileName(next) {
      if (next === currentFileName) {
        return
      }
      applyFileName(next)
    },
    setWritable(on) {
      view.dispatch({
        effects: writable.reconfigure(EditorState.readOnly.of(!on)),
      })
    },
    setSpellcheck(on) {
      spellcheckOn = on
      view.dispatch({
        effects: attrs.reconfigure(spellcheckAttributes(on, currentFileName)),
      })
    },
    setLineNumbers(on) {
      view.dispatch({
        effects: numbers.reconfigure(lineNumberExtension(on)),
      })
    },
    setSoftWrap(on) {
      view.dispatch({
        effects: wrap.reconfigure(wrapExtension(on)),
      })
    },
    setIndentUnit(spaces) {
      view.dispatch({
        effects: indent.reconfigure(indentExtension(spaces)),
      })
    },
    selection() {
      const range = view.state.selection.main
      return { start: range.from, end: range.to }
    },
    setTextAndSelection(text, start, end) {
      const length = text.length
      const from = clamp(start, length)
      const to = clamp(end, length)
      const current = view.state.doc.toString()
      view.dispatch({
        ...(current === text
          ? {}
          : { changes: { from: 0, to: view.state.doc.length, insert: text } }),
        selection: { anchor: from, head: to },
        scrollIntoView: true,
      })
    },
    focus() {
      view.focus()
    },
    refresh() {
      view.requestMeasure()
    },
    destroy() {
      languageGen += 1
      view.destroy()
    },
  }
}
