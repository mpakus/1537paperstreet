import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import ChromeToolbar from './ChromeToolbar.svelte'

describe('ChromeToolbar', () => {
  it('shows view modes and file actions', () => {
    const { body } = render(ChromeToolbar, {
      props: {
        mode: 'preview',
        canSave: true,
        onmode() {},
        oncommand() {},
      },
    })

    expect(body).toContain('aria-label="Document"')
    expect(body).toContain('aria-label="View"')
    expect(body).toContain('aria-label="File"')
    expect(body).toContain('Preview')
    expect(body).toContain('Edit')
    expect(body).toContain('Split')
    expect(body).toContain('Save')
    expect(body).toContain('Export')
    expect(body).toContain('title="Board"')
    expect(body).toContain('>AI</button')
    expect(body).not.toContain('Settings')
    expect(body).toContain('Open a file to preview, edit, or export.')
    expect(body).toContain('aria-label="Reading"')
    expect(body).toContain('aria-label="Text size"')
    expect(body).toContain('aria-label="Larger text"')
    expect(body).toContain('aria-label="Zoom in"')
    expect(body).toContain('100%')
    expect(body).not.toContain('aria-label="Editor"')
  })

  it('hides reading controls in editor mode', () => {
    const { body } = render(ChromeToolbar, {
      props: {
        mode: 'editor',
        canFormat: true,
        onmode() {},
        oncommand() {},
      },
    })

    expect(body).not.toContain('aria-label="Reading"')
    expect(body).toContain('aria-label="Editor"')
  })

  it('shows formatting controls in edit mode', () => {
    const { body } = render(ChromeToolbar, {
      props: {
        mode: 'editor',
        canFormat: true,
        onmode() {},
        oncommand() {},
      },
    })

    expect(body).toContain('aria-label="Editor"')
    expect(body).toContain('aria-label="Bold"')
  })

  it('hides formatting controls for source files', () => {
    const { body } = render(ChromeToolbar, {
      props: {
        mode: 'editor',
        canFormat: false,
        hasDocument: true,
        onmode() {},
        oncommand() {},
      },
    })

    expect(body).not.toContain('aria-label="Editor"')
    expect(body).not.toContain('aria-label="Bold"')
  })
})
