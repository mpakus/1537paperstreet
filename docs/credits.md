# Credits

Licenses and authorship for bundled themes, fonts, and third-party work.

## Themes

Color values are original mappings onto the 1537paperstreet token set. Palettes
follow these sources:

| Theme | Source | License |
| --- | --- | --- |
| Paper Light / Dark | 1537paperstreet | same as this repository |
| Solarized Light / Dark | Ethan Schoonover, [Solarized](https://ethanschoonover.com/solarized/) | MIT |
| Nord | Arctic Ice Studio, [Nord](https://www.nordtheme.com/) | MIT |
| Gruvbox Light / Dark | morhetz, [gruvbox](https://github.com/morhetz/gruvbox) | MIT |
| Catppuccin Latte / Mocha | [Catppuccin](https://github.com/catppuccin/catppuccin) | MIT |
| Tokyo Night | enkia, [tokyo-night-vscode-theme](https://github.com/enkia/tokyo-night-vscode-theme) | MIT |
| GitHub Light / Dark | GitHub Primer | MIT |

## Fonts

The reader prefers fonts already on macOS, then falls back:

- **New York**, **Iowan Old Style**, **Palatino**, **Georgia**, **system-ui** — body
- **JetBrains Mono** ([SIL Open Font License 1.1](https://github.com/JetBrains/JetBrainsMono)), **SF Mono**, **Menlo**, **ui-monospace** — code

JetBrains Mono is not bundled in v1; it is used when the user has it installed.

## Diagrams and mathematics

| Library | Version | License |
| --- | --- | --- |
| [Mermaid](https://github.com/mermaid-js/mermaid) | 11.17.2 | MIT |
| [KaTeX](https://github.com/KaTeX/KaTeX) | 0.19.0 | MIT |

Both are pinned in `ui/package.json` and loaded with a dynamic import only when a
document contains a diagram or a formula. `ui/vendor/mermaid.esm.min.mjs` is the
PLAN § 6 entry; Vite bundles it at build time. There is no CDN at runtime.

## Syntax highlighting

Fenced code blocks are highlighted in Rust with `syntect` (`regex-fancy`) and
the extra dumped syntax set from [`two-face`](https://github.com/CosmicHorrorDev/two-face)
(MIT/Apache-2.0), which adds Elixir and other languages missing from syntect's
defaults. Token colors come from theme CSS variables (`--hl-*`) mixed toward
body text so the preview stays readable.

The Markdown source editor uses [CodeMirror 6](https://codemirror.net/) (MIT),
loaded with a dynamic import the first time Edit or Split is opened.

## Icons

The file tree uses original inline SVG (folder, Markdown document, generic
file) in `ui/src/panes/Tree.svelte`. They are part of this repository and
follow the same license as the app. A larger vendored icon set is not bundled.
