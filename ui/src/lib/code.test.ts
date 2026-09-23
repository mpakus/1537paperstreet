// @vitest-environment happy-dom

import { describe, expect, it, vi } from 'vitest'

import { enhanceCodeBlocks } from './code'

function mount(html: string): HTMLElement {
  const root = document.createElement('div')
  root.innerHTML = html
  document.body.append(root)
  return root
}

describe('enhanceCodeBlocks', () => {
  it('puts a Copy button on code blocks and quotes', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    const root = mount(`
      <pre class="code"><code>let n = 1\n</code></pre>
      <pre><code>plain</code></pre>
      <blockquote><p>Keep this</p></blockquote>
      <figure class="mermaid"><pre>graph TD</pre></figure>
      <div class="front-matter"><pre>title: Hello</pre></div>
    `)
    const stop = enhanceCodeBlocks(root, () => {})
    const buttons = [...root.querySelectorAll('button.copy-control')]
    expect(buttons.map((button) => button.getAttribute('aria-label'))).toEqual([
      'Copy code',
      'Copy code',
      'Copy quote',
    ])
    for (const button of buttons) {
      const wrap = button.parentElement
      expect(wrap?.classList.contains('copy-block')).toBe(true)
      expect(button.classList.contains('copy-control')).toBe(true)
    }

    buttons[0]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await vi.waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('let n = 1')
    })
    buttons[2]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await vi.waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('Keep this')
    })

    stop()
    expect(root.querySelector('button.copy-control')).toBeNull()
    expect(root.querySelector('pre.code')).not.toBeNull()
    root.remove()
  })

  it('copies a quote without the nested Copy label', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    const root = mount(
      '<blockquote><p>Outer</p><blockquote><p>Inner</p></blockquote></blockquote>',
    )
    enhanceCodeBlocks(root, () => {})
    const [inner, outer] = root.querySelectorAll('button.copy-control')
    expect(inner?.getAttribute('aria-label')).toBe('Copy quote')
    expect(outer?.getAttribute('aria-label')).toBe('Copy quote')
    outer?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await vi.waitFor(() => {
      const copied = String(writeText.mock.calls.at(-1)?.[0])
      expect(copied).toContain('Outer')
      expect(copied).toContain('Inner')
      expect(copied).not.toContain('Copy')
    })
    root.remove()
  })
})
