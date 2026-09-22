/** Text and a caret/selection range after a formatting command. */
export type MarkdownEdit = {
  text: string
  start: number
  end: number
}

/** Wraps or unwraps an inline Markdown mark around the current selection. */
export function wrapInline(
  text: string,
  start: number,
  end: number,
  open: string,
  close: string,
): MarkdownEdit {
  const selected = text.slice(start, end)
  if (
    start >= open.length &&
    text.slice(start - open.length, start) === open &&
    text.slice(end, end + close.length) === close
  ) {
    return {
      text:
        text.slice(0, start - open.length) +
        selected +
        text.slice(end + close.length),
      start: start - open.length,
      end: end - open.length,
    }
  }
  if (
    selected.startsWith(open) &&
    selected.endsWith(close) &&
    selected.length >= open.length + close.length
  ) {
    const inner = selected.slice(open.length, selected.length - close.length)
    return {
      text: text.slice(0, start) + inner + text.slice(end),
      start,
      end: start + inner.length,
    }
  }
  return {
    text: text.slice(0, start) + open + selected + close + text.slice(end),
    start: start + open.length,
    end: end + open.length,
  }
}

/** Prefixes each selected line, or strips the prefix when every line already has it. */
export function toggleLinePrefix(
  text: string,
  start: number,
  end: number,
  prefix: string,
): MarkdownEdit {
  const lineStart = text.lastIndexOf('\n', Math.max(0, start - 1)) + 1
  const after = text.indexOf('\n', end)
  const lineEnd = after < 0 ? text.length : after
  const block = text.slice(lineStart, lineEnd)
  const lines = block.split('\n')
  const allPrefixed =
    lines.length > 0 &&
    lines.every((line) => line.startsWith(prefix) || line.length === 0)
  const nextLines = allPrefixed
    ? lines.map((line) =>
        line.startsWith(prefix) ? line.slice(prefix.length) : line,
      )
    : lines.map((line) => (line.length === 0 ? line : `${prefix}${line}`))
  const nextBlock = nextLines.join('\n')
  return {
    text: text.slice(0, lineStart) + nextBlock + text.slice(lineEnd),
    start: lineStart,
    end: lineStart + nextBlock.length,
  }
}

const TASK_CHECKED = '- [x] '
const TASK_OPEN = '- [ ] '
const TASK_MARK_OPEN = new Uint8Array([0x5b, 0x20, 0x5d])
const TASK_MARK_CHECKED = new Uint8Array([0x5b, 0x78, 0x5d])
const TASK_MARK_CHECKED_UPPER = new Uint8Array([0x5b, 0x58, 0x5d])

function sameBytes(left: Uint8Array, right: Uint8Array): boolean {
  return (
    left.length === right.length &&
    left.every((byte, index) => byte === right[index])
  )
}

/**
 * Flips one `[ ]` or `[x]` marker at a UTF-8 byte offset from the preview.
 * Returns null when that offset is not a task marker.
 */
export function toggleTaskAt(text: string, byteOffset: number): string | null {
  if (!Number.isInteger(byteOffset) || byteOffset < 0) {
    return null
  }
  const bytes = new TextEncoder().encode(text)
  if (byteOffset + 3 > bytes.length) {
    return null
  }
  const marker = bytes.subarray(byteOffset, byteOffset + 3)
  let replacement = TASK_MARK_OPEN
  if (sameBytes(marker, TASK_MARK_OPEN)) {
    replacement = TASK_MARK_CHECKED
  } else if (
    !sameBytes(marker, TASK_MARK_CHECKED) &&
    !sameBytes(marker, TASK_MARK_CHECKED_UPPER)
  ) {
    return null
  }
  const next = new Uint8Array(bytes.length)
  next.set(bytes.subarray(0, byteOffset), 0)
  next.set(replacement, byteOffset)
  next.set(bytes.subarray(byteOffset + 3), byteOffset + 3)
  return new TextDecoder().decode(next)
}

/** Cycles a task-list prefix on each selected line: none → open → checked → none. */
export function toggleTaskItem(
  text: string,
  start: number,
  end: number,
): MarkdownEdit {
  const lineStart = text.lastIndexOf('\n', Math.max(0, start - 1)) + 1
  const after = text.indexOf('\n', end)
  const lineEnd = after < 0 ? text.length : after
  const block = text.slice(lineStart, lineEnd)
  const nextLines = block.split('\n').map((line) => {
    if (line.startsWith(TASK_CHECKED) || line.startsWith('- [X] ')) {
      return line.slice(TASK_CHECKED.length)
    }
    if (line.startsWith(TASK_OPEN)) {
      return `${TASK_CHECKED}${line.slice(TASK_OPEN.length)}`
    }
    if (line.startsWith('- ')) {
      return `${TASK_OPEN}${line.slice(2)}`
    }
    return `${TASK_OPEN}${line}`
  })
  const nextBlock = nextLines.join('\n')
  return {
    text: text.slice(0, lineStart) + nextBlock + text.slice(lineEnd),
    start: lineStart,
    end: lineStart + nextBlock.length,
  }
}

/** Sets ATX heading level 1–6 on the selected lines, or clears a matching heading. */
export function toggleHeading(
  text: string,
  start: number,
  end: number,
  level: number,
): MarkdownEdit {
  const marks = '#'.repeat(Math.min(6, Math.max(1, level)))
  const prefix = `${marks} `
  const lineStart = text.lastIndexOf('\n', Math.max(0, start - 1)) + 1
  const after = text.indexOf('\n', end)
  const lineEnd = after < 0 ? text.length : after
  const block = text.slice(lineStart, lineEnd)
  const heading = /^(#{1,6})(\s+)/
  const nextLines = block.split('\n').map((line) => {
    const match = heading.exec(line)
    if (match && match[1] === marks) {
      return line.slice(match[0].length)
    }
    if (match) {
      return `${prefix}${line.slice(match[0].length)}`
    }
    return `${prefix}${line}`
  })
  const nextBlock = nextLines.join('\n')
  return {
    text: text.slice(0, lineStart) + nextBlock + text.slice(lineEnd),
    start: lineStart,
    end: lineStart + nextBlock.length,
  }
}

/** Applies a formatting command from the editor toolbar or menu. */
export function applyMarkdownCommand(
  text: string,
  start: number,
  end: number,
  command: string,
): MarkdownEdit {
  switch (command) {
    case 'edit-bold':
      return wrapInline(text, start, end, '**', '**')
    case 'edit-italic':
      return wrapInline(text, start, end, '*', '*')
    case 'edit-inline-code':
      return wrapInline(text, start, end, '`', '`')
    case 'edit-link':
      return wrapInline(text, start, end, '[', ']()')
    case 'edit-image':
      return wrapInline(text, start, end, '![', ']()')
    case 'edit-list':
      return toggleLinePrefix(text, start, end, '- ')
    case 'edit-task':
      return toggleTaskItem(text, start, end)
    case 'edit-wiki-link':
      return wrapInline(text, start, end, '[[', ']]')
    case 'edit-quote':
      return toggleLinePrefix(text, start, end, '> ')
    default:
      if (command.startsWith('edit-heading-')) {
        const level = Number(command.slice('edit-heading-'.length))
        if (level >= 1 && level <= 6) {
          return toggleHeading(text, start, end, level)
        }
      }
      return { text, start, end }
  }
}
