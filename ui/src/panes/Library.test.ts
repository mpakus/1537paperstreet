// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import Library from './Library.svelte'
import { mockIpc, resetIpc } from '../lib/ipc.mock'
import type { LibrarySnapshot } from '../lib/generated/core'

const snapshot: LibrarySnapshot = {
  revision: 'r1',
  sets: [],
  notices: [],
  noteTypes: ['note', 'spec', 'paper', 'experiment'],
  notes: Array.from({ length: 180 }, (_, i) => ({
    projectId: 'p',
    projectName: 'Research',
    path: `note-${i}.md`,
    title: `Note ${i}`,
    noteType: 'spec',
    status: 'draft',
    tags: ['agents'],
    source: '',
    hash: 'abc',
    freshness: 'unreviewed',
    sections: [
      { id: 'requirements', title: 'Requirements', start: 0, end: 20 },
    ],
  })),
}
let component: ReturnType<typeof mount> | undefined
let target: HTMLDivElement
afterEach(async () => {
  if (component) await unmount(component)
  component = undefined
  target?.remove()
  resetIpc()
  vi.useRealTimers()
})
async function settle() {
  flushSync()
  await vi.advanceTimersByTimeAsync(250)
  flushSync()
}
function button(label: string) {
  return [...target.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === label,
  )!
}
async function show(oncontext = vi.fn()) {
  vi.useFakeTimers()
  target = document.createElement('div')
  document.body.append(target)
  component = mount(Library, {
    target,
    props: {
      projectId: 'p',
      projectName: 'Research',
      onopen: vi.fn(),
      oncontext,
    },
  })
  await settle()
}
describe('Library workspace', () => {
  it('virtualizes documents and lets keyboard users reach the end', async () => {
    mockIpc({ library_get: () => snapshot })
    await show()
    expect(target.querySelectorAll('[role="option"]').length).toBeLessThan(40)
    const list = target.querySelector<HTMLElement>('[role="listbox"]')!
    list.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'End', bubbles: true }),
    )
    list.dispatchEvent(new Event('scroll'))
    flushSync()
    expect(target.textContent).toContain('Note 179')
  })
  it('preserves a selection after a failed save and never sends context automatically', async () => {
    const update = vi
      .fn()
      .mockRejectedValue('Library changed. Refresh before saving.')
    const context = vi.fn()
    mockIpc({
      library_get: () => snapshot,
      library_update: update,
      library_preview: () => ({
        sources: [],
        text: '# Context',
        estimatedTokens: 3,
        complete: true,
      }),
    })
    await show(context)
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    button('Add to selection').click()
    flushSync()
    const name = target.querySelector<HTMLInputElement>(
      '[aria-label="Selection name"]',
    )!
    name.value = 'Release'
    name.dispatchEvent(new Event('input', { bubbles: true }))
    flushSync()
    button('Save selection').click()
    await settle()
    expect(target.textContent).toContain('Library changed')
    expect(name.value).toBe('Release')
    expect(context).not.toHaveBeenCalled()
    button('Preview context').click()
    await settle()
    button('Use in Assistant').click()
    await settle()
    expect(context).toHaveBeenCalledExactlyOnceWith('# Context')
  })
  it('blocks context actions when a source is missing', async () => {
    mockIpc({
      library_get: () => snapshot,
      library_preview: () => ({
        sources: [],
        text: '',
        estimatedTokens: 0,
        complete: false,
      }),
    })
    await show()
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    button('Add to selection').click()
    flushSync()
    button('Preview context').click()
    await settle()
    expect(button('Copy context').disabled).toBe(true)
    expect(button('Use in Assistant').disabled).toBe(true)
  })
  it('reviews the current note hash after a refresh', async () => {
    let current = snapshot
    const update = vi.fn().mockResolvedValue('r2')
    mockIpc({ library_get: () => current, library_update: update })
    await show()
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    current = {
      ...snapshot,
      notes: snapshot.notes.map((note) => ({ ...note, hash: 'new-hash' })),
    }
    button('Refresh').click()
    await settle()
    button('Mark reviewed').click()
    await settle()
    expect(update.mock.calls[0][0].change.source.hash).toBe('new-hash')
  })
  it('rechecks context before handoff and requires review after a file changes', async () => {
    const context = vi.fn()
    let text = 'Original saved text'
    mockIpc({
      library_get: () => snapshot,
      library_preview: () => ({
        sources: [],
        text,
        estimatedTokens: 4,
        complete: true,
      }),
    })
    await show(context)
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    button('Add to selection').click()
    flushSync()
    button('Preview context').click()
    await settle()
    text = 'New saved text'
    button('Use in Assistant').click()
    await settle()
    expect(context).not.toHaveBeenCalled()
    expect(target.textContent).toContain('Sources changed')
    button('Use in Assistant').click()
    await settle()
    expect(context).toHaveBeenCalledExactlyOnceWith('New saved text')
  })
  it('keeps unsaved selections until replacement is explicitly confirmed', async () => {
    mockIpc({ library_get: () => snapshot })
    await show()
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    button('Add to selection').click()
    flushSync()
    button('New selection').click()
    flushSync()
    expect(target.textContent).toContain('Replace the unsaved selection?')
    button('Keep editing').click()
    flushSync()
    expect(
      target.querySelector('[aria-label="Remove source 1"]'),
    ).not.toBeNull()
    const pane = component as unknown as { requestClose: () => boolean }
    expect(pane.requestClose()).toBe(false)
    flushSync()
    expect(target.textContent).toContain('Close Library and discard')
  })
  it('does not silently expand a missing section to the whole document', async () => {
    let current = snapshot
    mockIpc({ library_get: () => current })
    await show()
    target.querySelector<HTMLElement>('[role="option"]')!.click()
    flushSync()
    const include =
      target.querySelector<HTMLSelectElement>('.inspector select')!
    include.value = 'requirements'
    include.dispatchEvent(new Event('change', { bubbles: true }))
    flushSync()
    current = {
      ...snapshot,
      notes: snapshot.notes.map((note) => ({ ...note, sections: [] })),
    }
    button('Refresh').click()
    await settle()
    expect(button('Add to selection').disabled).toBe(true)
  })
})
