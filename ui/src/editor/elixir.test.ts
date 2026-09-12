import { describe, expect, it } from 'vitest'

import { elixirKeywords, elixirLanguage } from './elixir'

describe('elixir stream language', () => {
  it('knows the forms used in typical Elixir modules', () => {
    expect(elixirKeywords).toEqual(
      expect.arrayContaining(['defmodule', 'defp', 'do', 'end']),
    )
    expect(elixirLanguage.name).toBe('elixir')
  })
})
