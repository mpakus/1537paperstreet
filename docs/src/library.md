# Library

Open **Library** in the toolbar or **View → Library**. It brings structured
notes, saved collections, review tracking and context packs into one workspace.
It reads saved plaintext Markdown from your registered projects; your files stay
in their original folders.

## Create structured notes

Choose **New structured note**, a title, a relative file path and a type. The
destination folder must already exist. Existing files are never overwritten.

Templates cover notes, specifications, decisions, papers, literature reviews,
research questions, investigations, benchmarks and experiments. Each supplies
appropriate headings and portable YAML properties:

```yaml
---
title: Agent memory evaluation
type: experiment
status: draft
tags:
  - agents
  - evaluation
source: https://example.org/reference
---
```

Edit these properties in the ordinary Markdown editor. Reading a note does not
rewrite its front matter or remove unknown properties. Invalid front matter
produces a notice; the note remains accessible using its filename.

Filter by title/path/project, type, status, topic or freshness. **All registered
projects** includes other project folders. Select a result to inspect its
properties, open it, mark it reviewed or add it to a selection. Arrow keys,
Home and End navigate the document list; Enter opens the selected note.

## Keep collections

Add whole documents or individual heading sections, arrange them with the arrow
buttons, name the selection, choose **Collection**, then **Save selection**.
Collections can include documents from different registered projects. Open one
from **Saved selections** and click a source to return to its document.

Collections are explicit lists, not saved filter queries. Saving does not move or
copy the documents. Deleting a collection only removes its saved references.
Unsaved selection edits survive switching workspace tabs, but must be saved to
survive an app restart. Closing the Library tab asks before discarding edits.

## Track freshness

**Mark reviewed** accepts the exact saved version currently displayed. Later file
changes show **changed**; documents without a review baseline show **unreviewed**.
Use the Freshness filter as a review queue. This tracks changed bytes, not the
correctness of a specification or research conclusion. No old document version
is retained, and refreshing does not mark anything reviewed.

## Build context packs

1. Add the sources and heading sections you want, in reading order.
2. Choose **Context pack**, name it, and save the selection.
3. Choose **Preview context** to inspect the current saved text, source paths,
   project identities, hashes and approximate token count.
4. **Copy context** puts the complete pack on the clipboard. **Use in Assistant**
   appends it to the existing prompt for your review. It does not send the prompt
   or start an agent.

Save document edits before previewing: packs use disk contents. A changed source
is marked explicitly. **Accept current source versions**, followed by **Save
selection**, updates the pack's baselines. It does not change the note's separate
review state. Missing files or headings block the entire pack until repaired or
removed. Copy and Assistant actions recheck sources; if they changed since the
preview, review the refreshed preview and try again.

Heading selections include their subsections. Renaming/removing the selected
heading makes it unavailable. Duplicate headings use the same numbered anchors
as the document preview; reordering duplicates can change their meaning.

## Local storage and limits

`~/.1537paperstreet/library.json` stores names, project identities, relative paths,
heading anchors and hash baselines, never document bodies. It is local to this
installation. Source notes remain ordinary portable Markdown. Back up this file
to preserve collections and review baselines.

Library excludes encrypted notes and symbolic links. It makes no network request.
An external agent may transmit a pack only when you send the staged prompt, under
that agent's settings. Sent prompts can appear in local Assistant History.

A scan covers up to 5,000 notes, 100,000 directory entries, 64 MB and 64 folder
levels; each note can be at most 2 MB. Partial scans show a notice. Narrow the
project scope when needed. You can save 100 selections with 100 sources each and
10,000 review baselines. Packs read at most approximately 16 MB of source files
and assemble at most 2 MB of text. Token counts are estimates, not model limits.
