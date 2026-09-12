import { LanguageDescription } from '@codemirror/language'
import { languages } from '@codemirror/language-data'
import { EditorState } from '@codemirror/state'
import { describe, expect, it } from 'vitest'

import { syntaxErrors } from './lint'

describe('syntax error linter', () => {
  it('reports nothing without a language parser', () => {
    const state = EditorState.create({ doc: 'const n =' })
    expect(syntaxErrors(state)).toEqual([])
  })

  it('flags broken JavaScript', async () => {
    const desc = LanguageDescription.matchFilename(languages, 'app.js')
    expect(desc).toBeTruthy()
    if (!desc) {
      return
    }
    const support = await desc.load()
    const state = EditorState.create({
      doc: 'const n =',
      extensions: [support],
    })
    expect(syntaxErrors(state).length).toBeGreaterThan(0)
  })
})
