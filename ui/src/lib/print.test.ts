import { describe, expect, it } from 'vitest'

import {
  escapeHtml,
  pdfFileName,
  themeSlug,
  typographyCss,
  wrapPrintHtml,
} from './print'

describe('print', () => {
  it('wraps the article with the active theme', () => {
    const html = wrapPrintHtml({
      title: 'Architecture <draft>',
      themeId: 'tokyo-night',
      themeCss: "[data-theme='tokyo-night']{--bg:#1a1b26;}",
      typographyCss: typographyCss({
        fontSize: 16,
        lineHeight: 1.65,
        measureCh: 72,
        bodyFont: 'New York',
        monoFont: 'Menlo',
      }),
      extraCss: '',
      bodyHtml: '<h1>Hello</h1>',
    })
    expect(html).toContain('data-theme="tokyo-night"')
    expect(html).toContain('Architecture &lt;draft&gt;')
    expect(html).toContain('<h1>Hello</h1>')
    expect(html).toContain('--font-size:16px')
    expect(html).toContain('table-scroll')
    expect(html).toContain('overflow-wrap: anywhere')
  })

  it('builds a pdf name from a markdown path', () => {
    expect(pdfFileName('docs/ARCHITECTURE.md')).toBe('ARCHITECTURE.pdf')
    expect(pdfFileName('note.markdown')).toBe('note.pdf')
    expect(escapeHtml('a&b')).toBe('a&amp;b')
    expect(themeSlug('Nord!')).toBe('Nord')
  })
})
