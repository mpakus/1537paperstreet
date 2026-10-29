// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import App from './App.svelte'
import { mockIpc, resetIpc } from './lib/ipc.mock'
import type { MarkdownEditorOptions } from './editor/types'
import type { DocOpenResult, DocumentSource, Project } from './lib/ipc'

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn().mockResolvedValue(undefined) }),
}))
vi.mock('./editor/setup', () => ({
  createMarkdownEditor: (host: HTMLElement, options: MarkdownEditorOptions) => {
    const input = document.createElement('textarea')
    input.setAttribute('aria-label', 'Test document editor')
    input.value = options.doc
    input.oninput = () => options.onChange(input.value)
    host.append(input)
    return {
      setDoc: (text: string) => (input.value = text),
      setFileName: vi.fn(),
      setWritable: vi.fn(),
      setSpellcheck: vi.fn(),
      setLineNumbers: vi.fn(),
      setSoftWrap: vi.fn(),
      setIndentUnit: vi.fn(),
      refresh: vi.fn(),
      destroy: () => input.remove(),
    }
  },
}))

const project: Project = {
  id: 'a',
  name: 'Alpha',
  path: '/fixture/alpha',
  added_at: '',
  last_opened_at: null,
  pinned: false,
  accent: null,
  last_file: 'note.md',
}
const opened: DocOpenResult = {
  meta: {
    security: 'plain',
    projectId: 'a',
    relPath: 'note.md',
    title: 'Note',
    hash: 'saved-hash',
    size: 11,
    writable: true,
    readonlyReason: null,
    sourceOnly: false,
    chunkCount: 1,
    toc: [],
  },
  firstChunk: '<p>Saved #work</p>',
}
const source: DocumentSource = {
  text: 'Saved #work',
  eol: 'lf',
  bom: false,
  trailingNewline: false,
  encoding: 'utf8',
  writable: true,
  readonlyReason: null,
}
let target: HTMLDivElement
let component: ReturnType<typeof mount> | undefined

async function settle() {
  flushSync()
  await vi.dynamicImportSettled()
  await vi.advanceTimersByTimeAsync(250)
  flushSync()
  await vi.dynamicImportSettled()
  flushSync()
}
function button(label: string) {
  const found = Array.from(target.querySelectorAll('button')).find(
    (item) => item.textContent?.trim() === label,
  )
  expect(found, `Button: ${label}`).toBeTruthy()
  return found!
}
beforeEach(() => {
  vi.useFakeTimers()
  target = document.createElement('div')
  document.body.append(target)
})
afterEach(async () => {
  if (component) await unmount(component)
  component = undefined
  target.remove()
  resetIpc()
  vi.useRealTimers()
})

describe('opening notes from Tags Cloud', () => {
  it('preserves and saves unsaved text when reopening the current note', async () => {
    const open = vi.fn(() => opened),
      save = vi.fn(() => ({
        hash: 'new-hash',
        size: 20,
        skipped: false,
      }))
    mockIpc({
      projects_list: () => ({ items: [project], total: 1 }),
      tree_expanded_get: () => [],
      tree_expanded_set: () => undefined,
      tree_read_dir: () => [],
      watch_start: () => undefined,
      watch_set_expanded: () => undefined,
      doc_open: open,
      doc_source: () => source,
      doc_save: save,
      tags_get: () => ({
        tags: [
          { key: 'work', label: '#work', documents: 1, projects: 1, weight: 4 },
        ],
        totalTags: 1,
        tagOffset: 0,
        projects: 1,
        notices: [],
        notes: [
          {
            projectId: 'a',
            projectName: 'Alpha',
            path: 'note.md',
            title: 'Note',
          },
        ],
      }),
    })
    component = mount(App, { target })
    await settle()
    target.querySelector<HTMLElement>('[data-project-id="a"]')!.click()
    await settle()
    button('Edit').click()
    await settle()
    const editor = target.querySelector<HTMLTextAreaElement>('textarea')!
    expect(editor.value).toBe(source.text)
    editor.value = 'Unsaved #work NEW TEXT'
    editor.dispatchEvent(new Event('input', { bubbles: true }))
    flushSync()
    button('Tags').click()
    await settle()
    await settle()
    target
      .querySelector<HTMLElement>(
        '[aria-label="Tagged documents"] [role="option"]',
      )!
      .click()
    flushSync()
    button('Open note').click()
    await settle()

    expect(target.querySelector<HTMLTextAreaElement>('textarea')!.value).toBe(
      'Unsaved #work NEW TEXT',
    )
    expect(open).toHaveBeenCalledTimes(1)
    expect(button('Save').disabled).toBe(false)
    button('Save').click()
    await settle()
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        text: 'Unsaved #work NEW TEXT',
        base_hash: 'saved-hash',
      }),
    )
  })

  it('ignores a pending different note after returning to and editing the current note', async () => {
    let resolveOther!: (value: DocOpenResult) => void
    const other = new Promise<DocOpenResult>((resolve) => {
      resolveOther = resolve
    })
    const open = vi.fn((args?: Record<string, unknown>) =>
      args?.rel_path === 'other.md' ? other : opened,
    )
    const save = vi.fn(() => ({ hash: 'new-hash', size: 20, skipped: false }))
    mockIpc({
      projects_list: () => ({ items: [project], total: 1 }),
      tree_expanded_get: () => [],
      tree_expanded_set: () => undefined,
      tree_read_dir: () => [],
      watch_start: () => undefined,
      watch_set_expanded: () => undefined,
      doc_open: open,
      doc_source: (args) =>
        args?.rel_path === 'other.md'
          ? { ...source, text: 'Other saved content' }
          : source,
      doc_save: save,
      tags_get: () => ({
        tags: [],
        totalTags: 0,
        tagOffset: 0,
        projects: 1,
        notices: [],
        notes: ['note.md', 'other.md'].map((path) => ({
          projectId: 'a',
          projectName: 'Alpha',
          path,
          title: path,
        })),
      }),
    })
    component = mount(App, { target })
    await settle()
    target.querySelector<HTMLElement>('[data-project-id="a"]')!.click()
    await settle()
    button('Edit').click()
    await settle()
    button('Tags').click()
    await settle()
    await settle()
    const chooseNote = (path: string) => {
      const row = Array.from(
        target.querySelectorAll<HTMLElement>(
          '[aria-label="Tagged documents"] [role="option"]',
        ),
      ).find((item) => item.textContent?.includes(path))!
      row.click()
      flushSync()
      button('Open note').click()
    }
    chooseNote('other.md')
    await settle()
    expect(open).toHaveBeenLastCalledWith({
      project_id: 'a',
      rel_path: 'other.md',
    })
    button('Tags').click()
    await settle()
    chooseNote('note.md')
    await settle()
    const editor = target.querySelector<HTMLTextAreaElement>('textarea')!
    editor.value = 'Latest edits to the original note'
    editor.dispatchEvent(new Event('input', { bubbles: true }))
    flushSync()

    resolveOther({
      ...opened,
      meta: { ...opened.meta, relPath: 'other.md', title: 'Other' },
    })
    await settle()
    expect(target.querySelector<HTMLTextAreaElement>('textarea')!.value).toBe(
      'Latest edits to the original note',
    )
    button('Save').click()
    await settle()
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        rel_path: 'note.md',
        text: 'Latest edits to the original note',
      }),
    )
  })
})
