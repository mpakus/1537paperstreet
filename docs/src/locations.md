# Where files live

App data lives in `~/.1537paperstreet/`, not next to your notes.

| File | Contents |
| --- | --- |
| `config.json` | Theme, font, preview colors, Dock, Mermaid/KaTeX flags, window geometry, external agents |
| `agents/prompts.json` | Recent Assistant prompts (not agent replies) |
| `projects.json` | Project list (name and path), not file contents |
| `ui-state.json` | Expanded folders, pane widths |
| `themes/` | Your JSON themes |
| `cache/mermaid/` | Cached diagram SVG |
| `logs/app.log` | Rotating log; no document text |

If the app misbehaves: quit, rename `config.json` to `config.json.bak`, and
launch again — defaults load. The project registry can be restored from
`projects.json.bak` if one appeared after a failed write.

The app does not move project documents. Removing a project from the list
does not delete the folder on disk.
