// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'

import { taskByteOffset } from './tasks'

function host(html: string): HTMLElement {
  const root = document.createElement('div')
  root.innerHTML = html
  document.body.appendChild(root)
  return root
}

describe('taskByteOffset', () => {
  it('reads the marker offset from the checkbox and from the item text', () => {
    const root = host(
      '<ul><li class="task-list-item"><input type="checkbox" data-task-at="2"/> Open</li></ul>',
    )
    const input = root.querySelector('input')
    const item = root.querySelector('li')
    expect(taskByteOffset(input)).toBe(2)
    expect(taskByteOffset(item)).toBe(2)
  })

  it('uses the checkbox inside a loose-list paragraph', () => {
    const root = host(
      '<ul><li class="task-list-item"><p><input type="checkbox" data-task-at="8"/> Loose</p></li></ul>',
    )
    expect(taskByteOffset(root.querySelector('p'))).toBe(8)
  })

  it('toggles the inner item when lists are nested', () => {
    const root = host(
      '<ul><li class="task-list-item"><input type="checkbox" data-task-at="2"/> Parent<ul><li class="task-list-item"><input type="checkbox" data-task-at="20"/> Child</li></ul></li></ul>',
    )
    const items = root.querySelectorAll('li')
    expect(taskByteOffset(items[1])).toBe(20)
    expect(taskByteOffset(items[0])).toBe(2)
  })

  it('leaves links and buttons alone', () => {
    const root = host(
      '<ul><li class="task-list-item"><input type="checkbox" data-task-at="2"/> <a href="note.md">Note</a> <button type="button">Copy</button></li></ul>',
    )
    expect(taskByteOffset(root.querySelector('a'))).toBeNull()
    expect(taskByteOffset(root.querySelector('button'))).toBeNull()
  })
})
