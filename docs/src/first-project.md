# First project

A project is an ordinary folder on disk. The app does not copy it and does not
change files when you only read them.

Ways to open a folder — each **adds it to Projects** if it is not there yet:

- Projects pane → Open Folder…
- File → Open Folder…
- File → Open File… — registers the folder that contains the `.md` and opens
  the file
- Drop a folder or a `.md` from Finder, Explorer, or the file manager onto the window

Opening the same folder again reuses the existing list entry. Dropping a
Markdown file opens it and registers the containing folder. Dropping onto a
tree row inside an already open project copies files into that folder.

The project list is virtualized, so thousands of rows still scroll. The search
field filters by name and path and highlights the match.

If you moved the folder, the row turns dim. Context menu → Find Folder…
points the record at the new location. Removing a project from the list only
removes the record; files on disk stay.
