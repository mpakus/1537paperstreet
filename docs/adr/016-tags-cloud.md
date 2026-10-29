# ADR-016: Tags connect registered project folders

Status: Accepted for the user-requested Tags Cloud (T-224).

## Context

[Bear 2.9](https://blog.bear.app/2026/07/bear-2-9-use-tag-as-workspace/)
uses a tag to scope notes and related tags. Its
[syntax](https://bear.app/faq/how-to-use-tags-in-bear/) supports `#tag`,
`#parent/child`, `#multiple words#`, and `#parent/multiple words#`.
An escaped `\#tag` is literal text.

## Decision

Tags Cloud is a lazy workspace spanning all registered project folders. A tag
selects documents carrying it or its descendants. Additional related tags use
AND semantics. Counts and cloud weights are scoped to matching documents;
shared tags connect documents across project boundaries.

`ps-render` extracts inline tags from Markdown text events, excluding code,
HTML blocks/attributes, links, images, math and metadata. Escapes remain literal. Unicode is
supported; matching uses lowercase keys with the first spelling for display.
Existing Library YAML tag lists join the same index. Tags are deduplicated per
document; inferred parents count descendants once. Empty hierarchy segments and
tags over 256 bytes or 16 levels are ignored. Extraction is capped at 256 unique
tags per note with a visible partial-results notice.

`ps-core` reuses Library's bounded scanner, normalizes tags, builds queries and
counts, and retains only note metadata in a rebuildable in-memory index. No new
dependencies, network calls, or changes to project/config schemas are needed.
The index is rebuilt on pane activation, registered-project changes, watcher
events, or explicit Refresh. Filter queries reuse the index. Choosing a folder rebuilds with that scope, so
it can recover notes omitted by a global limit. Unavailable roots
and limits remain visible. `.age`, symlinks, and unsaved buffers stay excluded.

The UI presents a weighted, paginated cloud (80 tags/page), filters, and a
virtualized document list. Opening a result uses the existing cross-project
navigation and unsaved-edit protection. Tags are managed in the document editor
and become visible after Save. Bulk source rewriting is outside this decision.
This workspace filter does not hide other app surfaces or provide a privacy
boundary like Bear's application-wide workspace.

## Acceptance and verification

- Test syntax, escapes, nested ancestors, excluded Markdown contexts, Unicode,
  deduplication, and bounded extraction before implementation.
- Test cross-project AND queries, project scope, refresh/removal, unreadable
  roots, encrypted/symlink exclusion, and unchanged source bytes using tempfile.
- Test UI loading/errors, stale requests, selection/reset, opening documents,
  pagination and virtualization. Inspect the real browser with fixture IPC.
- Run repository DoD: cargo fmt/clippy/workspace tests; npm check/lint/test/build;
  regenerate TypeScript bindings and record a scan/query benchmark.
- Ship T-224 in one PR; version synchronization is T-225 in its own PR.
