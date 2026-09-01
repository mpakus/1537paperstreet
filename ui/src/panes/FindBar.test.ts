import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import FindBar from './FindBar.svelte'

describe('FindBar', () => {
  it('shows a live search field for the open document', () => {
    const { body } = render(FindBar, {
      props: {
        root: null,
        revision: '',
        onclose() {},
      },
    })
    expect(body).toContain('role="search"')
    expect(body).toContain('aria-label="Find in document"')
    expect(body).toContain('type="search"')
    expect(body).toContain('Previous')
    expect(body).toContain('Next')
  })
})
