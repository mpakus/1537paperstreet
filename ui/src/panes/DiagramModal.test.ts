import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import DiagramModal from './DiagramModal.svelte'

describe('DiagramModal', () => {
  it('renders a titled window with icon actions and a resize grip', () => {
    const { body } = render(DiagramModal, {
      props: {
        svg: '<svg xmlns="http://www.w3.org/2000/svg"></svg>',
        onclose() {},
      },
    })

    expect(body).toContain('id="diagram-modal-title"')
    expect(body).toContain('Diagram')
    expect(body).toContain('aria-label="Copy SVG"')
    expect(body).toContain('aria-label="Save PNG"')
    expect(body).toContain('aria-label="Zoom in"')
    expect(body).toContain('aria-label="Zoom out"')
    expect(body).toContain('aria-label="Close"')
    expect(body).toContain('aria-label="Resize diagram"')
    expect(body).toContain('aria-modal="true"')
    expect(body).not.toContain('>Copy SVG<')
    expect(body).not.toContain('>Save PNG<')
    expect(body).not.toContain('>Zoom in<')
    expect(body).not.toContain('>Zoom out<')
    expect(body).not.toContain('>Close<')
  })
})
