import { describe, expect, it } from 'vitest'

import {
  formatMessageTime,
  HISTORY_LIMIT,
  lastHistoryId,
  recordMessage,
  TOAST_MS,
  unreadBadge,
  unreadCount,
} from './messages'

describe('messages', () => {
  it('skips blank text and assigns increasing ids', () => {
    expect(recordMessage([], '  ')).toEqual([])
    const once = recordMessage([], 'disk full', 1_000)
    expect(once).toEqual([{ id: 1, text: 'disk full', at: 1_000 }])
    const twice = recordMessage(once, 'saved', 2_000)
    expect(twice).toEqual([
      { id: 1, text: 'disk full', at: 1_000 },
      { id: 2, text: 'saved', at: 2_000 },
    ])
  })

  it('keeps only the newest HISTORY_LIMIT entries', () => {
    let history = recordMessage([], 'start', 0)
    for (let i = 1; i <= HISTORY_LIMIT; i += 1) {
      history = recordMessage(history, `m${i}`, i)
    }
    expect(history).toHaveLength(HISTORY_LIMIT)
    expect(history[0]?.text).toBe('m1')
    expect(history.at(-1)?.text).toBe(`m${HISTORY_LIMIT}`)
  })

  it('formats a clock time and exposes the toast timeout', () => {
    expect(formatMessageTime(Date.UTC(2026, 8, 6, 12, 4))).toMatch(
      /^\d{2}:\d{2}$/,
    )
    expect(TOAST_MS).toBe(8_000)
  })

  it('counts only messages newer than the last seen id', () => {
    const history = recordMessage(recordMessage([], 'one', 1), 'two', 2)
    expect(lastHistoryId(history)).toBe(2)
    expect(unreadCount(history, 0)).toBe(2)
    expect(unreadCount(history, 2)).toBe(0)
    expect(unreadCount(history, 1)).toBe(1)
    expect(unreadBadge(0)).toBe('')
    expect(unreadBadge(2)).toBe('2')
    expect(unreadBadge(12)).toBe('9+')
  })
})
