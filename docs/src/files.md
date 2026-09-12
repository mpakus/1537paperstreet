# Working with files

The tree shows one directory level at a time and loads children when you
expand a folder. Expanded folders and pane width persist across sessions.
⌘2 hides the tree. The Projects list collapses from the pane header or ⌘1;
a slim strip remains so you can open it again.

Icons distinguish folders, Markdown (`.md`, `.markdown`, `.mdown`, `.mdwn`),
and other files. Double-click a file to open it in a tab; a single click
selects it. UTF-8 text is editable. Preview and Edit highlight JavaScript,
TypeScript, Ruby, Elixir, Go, Rust, C#, Java, and PHP (and other languages
when a grammar exists). Binary files and files larger than 8 MB stay
source-only.

## Actions

Tree context menu and File / Go:

| Action | How |
| --- | --- |
| New File / New Folder | ⌘N / ⌘⇧N — in the selected folder |
| Rename | click the selected name again, F2, or the context menu |
| Duplicate | copy beside the original |
| Copy name | basename to the clipboard |
| Copy path | absolute path to the clipboard |
| Copy to… / Move to… | pick a destination folder |
| Reveal in Finder | ⌘⇧R |
| Open in External Editor | ⌘⇧O |
| Move to Trash | ⌘⌫, with confirmation |

Drag inside the tree moves a file. Hold ⌥ while dropping to copy. A stacked
ghost of the dragged names follows the pointer and eases into the folder. Drop from
Finder into a tree folder copies files into the project. Drag a file onto
another project in the Projects list to move it there; hold ⌥ to copy.

⇧ and ⌘ select several nodes for group copy, move, duplicate, and trash.

⌘, opens Settings (File → Settings…; same item in the application menu).
Themes are chosen there — light and dark palettes with a live preview
(Solarized, Nord, Gruvbox, Catppuccin, Tokyo Night, GitHub, and Paper). You
can also keep or hide the Dock icon after the window is hidden, and set
preview font and colors. File formats lists JavaScript, TypeScript, Ruby,
Elixir, Go, Rust, C#, Java, and PHP. Other UTF-8 still opens; it is
highlighted when a grammar exists.

⌘P quick-opens Markdown in the current project. Open files stay in tabs above
the preview. ⌘W closes the active tab only (a document, Dashboard, or
Assistant — not the window). ⌘⇧W hides the window. The tab close control
does the same for that tab. The active tab uses a stronger background and an
accent underline.

A new folder or file immediately offers a name. Clicking an already selected
name (not a double-click — that opens) also starts rename; F2 does the same.
Dragging in the tree moves into a folder (drop on a file targets that file's
folder); ⌥ copies; hovering a folder expands it. A stacked ghost of the
dragged names follows the pointer and eases into the folder. Copy name and
Copy path put the basename or the absolute path on the clipboard.

The title bar: Preview / Edit / Split, Save, Export, Board, and AI;
in Preview and Split — type size and zoom. Board opens the Dashboard tab; AI
opens the Assistant. More in [editing](editing.md) and
[assistant](assistant.md).

Images get width and height from the file header so layout does not jump.
A file that is too large (> 8 MB), binary, or missing shows a message instead
of a blank pane.
