/** One toast / history entry shown in the reader chrome. */
export type AppMessage = {
  id: number
  text: string
  at: number
}

/** How long a toast stays visible before it dismisses itself. */
export const TOAST_MS = 8_000

/** Newest messages kept in the on-screen history list. */
export const HISTORY_LIMIT = 50

/** Appends a non-empty message and drops the oldest entries past the cap. */
export function recordMessage(
  history: AppMessage[],
  text: string,
  at = Date.now(),
): AppMessage[] {
  const trimmed = text.trim()
  if (!trimmed) {
    return history
  }
  const id = (history.at(-1)?.id ?? 0) + 1
  const next = [...history, { id, text: trimmed, at }]
  if (next.length <= HISTORY_LIMIT) {
    return next
  }
  return next.slice(next.length - HISTORY_LIMIT)
}

/** `HH:MM` in the user's locale, for the history list. */
export function formatMessageTime(at: number): string {
  const date = new Date(at)
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  return `${hours}:${minutes}`
}
