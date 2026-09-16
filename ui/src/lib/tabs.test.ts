import { describe, expect, it } from 'vitest'

import {
  closeActiveTarget,
  closeWorkspaceTab,
  nextAfterClose,
  openWorkspaceTab,
  removeTab,
  retitleTab,
  tabTitle,
  upsertTab,
  placeDocTab,
  type DocTab,
} from './tabs'

function tab(relPath: string, preview = false): DocTab {
  return {
    relPath,
    title: tabTitle(relPath),
    html: '',
    docMeta: null,
    docSourceMeta: null,
    draftText: '',
    preview,
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

  it('closes the active workspace or document tab', () => {
    expect(closeActiveTarget('assistant', 'notes.md')).toEqual({
      kind: 'workspace',
      page: 'assistant',
    })
    expect(closeActiveTarget('document', 'notes.md')).toEqual({
      kind: 'document',
      relPath: 'notes.md',
    })
    expect(closeActiveTarget('document', null)).toEqual({ kind: 'none' })
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

  it('replaces a preview tab and pins on double open', () => {
    const previewA = placeDocTab([], tab('a.md'), 'preview')
    expect(previewA).toEqual([tab('a.md', true)])
    const previewB = placeDocTab(previewA, tab('b.md'), 'preview')
    expect(previewB.map((item) => item.relPath)).toEqual(['b.md'])
    expect(previewB[0]?.preview).toBe(true)
    const pinned = placeDocTab(previewB, tab('b.md'), 'pin')
    expect(pinned).toEqual([tab('b.md')])
    const kept = placeDocTab(
      previewA,
      { ...tab('a.md'), html: '<p>x</p>' },
      'keep',
    )
    expect(kept[0]?.preview).toBe(true)
    expect(kept[0]?.html).toBe('<p>x</p>')
    const besidePinned = placeDocTab([tab('a.md')], tab('b.md'), 'preview')
    expect(besidePinned.map((item) => [item.relPath, item.preview])).toEqual([
      ['a.md', false],
      ['b.md', true],
    ])
    const previewPinned = placeDocTab([tab('a.md')], tab('a.md'), 'preview')
    expect(previewPinned).toEqual([tab('a.md')])
  })

  it('pins a dirty preview instead of replacing unsaved text', () => {
    const dirty: DocTab = {
      ...tab('draft.md', true),
      docSourceMeta: {
        text: 'old',
        eol: 'lf',
        bom: false,
        trailingNewline: true,
        encoding: 'utf8',
        writable: true,
        readonlyReason: null,
      },
      draftText: 'new',
    }
    const next = placeDocTab([dirty], tab('other.md'), 'preview')
    expect(next.map((item) => [item.relPath, item.preview])).toEqual([
      ['draft.md', false],
      ['other.md', true],
    ])
  })
})
