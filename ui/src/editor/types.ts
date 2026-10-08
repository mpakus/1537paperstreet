/** Options for the lazily created source editor. */
export type MarkdownEditorOptions = {
  doc: string
  fileName?: string
  writable: boolean
  spellcheck: boolean
  lineNumbers: boolean
  softWrap: boolean
  indentUnit: number
  onChange: (text: string) => void
}

/** Imperative handle used by the toolbar and formatting commands. */
export type MarkdownEditor = {
  setDoc: (text: string, isolated?: boolean) => void
  /** Undo/redo also serve formatting actions made from Preview. */
  undo: () => boolean
  redo: () => boolean
  setFileName: (fileName: string) => void
  setWritable: (writable: boolean) => void
  setSpellcheck: (on: boolean) => void
  setLineNumbers: (on: boolean) => void
  setSoftWrap: (on: boolean) => void
  setIndentUnit: (spaces: number) => void
  selection: () => { start: number; end: number }
  /** Active cursor line and document length; null while focus is elsewhere. */
  activeSourceLine: () => { line: number; lines: number } | null
  setTextAndSelection: (text: string, start: number, end: number) => void
  /** Focuses the cursor at the start of a one-based line and scrolls it to the top. */
  scrollToLine: (line: number) => void
  focus: () => void
  refresh: () => void
  destroy: () => void
}
