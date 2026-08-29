/** Keyboard modifiers used to detect a composer submit chord. */
export type ComposerKey = Pick<
  KeyboardEvent,
  'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'
>

/** True when the Assistant composer should send (Shift+Enter, Ctrl+Enter, or ⌘Enter). */
export function isComposerSubmitKey(event: ComposerKey): boolean {
  return (
    event.key === 'Enter' &&
    !event.altKey &&
    (event.shiftKey || event.ctrlKey || event.metaKey)
  )
}
