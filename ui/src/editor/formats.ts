/** One highlighted language shown in Settings. */
export type SupportedFormat = {
  name: string
  patterns: string
}

/**
 * Source languages called out as first-class: highlighted in Preview and Edit.
 * Other UTF-8 still opens; a grammar is used when one exists.
 */
export const CORE_FORMATS: SupportedFormat[] = [
  { name: 'JavaScript', patterns: '.js, .mjs, .cjs, .jsx, .rsx' },
  { name: 'TypeScript', patterns: '.ts, .mts, .cts, .tsx' },
  { name: 'Ruby', patterns: '.rb, Gemfile, Rakefile' },
  { name: 'Elixir', patterns: '.ex, .exs' },
  { name: 'Go', patterns: '.go' },
  { name: 'Rust', patterns: '.rs' },
  { name: 'C#', patterns: '.cs' },
  { name: 'Java', patterns: '.java' },
  { name: 'PHP', patterns: '.php, .php3, .php4, .php5, .php7, .phtml' },
]

/** First-class source formats listed in Settings. */
export function supportedFormats(): SupportedFormat[] {
  return CORE_FORMATS
}
