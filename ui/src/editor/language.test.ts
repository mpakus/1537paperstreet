import { describe, expect, it } from 'vitest'

import { isEditablePath, isMarkdownPath, isSourcePath } from '../lib/tree'
import { matchedLanguageName } from './catalog'
import {
  editorFileName,
  usesCodeEditor,
  usesJsonLinter,
  usesSyntaxLinter,
} from './language'

describe('editor language from file name', () => {
  it('opens any file and colors languages that have a grammar', () => {
    expect(isEditablePath('LICENSE')).toBe(true)
    expect(isEditablePath('notes.txt')).toBe(true)
    expect(isEditablePath('cover.png')).toBe(true)
    expect(isSourcePath('notes.txt')).toBe(true)
    expect(isSourcePath('readme.md')).toBe(false)

    expect(matchedLanguageName('notes.md')).toBe('markdown')
    expect(matchedLanguageName('app.js')).toBe('JavaScript')
    expect(matchedLanguageName('main.ts')).toBe('TypeScript')
    expect(matchedLanguageName('Widget.rsx')).toBe('JSX')
    expect(matchedLanguageName('config.json')).toBeTruthy()
    expect(matchedLanguageName('lib.rs')).toBe('Rust')
    expect(matchedLanguageName('main.go')).toBe('Go')
    expect(matchedLanguageName('Program.cs')).toBe('C#')
    expect(matchedLanguageName('Main.java')).toBe('Java')
    expect(matchedLanguageName('index.php')).toBe('PHP')
    expect(matchedLanguageName('gem.rb')).toBe('Ruby')
    expect(matchedLanguageName('mix.exs')).toBe('elixir')
    expect(matchedLanguageName('theme.css')).toBeTruthy()
    expect(matchedLanguageName('index.html')).toBeTruthy()
    expect(matchedLanguageName('hi.py')).toBeTruthy()
    expect(matchedLanguageName('notes.txt')).toBeNull()
    expect(matchedLanguageName('weird.unknownext123')).toBeNull()
  })

  it('maps RSX onto JSX and keeps Markdown on the body font', () => {
    expect(editorFileName('src/Widget.rsx')).toBe('Widget.jsx')
    expect(usesCodeEditor('lib.rs')).toBe(true)
    expect(usesCodeEditor('notes.txt')).toBe(true)
    expect(usesCodeEditor('notes.md')).toBe(false)
    expect(isMarkdownPath('notes.md')).toBe(true)
    expect(usesJsonLinter('config.json')).toBe(true)
    expect(usesJsonLinter('lib.rs')).toBe(false)
    expect(usesSyntaxLinter('app.js')).toBe(true)
    expect(usesSyntaxLinter('main.ts')).toBe(true)
    expect(usesSyntaxLinter('main.go')).toBe(true)
    expect(usesSyntaxLinter('lib.rs')).toBe(true)
    expect(usesSyntaxLinter('Main.java')).toBe(true)
    expect(usesSyntaxLinter('index.php')).toBe(true)
    expect(usesSyntaxLinter('gem.rb')).toBe(false)
    expect(usesSyntaxLinter('mix.exs')).toBe(false)
    expect(usesSyntaxLinter('Program.cs')).toBe(false)
  })
})
