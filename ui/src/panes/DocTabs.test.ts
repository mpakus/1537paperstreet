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
  it('pins Dashboard and Assistant beside document tabs', () => {
    const { body } = render(DocTabs, {
      props: {
        tabs: [tab('notes/guide.md'), tab('todo.md')],
        page: 'document',
        activeRelPath: 'todo.md',
        onpage() {},
        onselect() {},
        onclose() {},
      },
    })

    expect(body).toContain('aria-label="Workspace"')
    expect(body).toContain('Dashboard')
    expect(body).toContain('Assistant')
    expect(body).toContain('guide.md')
    expect(body).toContain('todo.md')
    expect(body).toContain('aria-selected="true"')
    expect(body).toContain('Close todo.md')
  })

  it('selects the Assistant workspace tab', () => {
    const { body } = render(DocTabs, {
      props: {
        tabs: [],
        page: 'assistant',
        onpage() {},
        onselect() {},
        onclose() {},
      },
    })
    expect(body).toContain('Assistant')
    expect(body).toContain('aria-selected="true"')
  })
})
