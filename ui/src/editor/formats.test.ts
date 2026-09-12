import { describe, expect, it } from 'vitest'

import { CORE_FORMATS, supportedFormats } from './formats'

describe('supported formats catalog', () => {
  it('lists the first-class source languages', () => {
    const rows = supportedFormats()
    const byName = Object.fromEntries(
      rows.map((row) => [row.name, row.patterns]),
    )

    expect(rows).toEqual(CORE_FORMATS)
    expect(Object.keys(byName)).toEqual([
      'JavaScript',
      'TypeScript',
      'Ruby',
      'Elixir',
      'Go',
      'Rust',
      'C#',
      'Java',
      'PHP',
    ])
    expect(byName.JavaScript).toContain('.js')
    expect(byName.JavaScript).toContain('.rsx')
    expect(byName.TypeScript).toContain('.ts')
    expect(byName.TypeScript).toContain('.tsx')
    expect(byName.Ruby).toContain('.rb')
    expect(byName.Elixir).toBe('.ex, .exs')
    expect(byName.Go).toBe('.go')
    expect(byName.Rust).toBe('.rs')
    expect(byName['C#']).toBe('.cs')
    expect(byName.Java).toBe('.java')
    expect(byName.PHP).toContain('.php')
  })
})
