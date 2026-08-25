# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.7.0] - 2026-08-24

### Added

- Settings → External Agents and the Assistant **tab** (⌘⌥A) talk to local ACP
  CLIs (OpenCode, Claude, Codex on PATH, or a custom command). The Dashboard
  tab shows local library and agent counts. The app does not call a model API
  or the ACP Registry.

### Fixed

- Dashboard and Assistant are optional workspace tabs: they appear when opened
  and can be closed like document tabs instead of remaining pinned.
- Dashboard renders repeated metric and row labels without duplicate-key
  errors.

## [0.6.0] - 2026-08-21

### Added

- Opening a file that another program saved shows a dialog to reload it in
  Preview or Edit. Reloading discards unsaved edits only after you confirm.
- Drag a file from the tree onto another project in the list to move it.
  Hold ⌥ to copy.

### Fixed

- Dragging a tree file onto a project no longer shows the Finder drop overlay
  or re-opens the same file. The drop is applied to the project under the
  pointer.

## [0.5.0] - 2026-08-20

### Added

- Edit and Split load CodeMirror on first use and highlight Markdown source:
  markup marks fade to `--ed-syntax`, headings stay bold, links use the
  accent, and fenced/inline code uses the monospace font.

### Changed

- Preview code highlighting mixes token colors into the body text so fenced
  blocks stay readable. Paper Light/Dark highlight tokens are quieter.
- macOS tag releases now require Developer ID credentials, hardened-runtime
  signing, Apple notarization, stapled tickets, and Gatekeeper checks for both
  the universal app and DMG. The workflow refuses to publish unsigned builds.

## [0.3.0] - 2026-08-19

### Added

- File → Check for Updates… (same item in the application menu) asks GitHub
  Releases without downloading an installer. About has Check for Updates and
  Open Download when a newer version exists. Signed in-place install is still
  T-155.
- Open Folder…, File → Open Folder… / Open File…, and dropping a folder onto
  the window add that folder to Projects (the same path is not listed twice).

### Changed

- README, CONTRIBUTING, CHANGELOG, and the user guide are English. README
  leads with product information and three screenshots, then developer setup.

### Fixed

- After Open Folder… the Projects list reloads so the new folder appears
  without restarting the app.

## [0.2.1] - 2026-08-19

### Fixed

- The Projects list no longer highlights the first row while another project
  is open: only the active project is selected; keyboard focus shows when you
  move with the arrow keys.
- In Split the editor column follows the divider; width is stored as
  `window.editor_w`. Fractional pixels are rounded; the cap is workspace width
  minus preview.

## [0.2.0] - 2026-08-19

### Added

- Preview understands more GitHub Flavored Markdown: task lists, alerts
  (`> [!NOTE]`), definition lists, YAML front matter, and wiki links
  `[[Note]]` / `[[Note|label]]`. Fenced blocks (Rust, Python, Ruby, Elixir,
  YAML, JS/TS, and others) are highlighted with `syntect`. The editor has Task
  and Wiki buttons.
- Top-right of the preview — Full size: reading fills the window (under the
  title bar); click again or Escape to return.
- GitHub Actions on a `v*` tag builds one universal `.app`/DMG (Apple Silicon
  - Intel via `lipo`, macOS 12.0+) and publishes a GitHub Release. Developer
    ID signing and notarization are still ahead.

## [0.1.0] - 2026-08-19

### Fixed

- Mermaid diagrams draw in the preview: source is read from
  `template.content` (`textContent` on `<template>` is empty in WKWebView); a
  cache miss no longer leaves a gray rectangle.
- Dock icon comes from `icon.png`, not the letter P from the Tauri scaffold.
  macOS dev uses `icons/icon.icns` / `icons/icon.png` generated from the
  root `icon.png`.
- Settings open over the window (File → Settings…, ⌘,). The page no longer
  hides behind panes or crashes on `structuredClone` of a `$state` proxy.
- Settings: keep or hide the Dock icon after the window is hidden
  (`window.show_in_dock`); preview/Split font, size, and colors
  (`viewer.preview_*`) are written to `config.json`.
- Export and ⌘⌥E save the open document as PDF through the native Save
  dialog.
