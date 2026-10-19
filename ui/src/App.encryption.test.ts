// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { EditorView } from '@codemirror/view'
import { afterEach, expect, it, vi } from 'vitest'
import App from './App.svelte'
import { mockIpc, resetIpc } from './lib/ipc.mock'

const listeners = vi.hoisted(
  () => new Map<string, (event: { payload: unknown }) => void>(),
)

vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => 'test' }))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: async () => {} }),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (
    name: string,
    callback: (event: { payload: unknown }) => void,
  ) => {
    listeners.set(name, callback)
    return () => {
      listeners.delete(name)
    }
  },
}))
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }),
}))

let instance: ReturnType<typeof mount> | undefined
afterEach(async () => {
  if (instance) await unmount(instance)
  instance = undefined
  resetIpc()
  listeners.clear()
  Reflect.deleteProperty(window, '__TAURI_INTERNALS__')
  document.body.innerHTML = ''
})
const settle = (check: () => void) =>
  vi.waitFor(
    () => {
      flushSync()
      check()
    },
    { timeout: 3000 },
  )
function click(text: string, root: ParentNode = document) {
  const button = [...root.querySelectorAll('button')].find(
    (button) => button.textContent?.trim() === text,
  )
  expect(button, text).toBeTruthy()
  button!.click()
  flushSync()
}
function fill(label: string, value: string) {
  const input = document.querySelector<HTMLInputElement>(
    `[aria-label="${label}"]`,
  )!
  input.value = value
  input.dispatchEvent(new Event('input', { bubbles: true }))
  flushSync()
}
function submit() {
  document
    .querySelector('dialog form')!
    .dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  flushSync()
}
async function setup(options: { plainPath?: string; empty?: boolean } = {}) {
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

  let plain = Boolean(options.plainPath)
  let locked = !plain
  let text = options.empty ? '' : 'Protected source'
  let currentPath = options.plainPath ?? 'secret.md.age'
  const unlock = vi.fn((args) => {
    if (args?.passphrase !== 'correct password')
      throw new Error('Incorrect password or damaged file.')
    locked = false
  })
  const lock = vi.fn(() => {
    locked = true
  })
  const save = vi.fn((args) => {
    text = String(args?.text)
    return { hash: 'saved', size: 140, skipped: false }
  })
  const snapshot = vi.fn()
  const finishClose = vi.fn()
  const create = vi.fn((args) => {
    currentPath = String(args?.rel_path)
    locked = false
    text = ''
  })
  const encrypt = vi.fn((args) => {
    currentPath = String(args?.destination)
    text = String(args?.text)
    plain = false
    locked = false
  })
  const deleteOriginal = vi.fn()
  const plaintextCreate = vi.fn()
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
          last_file: currentPath,
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
    tree_read_dir: () => [
      {
        name: currentPath.split('/').at(-1),
        relPath: currentPath,
        kind: 'file',
      },
    ],
    watch_start: () => {},
    watch_set_expanded: () => {},
    doc_open: () => ({
      meta: {
        security: plain ? 'plain' : locked ? 'locked' : 'unlocked',
        projectId: 'p',
        relPath: currentPath,
        title: currentPath,
        hash: 'base',
        size: 130,
        writable: !locked,
        readonlyReason: null,
        sourceOnly: locked,
        chunkCount: locked ? 0 : 1,
        toc: [],
      },
      firstChunk:
        locked || !text
          ? null
          : `<section data-block="0" data-hash="a"><p>${text}</p></section>`,
    }),
    doc_source: () => ({
      text,
      eol: 'lf',
      bom: false,
      trailingNewline: false,
      encoding: 'utf8',
      writable: true,
      readonlyReason: null,
    }),
    doc_preview: (args) => ({ html: `<p>${String(args?.text)}</p>`, toc: [] }),
    doc_unlock: unlock,
    doc_lock: lock,
    doc_lock_all: () => {
      locked = true
    },
    doc_save: save,
    doc_snapshot_encrypted: snapshot,
    doc_stat: () => ({ hash: 'external-version', size: 140 }),
    finish_window_close: finishClose,
    doc_create_encrypted: create,
    doc_encrypt_existing: encrypt,
    doc_delete_original: deleteOriginal,
    fs_create_file: plaintextCreate,
  })
  const target = document.createElement('div')
  document.body.append(target)
  instance = mount(App, { target })
  await settle(() =>
    expect(target.textContent).toContain(
      plain ? 'Protected source' : 'Unlock note',
    ),
  )
  return {
    target,
    unlock,
    lock,
    save,
    snapshot,
    create,
    plaintextCreate,
    encrypt,
    deleteOriginal,
    finishClose,
  }
}
async function unlockNote() {
  click('Unlock note')
  fill('Password', 'correct password')
  submit()
  await settle(() =>
    expect(document.querySelector('article')?.textContent).toContain(
      'Protected source',
    ),
  )
}

