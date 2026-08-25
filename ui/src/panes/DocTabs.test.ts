import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import { tabTitle, type DocTab } from '../lib/tabs'
import DocTabs from './DocTabs.svelte'

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

describe('DocTabs', () => {
  it('shows only workspace tabs the user opened', () => {
    const { body } = render(DocTabs, {
      props: {
        tabs: [tab('notes/guide.md'), tab('todo.md')],
        workspaceTabs: [],
        page: 'document',
        activeRelPath: 'todo.md',
        onpage() {},
        onclosepage() {},
        onselect() {},
        onclose() {},
      },
    })

    expect(body).toContain('aria-label="Workspace"')
    expect(body).not.toContain('Dashboard')
    expect(body).not.toContain('Assistant')
    expect(body).toContain('guide.md')
    expect(body).toContain('todo.md')
    expect(body).toContain('aria-selected="true"')
    expect(body).toContain('Close todo.md')
  })

  it('selects the Assistant workspace tab', () => {
    const { body } = render(DocTabs, {
      props: {
        tabs: [],
        workspaceTabs: ['assistant'],
        page: 'assistant',
        onpage() {},
        onclosepage() {},
        onselect() {},
        onclose() {},
      },
    })
    expect(body).toContain('Assistant')
    expect(body).toContain('aria-selected="true"')
    expect(body).toContain('aria-label="Close Assistant"')
    expect(body).not.toContain('Dashboard')
  })
})
