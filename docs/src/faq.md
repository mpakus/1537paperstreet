# FAQ

**Why can’t I type in the preview?** Preview is read-only. Switch to Edit or
Split (⌘E / ⌘⇧E) and save with ⌘S, or Open in External Editor (⌘⇧O).

**A project disappeared from the list?** The folder was probably moved.
Context menu → Find Folder….

**The diagram is empty.** Check Settings → Render Mermaid diagrams. If it is
on, check the fence syntax; errors show under the source.

**Formulas don’t render.** Settings → mathematics (KaTeX). You need `$...$` /
`$$...$$` in the Markdown, as in the renderer corpus.

**Release 0.3.0 won’t open from the DMG.** That release was not notarized. Use
Right-click → Open / `xattr -cr` as in [Install](install.md), or download a
newer signed release. From source, use `cargo tauri dev`.

**I closed the window and the menu-bar icon is still there?** That is
intentional. Click the icon to show the window; Quit in its menu or ⌘Q
quits the process.

**Is there a cloud, account, or telemetry?** No. Update checking runs only
when you choose File → Check for Updates… or the button in About; it asks
GitHub Releases and does not send documents.

**How do I send an Assistant prompt?** Shift+Enter, Ctrl+Enter, or ⌘Enter.
Plain Enter is a new line. While the agent is working, Stop cancels the turn.

**Codex never becomes ready?** Use the Codex preset or `codex` with
`app-server --stdio`. `codex acp` is a terminal UI, not the protocol this
app speaks. More in [Assistant](assistant.md).
