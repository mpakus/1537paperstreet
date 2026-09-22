import { describe, expect, it } from 'vitest'

import type { DocumentMeta, DocumentSource } from './generated/core'

import {
  closeActiveTarget,
  closeWorkspaceTab,
  followOpenRename,
  nextAfterClose,
  openWorkspaceTab,
  persistLeavingTab,
  placeDocTab,
  promptAfterRename,
  removeTab,
  retitleTab,
  tabTitle,
  tabsToReopen,
  upsertTab,
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
    viewMode: 'preview',
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

  it('does not put a just-closed tab back when leaving it', () => {
    const leaving = { ...tab('a.md'), html: '<p>draft</p>' }
    expect(
      persistLeavingTab([tab('b.md'), tab('c.md')], leaving).map(
        (item) => item.relPath,
      ),
    ).toEqual(['b.md', 'c.md'])
  })

  it('keeps the tab being left when it is still in the strip', () => {
    const leaving = { ...tab('a.md'), html: '<p>draft</p>' }
    const next = persistLeavingTab([tab('a.md'), tab('b.md')], leaving)
    expect(next.map((item) => item.relPath)).toEqual(['a.md', 'b.md'])
    expect(next[0]?.html).toBe('<p>draft</p>')
  })

  it('renames a tab path', () => {
    expect(retitleTab([tab('old.md')], 'old.md', 'notes/new.md')).toEqual([
      {
        ...tab('notes/new.md'),
        title: 'new.md',
      },
    ])
  })

  it('keeps unsaved editor text when the open file is renamed', () => {
    const source: DocumentSource = {
      text: 'on disk',
      eol: 'lf',
      bom: false,
      trailingNewline: true,
      encoding: 'utf8',
      writable: true,
      readonlyReason: null,
    }
    const meta: DocumentMeta = {
      projectId: 'p',
      relPath: 'draft.md',
      title: 'Draft',
      hash: 'abc',
      size: 7,
      writable: true,
      readonlyReason: null,
      sourceOnly: false,
      chunkCount: 1,
      toc: [],
    }
    const dirty: DocTab = {
      ...tab('draft.md'),
      html: '<p>on disk</p>',
      docMeta: meta,
      docSourceMeta: source,
      draftText: 'unsaved words',
    }
    const stale = placeDocTab(
      [dirty],
      { ...dirty, draftText: 'unsaved words' },
      'keep',
    )
    const followed = followOpenRename(
      stale,
      'draft.md',
      meta,
      'draft.md',
      'renamed.md',
    )
    expect(followed.openRelPath).toBe('renamed.md')
    expect(followed.docMeta?.relPath).toBe('renamed.md')
    expect(followed.tabs).toHaveLength(1)
    expect(followed.tabs[0]?.relPath).toBe('renamed.md')
    expect(followed.tabs[0]?.title).toBe('renamed.md')
    expect(followed.tabs[0]?.draftText).toBe('unsaved words')
    expect(followed.tabs[0]?.docSourceMeta?.text).toBe('on disk')
    expect(followed.tabs[0]?.docMeta?.relPath).toBe('renamed.md')
    expect(followed.tabs[0]?.docMeta?.hash).toBe('abc')
  })

  it('remaps nested tabs when a parent folder is renamed', () => {
    const nested = tab('notes/chapter.md')
    const other = tab('readme.md')
    const followed = followOpenRename(
      [nested, other],
      'notes/chapter.md',
      null,
      'notes',
      'inbox',
    )
    expect(followed.openRelPath).toBe('inbox/chapter.md')
    expect(followed.tabs.map((item) => item.relPath)).toEqual([
      'inbox/chapter.md',
      'readme.md',
    ])
  })

  it('does not treat a prefix-sharing neighbor as the renamed file', () => {
    const tabs = [tab('notes.md'), tab('notes/extra.md')]
    expect(
      retitleTab(tabs, 'notes', 'inbox').map((item) => item.relPath),
    ).toEqual(['notes.md', 'inbox/extra.md'])
  })

  it('drops a missing-file prompt caused by our own rename', () => {
    expect(
      promptAfterRename(
        { relPath: 'old.md', missing: true },
        'old.md',
        'new.md',
      ),
    ).toBeNull()
    expect(
      promptAfterRename(
        { relPath: 'keep.md', missing: true },
        'old.md',
        'new.md',
      ),
    ).toEqual({ relPath: 'keep.md', missing: true })
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
    const linked = placeDocTab(previewA, tab('notes/lib.rs'), 'pin')
    expect(linked.map((item) => [item.relPath, item.preview])).toEqual([
      ['a.md', true],
      ['notes/lib.rs', false],
    ])
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

  it('keeps each tab’s view mode when leaving it', () => {
    const leaving = { ...tab('notes.md'), viewMode: 'preview' as const }
    const next = persistLeavingTab(
      [leaving, { ...tab('lib/app.rb'), viewMode: 'editor' }],
      leaving,
    )
    expect(next.map((item) => [item.relPath, item.viewMode])).toEqual([
      ['notes.md', 'preview'],
      ['lib/app.rb', 'editor'],
    ])
  })

  it('reopens pinned tabs, then the preview, then the active file', () => {
    const saved = [
      { rel_path: 'a.md', preview: false },
      { rel_path: 'b.md', preview: true },
      { rel_path: 'c.md', preview: false },
    ]
    expect(tabsToReopen(saved, 'c.md').map((item) => item.rel_path)).toEqual([
      'a.md',
      'b.md',
      'c.md',
    ])
    expect(tabsToReopen(saved, 'b.md').map((item) => item.rel_path)).toEqual([
      'a.md',
      'c.md',
      'b.md',
    ])
  })
})
