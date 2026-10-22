# ADR-015: Structured notes and reusable source selections

Status: accepted for the explicitly requested T-222 workflow.

## Decision

Library connects structured Markdown notes, saved collections, explicit review
baselines and context packs. Source documents remain in registered project folders.
The UI is a lazy workspace tab with metadata filters, a virtualized document list,
a note creation form and an ordered selection editor. Narrow layouts stack the
selection below the document list. Theme tokens and standard controls match the app.

`ps-core::library` owns bounded discovery, YAML metadata, templates, filtering,
sorting, selection validation, exact-byte hashes and context assembly. `ps-render`
provides section ranges through the existing parser and heading-ID implementation.
Tauri commands run blocking work off the UI thread. Generated types define IPC.

`library.json` is separate versioned application data. It stores project identities,
relative paths, optional heading IDs, accepted hashes, selection names and an
optimistic revision. `JsonStore` provides atomic replacement and corruption
preservation; an app mutex serializes reads and mutations. A failed save leaves
the previous file intact. Configuration and project registry schemas do not change.

New notes use `fsops::create_file`, then `docio::write_doc` with the known empty-file
hash. A concurrent write is a conflict, never an adopted baseline. If initialization
fails, the new empty file may remain; existing documents are never replaced.
Existing metadata is edited through the ordinary editor to preserve unknown keys,
comments, BOM and line endings. Merely scanning a file never rewrites it.

The direct dependency `serde_yaml_ng = 0.10.0` reads real YAML mappings and tag
sequences instead of extending the renderer's display-only line splitter. Its
crate archive and transitive YAML parser are each below 500 KB. See the
[Serde YAML API](https://docs.rs/serde_yaml_ng/0.10.0/serde_yaml_ng/).

## Semantics and limits

- Review means a user accepted a particular saved content hash. Changed means
  different bytes, not an incorrect conclusion. No document history is stored.
- Collections and packs are explicit ordered selections, not dynamic saved queries.
- Packs use saved files only. Unsaved editor buffers are not included or saved.
- Heading IDs survive unrelated line insertions. A removed or renamed heading is
  unavailable; duplicate headings retain the preview's ordinal suffix behavior.
- Missing/unreadable sources make the whole preview non-copyable. Changed sources
  are visible and use current saved text. Refreshing does not accept a new baseline.
- Library scans at most 5,000 notes, 100,000 entries, 64 MB, 64 folder levels and
  2 MB per note, with visible partial-result notices. Metadata is limited to 64 KB.
- At most 100 selections, 100 sources per selection and 10,000 review baselines.
  Pack reading is limited to 16 MB; assembled output to 2 MB.
- `.age`, renamed age payloads and symbolic links are excluded. Project reads use
  `fsops::resolve`. No decrypted content is indexed or persisted.
- Context contains provenance and source hashes. Token counts are approximations.
  Source text is reference material; it is not trusted agent authorization.
- Copy is an explicit clipboard action. Use in Assistant stages text for review;
  it does not start an agent or send a prompt. External agents retain their own
  permissions and network behavior. Library introduces no network request.

## Tradeoffs

On-demand scanning keeps the document files authoritative without a new full-text
database. Larger libraries show limits rather than claiming complete results.
Metadata and selection storage stay local to this installation; source documents
remain portable. Citation management, PDF reading, automatic dependency analysis,
Git comparisons and general document history are separate features.
