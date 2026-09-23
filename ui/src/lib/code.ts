/** Adds a top-right Copy button to every code block and quote in `root`. */
export function enhanceCodeBlocks(
  root: HTMLElement,
  onerror: (message: string) => void,
): () => void {
  const blocks = [...root.querySelectorAll<HTMLElement>('pre, blockquote')]
    .filter((block) => block.closest('figure.mermaid, .front-matter') === null)
    .sort((left, right) => {
      if (left.contains(right)) {
        return 1
      }
      if (right.contains(left)) {
        return -1
      }
      return 0
    })
  const wraps: HTMLElement[] = []
  for (const block of blocks) {
    if (block.parentElement?.classList.contains('copy-block')) {
      continue
    }
    const wrap = document.createElement('div')
    wrap.className = 'copy-block'
    block.replaceWith(wrap)
    wrap.append(block)
    const button = document.createElement('button')
    button.type = 'button'
    button.className = 'copy-control'
    button.textContent = 'Copy'
    button.setAttribute(
      'aria-label',
      block.tagName === 'PRE' ? 'Copy code' : 'Copy quote',
    )
    button.addEventListener('click', (event) => {
      event.preventDefault()
      event.stopPropagation()
      const text = copyText(block)
      void navigator.clipboard.writeText(text).then(
        () => {
          button.textContent = 'Copied'
          window.setTimeout(() => {
            if (button.isConnected) {
              button.textContent = 'Copy'
            }
          }, 1200)
        },
        (cause: unknown) => {
          onerror(cause instanceof Error ? cause.message : String(cause))
        },
      )
    })
    wrap.append(button)
    wraps.push(wrap)
  }
  return () => {
    for (const wrap of wraps) {
      const block = wrap.firstElementChild
      if (block && wrap.parentElement) {
        wrap.replaceWith(block)
      }
    }
  }
}

/** Text of a code block or quote, without nested Copy buttons. */
function copyText(block: HTMLElement): string {
  const clone = block.cloneNode(true) as HTMLElement
  for (const button of clone.querySelectorAll('button')) {
    button.remove()
  }
  const text = clone.innerText || clone.textContent || ''
  if (block.tagName === 'PRE') {
    return text.replace(/\n$/, '')
  }
  return text.trim()
}
