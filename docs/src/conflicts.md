# Conflicts

Today the app has one kind of conflict — a **file name** clash on copy, move,
or import from the file manager.

The dialog offers Replace, Keep Both, or Skip. A checkbox applies the choice
to the rest of the clashes in the same operation. Replace snapshots the
content that would be overwritten first.

**Version conflict** (`base_hash` diverged while the document was open):
`write_doc` returns an error and **does not write**.

If another program changes the open file on disk, a dialog offers **Reload**
or **Keep this version**, in Preview and Edit. Reload discards unsaved edits
in this app. A three-way banner with a line diff will follow.
