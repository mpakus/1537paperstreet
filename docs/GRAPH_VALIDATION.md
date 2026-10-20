# Document graph validation — T-220

Local verification: 2026-10-20. This record covers the implementation on the
task branch; it does not claim that Graph is in the published 0.9.0 packages.

## Automated checks

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: 372 passed; one opt-in timing test ignored,
  including the large-neighborhood focus regression.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`: passed.
- `npm run check`, `npm run lint`, `npm run format:check`, `npm run build`: passed.
- `npm run test`: 202 passed across 57 files.
- `npm audit --audit-level=high`: zero vulnerabilities after the approved
  Vite 8.2.4 security patch. A workspace override deduplicates development and
  test consumers onto the patched version; other package versions are unchanged.
- `cargo deny --locked check --deny warnings`: advisories, bans, licenses and
  sources passed. redb adds no additional runtime dependency beyond libc
  already present in the workspace.
- Existing ts-rs serde-attribute diagnostics and the existing Mermaid chunk-size
  build notice remain; no new Svelte or lint diagnostics were introduced.

Graph integration tests cover Markdown/reference/wiki links, aliases, fragments,
percent-encoded paths, local wiki precedence, external/code/image/HTML exclusion,
persistent cache reuse, edits, additions, deletions, isolated notes, neighborhood
queries, project separation, path escapes, Unix symlinks, bounded results,
unsupported files, preservation of the focus in large neighborhoods, and explicit
recovery from a corrupt database. Tests use only temporary folders. Windows CI
now also runs this graph suite.

Mutation check: temporarily removing encrypted-note exclusion made the privacy
test fail (four indexed files instead of three). The mutation was restored
before the passing full suite. Tests also inspect the database bytes for body
text and the excluded encrypted-note fixture's name/content.

UI tests cover stale replies across project changes, stale errors after closing,
current errors, explicit rebuild requests, empty project state, virtualized
keyboard navigation, note opening and Graph tab close/session behavior.

## Native and browser inspection

An isolated macOS debug app used `PAPERSTREET_HOME` and a temporary 28-note
research collection; it did not register or edit personal projects.

- Restored directly into the Graph workspace.
- Opened Graph using its toolbar control.
- Queried a note through real Rust IPC; the footer showed 28 reused cache entries.
- Focused a neighborhood and opened the selected note in the normal document tab.
- Created a temporary fixture note: 29 nodes / 81 connections. Deleted it while
  Graph was open: watcher refresh returned to 28 nodes / 80 connections.
- Inspected theme-colored canvas, directional selection, and the compact pane
  layout with both sidebars visible; full workspace view also checked.
- Browser canvas inspection used an exported Rust snapshot and had no console
  warnings or errors. Temporary preview files were removed before committing.

Native installed Windows/Linux UI acceptance remains a release check; CI is
tracked separately on the PR. Graph has no persistent animation loop, so reduced
motion needs no alternate animation path.

## Performance

Release-mode local benchmark: 300 notes, approximately 13 MB of UTF-8 text,
600 directed links. Complete scan, query and layout took **129.9 ms cold** and
**108.2 ms warm**; warm reused all 300 parsed-link entries. This is an on-demand
operation on a blocking worker, not startup work. Reproduce with:

```sh
cargo test -p ps-render --release --test graph graph_cold_and_warm_timings -- --ignored --nocapture
```

Production entry JavaScript compared with clean `1bf6ad1` source: 229,275 bytes
before and 231,535 bytes after (+2,260 bytes, 0.99%). Graph's 14,460-byte JavaScript
and 8,633-byte stylesheet load only when its workspace tab is opened. No graph
database is opened during ordinary app startup. The database page cache is
limited to 8 MB; scans, links and displayed nodes have explicit documented caps.

## Review

Correctness, readability, architecture, privacy/path boundaries, and performance
review completed. Project files remain read-only; database rebuild targets only
the hashed internal cache path. Core and renderer remain independent of Tauri.
No network endpoint, project/config schema change, or graph UI dependency was
added. Supported limits and cache metadata retention are documented in the guide.
