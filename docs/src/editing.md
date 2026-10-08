# Editing and saving

Open a file in the tree. A tiny red dot at the top-left of a file icon means
the editor has changes that are not saved yet. Saving the file removes the
dot. Markdown is rendered; other UTF-8 files show as
highlighted source when a language is known, or as plain text. The title
bar shows what you can do with the document:

- **Preview** — reading (⌘E toggles from the editor)
- **Edit** — source with syntax highlighting (⌘E)
- **Split** — preview and source side by side (⌘⇧E). Typing updates the
  preview in place and keeps the edited text visible. Choosing a
  heading in the contents list scrolls the preview and the editor to that
  heading. Drag the divider to change editor width (stored as
  `window.editor_w`).
- **Save** — write the file (⌘S); the button is disabled until a file is open
  and writable
- **Format** — align Markdown tables or pretty-print JSON
- **Export** — PDF of the current document (⌘⌥E); the Save dialog picks the
  folder
- **Board** — Dashboard tab (local counts; no telemetry)
- **AI** — Assistant tab (⌘⌥A)

Top-right of the preview — a small **Full size** button: preview fills the
window (under the title bar). Click again or press Escape to return. Beside
it, a contents button shows or hides the table of contents. Pressed in, the
list is visible; raised, it is hidden. The choice is saved as
`viewer.show_toc`, the same setting as Settings → Show table of contents.
In Split, choosing a heading in that list scrolls the preview to the heading
and the editor to the same place in the source, with the cursor focused at the
start of the heading's line. Live updates follow the editor's active source
line when `editor.sync_scroll` is enabled (the default); an edit that is already
visible does not move the preview.

In Edit, Text / Links / Media buttons appear for Markdown files (bold, italic,
link, wiki link `[[Note]]`, task item `- [ ]`, heading, and so on). Source
files hide those commands.

Preview renders GitHub Flavored Markdown: tables, strikethrough, task lists,
footnotes, alerts (`> [!NOTE]`), definition lists. Click a task item to
switch it between `[ ]` and `[x]` in the file. Tables fill the reading
column with even column widths; long cell text wraps. YAML front matter at the
top of the file (`---` … `---`) shows as a key/value card, not as horizontal
rules. `[[Note]]` and `[[Note|label]]` open `Note.md` in the project (next to
the document or at the root); a wiki target that already names a file, such as
`[[src/main.rs]]`, opens that file. Missing links are drawn dashed. Click a
link to another file in the project to open it in a new tab at that heading
when the link has one. The file you were reading stays at the same place
when you come back to its tab. A web address opens in the browser and leaves
the reading pane where it was. Markdown stays in
Preview. A source file (`.rb`, `.js`, `.ex`, and the other editor languages)
opens in the code editor with syntax highlighting. An image (PNG, JPEG, GIF,
WebP, SVG, BMP, ICO, AVIF, or TIFF) opens in Preview; Edit and Split stay
off, and the file is not written back as text. Fenced code is
highlighted (Rust, Python, Ruby, Elixir, YAML, JS/TS, and others) with
quiet colors mixed into the body text. Each code block and quote has a Copy
button in its top-right corner.

`write_doc` restores BOM, line endings, and a trailing newline atomically,
skips the write when the encoded bytes already match disk, and does not
overwrite when `base_hash` disagrees with the file. Open and save with no
edits — the file is byte-for-byte the same.

The editor loads CodeMirror the first time you leave Preview or apply a highlight. Markdown markup
marks (`#`, `*`, `` ` ``) are faded; headings, emphasis, and links stay readable.
Other files use a language highlighter when one is available. JavaScript,
TypeScript, Go, Rust, Java, and PHP underline syntax errors; JSON shows parse
errors; otherwise they edit as plain text in the monospace font.
Settings → File formats lists the first-class source languages.
Save with ⌘S or Save. Format aligns Markdown tables or pretty-prints JSON.
Renaming the file in the tree does not reload it from
disk, so unsaved edits stay in the editor. An external editor is still
available (⌘⇧O). If that editor (or another program) saves the open file, a
dialog asks whether to reload it.

## Text highlights

Select text in Preview or the reading side of Split. A small palette offers
red, orange, yellow, green, blue, purple, and the theme's default marker color.
Enter a three- or six-digit HTML hex color (`#fc0` or `#ffcc00`), or use the
color picker, then choose **Apply**. Custom colors use black or white text for
contrast. Escape closes the palette; right-click a selection to focus its
controls and navigate them with Tab, Enter, or Space.

Click a highlight, or select part of it, to recolor or remove the whole marker.
New highlights preserve bold, italic and explicit link labels. Each paragraph
or list item uses one marker, including source line breaks and inline formatting.
To join an older fragmented highlight, select its text and apply a color again.
Code, equations,
images, generated content, wiki links and automatic/shortcut links cannot be
highlighted. If the preview is stale, select the text again after it refreshes.

Changes stay in the editor buffer until **Save** / ⌘S. Split updates the source
immediately; Preview also reflects unsaved changes. Undo and redo work with
⌘Z / ⌘⇧Z (Ctrl on Windows/Linux). Switching modes retains the current document's
undo history; switching documents starts a new history.

```markdown
==Default highlight==
==🟢Green highlight==
=={#ffcc00}Custom hex highlight==
```

The default and emoji presets follow [Obsidian's documented highlight syntax](https://obsidian.md/help/syntax#Highlight%20colors).
The hex form is a 1537paperstreet extension; three-digit values entered through
the palette are saved as six digits. Other Markdown viewers may show the
delimiters or color metadata literally. No plugin or sidecar file is required.
