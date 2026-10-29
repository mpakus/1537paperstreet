# Tags Cloud

Open **Tags** in the toolbar or **View → Tags Cloud** to follow topics across
all project folders registered in 1537paperstreet. The files stay in their folders.

## Write tags in your notes

| Syntax | Meaning |
| --- | --- |
| `#research` | A simple tag |
| `#research/agents` | A nested tag; also appears under research |
| `#project alpha#` | A tag containing spaces |
| `#project alpha/design notes#` | Nested tags with spaces |
| `#!#` / `#2021#` | Punctuation or numeric tag, enclosed in hashes |
| `\#literal` | Escaped text, excluded from the cloud |

This follows [Bear's tag syntax](https://bear.app/faq/how-to-use-tags-in-bear/).
Tags can appear in paragraphs and headings. Code, Markdown links/images, math,
HTML blocks and YAML text are excluded from inline extraction. Existing YAML
`tags: [research, agents]` lists are included as metadata. Matching ignores case.
One document counts once for each tag, even if it repeats the tag or has several
children under the same parent.

## Follow connections

- Select a tag to focus on its notes, including nested tags. The cloud then shows
  tags that occur in those notes, with counts for documents and project folders.
- Select another tag to intersect the two topics. Remove a selected chip or use
  **Clear tags** to broaden the workspace again.
- Search matches document titles, relative paths, folder names and tags.
- Select a document and choose **Open note**, or double-click it. Arrow keys and
  Enter work in the document list. **Only this folder** narrows the workspace;
  **All projects** removes that folder filter. **Choose folder…** searches the
  complete project registry and rebuilds within that folder, which also recovers
  notes omitted by an all-project scan limit.
- Edit tags in the existing editor and **Save**. Returning to Tags Cloud refreshes
  saved content. **Refresh** also picks up external changes in other folders.

Larger labels indicate more matching documents. The cloud shows 80 tags per
page; the document list is virtualized. Counts always describe the current
filters. A shared tag links notes across folders without moving or copying files.

## Scope and limits

The tag workspace is a navigation view. It does not hide other app surfaces,
change Assistant permissions, or provide a privacy boundary. It reads saved
plaintext Markdown only; unsaved text and `.md.age` files are excluded. Symlinks
are not followed. No document bodies are retained in the index or sent online.

Scanning uses the Library limits: 5,000 notes, 100,000 filesystem entries, 64 MB,
64 folder levels, and 2 MB per file. Tags are limited to 256 bytes, 16 hierarchy
levels, 256 distinct tags per note, and 100,000 indexed tag connections. A notice
reports partial scans and unavailable sources. Invalid tag names are ignored.
Bulk renaming or deleting tags across files is not provided; use the editor.
