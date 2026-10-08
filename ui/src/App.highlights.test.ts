// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import App from './App.svelte'
import { mockIpc, resetIpc } from './lib/ipc.mock'

vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => 'test' }))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: async () => {} }),
}))
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }),
}))

let instance: ReturnType<typeof mount> | undefined
afterEach(async () => {
  if (instance) await unmount(instance)
  instance = undefined
  resetIpc()
  Reflect.deleteProperty(window, '__TAURI_INTERNALS__')
  document.body.innerHTML = ''
  window.getSelection()?.removeAllRanges()
})

it('keeps edits made during Save and supports undo while remaining in Preview', async () => {
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    value: {},
    configurable: true,
  })
  const config = {
    appearance: {
      theme: 'paper-light',
      theme_dark: 'paper-dark',
      follow_system: false,
    },
    typography: {
      body_font: 'serif',
      mono_font: 'monospace',
      font_size: 16,
      line_height: 1.65,
      measure_ch: 72,
    },
    viewer: {
      default_mode: 'preview',
      show_toc: false,
      mermaid_enabled: false,
      math_enabled: false,
    },
    files: { confirm_delete: true, show_hidden: false },
    updates: { check_on_launch: false },
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
    },
    editor: { spellcheck: false, sync_scroll: true },
  }
  const block = (inner: string) =>
    `<section data-block="0" data-hash="abc" data-src-line="1"><p>${inner}</p></section>`
  const rendered = block('Visible')
  let finishSave: (value: unknown) => void = () => {
    throw new Error('Save has not started')
  }
  const save = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishSave = resolve
        }),
    )
    .mockResolvedValue({ hash: 'saved-again', size: 30, skipped: false })
  const highlight = vi
    .fn()
    .mockResolvedValueOnce('Highlighted draft')
    .mockResolvedValueOnce('Highlighted draft plus unsaved')
  const preview = vi.fn().mockResolvedValue({ html: rendered, toc: [] })
  mockIpc({
    config_get: () => config,
    themes_css: () => '',
    themes_list: () => [],
    projects_list: () => ({
      items: [
        {
          id: 'p',
          name: 'Notes',
          path: '/test',
          available: true,
          last_file: 'note.md',
          pinned: false,
        },
      ],
      total: 1,
    }),
    session_restore: () => ({
      project_id: null,
      notices: [],
      tabs: [],
      workspace_tabs: [],
    }),
    session_set: () => {},
    tree_expanded_get: () => [],
    tree_expanded_set: () => {},
    tree_read_dir: () => [],
    watch_start: () => {},
    watch_set_expanded: () => {},
    doc_open: () => ({
      meta: {
        projectId: 'p',
        relPath: 'note.md',
        title: 'Note',
        hash: 'original',
        size: 7,
        writable: true,
        readonlyReason: null,
        sourceOnly: false,
        chunkCount: 1,
        toc: [],
      },
      firstChunk: rendered,
    }),
    doc_source: () => ({
      text: 'Visible',
      writable: true,
      readonlyReason: null,
      eol: 'lf',
      bom: false,
      trailingNewline: false,
    }),
    doc_preview: preview,
    doc_highlight_source: (args) => ({
      hash: 'nonce',
      html: block(
        `<span data-ps-map="nonce:0:${String(args?.text).length}">Visible</span>`,
      ),
    }),
    doc_highlight: highlight,
    doc_save: save,
  })
  const target = document.createElement('div')
  document.body.append(target)
  instance = mount(App, { target })
  const settle = async (check: () => void) =>
    vi.waitFor(
      () => {
        flushSync()
        check()
      },
      { timeout: 3000 },
    )
  await settle(() =>
    expect(target.querySelector('article')?.textContent).toBe('Visible'),
  )

  async function apply() {
    const range = document.createRange()
    range.selectNodeContents(target.querySelector('article p')!)
    window.getSelection()?.removeAllRanges()
    window.getSelection()?.addRange(range)
    window.getSelection()!.getRangeAt(0).getBoundingClientRect = () =>
      ({ left: 100, bottom: 100 }) as DOMRect
    target
      .querySelector('.preview')!
      .dispatchEvent(new Event('pointerup', { bubbles: true }))
    flushSync()
    target
      .querySelector<HTMLButtonElement>('[aria-label="green highlight"]')!
      .click()
    await settle(() =>
      expect(target.querySelector('.highlight-palette')).toBeNull(),
    )
  }
  const saveButton = () =>
    Array.from(target.querySelectorAll('button')).find(
      (button) => button.textContent?.trim() === 'Save',
    )!
  await apply()
  expect(target.querySelector('.cm-content')?.textContent).toBe(
    'Highlighted draft',
  )
  saveButton().click()
  await settle(() => expect(save).toHaveBeenCalledTimes(1))
  await apply()
  expect(highlight.mock.calls[1]?.[0]?.text).toBe('Highlighted draft')
  finishSave({ hash: 'saved-first', size: 17, skipped: false })
  await new Promise((resolve) => setTimeout(resolve, 0))
  await settle(() =>
    expect(target.querySelector('.cm-content')?.textContent).toBe(
      'Highlighted draft plus unsaved',
    ),
  )
  saveButton().click()
  await settle(() => expect(save).toHaveBeenCalledTimes(2))
  expect(save.mock.calls[1]?.[0]).toMatchObject({
    text: 'Highlighted draft plus unsaved',
    base_hash: 'saved-first',
  })
  window.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true }),
  )
  await settle(() =>
    expect(target.querySelector('.cm-content')?.textContent).toBe(
      'Highlighted draft',
    ),
  )
  expect(target.querySelector('.editor')?.classList.contains('is-hidden')).toBe(
    true,
  )
})
