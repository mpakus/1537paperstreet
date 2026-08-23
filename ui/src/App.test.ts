import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import App from './App.svelte'

describe('App', () => {
  it('renders the empty file tree and drop hint', () => {
    const { body, head } = render(App)

    expect(head).toContain('<title>1537paperstreet</title>')
    expect(body).toContain('data-tauri-drag-region')
    expect(body).toContain('aria-label="Projects"')
    expect(body).toContain('aria-label="File tree"')
    expect(body).toContain('Your Markdown projects will appear here.')
    expect(body).toContain('Drop a Markdown file or a folder to open it.')
    expect(body).toContain('aria-label="Document"')
    expect(body).toContain('Preview')
    expect(body).toContain('Edit')
    expect(body).toContain('Split')
    expect(body).toContain('Save')
    expect(body).toContain('Export')
    expect(body).not.toContain('title="Settings (⌘,)"')
    expect(body).toContain('title="Assistant (⌘⌥A)"')
    expect(body).toContain('aria-label="Workspace"')
    expect(body).toContain('Dashboard')
    expect(body).toContain('>Assistant</button')
    expect(body).toContain('aria-label="Hide projects"')
    expect(body).toContain('Open a Markdown file to preview, edit, or export.')
    expect(body).toContain('Open Folder…')
  })
})
