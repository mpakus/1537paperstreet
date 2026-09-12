import { markdown } from '@codemirror/lang-markdown'
import { LanguageDescription } from '@codemirror/language'
import { languages } from '@codemirror/language-data'
import type { Extension } from '@codemirror/state'

import { isMarkdownPath } from '../lib/tree'
import { elixirLanguage } from './elixir'
import { editorFileName } from './language'

/** CodeMirror / Elixir language name for `relPath`, or null for plain text. */
export function matchedLanguageName(relPath: string): string | null {
  if (!relPath) {
    return null
  }
  if (isMarkdownPath(relPath)) {
    return 'markdown'
  }
  const fileName = editorFileName(relPath)
  if (/\.(ex|exs)$/i.test(fileName)) {
    return 'elixir'
  }
  return LanguageDescription.matchFilename(languages, fileName)?.name ?? null
}

/** Language support for `relPath`, or an empty extension for plain text. */
export async function languageSupport(relPath: string): Promise<Extension> {
  if (isMarkdownPath(relPath)) {
    return markdown({ addKeymap: true })
  }
  const fileName = editorFileName(relPath)
  if (/\.(ex|exs)$/i.test(fileName)) {
    return elixirLanguage
  }
  const desc = LanguageDescription.matchFilename(languages, fileName)
  if (!desc) {
    return []
  }
  return desc.load()
}
