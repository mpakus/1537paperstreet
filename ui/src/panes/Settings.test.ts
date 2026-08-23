import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import type { Config, ThemeInfo } from '../lib/ipc'
import Settings from './Settings.svelte'

const config = {
  appearance: {
    theme: 'paper-light',
    theme_dark: 'paper-dark',
    follow_system: true,
  },
  typography: {
    body_font: 'New York',
    mono_font: 'JetBrains Mono',
    font_size: 16,
    line_height: 1.65,
    measure_ch: 72,
  },
  viewer: {
    show_toc: true,
    mermaid_enabled: true,
    math_enabled: true,
    default_mode: 'preview',
    preview_font: '',
    preview_font_size: 0,
    preview_bg: '',
    preview_fg: '',
  },
  files: { confirm_delete: true, show_hidden: false },
  window: {
    sidebar_w: 220,
    tree_w: 260,
    toc_w: 224,
    editor_w: 480,
    show_in_dock: true,
  },
  editor: { spellcheck: true },
  agents: { servers: [], default_server_id: null, permission: 'allowance' },
} as unknown as Config

const themes: ThemeInfo[] = [
  {
    id: 'paper-light',
    name: 'Paper Light',
    appearance: 'light',
    builtin: true,
  },
  { id: 'paper-dark', name: 'Paper Dark', appearance: 'dark', builtin: true },
]

describe('Settings', () => {
  it('shows theme swatches and typography', () => {
    const { body } = render(Settings, {
      props: {
        config,
        themes,
        onsave() {},
        onclose() {},
      },
    })

    expect(body).toContain('aria-label="Settings"')
    expect(body).toContain('Themes')
    expect(body).toContain('aria-label="Light theme"')
    expect(body).toContain('Paper Light')
    expect(body).toContain('Paper Dark')
    expect(body).toContain('data-theme="paper-light"')
    expect(body).toContain('Typography')
    expect(body).toContain('Preview &amp; Split')
    expect(body).toContain('Keep icon in Dock when the window is hidden')
    expect(body).toContain('Custom reading colors')
    expect(body).toContain('Done')
    expect(body).toContain('External Agents')
    expect(body).toContain('Add Custom Agent')
  })
})
