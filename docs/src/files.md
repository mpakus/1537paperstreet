# Working with files

The tree shows one directory level at a time and loads children when you
expand a folder. Expanded folders and pane width persist across sessions.
⌘2 hides the tree. The Projects list collapses from the pane header or ⌘1;
a slim strip remains so you can open it again.

Icons distinguish folders, Markdown (`.md`, `.markdown`, `.mdown`, `.mdwn`),
and other files.

## Actions

Tree context menu and File / Go:

| Action | How |
| --- | --- |
| New File / New Folder | ⌘N / ⌘⇧N — in the selected folder |
| Rename | click the selected name again, F2, or the context menu |
| Duplicate | copy beside the original |
| Copy to… / Move to… | pick a destination folder |
| Reveal in Finder | ⌘⇧R |
| Open in External Editor | ⌘⇧O |
| Move to Trash | ⌘⌫, with confirmation |

Drag inside the tree moves a file. Hold ⌥ while dropping to copy. Drop from
Finder into a tree folder copies files into the project. Drag a file onto
another project in the Projects list to move it there; hold ⌥ to copy.

⇧ and ⌘ select several nodes for group copy, move, duplicate, and trash.

⌘, opens Settings (File → Settings…; same item in the application menu).
Themes are chosen there — light and dark palettes with a live preview
(Solarized, Nord, Gruvbox, Catppuccin, Tokyo Night, GitHub, and Paper). You
can also keep or hide the Dock icon after the window is hidden, and set
preview font and colors.

⌘P quick-opens Markdown in the current project. Open files stay in tabs above
the preview, after pinned **Dashboard** and **Assistant** tabs. The close
control closes a document tab; Dashboard and Assistant stay in the strip.

A new folder or file immediately offers a name. Clicking an already selected
name (not a double-click — that opens) also starts rename; F2 does the same.
Dragging in the tree moves into a folder; ⌥ copies; hovering a folder expands
it.

The title bar: Preview / Edit / Split, Save, Export, Board, and AI;
in Preview and Split — type size and zoom. Board opens the Dashboard tab; AI
opens the Assistant. More in [editing](editing.md) and
[assistant](assistant.md).

Images get width and height from the file header so layout does not jump.
A file that is too large (> 8 MB), binary, or missing shows a message instead
of a blank pane.
