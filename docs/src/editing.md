# Editing and saving

Open a file in the tree. Markdown is rendered; other UTF-8 files show as
highlighted source when a language is known, or as plain text. The title
bar shows what you can do with the document:

- **Preview** — reading (⌘E toggles from the editor)
- **Edit** — source with syntax highlighting (⌘E)
- **Split** — preview and source side by side (⌘⇧E); drag the divider to
  change editor width (stored as `window.editor_w`).
- **Save** — write the file (⌘S); the button is disabled until a file is open
  and writable
- **Export** — PDF of the current document (⌘⌥E); the Save dialog picks the
  folder
- **Board** — Dashboard tab (local counts; no telemetry)
- **AI** — Assistant tab (⌘⌥A)

Top-right of the preview — a small **Full size** button: preview fills the
window (under the title bar). Click again or press Escape to return.

In Edit, Text / Links / Media buttons appear for Markdown files (bold, italic,
link, wiki link `[[Note]]`, task item `- [ ]`, heading, and so on). Source
files hide those commands.

Preview renders GitHub Flavored Markdown: tables, strikethrough, task lists,
footnotes, alerts (`> [!NOTE]`), definition lists. YAML front matter at the
top of the file (`---` … `---`) shows as a key/value card, not as horizontal
rules. `[[Note]]` and `[[Note|label]]` open `Note.md` in the project (next to
the document or at the root); missing links are drawn dashed. Fenced code is
highlighted (Rust, Python, Ruby, Elixir, YAML, JS/TS, and others) with
quiet colors mixed into the body text.

`write_doc` restores BOM, line endings, and a trailing newline atomically,
skips the write when the encoded bytes already match disk, and does not
overwrite when `base_hash` disagrees with the file. Open and save with no
edits — the file is byte-for-byte the same.

The editor loads CodeMirror the first time you leave Preview. Markdown markup
marks (`#`, `*`, `` ` ``) are faded; headings, emphasis, and links stay readable.
Other files use a language highlighter when one is available. JavaScript,
TypeScript, Go, Rust, Java, and PHP underline syntax errors; JSON shows parse
errors; otherwise they edit as plain text in the monospace font.
Settings → File formats lists the first-class source languages.
Save with ⌘S or Save. An external editor is still available (⌘⇧O). If that
editor (or another program) saves the open file, a dialog asks whether to
reload it.