- The window is draggable from the top strip again: opaque overlay title bar
  with `data-tauri-drag-region` and `startDragging`.
- Window title: `1537paperstreet - {path of the open file}`.
- Image in the editor bar inserts `![]()`.
- The tree no longer shows “This folder is empty” for a folder that has
  files: IPC commands take snake_case arguments as in PLAN § 3.3 (Tauri 2
  expects camelCase by default).
- A transparent window no longer prints Tauri’s `macos-private-api` warning:
  `app.macOSPrivateApi` and the matching Cargo feature are on; vibrancy
  failures go to `logs/app.log`.

### Added

- In Preview / Split: small A− / A+ change reading size; − / % / + zoom the
  preview (50–200 %).
- App icon is `icon.png`; File and the application menu have About with the
  logo, version, “Made in Austin ✩ Texas”, and a link to aomega.co.
- macOS menu-bar icon (`icon-system.png`): closing the window hides it, the
  process stays; click the icon to show the window, Quit in that menu exits.

- Table of contents column is resizable; `window.toc_w` is stored in config.
- Document bar: Preview / Edit / Split, Save, Export, and Settings; themes in
  Settings are chosen from a palette and apply immediately. Open documents
  stay in tabs. A new folder immediately offers a name; clicking a selected
  name again renames. Drag in the tree highlights the destination folder and
  expands it on hover.
- Editor (⌘E) and Split (⌘⇧E): source in a text field, save through
  `doc_save`.
- `docio::read_doc` reports `trailingNewline` and returns text with LF;
  `write_doc` restores BOM, EOL, and a trailing newline atomically, skips the
  write when bytes match, and returns a conflict when `base_hash` diverges.
- Drop from Finder onto the window registers a folder or opens a Markdown
  file; Open Folder… / Open File… remain in the sidebar and the File menu.
- User guide (mdBook in `docs/src/`) for the reader: install, projects,
  files, Mermaid, themes, shortcuts, privacy; stubs for editor, history, and
  export. Manual smoke test: `docs/release-checklist.md`.
- Doctests for the public API (`resolve`, `read_doc`, `render`, Mermaid
  cache); CI rejects `cargo doc` warnings.
- Coverage gates in CI: `ps-core` / `ps-render` ≥ 85 %, `ps-app` ≥ 60 %
  (excluding thin IPC wrappers), `docio` ≥ 95 %, UI helpers `tree.ts` /
  `text.ts` ≥ 70 %.
- Initial Cargo workspace and Tauri/Svelte app shell.
- TypeScript types for IPC structs are generated from `ps-core` (`ts-rs`)
  into `ui/src/lib/generated/core.ts`; CI rejects uncommitted drift.
- IPC `config_*` and `projects_*` — thin wrappers over `ps-core`; config and
  project registry writes go through `spawn_blocking`.
- IPC `tree_read_dir` and `fs_*` — thin wrappers over `tree` and `fsops`;
  every path goes through `fsops::resolve`, writes through `spawn_blocking`.
- IPC `doc_open` and `doc_source`: the first HTML chunk returns synchronously,
  the rest as `doc://chunk` / `doc://done`; reads go through `docio` (BOM,
  EOL, UTF-8, 8 MB cap).
- IPC `open_dropped_paths`: a folder is registered as a project (dropping the
  same folder again reuses the record); `.md` opens inside a containing
  project or the parent folder.
- Left file tree with lazy expand and virtualization; dropping a Markdown
  file or folder onto the window opens a project.
- File → Open File… / Open Folder… via the native dialog; expanded tree
  nodes persist in `ui-state.json`; the tree width is adjustable, ⌘2 hides
  the pane.
- Tree context menu: New File, New Folder, Rename, Duplicate, Reveal in
  Finder, Open in External Editor, Move to Trash. The same actions are in
  File and Go; new files are created in the selected folder.
- Preview shows a TOC from `DocumentMeta.toc` and a banner if the file is
  read-only.
- Context menu Copy to… / Move to… and drag in the tree (⌥ copies); on a
  name clash — Replace / Keep Both / Skip. Drop from Finder into a tree
  folder copies into the project; ⇧/⌘ select several nodes for group
  operations.
