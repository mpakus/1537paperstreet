import { describe, expect, it, vi } from 'vitest'
import { GraphRequest } from './graph-request'
import { mockIpc, resetIpc } from './ipc.mock'
import type { GraphSnapshot } from './generated/core'

const empty: GraphSnapshot = {
  nodes: [],
  edges: [],
  totalNodes: 0,
  totalEdges: 0,
  cachedFiles: 0,
  parsedFiles: 0,
  notices: [],
}
const query = { search: '', focus: null, hideIsolated: false, rebuild: false }

describe('graph requests', () => {
  it('rejects stale replies after a project switch and invalidation', async () => {
    let finish: (value: GraphSnapshot) => void = () => {}
    mockIpc({
      graph_get: (args) =>
        args?.project_id === 'old'
          ? new Promise<GraphSnapshot>((resolve) => {
              finish = resolve
            })
          : empty,
    })
    const request = new GraphRequest()
    const accept = vi.fn()
    const fail = vi.fn()
    const old = request.load('old', query, accept, fail)
    await request.load('new', query, accept, fail)
    finish({ ...empty, totalNodes: 123 })
    await old
    expect(accept).toHaveBeenCalledExactlyOnceWith(empty)
    expect(fail).not.toHaveBeenCalled()
    request.invalidate()
    resetIpc()
  })

  it('shows current IPC errors but ignores errors from a closed pane', async () => {
    mockIpc({ graph_get: () => Promise.reject(new Error('Cache is damaged')) })
    const request = new GraphRequest()
    const fail = vi.fn()
    await request.load('project', query, vi.fn(), fail)
    expect(fail).toHaveBeenCalledWith('Cache is damaged')
    fail.mockClear()
    const pending = request.load('project', query, vi.fn(), fail)
    request.invalidate()
    await pending
    expect(fail).not.toHaveBeenCalled()
    resetIpc()
  })
})
