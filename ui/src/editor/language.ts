import { isConfigPath, isJsonPath, isMarkdownPath } from '../lib/tree'

/** Last path segment of a project-relative path. */
export function fileBasename(relPath: string): string {
  return relPath.split(/[/\\]/).pop() ?? relPath
}

/** Filename used to look up a CodeMirror grammar. */
export function editorFileName(relPath: string): string {
  const base = fileBasename(relPath).replace(
    /\.(md|markdown|mdown|mdwn)\.age$/i,
    '.$1',
  )
  if (/\.rsx$/i.test(base)) {
    return base.replace(/rsx$/i, 'jsx')
  }
  if (isConfigPath(base))
    return base.replace(/\.[^.]+$/, (ext) => ext.toLowerCase())
  return base
}

/** True when the editor should use a monospace source theme. */
export function usesCodeEditor(relPath: string): boolean {
  return relPath.length > 0 && !isMarkdownPath(relPath)
}

/** True when a parse linter is available for this file. */
export function usesJsonLinter(relPath: string): boolean {
  return isJsonPath(fileBasename(relPath))
}

/** True when the Rust configuration parser provides lint diagnostics. */
export function usesConfigLinter(relPath: string): boolean {
  return isConfigPath(relPath) && !usesJsonLinter(relPath)
}

/** True when a Lezer parser can underline syntax errors. */
export function usesSyntaxLinter(relPath: string): boolean {
  return /\.(js|mjs|cjs|jsx|rsx|ts|mts|cts|tsx|go|rs|java|php|php[3-7]|phtml)$/i.test(
    fileBasename(relPath),
  )
}