- Preview images get `width`/`height` from the file header so layout does not
  jump on lazy load.
- Live tree from `fs://changed`, quick-open (⌘P), find in the document (⌘F),
  Settings (⌘,), `asset://` only from project roots, `.md` links open in the
  viewer, http(s) in the browser.
- Overlay title bar and vibrancy sidebar on macOS; a second launch focuses
  the existing window; logs go to `~/.1537paperstreet/logs/` with rotation.
- 12 built-in themes plus user JSON from `~/.1537paperstreet/themes/`;
  switching is one `data-theme` attribute and follows the system appearance.
- Three panes (projects / tree / preview) with stored widths and ⌘1 / ⌘2;
  virtualized project list, search highlight, ⌘⇧P, relocate an unavailable
  project without touching files on disk.
- Lazy Mermaid (dynamic import, IntersectionObserver, SVG cache) and KaTeX;
  Copy on code blocks; click a diagram to zoom.
- Native macOS menu with every PLAN § 11.3 shortcut; custom commands go to
  the UI as `menu://action`; font size is written to config.
- `AppState` opens config and the project registry at launch; the window
  restores size and position without a visible flash.
- Versioned atomic JSON store with migrations, backups, and preservation of
  corrupt files.
- App config with defaults and typography range checks.
- Project registry with ULID, safe record removal, and lazy folder
  availability checks.
- Incremental fuzzy search of projects by name and path.
- Safe project path resolution with NFC normalization and protection against
  escape via absolute paths, `..`, and symlinks.
- Folder and empty-file creation, atomic rename, and safe free-name
  suggestions on clash.
- Recursive copy with progress for large files, Replace/Keep Both/Skip,
  snapshot before replace, and rollback on error.
- Batch move: atomic `rename` on the same volume and staged copy/delete with
  source restore on cross-volume failure; concurrently created destination
  files stay in recovery staging and are not deleted.
- Move to the system Trash and a separate permanent delete; both require
  successful `pre_trash` snapshots for the whole batch first.
- Lazy one-level tree read with a hidden-file filter, directories first, and
  natural sort; 50,000 files average 37.9 ms.
- Project watch via FSEvents with 150 ms debounce, path coalescing, and
  overflow recovery limited to expanded nodes.
- Base Markdown renderer with tables, footnotes, strikethrough, task lists,
  smart punctuation, heading attributes, and math.
- Fenced blocks of known languages highlighted through `syntect` with the
  pure-Rust `regex-fancy` backend and classed `syntax-*` output, no inline
  styles.
- Unique heading slug ids and a TOC in source order.
- Mermaid fences become safe templates with a BLAKE3 of the source for lazy
  render and cache.
- Relative links and images become project `asset://` URLs; escape via `..`
  or symlinks is rejected in Rust.
- Raw HTML is sanitized by default; `javascript:` and `data:` links are
  blocked; bypass is an explicit option only.
- Top-level Markdown blocks are grouped near 64 KiB with
  `content-visibility: auto`, without splitting one oversized block.
- Each top-level block is tied to an exact source byte range, line number,
  and BLAKE3 hash for synced scroll and future incremental render.
- Two-level LRU cache of finished HTML: up to 16 documents / 64 MiB in
  memory and 200 MiB on disk with atomic writes and pruning of old entries.
- Snapshot corpus of the full Markdown pipeline: CommonMark/GFM, Mermaid,
  math, unsafe and broken input, Unicode/RTL, and a generated 5 MiB document.
- Criterion benches for Markdown from 10 KiB–5 MiB; quadratic suffix search
  for colliding heading ids was removed.
- `cargo-fuzz` target for the full Markdown pipeline; control characters are
  normalized without shifting byte offsets; a parser crash is pinned with a
  minimized regression test.
- ADR-001…ADR-007 and the mdBook user-guide skeleton.

[Unreleased]: https://github.com/mpakus/1537paperstreet/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.7.0
[0.6.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.6.0
[0.5.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.5.0
[0.3.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.3.0
[0.2.1]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.2.1
[0.2.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.2.0
[0.1.0]: https://github.com/mpakus/1537paperstreet/releases/tag/v0.1.0
