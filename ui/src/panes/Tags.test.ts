// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import Tags from './Tags.svelte'
import { mockIpc, resetIpc } from '../lib/ipc.mock'
import type { TagSnapshot } from '../lib/generated/core'

const snapshot: TagSnapshot = {
  tags: [{ key: 'work', label: '#work', documents: 2, projects: 2, weight: 4 }],
  totalTags: 1,
  tagOffset: 0,
  projects: 2,
  notices: [],
  notes: [
    { projectId: 'a', projectName: 'Alpha', path: 'note.md', title: 'Note' },
  ],
}
let target: HTMLDivElement
let component: ReturnType<typeof mount>
async function settle() {
  await vi.advanceTimersByTimeAsync(200)
  flushSync()
}
function button(label: string) {
  return Array.from(target.querySelectorAll('button')).find(
    (b) => b.textContent?.trim() === label,
  )!
}
beforeEach(() => {
  vi.useFakeTimers()
  target = document.createElement('div')
  document.body.append(target)
})
afterEach(async () => {
  if (component) await unmount(component)
  target.remove()
  resetIpc()
  vi.useRealTimers()
})

describe('Tags Cloud', () => {
  it('starts across all projects, intersects tags, and opens the original document', async () => {
    const get = vi.fn<(args?: Record<string, unknown>) => TagSnapshot>(
        () => snapshot,
      ),
      open = vi.fn()
    mockIpc({ tags_get: get })
    component = mount(Tags, { target, props: { onopen: open } })
    await settle()
    expect(get.mock.calls[0][0]).toMatchObject({
      refresh: true,
      query: { tags: [], projectId: '' },
    })
    target.querySelector<HTMLButtonElement>('[data-tag="work"]')!.click()
    await settle()
    expect(get.mock.lastCall?.[0]).toMatchObject({
      refresh: false,
      query: { tags: ['work'] },
    })
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    button('Open note').click()
    await settle()
    expect(open).toHaveBeenCalledWith('a', 'note.md')
    button('Clear tags').click()
    await settle()
    expect(get.mock.lastCall?.[0]).toMatchObject({ query: { tags: [] } })
  })
  it('chooses any registered folder even when the global index has no results', async () => {
    const get = vi.fn().mockResolvedValue({ ...snapshot, notes: [], tags: [] })
    mockIpc({
      tags_get: get,
      projects_list: () => ({
        items: [
          {
            id: 'outside-page',
            name: 'Remote folder',
            path: '/fixture/remote',
          },
        ],
        total: 1,
      }),
    })
    component = mount(Tags, { target, props: { onopen: vi.fn() } })
    await settle()
    button('Choose folder…').click()
    flushSync()
    await settle()
    const item = target.querySelector<HTMLButtonElement>(
      '[role="dialog"] li button',
    )!
    item.click()
    flushSync()
    await settle()
    expect(get.mock.lastCall?.[0].query.projectId).toBe('outside-page')
    button('All projects').click()
    flushSync()
    await settle()
    expect(get.mock.lastCall?.[0].query.projectId).toBe('')
  })

  it('reports errors and retries with a fresh scan', async () => {
    const get = vi
      .fn()
      .mockRejectedValueOnce('Folder is unavailable')
      .mockResolvedValue(snapshot)
    mockIpc({ tags_get: get })
    component = mount(Tags, { target, props: { onopen: vi.fn() } })
    await settle()
    expect(target.textContent).toContain('Folder is unavailable')
    button('Refresh').click()
    await settle()
    expect(get.mock.lastCall?.[0].refresh).toBe(true)
    expect(target.textContent).not.toContain('Folder is unavailable')
  })
  it('ignores a late response after filters change', async () => {
    let resolve!: (value: TagSnapshot) => void
    mockIpc({
      tags_get: vi
        .fn()
        .mockImplementationOnce(
          () =>
            new Promise((r) => {
              resolve = r
            }),
        )
        .mockResolvedValue({ ...snapshot, notes: [] }),
    })
    component = mount(Tags, { target, props: { onopen: vi.fn() } })
    await settle()
    const input = target.querySelector<HTMLInputElement>('input')!
    input.value = 'unmatched'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    flushSync()
    await settle()
    resolve(snapshot)
    await settle()
    expect(target.querySelectorAll('[role="option"]')).toHaveLength(0)
  })
  it('reaches virtualized results with End and opens them with Enter', async () => {
    const open = vi.fn()
    mockIpc({
      tags_get: () => ({
        ...snapshot,
        notes: Array.from({ length: 200 }, (_, i) => ({
          ...snapshot.notes[0],
          path: `${i}.md`,
        })),
      }),
    })
    component = mount(Tags, { target, props: { onopen: open } })
    await settle()
    const list = target.querySelector<HTMLElement>('[role="listbox"]')!
    list.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'End', bubbles: true }),
    )
    flushSync()
    expect(
      target.querySelector('[aria-selected="true"]')?.textContent,
    ).toContain('199.md')
    list.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }),
    )
    await settle()
    expect(open).toHaveBeenCalledWith('a', '199.md')
  })

  it('virtualizes results and shows partial scan notices', async () => {
    mockIpc({
      tags_get: () => ({
        ...snapshot,
        notes: Array.from({ length: 200 }, (_, i) => ({
          ...snapshot.notes[0],
          path: `${i}.md`,
        })),
        notices: ['Partial results'],
      }),
    })
    component = mount(Tags, { target, props: { onopen: vi.fn() } })
    await settle()
    expect(
      target.querySelectorAll('[role="option"]').length,
    ).toBeLessThanOrEqual(30)
    expect(target.textContent).toContain('Partial results')
  })
})
