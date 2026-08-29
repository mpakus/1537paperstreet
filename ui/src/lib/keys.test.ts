import { describe, expect, it } from 'vitest'

import { isComposerSubmitKey } from './keys'

function key(
  key: string,
  mods: { ctrl?: boolean; meta?: boolean; shift?: boolean; alt?: boolean } = {},
) {
  return {
    key,
    ctrlKey: mods.ctrl ?? false,
    metaKey: mods.meta ?? false,
    shiftKey: mods.shift ?? false,
    altKey: mods.alt ?? false,
  }
}

describe('isComposerSubmitKey', () => {
  it('sends on Shift+Enter, Ctrl+Enter, and ⌘Enter', () => {
    expect(isComposerSubmitKey(key('Enter', { shift: true }))).toBe(true)
    expect(isComposerSubmitKey(key('Enter', { ctrl: true }))).toBe(true)
    expect(isComposerSubmitKey(key('Enter', { meta: true }))).toBe(true)
  })

  it('leaves plain Enter for a new line', () => {
    expect(isComposerSubmitKey(key('Enter'))).toBe(false)
    expect(isComposerSubmitKey(key('Enter', { alt: true }))).toBe(false)
  })
})
