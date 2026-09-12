import { syntaxTree } from '@codemirror/language'
import type { Diagnostic } from '@codemirror/lint'
import type { EditorState } from '@codemirror/state'

const MAX_ERRORS = 20

/** Parse errors from a Lezer tree (JS, TS, Go, Rust, Java, PHP). */
export function syntaxErrors(state: EditorState): Diagnostic[] {
  const diagnostics: Diagnostic[] = []
  const length = state.doc.length
  syntaxTree(state).iterate({
    enter(node) {
      if (diagnostics.length >= MAX_ERRORS) {
        return false
      }
      if (!node.type.isError) {
        return
      }
      const from = node.from
      const to = node.to > from ? node.to : Math.min(from + 1, length)
      diagnostics.push({
        from,
        to,
        severity: 'error',
        message: 'Syntax error',
      })
    },
  })
  return diagnostics
}
