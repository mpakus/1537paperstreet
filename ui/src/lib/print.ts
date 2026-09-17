import syntaxCss from '../styles/syntax.css?raw'
import gfmCss from '../styles/gfm.css?raw'

import { exportPdf } from './ipc'

const PRINT_CSS = `
html, body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
}
article {
  font-family: var(--font-body);
  font-size: var(--font-size);
  line-height: var(--line-height);
  max-width: calc(var(--measure-ch) * 1ch);
  margin: 0 auto;
  padding: 24px 20px;
}
article a { color: var(--accent); }
article img, article video, article svg { max-width: 100%; height: auto; }
article pre, article code { font-family: var(--font-mono); background: var(--code-bg); }
article pre { padding: 12px; overflow: auto; border-radius: 6px; }
article table { border-collapse: collapse; width: max-content; min-width: 100%; }
article .table-scroll { max-width: 100%; overflow-x: auto; }
article th, article td {
  border: 1px solid var(--border);
  padding: 6px 10px;
  text-align: left;
  vertical-align: top;
}
article .cell {
  max-width: 24em;
  overflow-wrap: anywhere;
  word-break: break-word;
}
article figure.mermaid {
  margin: 16px 0;
  padding: 12px;
  overflow: auto;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: 6px;
}
article figure.mermaid svg { max-width: 100%; height: auto; }
@page { margin: 16mm; }
`

/** Escapes text for an HTML text node or attribute. */
export function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

/** Keeps only a theme id slug. */
export function themeSlug(value: string): string {
  return value.replace(/[^a-z0-9-]/gi, '') || 'paper-light'
}

/** Builds a self-contained HTML document for WKWebView PDF capture. */
export function wrapPrintHtml(options: {
  title: string
  themeId: string
  themeCss: string
  typographyCss: string
  extraCss?: string
  bodyHtml: string
}): string {
  const themeId = themeSlug(options.themeId)
  const title = escapeHtml(options.title)
  return `<!DOCTYPE html><html data-theme="${themeId}"><head><meta charset="utf-8"><title>${title}</title><style>${options.themeCss}\n${options.typographyCss}\n${PRINT_CSS}\n${syntaxCss}\n${gfmCss}\n${options.extraCss ?? ''}</style></head><body><article class="preview">${options.bodyHtml}</article></body></html>`
}

/** CSS variables for measure and type, taken from the live config. */
export function typographyCss(options: {
  fontSize: number
  lineHeight: number
  measureCh: number
  bodyFont: string
  monoFont: string
}): string {
  return `html{--font-size:${options.fontSize}px;--line-height:${options.lineHeight};--measure-ch:${options.measureCh};--font-body:${options.bodyFont},serif;--font-mono:${options.monoFont},ui-monospace,monospace;}`
}

/** Default PDF file name from a project-relative Markdown path. */
export function pdfFileName(relPath: string): string {
  const base = relPath.split('/').filter(Boolean).at(-1) ?? 'document.md'
  return `${base.replace(/\.(md|markdown|mdown|mdwn)$/i, '')}.pdf`
}

async function inlineImages(html: string): Promise<string> {
  if (typeof DOMParser === 'undefined') {
    return html
  }
  const document = new DOMParser().parseFromString(
    `<div id="root">${html}</div>`,
    'text/html',
  )
  const root = document.getElementById('root')
  if (!root) {
    return html
  }
  for (const image of root.querySelectorAll('img')) {
    const src = image.getAttribute('src')
    if (!src || src.startsWith('data:')) {
      continue
    }
    try {
      const response = await fetch(src)
      if (!response.ok) {
        continue
      }
      const blob = await response.blob()
      image.setAttribute('src', await blobToDataUrl(blob))
    } catch {
      // Leave the original src; WKWebView may still resolve asset://.
    }
  }
  return root.innerHTML
}

function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(new Error('Could not read the image.'))
    reader.readAsDataURL(blob)
  })
}

/** Save-dialog + PDF write of the current preview HTML. */
export async function exportDocumentPdf(options: {
  relPath: string
  articleHtml: string
  themeId: string
  themeCss: string
  fontSize: number
  lineHeight: number
  measureCh: number
  bodyFont: string
  monoFont: string
}): Promise<boolean> {
  const { save } = await import('@tauri-apps/plugin-dialog')
  const path = await save({
    defaultPath: pdfFileName(options.relPath),
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!path) {
    return false
  }
  const bodyHtml = await inlineImages(options.articleHtml)
  const html = wrapPrintHtml({
    title: pdfFileName(options.relPath).replace(/\.pdf$/i, ''),
    themeId: options.themeId,
    themeCss: options.themeCss,
    typographyCss: typographyCss(options),
    bodyHtml,
  })
  await exportPdf(path, html)
  return true
}
