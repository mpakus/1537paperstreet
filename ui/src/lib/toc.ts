/** One-based source line of the Markdown block that contains `heading`. */
export function sourceLineForHeading(heading: Element): number | null {
  const raw = heading.closest('[data-src-line]')?.getAttribute('data-src-line')
  if (!raw || !/^[1-9]\d*$/.test(raw)) {
    return null
  }
  const line = Number(raw)
  return Number.isSafeInteger(line) ? line : null
}
