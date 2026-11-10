# First project

A project is an ordinary folder on disk. The app does not copy it and does not
change files when you only read them.

Ways to open a folder — each **adds it to Projects** if it is not there yet:

- Projects pane → Open Folder…
- File → Open Folder…
- File → Open File… — registers the folder that contains the `.md` or `.pdf`
  and opens the file
- Drop a folder, a `.md`, or a `.pdf` from Finder, Explorer, or the file
  manager onto the window
- Open a `.md` or `.pdf` with 1537paperstreet from the file manager. The file
  opens and its folder is added to Projects when it is not there yet. The app
  is listed under Open With and does not replace your default editor. After
  installing a new build, open the app once so the system records that list.

Opening the same folder again reuses the existing list entry. Opening a
Markdown or PDF file opens it and registers the containing folder. Dropping
onto a tree row inside an already open project copies files into that folder.

Which names open as notes, PDFs, images, or source is listed in
[Formats](formats.md).

The project list is virtualized, so thousands of rows still scroll. The search
field filters by name and path and highlights the match.

If you moved the folder, the row turns dim. Context menu → Find Folder…
points the record at the new location. Removing a project from the list only
removes the record; files on disk stay.
