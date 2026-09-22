import { describe, expect, it } from 'vitest'

import { modeForTab, openSession } from './session'

describe('openSession', () => {
  it('keeps the focused file even when it is not in the strip yet', () => {
    expect(
      openSession({
        projectId: 'proj',
        tabs: [{ relPath: 'a.md', preview: false, viewMode: 'preview' }],
        activeRelPath: 'b.md',
        workspaceTabs: ['assistant'],
        page: 'assistant',
        viewMode: 'editor',
      }),
    ).toEqual({
      project_id: 'proj',
      tabs: [
        { rel_path: 'a.md', preview: false, view_mode: 'preview' },
        { rel_path: 'b.md', preview: false, view_mode: 'editor' },
      ],
      active_rel_path: 'b.md',
      workspace_tabs: ['assistant'],
      page: 'assistant',
      view_mode: 'editor',
    })
  })

  it('keeps a requested mode, then the tab’s own mode', () => {
    expect(modeForTab('preview', 'editor', 'split')).toBe('editor')
    expect(modeForTab('preview', undefined, 'editor')).toBe('preview')
    expect(modeForTab(undefined, undefined, 'split')).toBe('split')
  })
})
