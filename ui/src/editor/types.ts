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
  setDoc: (text: string) => void
  setFileName: (fileName: string) => void
  setWritable: (writable: boolean) => void
  setSpellcheck: (on: boolean) => void
  setLineNumbers: (on: boolean) => void
  setSoftWrap: (on: boolean) => void
  setIndentUnit: (spaces: number) => void
  selection: () => { start: number; end: number }
  setTextAndSelection: (text: string, start: number, end: number) => void
  /** Moves the cursor to a one-based source line and scrolls it to the top. */
  scrollToLine: (line: number) => void
  focus: () => void
  refresh: () => void
  destroy: () => void
}
