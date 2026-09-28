import { describe, expect, it } from 'vitest'

import { hostLabel, revealLabel, usesTrafficLights } from './platform'

describe('platform', () => {
  it('keeps macOS wording unless the host is Windows or Linux', () => {
    expect(hostLabel('MacIntel', '')).toBe('mac')
    expect(hostLabel('', '')).toBe('mac')
    expect(hostLabel('Win32', 'Windows NT')).toBe('windows')
    expect(hostLabel('Linux x86_64', 'X11')).toBe('linux')
    expect(revealLabel('mac')).toBe('Reveal in Finder')
    expect(revealLabel('windows')).toBe('Show in Explorer')
    expect(revealLabel('linux')).toBe('Show in Files')
    expect(usesTrafficLights('mac')).toBe(true)
    expect(usesTrafficLights('linux')).toBe(false)
    expect(usesTrafficLights('windows')).toBe(false)
  })
})
