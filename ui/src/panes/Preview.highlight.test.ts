// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import Preview from './Preview.svelte'

describe('Preview highlight palette', () => {
  const mounted: ReturnType<typeof mount>[] = []
  afterEach(async () => {
    for (const instance of mounted.splice(0)) await unmount(instance)
    document.body.innerHTML = ''
    window.getSelection()?.removeAllRanges()
  })

  function setup(html = '<p>Pick this word</p>') {
    const target = document.createElement('div')
    document.body.append(target)
    const onhighlight = vi.fn().mockResolvedValue(undefined)
    const onerror = vi.fn()
    mounted.push(
      mount(Preview, {
        target,
        props: {
          html: `<section data-block="0" data-hash="abc">${html}</section>`,
          emptyMessage: '',
          onnavigate() {},
          onhighlight,
          onerror,
        },
      }),
    )
    flushSync()
    return { target, onhighlight, onerror }
  }

  function select(target: HTMLElement) {
    const range = document.createRange()
    range.selectNodeContents(target.querySelector('p')!)
    range.getBoundingClientRect = () => ({ left: 100, bottom: 100 }) as DOMRect
    window.getSelection()?.addRange(range)
    // happy-dom copies Range objects when adding them to Selection.
    const selected = window.getSelection()!.getRangeAt(0)
    selected.getBoundingClientRect = range.getBoundingClientRect
    target
      .querySelector('.preview')!
      .dispatchEvent(new Event('pointerup', { bubbles: true }))
    flushSync()
  }

  it('offers presets and sends a captured selection with a custom hex color', async () => {
    const { target, onhighlight } = setup()
    select(target)
    expect(target.querySelector('[aria-label="green highlight"]')).toBeTruthy()
    const input = target.querySelector<HTMLInputElement>(
      '[aria-label="Hex color"]',
    )!
    input.value = '#123abc'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    expect(input.checkValidity()).toBe(true)
    target
      .querySelector('form')!
      .dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await Promise.resolve()
    flushSync()
    expect(onhighlight).toHaveBeenCalledWith(
      {
        blocks: [
          { index: '0', hash: 'abc', text: 'Pick this word', from: 0, to: 14 },
        ],
      },
      '#123abc',
    )
    await vi.waitFor(() => {
      flushSync()
      expect(target.querySelector('.highlight-palette')).toBeNull()
    })
  })

  it('removes highlights and closes with Escape without changing text', async () => {
    const { target, onhighlight } = setup(
      '<p><mark class="text-highlight" data-highlight="green">word</mark></p>',
    )
    select(target)
    const remove = Array.from(target.querySelectorAll('button')).find(
      (button) => button.textContent === 'Remove highlight',
    )!
    remove.click()
    await Promise.resolve()
    flushSync()
    expect(onhighlight.mock.calls[0]?.[1]).toBeNull()
    select(target)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    flushSync()
    expect(target.querySelector('.highlight-palette')).toBeNull()
    expect(onhighlight).toHaveBeenCalledTimes(1)
    expect(target.querySelector('article')!.textContent).toBe('word')
  })
})
