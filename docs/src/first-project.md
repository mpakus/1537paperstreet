# First project

A project is an ordinary folder on disk. The app does not copy it and does not
change files when you only read them.

Ways to open a folder — each **adds it to Projects** if it is not there yet:

- Projects pane → Open Folder…
- File → Open Folder…
- File → Open File… — opens Markdown, PDF, YAML, JSON, and TOML files and
  registers their folder if needed
- Drop files or folders from another app onto the window. Choose **Add to
  Projects** when the app asks about a new folder.
- Open a `.md` or `.pdf` with 1537paperstreet from the file manager. The file
  opens and its folder is added to Projects when it is not there yet. The app
  is listed under Open With and does not replace your default editor. After
  installing a new build, open the app once so the system records that list.

Opening the same folder again reuses the existing list entry. Dropped files
open in tabs from their current locations. Any tabs you already had open stay
in the workspace.

Which names open as notes, PDFs, images, or source is listed in
[Formats](formats.md).

The project list is virtualized, so thousands of rows still scroll. The search
field filters by name and path and highlights the match.

If you moved the folder, the row turns dim. Context menu → Find Folder…
points the record at the new location. Removing a project from the list only
removes the record; files on disk stay.
