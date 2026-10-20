# ADR-014: Local document relationship graph

Status: accepted for T-220, explicitly requested by the user.

Validation: [GRAPH_VALIDATION.md](../GRAPH_VALIDATION.md).

Graph is an on-demand workspace tab for exploring explicit Markdown and wiki links. It indexes saved plaintext Markdown, never editor buffers or encrypted notes. Images, external URLs, fragments, raw HTML and code samples do not create relationships. Missing destinations do not become synthetic documents.

`ps-render` extracts links with the existing Markdown parser and resolves them with the preview's path rules. `ps-core` owns bounded filesystem discovery, a rebuildable database, graph queries and layout. Every project path passes through `fsops::resolve`; symlinks are not traversed. The app delegates through a blocking IPC command. No network requests or project-file writes are introduced.

The cache uses redb 4.4 (MIT/Apache-2.0, 422 KB crate archive, pure Rust, no new native runtime). This dependency supplies transactional embedded storage on all three desktop platforms. One versioned database per canonical project root lives under the internal cache directory. It stores relative paths, content hashes and extracted link destinations, never document bodies. Files are hashed on refresh; unchanged files reuse parsed links. Destinations are resolved again so additions, deletions and renames cannot leave stale edges. Rebuild replaces only this disposable cache. A serialized graph service prevents overlapping database opens. Cache errors are visible and offer an explicit rebuild.

Discovery and rendering are bounded, with visible partial-result notices. Queries and layout run off the UI thread. The canvas is presentation only: projection, pan, zoom, hit testing and selection. The searchable result list and connection counts come from Rust. Keyboard users can select notes, focus a neighborhood, open notes, zoom and fit. Reduced motion disables ambient effects. Graph is loaded only when opened.

Design contract: a spacious constellation canvas, fine orbital guides, theme-colored glow, directional connections on selection, compact controls, and a readable inspector. The primary action is opening a connected note. Loading, no project, no Markdown, no matches, isolated notes, errors and truncated graphs have explicit states. On narrow windows the inspector becomes a compact lower panel. No external graph library, forced dark theme, autonomous camera motion or always-running animation loop.

This is not semantic/AI similarity, a document history store, or an encrypted-content index. The graph can be deleted and rebuilt without losing user data.
