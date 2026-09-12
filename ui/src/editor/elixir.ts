import { StreamLanguage } from '@codemirror/language'

/** Keywords colored by the lightweight Elixir stream mode. */
export const elixirKeywords = [
  'after',
  'alias',
  'and',
  'case',
  'catch',
  'cond',
  'def',
  'defdelegate',
  'defexception',
  'defguard',
  'defguardp',
  'defimpl',
  'defmacro',
  'defmacrop',
  'defmodule',
  'defp',
  'defprotocol',
  'defstruct',
  'do',
  'else',
  'end',
  'fn',
  'for',
  'if',
  'import',
  'in',
  'not',
  'or',
  'quote',
  'raise',
  'receive',
  'require',
  'rescue',
  'super',
  'then',
  'try',
  'unless',
  'unquote',
  'unquote_splicing',
  'use',
  'when',
  'with',
] as const

const KEYWORDS = new Set<string>(elixirKeywords)

/** Stream language for `.ex` / `.exs` (no official CodeMirror Elixir pack). */
export const elixirLanguage = StreamLanguage.define({
  name: 'elixir',
  token(stream) {
    if (stream.eatSpace()) {
      return null
    }
    if (stream.match('#')) {
      stream.skipToEnd()
      return 'comment'
    }
    if (stream.match('"""') || stream.match("'''")) {
      stream.skipToEnd()
      return 'string'
    }
    if (
      stream.match(/^"(?:[^"\\]|\\.)*"/) ||
      stream.match(/^'(?:[^'\\]|\\.)*'/)
    ) {
      return 'string'
    }
    if (stream.match(/^@[A-Za-z_]\w*/)) {
      return 'meta'
    }
    if (
      stream.match(/^:[A-Za-z_]\w*[?!]?/) ||
      stream.match(/^[A-Za-z_]\w*[?!]?:/)
    ) {
      return 'atom'
    }
    if (stream.match(/^\d[\d_]*(?:\.\d[\d_]*)?/)) {
      return 'number'
    }
    if (stream.match(/^[A-Z][\w.]*/)) {
      return 'type'
    }
    if (stream.match(/^[a-z_]\w*[?!]?/)) {
      const word = stream.current()
      if (word === 'true' || word === 'false' || word === 'nil') {
        return 'atom'
      }
      if (KEYWORDS.has(word)) {
        return 'keyword'
      }
      return 'variable'
    }
    stream.next()
    return null
  },
})
