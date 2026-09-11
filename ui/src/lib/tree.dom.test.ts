// @vitest-environment happy-dom

import { afterEach, describe, expect, it } from 'vitest'

import { dropDirAtPoint, projectIdAtPoint, treeDropSiteAt } from './tree'

describe('tree drop targeting in the DOM', () => {
  const nodes: HTMLElement[] = []

  afterEach(() => {
    for (const node of nodes) {
      node.remove()
    }
    nodes.length = 0
  })

  function mount(el: HTMLElement): HTMLElement {
    document.body.append(el)
    nodes.push(el)
    return el
  }

  it('drops onto a folder row, or the parent of a file row', () => {
    const folder = mount(document.createElement('div'))
    folder.dataset.rel = 'docs/task'
    folder.dataset.kind = 'directory'
    document.elementFromPoint = () => folder
    expect(dropDirAtPoint(8, 8)).toBe('docs/task')
    expect(treeDropSiteAt(8, 8)).toEqual({ kind: 'tree', dir: 'docs/task' })

    const file = mount(document.createElement('div'))
    file.dataset.rel = 'docs/a.md'
    file.dataset.kind = 'file'
    document.elementFromPoint = () => file
    expect(dropDirAtPoint(8, 8)).toBe('docs')
  })

  it('treats empty tree chrome as the project root', () => {
    const scroll = mount(document.createElement('div'))
    scroll.className = 'tree-scroll'
    document.elementFromPoint = () => scroll
    expect(dropDirAtPoint(4, 4)).toBe('')
  })

  it('prefers a project row over the tree', () => {
    const project = mount(document.createElement('div'))
    project.dataset.projectId = 'proj-b'
    document.elementFromPoint = () => project
    expect(projectIdAtPoint(1, 1)).toBe('proj-b')
    expect(treeDropSiteAt(1, 1)).toEqual({ kind: 'project', id: 'proj-b' })
  })
})
