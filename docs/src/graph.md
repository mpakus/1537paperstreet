# Document graph

Open a project and choose **Graph** in the toolbar or **View → Document Graph**.
The constellation shows saved Markdown documents as stars and links as lines.
Colors distinguish folders; larger stars have more connections.

- Select a star or a note in the list to see incoming and outgoing counts.
  Arrows show link direction around the selection.
- Choose **Open note**, or double-click a star, to open the document normally.
- Choose **Focus here** or **Neighborhood** to show the selected note and its
  immediate incoming/outgoing neighbors. This clears the search filter.
- **Whole project** returns to the wider map. **Find a note** filters paths;
  **Hide isolated notes** hides notes without links.
- Drag to pan, scroll to zoom, or use **+**, **−**, and **Fit**. With the canvas
  focused, arrow keys pan, `+`/`−` zoom, and `0` fits the map.
- The note list supports arrow keys, Home/End, and Enter to open the selection.
  The map has no continuous animation or automatic camera movement.

## What connects

Graph recognizes Markdown links (including reference links) and `[[wiki links]]`
using the preview's resolution rules. Wiki links try `.md`, `.markdown`, and
`index.md` beside the current document and at the project root. Links to existing
indexed Markdown documents create directed relationships. Repeated links are
deduplicated. Same-document links, images, external URLs, code samples, raw HTML,
and missing destinations do not create edges. This is an explicit-link map;
similar wording alone does not connect notes.

Only saved UTF-8 files are indexed. Unsaved editor changes appear after Save.
Filesystem changes refresh an open graph; **Refresh** also rescans it manually.
Encrypted notes are excluded, even while unlocked, and their contents never
enter this cache. Links to encrypted notes are omitted.

## Local cache and limits

The graph's embedded database lives in `~/.1537paperstreet/cache/graph/` (or the
corresponding `PAPERSTREET_HOME`). Each canonical project root has a separate
versioned database. It stores relative paths, hashes, and link destinations;
document bodies and passwords are never stored. Hashes detect changes even when
a file's size stays the same. Unchanged notes reuse parsed links across launches.
The footer shows how many notes were cached or updated.

**Rebuild cache** discards only this project's graph database and reads the
source files again. It never changes your Markdown. Cache failures are shown
with a rebuild option. Ordinary reading and editing do not depend on the graph.
The cache can retain filenames and links from earlier revisions until rebuilt;
it is disposable metadata, not encrypted storage or secure erasure.

To keep large folders responsive, a scan is limited to 5,000 notes, 100,000
directory entries, 64 MB of text, 2 MB per note, 64 folder levels, and 2,000 links
per note. At most 50,000 links are resolved per refresh. Symlinks and `.git`,
`node_modules`, `target`, and `.1537paperstreet` directories are skipped.
The map displays up to 600 matching notes and 6,000 connections. Partial results
and unsupported files show a notice; use search or a neighborhood to narrow the
map. The note list is virtualized.