it('keeps locked content absent, rejects a bad password, and clears decrypted content on Lock', async () => {
  const { target, unlock, lock } = await setup()
  expect(target.textContent).not.toContain('Protected source')
  click('Unlock note')
  fill('Password', 'wrong')
  submit()
  await settle(() => expect(target.textContent).toContain('Incorrect password'))
  expect(target.textContent).not.toContain('Protected source')
  expect(
    target.querySelector<HTMLInputElement>('[aria-label="Password"]')!.value,
  ).toBe('')
  fill('Password', 'correct password')
  submit()
  await settle(() =>
    expect(target.querySelector('article')?.textContent).toContain(
      'Protected source',
    ),
  )
  expect(unlock).toHaveBeenCalledTimes(2)
  click('🔒 Encrypted')
  click('Lock note')
  await settle(() => expect(lock).toHaveBeenCalledTimes(1))
  await settle(() =>
    expect(target.textContent).not.toContain('Protected source'),
  )
  expect(target.querySelector('.cm-content')).toBeNull()
})

it('retains dirty text after failed Save and snapshots ciphertext before Discard and Lock', async () => {
  const { target, save, lock, snapshot } = await setup()
  await unlockNote()
  click('Edit')
  await settle(() => expect(target.querySelector('.cm-content')).toBeTruthy())
  const view = EditorView.findFromDOM(target.querySelector('.cm-content')!)!
  view.dispatch({
    changes: {
      from: 0,
      to: view.state.doc.length,
      insert: 'Unsaved private edit',
    },
  })
  flushSync()
  save.mockImplementationOnce(() => {
    throw new Error('Disk full')
  })
  click('🔒 Encrypted')
  click('Lock note')
  await settle(() =>
    expect(target.textContent).toContain('Save changes before closing?'),
  )
  click('Save', target.querySelector('dialog')!)
  await settle(() => expect(target.textContent).toContain('Disk full'))
  expect(lock).not.toHaveBeenCalled()
  expect(target.querySelector('.cm-content')?.textContent).toContain(
    'Unsaved private edit',
  )
  click('🔒 Encrypted')
  click('Lock note')
  await settle(() =>
    expect(target.textContent).toContain('Save changes before closing?'),
  )
  click('Cancel', target.querySelector('dialog')!)
  await settle(() => expect(target.querySelector('dialog')).toBeNull())
  expect(lock).not.toHaveBeenCalled()
  click('🔒 Encrypted')
  click('Lock note')
  await settle(() =>
    expect(target.textContent).toContain('Save changes before closing?'),
  )
  click('Discard', target.querySelector('dialog')!)
  await settle(() => expect(lock).toHaveBeenCalledTimes(1))
  expect(snapshot).toHaveBeenCalledWith(
    expect.objectContaining({
      text: 'Unsaved private edit',
      rel_path: 'secret.md.age',
    }),
  )
  expect(snapshot.mock.invocationCallOrder[0]).toBeLessThan(
    lock.mock.invocationCallOrder[0]!,
  )
  await settle(() =>
    expect(target.textContent).not.toContain('Unsaved private edit'),
  )
})

it('creates a protected note without invoking the plaintext file creator', async () => {
  const { target, create, plaintextCreate } = await setup()
  click('🔒 Encrypted')
  click('New encrypted note')
  fill('File path', 'new.md.age')
  fill('Password', 'test-only secret')
  fill('Confirm password', 'test-only secret')
  submit()
  await settle(() => expect(create).toHaveBeenCalledTimes(1))
  expect(create).toHaveBeenCalledWith({
    project_id: 'p',
    rel_path: 'new.md.age',
    text: '',
    passphrase: 'test-only secret',
  })
  expect(plaintextCreate).not.toHaveBeenCalled()
  await settle(() => expect(target.querySelector('.cm-content')).toBeTruthy())
  expect(target.querySelector('input[type="password"]')).toBeNull()
})

it('keeps the window open if recovery fails and clears it only after an explicit discard', async () => {
  const { target, snapshot, finishClose } = await setup()
  await unlockNote()
  click('Edit')
  await settle(() => expect(target.querySelector('.cm-content')).toBeTruthy())
  const view = EditorView.findFromDOM(target.querySelector('.cm-content')!)!
  view.dispatch({
    changes: {
      from: 0,
      to: view.state.doc.length,
      insert: 'Window-close private edit',
    },
  })
  flushSync()
  snapshot.mockImplementationOnce(() => {
    throw new Error('Recovery disk full')
  })
  listeners.get('menu://action')!({ payload: 'window-close' })
  await settle(() =>
    expect(target.textContent).toContain('Save changes before closing?'),
  )
  click('Discard', target.querySelector('dialog')!)
  await settle(() => expect(target.textContent).toContain('Recovery disk full'))
  expect(finishClose).not.toHaveBeenCalled()
  expect(target.querySelector('.cm-content')?.textContent).toContain(
    'Window-close private edit',
  )
  listeners.get('menu://action')!({ payload: 'window-close' })
  await settle(() =>
    expect(target.textContent).toContain('Save changes before closing?'),
  )
  click('Discard', target.querySelector('dialog')!)
  await settle(() => expect(finishClose).toHaveBeenCalledWith({ quit: false }))
  expect(target.textContent).not.toContain('Window-close private edit')
})

