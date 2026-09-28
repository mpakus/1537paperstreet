/** Byte offset of a preview task marker, or null when the click is not one. */
export function taskByteOffset(target: EventTarget | null): number | null {
  if (!(target instanceof Element)) {
    return null
  }
  if (target.closest('a, button, textarea, select')) {
    return null
  }
  const direct = target.closest('input[type="checkbox"][data-task-at]')
  const checkbox =
    direct instanceof HTMLInputElement ? direct : checkboxInTaskItem(target)
  if (!checkbox) {
    return null
  }
  const at = Number(checkbox.dataset.taskAt)
  if (!Number.isInteger(at) || at < 0) {
    return null
  }
  return at
}

function checkboxInTaskItem(target: Element): HTMLInputElement | null {
  const item = target.closest('li.task-list-item')
  if (!item) {
    return null
  }
  for (const child of item.children) {
    if (isTaskCheckbox(child)) {
      return child
    }
    if (child.tagName === 'P') {
      for (const nested of child.children) {
        if (isTaskCheckbox(nested)) {
          return nested
        }
      }
    }
  }
  return null
}

function isTaskCheckbox(node: Element): node is HTMLInputElement {
  return (
    node instanceof HTMLInputElement &&
    node.type === 'checkbox' &&
    node.dataset.taskAt !== undefined
  )
}
