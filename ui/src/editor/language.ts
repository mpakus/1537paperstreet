import { isMarkdownPath } from '../lib/tree'

/** Last path segment of a project-relative path. */
export function fileBasename(relPath: string): string {
  return relPath.split(/[/\\]/).pop() ?? relPath
}

/** Filename used to look up a CodeMirror grammar. */
export function editorFileName(relPath: string): string {
  const base = fileBasename(relPath)
  if (/\.rsx$/i.test(base)) {
    return base.replace(/rsx$/i, 'jsx')
  }
  return base
}

/** True when the editor should use a monospace source theme. */
export function usesCodeEditor(relPath: string): boolean {
  return relPath.length > 0 && !isMarkdownPath(relPath)
}

/** True when a parse linter is available for this file. */
export function usesJsonLinter(relPath: string): boolean {
  return /\.json$/i.test(fileBasename(relPath))
}

/** True when a Lezer parser can underline syntax errors. */
export function usesSyntaxLinter(relPath: string): boolean {
  return /\.(js|mjs|cjs|jsx|rsx|ts|mts|cts|tsx|go|rs|java|php|php[3-7]|phtml)$/i.test(
    fileBasename(relPath),
  )
}
