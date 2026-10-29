# Themes

Twelve built-in themes (Paper, Solarized, Nord, Gruvbox, Catppuccin, Tokyo
Night, GitHub — light and dark pairs where they exist). Colors live in CSS
variables; the UI only sets `data-theme`.

⌘⌥T toggles light and dark for the current session and follows the system
appearance until you pin a theme yourself. That does not rewrite the
light/dark pair in Settings.

Open Settings (⌘, or File → Settings…): Light / Dark sections show every
available theme as a palette card; the choice applies immediately.

Preview and Split can override reading font, size, text color, and background
— separate from the chrome theme. Empty values in `config.json`
(`preview_font`, `preview_font_size: 0`, `preview_bg`, `preview_fg`) mean
“use the theme.”

In **Settings → Preview & Split → Line width (characters)**, choose **80**,
**100** or **120**, or enter any value from 40 to 120. Changes save immediately
and apply to Preview and the preview pane in Split. The default is 72. Line
length is approximate: it depends on the font, and narrower panes wrap sooner.

A custom theme is JSON in `~/.1537paperstreet/themes/`. Required fields:
`id`, `name`, `appearance` (`light` or `dark`), and a `tokens` object with
kebab-case keys (`bg`, `fg`, `accent`, `hl-kw`, …). An incomplete or broken
file is skipped; other themes keep working. Token examples live in
`crates/ps-core/themes/`.

Palette licenses: [`docs/credits.md`](../credits.md).
