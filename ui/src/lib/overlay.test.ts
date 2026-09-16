import { describe, expect, it } from 'vitest'

import { isBackdropEvent } from './overlay'

describe('isBackdropEvent', () => {
  it('is true only when the click target is the overlay itself', () => {
    const overlay = { id: 'scrim' }
    const sheet = { id: 'sheet' }
    expect(
      isBackdropEvent({ target: overlay, currentTarget: overlay }),
    ).toBe(true)
    expect(
      isBackdropEvent({ target: sheet, currentTarget: overlay }),
    ).toBe(false)
  })
})
