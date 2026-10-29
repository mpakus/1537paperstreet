// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import Settings from './Settings.svelte'
import type { Config } from '../lib/ipc'
import { mockIpc, resetIpc } from '../lib/ipc.mock'

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
  updates: { check_on_launch: true },
  window: {
    sidebar_w: 220,
    tree_w: 260,
    toc_w: 224,
    editor_w: 480,
    diagram_w: 896,
    diagram_h: 576,
    diagram_zoom: 1,
    diagram_x: null,
    diagram_y: null,
    show_in_dock: true,
  },
  editor: { spellcheck: true },
  agents: { servers: [], default_server_id: null, permission: 'allowance' },
} as unknown as Config

let component: ReturnType<typeof mount> | undefined
let target: HTMLDivElement

afterEach(async () => {
  if (component) await unmount(component)
  component = undefined
  target?.remove()
  resetIpc()
})

function open(config: Config, onlive = vi.fn(), onsave = vi.fn()) {
  target = document.createElement('div')
  document.body.append(target)
  mockIpc({ agent_presets: () => [] })
  component = mount(Settings, {
    target,
    props: { config, themes: [], onlive, onsave, onclose: vi.fn() },
  })
  flushSync()
  return { onlive, onsave }
}

function preset(width: number) {
  return target.querySelector<HTMLButtonElement>(
    `button[aria-label="${width} characters per line"]`,
  )!
}

it('applies line-width presets immediately and saves the chosen width', () => {
  const { onlive, onsave } = open(config)
  const group = target.querySelector('[aria-label="Line width presets"]')
  expect(group).not.toBeNull()
  for (const width of [80, 100, 120]) {
    preset(width).click()
    flushSync()
    expect(onlive).toHaveBeenLastCalledWith({
      ...config,
      typography: { ...config.typography, measure_ch: width },
    })
    expect(preset(width).getAttribute('aria-pressed')).toBe('true')
    expect(
      target.querySelector<HTMLInputElement>('#preview-line-width')!.value,
    ).toBe(String(width))
  }
  target.querySelector<HTMLButtonElement>('.done')!.click()
  expect(onsave.mock.lastCall?.[0].typography.measure_ch).toBe(120)
  expect(config.typography.measure_ch).toBe(72)
})

it('preserves custom saved widths when Settings reopens', async () => {
  const initial = {
    ...config,
    typography: { ...config.typography, measure_ch: 93 },
  }
  const { onlive, onsave } = open(initial)
  const input = target.querySelector<HTMLInputElement>('#preview-line-width')
  expect(input).not.toBeNull()
  expect(input!.value).toBe('93')
  expect(input!.min).toBe('40')
  expect(input!.max).toBe('120')
  input!.value = '110'
  input!.dispatchEvent(new Event('input', { bubbles: true }))
  input!.dispatchEvent(new Event('change', { bubbles: true }))
  flushSync()
  expect(onlive.mock.lastCall?.[0].typography.measure_ch).toBe(110)
  expect(
    target.querySelectorAll(
      '[aria-label="Line width presets"] [aria-pressed="true"]',
    ),
  ).toHaveLength(0)
  target.querySelector<HTMLButtonElement>('.done')!.click()
  const saved = onsave.mock.lastCall![0] as Config
  await unmount(component!)
  target.remove()
  open(saved)
  expect(
    target.querySelector<HTMLInputElement>('#preview-line-width')!.value,
  ).toBe('110')
})
