import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import Preview from './Preview.svelte'

describe('Preview', () => {
  it('renders the empty hint when there is no HTML', () => {
    const { body } = render(Preview, {
      props: {
        html: '',
        emptyMessage: 'Select a Markdown file.',
        onnavigate() {},
      },
    })

    expect(body).toContain('Select a Markdown file.')
  })

  it('renders a table of contents and a read-only banner', () => {
    const { body } = render(Preview, {
      props: {
        html: '<h1 id="hello">Hello</h1>',
        emptyMessage: 'unused',
        banner: 'This file is read-only.',
        toc: [{ level: 1, title: 'Hello', id: 'hello' }],
        onnavigate() {},
      },
    })

    expect(body).toContain('aria-label="Table of contents"')
    expect(body).toContain('aria-label="Resize table of contents"')
    expect(body).toContain('Hello')
    expect(body).toContain('This file is read-only.')
    expect(body).toContain('id="hello"')
    expect(body).toContain('aria-label="Full size"')
    expect(body).toContain('aria-pressed="true"')
    expect(body).toContain('Hide table of contents')
  })

  it('hides the contents list when the toggle is out', () => {
    const { body } = render(Preview, {
      props: {
        html: '<h1 id="hello">Hello</h1>',
        emptyMessage: 'unused',
        toc: [{ level: 1, title: 'Hello', id: 'hello' }],
        tocOpen: false,
        onnavigate() {},
      },
    })

    expect(body).not.toContain('aria-label="Table of contents"')
    expect(body).toContain('Show table of contents')
    expect(body).toContain('aria-pressed="false"')
  })

  it('applies saved reading colors on the preview pane', () => {
    const { body } = render(Preview, {
      props: {
        html: '<p>Note</p>',
        emptyMessage: 'unused',
        previewFont: 'Georgia',
        previewFontSize: 18,
        previewBg: '#112233',
        previewFg: '#abcdef',
        onnavigate() {},
      },
    })

    expect(body).toContain('--read-bg: #112233')
    expect(body).toContain('--read-fg: #abcdef')
    expect(body).toContain('Georgia')
    expect(body).toContain('18px')
  })

  it('applies reading zoom on the article', () => {
    const { body } = render(Preview, {
      props: {
        html: '<p>Note</p>',
        emptyMessage: 'unused',
        readingZoom: 1.2,
        onnavigate() {},
      },
    })

    expect(body).toContain('zoom: 1.2')
  })

  it('embeds sanitized mermaid and math markup in the article', () => {
    const { body } = render(Preview, {
      props: {
        html: '<figure class="mermaid" data-hash="abc"><template>graph TD\nA</template></figure><span class="math math-inline">x</span><pre class="code"><code>let x = 1</code></pre>',
        emptyMessage: 'unused',
        mermaidEnabled: true,
        mathEnabled: true,
        onnavigate() {},
      },
    })

    expect(body).toContain('class="mermaid"')
    expect(body).toContain('data-hash="abc"')
    expect(body).toContain('graph TD')
    expect(body).toContain('class="math math-inline"')
    expect(body).toContain('pre class="code"')
  })
})
