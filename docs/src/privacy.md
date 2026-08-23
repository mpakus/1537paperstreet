# Privacy

Documents do not leave the computer. There is no account, no analytics, and
no CDN for themes, fonts, Mermaid, or KaTeX.

The only outgoing request from the **app** is an update check from the button or
File → Check for Updates…. It talks to GitHub Releases and does not include
documents.

If you add an External Agent in Settings, that **local CLI** may use the
network under its own terms. The app does not send your notes to a model API
and does not install agents from a registry.

`asset://` serves files only from registered project roots. `javascript:`,
`data:`, and local `file:` links in a document are not used as navigation.

Logs record actions and errors, not Markdown contents.
