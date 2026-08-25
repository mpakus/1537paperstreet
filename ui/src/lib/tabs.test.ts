import { describe, expect, it } from 'vitest'

import {
  closeWorkspaceTab,
  nextAfterClose,
  openWorkspaceTab,
  removeTab,
  retitleTab,
  tabTitle,
  upsertTab,
  type DocTab,
} from './tabs'

function tab(relPath: string): DocTab {
  return {
    relPath,
    title: tabTitle(relPath),
    html: '',
    docMeta: null,
    docSourceMeta: null,
    draftText: '',
  }
}

describe('tabs', () => {
  it('opens each optional workspace tab once and closes it', () => {
    expect(openWorkspaceTab([], 'dashboard')).toEqual(['dashboard'])
    expect(openWorkspaceTab(['assistant'], 'assistant')).toEqual(['assistant'])
    expect(closeWorkspaceTab(['dashboard', 'assistant'], 'dashboard')).toEqual([
      'assistant',
    ])
  })

  it('uses the file name as the label', () => {
    expect(tabTitle('notes/guide.md')).toBe('guide.md')
  })

  it('upserts by relative path and removes tabs', () => {
    const first = upsertTab([], tab('a.md'))
    const two = upsertTab(first, tab('b.md'))
    const replaced = upsertTab(two, { ...tab('a.md'), html: '<p>x</p>' })
    expect(replaced.map((item) => item.relPath)).toEqual(['a.md', 'b.md'])
    expect(replaced[0]?.html).toBe('<p>x</p>')
    expect(removeTab(replaced, 'a.md').map((item) => item.relPath)).toEqual([
      'b.md',
    ])
  })

  it('activates a neighbor after close', () => {
    const tabs = [tab('a.md'), tab('b.md'), tab('c.md')]
    expect(nextAfterClose(tabs, 'b.md')).toBe('c.md')
    expect(nextAfterClose(tabs, 'c.md')).toBe('b.md')
    expect(nextAfterClose([tab('a.md')], 'a.md')).toBeNull()
  })

  it('renames a tab path', () => {
    expect(retitleTab([tab('old.md')], 'old.md', 'notes/new.md')).toEqual([
      {
        ...tab('notes/new.md'),
        title: 'new.md',
      },
    ])
  })
})
