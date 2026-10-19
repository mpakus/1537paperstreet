// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vitest'
import { mockIpc, resetIpc } from './ipc.mock'
import { renderMermaidFigure } from './diagrams'

vi.mock('../../vendor/mermaid.esm.min.mjs', () => ({
  default: {
    initialize: vi.fn(),
    render: async () => ({ svg: '<svg><text>Private diagram</text></svg>' }),
  },
}))
afterEach(resetIpc)
it('renders protected diagrams without reading or writing disk or shared memory caches', async () => {
  const get = vi.fn(() => null)
  const put = vi.fn()
  mockIpc({ mermaid_cache_get: get, mermaid_cache_put: put })
  const figure = () => {
    const element = document.createElement('figure')
    element.dataset.hash = 'encrypted-only-source-hash'
    element.innerHTML = '<template>graph TD; A--&gt;B</template>'
    return element
  }
  const secret = figure()
  await renderMermaidFigure(
    secret,
    'paper-light',
    getComputedStyle(document.documentElement),
    undefined,
    true,
  )
  expect(secret.textContent).toContain('Private diagram')
  expect(get).not.toHaveBeenCalled()
  expect(put).not.toHaveBeenCalled()
  // A subsequent ordinary document must not find a shared cache entry.
  await renderMermaidFigure(
    figure(),
    'paper-light',
    getComputedStyle(document.documentElement),
  )
  expect(get).toHaveBeenCalledTimes(1)
})
