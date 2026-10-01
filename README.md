# 1537paperstreet

**Your Markdown. Your folders. A quieter way to work.**

1537paperstreet turns ordinary folders of Markdown into a focused workspace
on macOS, Linux, and Windows. Browse a project, read beautifully rendered
documents, edit the source, preview diagrams and math, export a PDF, or work
with a local AI agent — without importing files into a vault or signing in to
a service.

Made in Austin ✩ Texas · [aomega.co](https://aomega.co)

[1537paperstreet.com](https://1537paperstreet.com) · [Download the latest release](https://github.com/mpakus/1537paperstreet/releases/latest)

![Preview of a Markdown document with Projects, file tree, table of contents, and rendered headings](docs/screen-preview.png)

## Plain-text knowledge, with a real workspace around it

Your notes, specifications, READMEs, and wikis already belong beside your
projects. 1537paperstreet makes those files comfortable to navigate and read
without changing where they live or how they are stored.

- **Open any folder.** There is no migration, database, or proprietary vault.
  Add an existing folder and keep using the same files with Git, your file
  manager, and every other tool you trust.
- **Read more than basic Markdown.** GitHub Flavored Markdown, syntax-highlighted
  code, tables, task lists, alerts, footnotes, wiki links, images, Mermaid
  diagrams, and KaTeX math render in a calm, themeable reading view.
- **Move naturally between reading and writing.** Preview, Edit, and Split modes
  share one window. Saves are atomic, preserve file characteristics, and refuse
  to overwrite a file changed elsewhere.
- **Find the document, not the app feature.** Projects, a virtualized file tree,
  tabs, quick open, table of contents, and full-size reading keep large folders
  manageable.
- **Use AI on your terms.** Connect a local ACP-compatible CLI such as Codex,
  Claude, or OpenCode. The Assistant works in the open project; the app does not
  send documents to a model service itself.
- **Stay local by default.** Reading, editing, themes, Mermaid, KaTeX, search,
  and the Dashboard run on this computer. There is no account, cloud library,
  or telemetry.

## Why it exists

Most Markdown tools want a vault format, a sync service, or a browser tab.
1537paperstreet is a native window over ordinary folders: notes, specs, READMEs,
and wikis that already live next to your code.

- **Your files stay yours.** Projects are paths on disk. Removing a project from
  the list does not delete the folder.
- **Read first.** Preview GitHub Flavored Markdown with a table of contents,
  tabs, and a full-size reading view. Edit the source when you need to; save
  with ⌘S.
- **Private by default.** Themes, Mermaid, and KaTeX are local. The app’s only
  optional network request is **Check for Updates**. External Agents you add
  in Settings are local CLIs; they may use the network under their own terms.

## What you can do

### Turn folders into workspaces

Keep several folders in the **Projects** list. Search filters by name and path.

Add a folder with Open Folder…, File → Open Folder…, File → Open File…
(registers the containing folder and opens the `.md`), or drop a folder or
Markdown file from Finder, Explorer, or the file manager onto the window.
Opening the same folder again reuses the existing entry.

If you moved a folder, the row turns dim; context menu → Find Folder… points
the record at the new location. Remove from List only drops the list entry.

⌘⇧P switches project. ⌘1 hides the Projects list; a slim strip remains so
you can open it again.

### Navigate and organize

The tree loads one directory level at a time. Expanded folders and pane widths
persist. Icons distinguish folders, Markdown (`.md`, `.markdown`, `.mdown`,
`.mdwn`), and other files. A single click opens a file in a preview tab that
the next single click replaces. Double-click pins the tab. A single click on
a folder expands it. A red dot at the top-left of a file icon means the editor
has changes that are not saved yet. UTF-8 text is editable, with highlighting
when a language mode exists. Images open in Preview and are not saved as text.
Folders such as `.docs` and `.git` are always listed. ⌘2 hides the tree. ⌘P
quick-opens a file in the current project. ⌘⇧F, the toolbar search icon, or
Search on a folder searches file contents (plain text or a regular expression;
a plain query also fuzzy-matches paths).

Context menu and File / Go:

| Action                  | How                                                    |
| ----------------------- | ------------------------------------------------------ |
| New File / New Folder   | ⌘N / ⌘⇧N in the selected folder                        |
| Refresh                 | context menu on a folder — reload that folder from disk |
| Search                  | context menu on a folder, or ⌘⇧F                       |
| Rename                  | click the selected name again, F2, or the context menu |
| Duplicate               | copy beside the original                               |
| Copy to… / Move to…     | pick a destination folder                              |
| Reveal                  | ⌘⇧R — Reveal in Finder, Show in Explorer, or Show in Files |
| Open in External Editor | ⌘⇧O                                                    |
| Move to Trash           | ⌘⌫, with confirmation                                  |

Drag inside the tree to move; hold ⌥ to copy. Hovering a folder expands it.
Drop from the file manager into a tree folder copies into the project. Drag a file onto
another project in the Projects list to move it there; hold ⌥ to copy.

Name clashes offer Replace, Keep Both, or Skip (optionally apply to all).
⇧ and ⌘ select several nodes for group copy, move, duplicate, and trash.

Settings can hide dot-prefixed files and require confirmation before Trash.

### Read rich Markdown

Open files stay in tabs. Preview (⌘E from the editor) renders GitHub Flavored
Markdown:

- tables, strikethrough, task lists, footnotes
- alerts (`> [!NOTE]`), definition lists
- YAML front matter as a key/value card
- wiki links `[[Note]]` / `[[Note|label]]` to `Note.md` in the project
- fenced code (Rust, Python, Ruby, Elixir, YAML, JS/TS, and others); each code
  block and quote has a Copy button
- Mermaid diagrams and KaTeX math (`$...$` / `$$...$$`)
- images (PNG, JPEG, GIF, WebP, SVG, and the other common formats) in Preview

A line written `[ ] one` or `- [x] two` is a checkbox. Clicking it writes
`[x]` or `[ ]` back into the file. Split updates the page from the editor,
including text that is not saved yet.

The table of contents tracks headings. A button in the preview corner shows
or hides that list; pressed in, it is visible, and the choice is saved with
Settings → Show table of contents. A link to another project file opens a new
tab, at the heading when the link has one. The document you left stays where
it was. A web address opens in the browser and leaves the reading pane in
place. A source file (Ruby, JavaScript, Elixir, and the other editor
languages) opens in the code editor. Images reserve width and height from the
file header so layout does not jump.

A file that is too large (> 8 MB), binary, or missing shows a message instead
of a blank pane. ⌘F finds text in the preview and highlights matches as you
type; ⌘G / ⌘⇧G move to the next or previous hit.

Top-right of the preview — **Full size**: reading fills the window under the
title bar. Click again or Escape to return. The contents button sits beside it.

Each document tab remembers Preview, Edit, or Split.
The last folder, those tabs, Dashboard, and Assistant come back the next time
the app starts. A missing file is skipped, and a note is added to Message
history.

In Preview and Split, the bar has A− / A+ (reading size) and − / % / +
(session zoom, 50–200 %). ⌘+ / ⌘− / ⌘0 change chrome type size.

![Split: Markdown source with highlighting, outline, and live preview including a Mermaid diagram](docs/screen-edit-split.png)

### Edit without giving up file control

**Edit** (⌘E) and **Split** (⌘⇧E) show the source. Split places the
editor beside a live preview; drag the divider (width is remembered).

The editor loads CodeMirror the first time you leave Preview. Markup marks
(`#`, `*`, `` ` ``) are faded; headings, emphasis, links, and fenced code stay
readable. Other UTF-8 files use a language highlighter when one exists, or
plain text. JSON reports parse errors. Soft wrap and the current line are highlighted.

The toolbar (and Edit menu) applies Markdown formatting: bold, italic, inline code,
headings, lists, task items, quotes, links, wiki links, and images (⌘B, ⌘I,
⌘K, and heading shortcuts).

**Save** (⌘S) writes atomically and restores BOM, line endings, and a trailing
newline. The red dot on that file goes away. Open and save with no edits — the
file is byte-for-byte the same. If the file on disk no longer matches what was
opened, the write is refused instead of overwriting. Renaming a file that is
open keeps the unsaved text.

**Format** aligns Markdown tables or pretty-prints JSON. Find in the open page
is ⌘F. Find and replace is not available yet.

If another program (including Open in External Editor) saves the open file, a
dialog asks to **Reload** or **Keep this version**, in Preview and Edit.
Reload discards unsaved edits in this app.

### Make reading comfortable

⌘, (File → Settings…) has:

- twelve built-in themes (Paper, Solarized, Nord, Gruvbox, Catppuccin, Tokyo
  Night, GitHub — light and dark pairs)
- follow system appearance; ⌘⌥T flips light/dark for the session without
  rewriting the Settings pair
- body and mono fonts (faces already on this computer; missing faces fall
  through the stack), size, line height, and measure
- Preview & Split reading font, size, and optional custom colors (empty / `0`
  means the theme)
- on macOS, keep the Dock icon when the window is hidden
- table of contents, confirm Trash, show hidden files (dot folders such as
  `.docs` and `.git` are always listed)
- render Mermaid diagrams and KaTeX mathematics
- **External Agents** — add OpenCode, Claude, or Codex if they are on `PATH`
  (`codex` runs as `app-server --stdio`), or a custom ACP command

Custom themes are JSON in `~/.1537paperstreet/themes/`.

### Work with local AI agents

⌘⌥A, View → Assistant, the Assistant tab, or the toolbar **AI** button
(next to Board) opens the Assistant **tab**: Configure (Settings), agent,
model when the CLI lists one, permissions (Allowance / Plan / Full),
prompt history, and New chat. Shift+Enter, Ctrl+Enter, or ⌘Enter send;
plain Enter is a newline. While the agent is working, Send becomes **Stop**
with a spinner. Click a History prompt to reuse it; × removes that prompt;
**Clear History** deletes every stored prompt.

Codex is spawned as `codex app-server --stdio` (not `codex acp`). The
session uses the open project as its working directory. This app does not
implement ACP file methods; Full permission can still let the CLI change
files.

The **Dashboard** tab (View → Dashboard) is a local analytics page: project
and Markdown counts, configured agents, the live ACP session, and recent
prompts. It does not send telemetry.

![Settings: theme palettes, typography, preview colors, Dock, and render toggles](docs/screen-settings.png)

### Share finished documents

**Export** / File → Export PDF… / ⌘⌥E saves the open document as PDF: native
Save dialog, then a snapshot of the preview (theme, diagrams, images).

Click a Mermaid diagram to open its window: drag the title, resize from the
edges, zoom up to 3200%, pan, Copy SVG, Save PNG. Size, zoom, and position are
remembered. PDF export is on macOS; on Linux and Windows the command says so.

ZIP of a whole project is not in this version.

### Keep it ready, without keeping it in the way

On macOS, closing the red traffic light hides the window; the menu-bar icon
stays. Click it to show the window again. Quit from that menu or ⌘Q. Linux and
Windows use a normal window frame.

File → About 1537paperstreet shows version and a link to aomega.co.
File → Check for Updates… (same control in About) compares your version to
GitHub Releases, downloads the macOS zip when a newer build exists, installs
it, and restarts. Settings can disable the launch check.

Documents never leave the computer unless you run an External Agent you
installed yourself. Logs record actions and errors, not Markdown contents.
App data lives in `~/.1537paperstreet/` (`config.json`, `projects.json`,
`ui-state.json`, `agents/prompts.json`, themes, Mermaid cache, logs).

## Install

macOS 12 (Monterey) or newer, Apple Silicon or Intel. GitHub Releases ship a
**universal** `.app` and DMG, a Linux `.deb` (x86_64), and a Windows NSIS
installer.

Current macOS releases are Developer ID–signed, notarized, and Gatekeeper-checked.
The Linux and Windows packages on the same release are unsigned. PDF export and
installing an update over the running app are macOS-only; on Linux and Windows,
Check for Updates opens the release page.

Release **0.3.0 and earlier** are unsigned: Right-click → Open, or after
copying to Applications:

```sh
xattr -cr /Applications/1537paperstreet.app
```

Homebrew cask is not available yet.

More detail: the [user guide](docs/src/index.md) (`docs/src/`).

---

## For developers

Rust + Tauri 2 + Svelte 5. Architecture is [`docs/PLAN.md`](docs/PLAN.md),
tasks are [`docs/CHECKLIST.md`](docs/CHECKLIST.md), agent rules are
[`AGENTS.md`](AGENTS.md).

### Run from source

Rust (stable), Node.js 22+, and Xcode Command Line Tools:

```sh
git clone https://github.com/mpakus/1537paperstreet.git
cd 1537paperstreet
npm install
./bin/test.dev
```

`./bin/test.dev` installs npm deps if needed and runs the Tauri development
window (`npm run tauri -- dev`). Extra arguments are forwarded.

Release `.app` / DMG (macOS 12+), Linux `.deb`, or Windows NSIS installer:

```sh
./bin/build              # on macOS: universal Apple Silicon + Intel
./bin/build host         # this Mac only
./bin/build deb          # on Linux: .deb
./bin/build nsis         # on Windows: NSIS installer
```

`./bin/build --help` lists targets. The same universal macOS build is `npm run tauri:build:universal`.

A `v*` tag builds the universal macOS app, a Linux `.deb`, and a Windows
installer, and publishes a GitHub Release after the macOS app and DMG pass
Developer ID signature, notarization-ticket, and Gatekeeper verification.
Linux and Windows packages are unsigned. Required Apple secrets are documented
in [`CONTRIBUTING.md`](CONTRIBUTING.md).

Before a pull request:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run lint
npm run test
```

How we work: [`CONTRIBUTING.md`](CONTRIBUTING.md). After changing IPC types:

```sh
UPDATE_TS_BINDINGS=1 cargo test -p ps-core --test typescript
```
