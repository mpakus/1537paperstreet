/** True when a click landed on the dimmed overlay, not the dialog inside it. */
export function isBackdropEvent(event: {
  target: unknown
  currentTarget: unknown
}): boolean {
  return event.target === event.currentTarget
}
