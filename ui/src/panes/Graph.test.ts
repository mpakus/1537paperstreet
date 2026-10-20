// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import Graph from './Graph.svelte'
import { mockIpc, resetIpc } from '../lib/ipc.mock'
import type { GraphSnapshot } from '../lib/generated/core'

const data: GraphSnapshot = {
  nodes: Array.from({ length: 160 }, (_, i) => ({
    path: `note-${i}.md`,
    title: `Note ${i}`,
    folder: '',
    incoming: 1,
    outgoing: 2,
    x: i,
    y: i,
  })),
  edges: [],
  totalNodes: 160,
  totalEdges: 200,
  cachedFiles: 160,
  parsedFiles: 0,
  notices: [],
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

async function show(projectId: string | null, onopen = vi.fn()) {
  vi.useFakeTimers()
  target = document.createElement('div')
  document.body.append(target)
  component = mount(Graph, {
    target,
    props: { projectId, projectName: 'Research', onopen },
  })
  flushSync()
  await vi.advanceTimersByTimeAsync(200)
  flushSync()
}

describe('Graph workspace', () => {
  it('does not request a graph before a project is selected', async () => {
    const get = vi.fn()
    mockIpc({ graph_get: get })
    await show(null)
    expect(target.textContent).toContain('Choose a project')
    expect(get).not.toHaveBeenCalled()
  })

  it('virtualizes notes and opens the keyboard selection', async () => {
    mockIpc({ graph_get: () => data })
    const open = vi.fn()
    await show('project', open)
    expect(target.querySelectorAll('[role="option"]').length).toBeLessThan(40)
    const list = target.querySelector<HTMLElement>('[role="listbox"]')!
    list.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'End', bubbles: true }),
    )
    // Happy DOM does not emit scroll events for programmatic scrolling.
    list.dispatchEvent(new Event('scroll'))
    flushSync()
    expect(target.textContent).toContain('Note 159')
    list.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }),
    )
    expect(open).toHaveBeenCalledExactlyOnceWith('note-159.md')
  })

  it('exposes failures and requests an explicit cache rebuild', async () => {
    const get = vi
      .fn()
      .mockRejectedValueOnce('Could not read graph cache')
      .mockResolvedValue(data)
    mockIpc({ graph_get: get })
    await show('project')
    expect(target.querySelector('[role="alert"]')?.textContent).toContain(
      'Could not read graph cache',
    )
    const button = [...target.querySelectorAll('button')].find(
      (button) => button.textContent === 'Rebuild cache',
    )!
    button.click()
    flushSync()
    await vi.advanceTimersByTimeAsync(200)
    flushSync()
    expect(get.mock.calls[1][0].query.rebuild).toBe(true)
    expect(target.querySelector('[role="alert"]')).toBeNull()
  })
})
