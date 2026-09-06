import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import MessageToasts from './MessageToasts.svelte'

describe('MessageToasts', () => {
  it('renders the history button even when the list is empty', () => {
    const { body } = render(MessageToasts, {
      props: {
        toast: '',
        history: [],
        open: false,
        ondismiss() {},
        ontoggle() {},
        onclose() {},
      },
    })

    expect(body).toContain('aria-label="Message history"')
    expect(body).toContain('title="Message history"')
    expect(body).toContain('aria-expanded="false"')
    expect(body).not.toContain('role="status"')
    expect(body).not.toContain('aria-label="Dismiss"')
  })

  it('shows a dismissible toast and an open history list', () => {
    const { body } = render(MessageToasts, {
      props: {
        toast: "Couldn't open the folder.",
        history: [
          { id: 1, text: 'First', at: 1_000 },
          { id: 2, text: "Couldn't open the folder.", at: 2_000 },
        ],
        open: true,
        ondismiss() {},
        ontoggle() {},
        onclose() {},
      },
    })

    expect(body).toContain('aria-expanded="true"')
    expect(body).toContain('aria-label="Message history list"')
    expect(body).toContain('First')
    expect(body).toContain("Couldn't open the folder.")
    expect(body).toContain('aria-label="Dismiss"')
    expect(body).toContain('role="status"')
    expect(body).toContain('>2</span')
  })
})
