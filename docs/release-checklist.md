# Manual smoke test before a release

Items CI does not cover. Run on a clean machine or a guest macOS 12+ account,
both architectures if you ship a universal binary. Check only what you
actually verified on this build.

Distribution:

1. Download the published DMG through a browser on a clean Mac. Open the DMG,
   copy the app to Applications, and launch it by double-clicking. Gatekeeper
   must not show an unidentified-developer or malware-verification warning.
2. With the downloaded artifacts, `codesign --verify --deep --strict`,
   `xcrun stapler validate`, and `spctl --assess` must accept both the app and
   DMG. Do not substitute artifacts from the Actions workspace.

Reader (current product):

1. First launch: the window appears without a flash of the wrong geometry,
   traffic lights in place, overlay title bar, vibrancy sidebar; Open Folder…
   in Projects and File → Open Folder… / Open File… open content. The folder
   appears in the Projects list.
2. Launching the same `.app` again focuses the existing window instead of
   creating a second one.
3. File → Open Folder… registers the folder; opening the same folder again
   does not duplicate the project.
4. Drop a folder on the window and drop a `.md` on the window open the
   project and the file; the folder is listed under Projects.
5. Tree: lazy expand, virtualization on a large folder, ⌘2 hides the pane,
   width is remembered after relaunch.
6. Icons: folder, Markdown, and other files are visually distinct.
7. Context menu and File/Go: New File/Folder, Rename, Duplicate, Copy/Move
   to…, Reveal in Finder, Open in External Editor, Move to Trash.
8. DnD inside the tree moves; ⌥ copies; drop from Finder into a tree folder
   imports. Drag a file onto another project in the list to move it; ⌥ copies.
9. Name clash: Replace / Keep Both / Skip, apply to all.
10. ⇧/⌘ select several nodes; group Move to Trash asks for confirmation.
11. Live tree: editing a file in Finder updates the tree without losing
    selection. An open file changed in another program asks to Reload or Keep
    this version.
12. ⌘P opens a file inside the project; ⌘⇧P switches project; project search
    highlights the match.
13. Unavailable project: Find Folder… relocates the record and does not
    touch files on disk. Removing a project from the list does not delete the
    folder.
14. Preview: TOC, jump to an anchor, read-only banner, images without layout
    jump, broken / too large / binary file — a clear error, not a crash.
15. Links: `.md` inside the project opens in the viewer; http(s) in the
    browser; `javascript:` does not open. A `file:` link opens only if it is
    inside the current project.
16. ⌘F searches the preview; ⌘, opens Settings; ⌘1 / ⌘2 hide panes.
17. Themes: ⌘⌥T follows the system or the session and does not rewrite the
    light/dark pair in config; user JSON from `~/.1537paperstreet/themes/`
    appears in the list.
18. Mermaid: the diagram draws on scroll-in, click opens zoom, Copy SVG and
    Save PNG use the native dialog; a syntax error shows the source and does
    not break the page. Turning it off in Settings leaves a clear message.
19. KaTeX: formulas draw lazily; turning it off in Settings does not crash
    preview.
20. Code block: Copy copies the text; highlight colors come from theme
    tokens.
21. ⌘+ / ⌘− / ⌘0 change preview type size and persist in config.
22. No outgoing network requests while reading a local document (Activity
    Monitor / Little Snitch if you want to confirm).
23. `~/.1537paperstreet/logs/app.log` does not contain the open document’s
    text.
24. Dock icon is `icon.png`; File → About and About in the application menu
    show the logo, version, “Made in Austin ✩ Texas”, and open aomega.co in
    the browser. File → Check for Updates… opens About, asks GitHub Releases,
    and installs a newer zip when one exists, then restarts.
25. The red traffic light hides the window; the icon stays on the right of
    the menu bar; click shows the window; Quit in that menu / ⌘Q quit the
    process.

After the editor (P9+) add: autosave, ⌘S, `base_hash` conflict, crash draft,
history pane, ZIP export. Do not check those items until the features exist.
