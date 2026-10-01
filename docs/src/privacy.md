# Privacy

Documents do not leave the computer. There is no account, no outbound
analytics, and no CDN for themes, fonts, Mermaid, or KaTeX.

The Dashboard tab shows local counts only (projects, files, agents, session).
Those numbers stay on this computer.

The only outgoing request from the **app** is an optional update check (at
launch if enabled, from the button, or File → Check for Updates…). It talks
to GitHub Releases. On macOS it may download the universal zip and install
it. On Linux and Windows it opens the release page. It does not include
documents.

If you add an External Agent in Settings, that **local CLI** may use the
network under its own terms. The app does not send your notes to a model API
and does not install agents from a registry. Assistant History stores only
prompts you typed, on this computer; × and Clear History delete that file’s entries
and do not send anything off-device.

`asset://` serves files only from registered project roots. `javascript:` and
`data:` links are not followed. A `file:` link opens only when it points at a
file inside the current project; otherwise it is ignored.

Logs record actions and errors, not Markdown contents.
