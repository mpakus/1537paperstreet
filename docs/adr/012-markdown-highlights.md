# ADR-012: Source-backed text highlights

Status: Accepted for T-217, explicitly requested by the user.

## Decision

Persist highlights in Markdown: `==text==`, six Obsidian-style emoji presets,
and the local extension `=={#rrggbb}text==`. Hex accepts three or six digits;
the UI writes canonical lowercase six-digit values. No dependency, schema,
sidecar, network access, or new file-writing path is introduced.

Rust pairs delimiters inside editable inline blocks and keeps consecutive text
runs and source line breaks in one mark. Nested emphasis keeps valid HTML tag
boundaries without adding extra delimiters to Markdown.
Code, math, image labels, front matter, and links whose label determines their
destination are excluded. Styles are limited to validated marker background
hex and a computed black/white foreground. The sanitizer does not accept other
inline styles.

## Selecting rendered text

Normal rendering does not attach text-level source spans. When applying a
highlight, `doc_highlight_source` produces temporary sanitized HTML from the
current buffer. Each mapped span contains that buffer's BLAKE3 hash and parser
byte range. UI compares captured block hashes and visible text, then maps DOM
UTF-16 offsets onto those spans. A source HTML attribute cannot impersonate a
generated span without embedding the hash of its own entire source.

`doc_highlight` validates every run and Unicode boundary again against the
parser before returning a replacement buffer. Unsupported or stale selections
fail without changing text. Before returning, rendering with marker tags
removed must match the original, so delimiters cannot silently change other
formatting or visible text. Both IPC calls run off the command thread and do
no filesystem I/O. App rejects results after a document switch or intervening
edit. A highlight is one isolated CodeMirror transaction; Preview can undo it
without opening Edit. Save uses the existing atomic, base-hash-checked path.

## Consequences

Each continuous selection uses one marker per paragraph or list item, including
source line breaks and inline formatting. Recoloring a selection joins older
adjacent markers. Separate blocks and unselected text remain separate. Selecting
part of an existing marker recolors/removes the entire original marker. Exact hex
colors are not promised to render in other Markdown applications. Source maps
are produced only during an explicit action, keeping ordinary previews small.
Changing document identity creates a fresh editor history; changing view mode
keeps it. Autosave, drafts and document-history storage remain out of scope.
