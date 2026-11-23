# Formats

1537paperstreet opens the files that already live in a folder. Markdown is the
reading and writing format. PDFs and images open in Preview. Other UTF-8 text
opens in the editor.

![A PDF open in Preview, with its folder registered under Projects](../screen-pdf.png)

## Open from the file manager

Open Markdown, PDF, YAML, JSON, or TOML with File → Open File… to open it from
its folder. The picker adds the folder to Projects when needed. Markdown and PDF
are also associated with 1537paperstreet in the system’s Open With list.

The app is listed under Open With. It does not replace the default editor.
After a new build is installed, open the app once so the system records that
list. To make 1537paperstreet the default, use the file manager’s Get Info /
Properties → Open with.

Drag files from Finder, Explorer, or another app onto the window to open them
from their current locations. Files already in Projects open right away. For
other files, choose **Add to Projects** to register their folders and open them.
You can also drop a folder to add it to Projects.

## What opens where

| Kind | Names | In the app |
| --- | --- | --- |
| Markdown | `.md`, `.markdown`, `.mdown`, `.mdwn` | Preview, Edit, and Split. Save and Format work. |
| Protected notes | the same names with `.age` added | Preview until you unlock. Then the same editing as Markdown. |
| PDF | `.pdf` | Preview only. Pages are drawn on macOS. Linux and Windows use the system viewer when it can show a PDF. |
| Images | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg`, `.bmp`, `.ico`, `.avif`, `.tif`, `.tiff` | Preview only. SVG is shown as an image, so scripts in the file do not run. |
| Source | `.js`, `.ts`, `.rb`, `.ex`, `.go`, `.rs`, `.cs`, `.java`, `.php`, and the related names | The editor, with highlighting when a grammar exists. |
| Configuration | `.yaml`, `.yml`, `.json`, `.toml` | Preview and Edit with highlighting. Edit flags syntax errors. Format adjusts the editor buffer. |
| Other text | UTF-8, including names with no extension | The editor. Highlighting is used when a grammar exists. |
| Too large or not text | over 8 MB, or bytes that are not UTF-8 | A short message. The bytes are not shown as text. |

PDF and images cannot be saved from the editor. Use Open in External Editor
when another program should change them.

The first 40 pages of a PDF are drawn on macOS. A longer file says how many
pages were left out. Content search does not read PDF bytes.

## Markdown on the page

Preview renders GitHub Flavored Markdown: tables, strikethrough, task lists,
footnotes, alerts (`> [!NOTE]`), definition lists, YAML front matter, wiki
links (`[[Note]]`, `[[Note|label]]`), fenced code, Mermaid diagrams, and KaTeX
math (`$...$`, `$$...$$`). Images in a note use the formats in the table above.

Export PDF (⌘⌥E) writes a snapshot of that page. It is separate from opening
a `.pdf` you already have. See [Export](export.md).