it('preserves dirty text before reloading externally changed encrypted content', async () => {
  const { target, snapshot } = await setup()
  await unlockNote()
  click('Edit')
  await settle(() => expect(target.querySelector('.cm-content')).toBeTruthy())
  const view = EditorView.findFromDOM(target.querySelector('.cm-content')!)!
  view.dispatch({
    changes: {
      from: 0,
      to: view.state.doc.length,
      insert: 'Private edit before reload',
    },
  })
  flushSync()
  listeners.get('fs://changed')!({
    payload: {
      projectId: 'p',
      update: { pathsChanged: { paths: ['secret.md.age'] } },
    },
  })
  await settle(() =>
    expect(target.textContent).toContain('File changed on disk'),
  )
  snapshot.mockImplementationOnce(() => {
    throw new Error('Recovery unavailable')
  })
  click('Reload')
  await settle(() =>
    expect(target.textContent).toContain('Recovery unavailable'),
  )
  expect(target.querySelector('.cm-content')?.textContent).toContain(
    'Private edit before reload',
  )
  expect(snapshot).toHaveBeenCalledWith(
    expect.objectContaining({ text: 'Private edit before reload' }),
  )
})

async function encryptNote() {
  click('Protect')
  click('Encrypt this note')
  fill('Password', 'test-only secret')
  fill('Confirm password', 'test-only secret')
  submit()
}

it('appends .age to the exact original path and shows the encrypted copy before offering deletion', async () => {
  const { target, encrypt, create, deleteOriginal } = await setup({
    plainPath: 'notes/Original.MARKDOWN',
  })
  click('Protect')
  click('Encrypt this note')
  expect(
    target.querySelector<HTMLInputElement>('[aria-label="File path"]')!.value,
  ).toBe('notes/Original.MARKDOWN.age')
  fill('Password', 'test-only secret')
  fill('Confirm password', 'test-only secret')
  submit()
  await settle(() =>
    expect(target.textContent).toContain('Delete the original?'),
  )
  expect(encrypt).toHaveBeenCalledWith(
    expect.objectContaining({
      rel_path: 'notes/Original.MARKDOWN',
      destination: 'notes/Original.MARKDOWN.age',
      text: 'Protected source',
    }),
  )
  expect(create).not.toHaveBeenCalled()
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
  expect(deleteOriginal).not.toHaveBeenCalled()
  click('Keep original')
  await settle(() => expect(target.querySelector('dialog')).toBeNull())
  expect(deleteOriginal).not.toHaveBeenCalled()
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
})

it('deletes only after confirmation and keeps the protected preview and a retry choice after a deletion error', async () => {
  const { target, encrypt, deleteOriginal } = await setup({
    plainPath: 'original.md',
  })
  await encryptNote()
  await settle(() =>
    expect(target.textContent).toContain('Delete the original?'),
  )
  deleteOriginal.mockRejectedValueOnce(new Error('Original changed on disk.'))
  click('Delete original')
  await settle(() =>
    expect(target.textContent).toContain('Original changed on disk.'),
  )
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
  click('Delete original')
  await settle(() => expect(target.querySelector('dialog')).toBeNull())
  expect(deleteOriginal).toHaveBeenLastCalledWith({
    project_id: 'p',
    rel_path: 'original.md.age',
  })
  expect(encrypt).toHaveBeenCalledTimes(1)
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
})

it('does not offer deletion or create an empty note when conversion fails', async () => {
  const { target, encrypt, create, deleteOriginal } = await setup({
    plainPath: 'original.md',
  })
  encrypt.mockRejectedValueOnce(new Error('Encryption failed'))
  await encryptNote()
  await settle(() => expect(target.textContent).toContain('Encryption failed'))
  expect(target.textContent).not.toContain('Delete the original?')
  expect(create).not.toHaveBeenCalled()
  expect(deleteOriginal).not.toHaveBeenCalled()
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
})

it('shows an empty-note state after unlocking an empty file instead of asking to select a file', async () => {
  const { target } = await setup({ empty: true })
  click('Unlock note')
  fill('Password', 'correct password')
  submit()
  await settle(() => expect(target.querySelector('dialog')).toBeNull())
  expect(target.textContent).not.toContain('Select a file in the tree')
  expect(target.textContent).toContain('This note is empty.')
})

it('reopens a closed encrypted note with one click on its filename', async () => {
  const { target } = await setup()
  await unlockNote()
  target
    .querySelector<HTMLButtonElement>('[aria-label="Close secret.md.age"]')!
    .click()
  await settle(() => expect(target.querySelector('[role="tab"]')).toBeNull())
  const filename = target.querySelector<HTMLElement>(
    '[data-rel="secret.md.age"] .name',
  )!
  filename.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }))
  await settle(() => expect(target.textContent).toContain('Unlock note'))
  expect(target.textContent).not.toContain('Select a file in the tree')
  await unlockNote()
  expect(target.querySelector('article')?.textContent).toContain(
    'Protected source',
  )
})
