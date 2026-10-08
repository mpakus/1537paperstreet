import { describe, expect, it } from 'vitest'
import { markdown } from '@codemirror/lang-markdown'
import { syntaxTree } from '@codemirror/language'
import { EditorState } from '@codemirror/state'
import { highlightTree } from '@lezer/highlight'

import {
  codeHighlightStyle,
  codeTokenClasses,
  markdownHighlightStyle,
  markdownTokenClasses,
} from './highlight'

describe('markdown editor highlighting', () => {
  it('dims list punctuation without dimming the content or marker text', () => {
    const doc = '- ==🟢first line\n  second line==\n- normal list text'
    const state = EditorState.create({ doc, extensions: [markdown()] })
    let muted = ''
    highlightTree(
      syntaxTree(state),
      markdownHighlightStyle,
      (from, to, classes) => {
        if (classes.includes('cm-md-mark')) muted += doc.slice(from, to)
      },
    )
    expect(muted).toBe('--')
  })

  it('maps Markdown tokens onto theme CSS classes', () => {
    expect(markdownTokenClasses).toEqual([
      'cm-md-heading',
      'cm-md-em',
      'cm-md-strong',
      'cm-md-strike',
      'cm-md-link',
      'cm-md-code',
      'cm-md-quote',
      'cm-md-comment',
      'cm-md-mark',
    ])
    const classes = markdownHighlightStyle.specs.map((spec) => spec.class)
    for (const name of markdownTokenClasses) {
      expect(classes).toContain(name)
    }
  })
})

describe('source editor highlighting', () => {
  it('maps language tokens onto theme CSS classes', () => {
    expect(codeTokenClasses).toEqual([
      'cm-code-kw',
      'cm-code-str',
      'cm-code-num',
      'cm-code-com',
      'cm-code-fn',
      'cm-code-type',
      'cm-code-prop',
      'cm-code-name',
      'cm-code-punct',
    ])
    const classes = codeHighlightStyle.specs.map((spec) => spec.class)
    for (const name of codeTokenClasses) {
      expect(classes).toContain(name)
    }
  })
})
