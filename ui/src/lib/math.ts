/** Loads KaTeX the first time a document contains math. */
let katexReady: Promise<typeof import('katex').default> | null = null

function loadKatex() {
  katexReady ??= Promise.all([
    import('katex'),
    import('katex/dist/katex.min.css'),
  ]).then(([mod]) => mod.default)
  return katexReady
}

/** True when the renderer emitted a math span. */
export function hasMath(root: ParentNode): boolean {
  return root.querySelector('.math') !== null
}

/** Renders one pulldown-cmark math span with KaTeX. */
export function renderMathSpan(
  node: HTMLElement,
  katex: typeof import('katex').default,
): void {
  if (node.dataset.rendered === 'katex') {
    return
  }
  const tex = node.textContent ?? ''
  node.dataset.rendered = 'katex'
  try {
    katex.render(tex, node, {
      throwOnError: false,
      displayMode: node.classList.contains('math-display'),
    })
  } catch (cause) {
    node.dataset.rendered = 'error'
    node.textContent =
      cause instanceof Error
        ? cause.message
        : 'This formula could not be rendered.'
  }
}

/**
 * Observes math spans and typesets them just before they enter view.
 * `scope` limits which spans are new; `root` is the scrolling pane.
 */
export function observeMath(
  root: HTMLElement,
  enabled: boolean,
  scope: ParentNode = root,
): () => void {
  const nodes = [...scope.querySelectorAll<HTMLElement>('.math')]
  if (nodes.length === 0 || !enabled) {
    return () => {}
  }

  const observer = new IntersectionObserver(
    (entries) => {
      const visible = entries.filter((entry) => entry.isIntersecting)
      if (visible.length === 0) {
        return
      }
      void loadKatex()
        .then((katex) => {
          for (const entry of visible) {
            if (!(entry.target instanceof HTMLElement)) {
              continue
            }
            observer.unobserve(entry.target)
            renderMathSpan(entry.target, katex)
          }
        })
        .catch((cause) => {
          for (const entry of visible) {
            if (entry.target instanceof HTMLElement) {
              entry.target.textContent =
                cause instanceof Error
                  ? cause.message
                  : 'Mathematics could not be loaded.'
            }
          }
        })
    },
    { root, rootMargin: '400px', threshold: 0 },
  )
  for (const node of nodes) {
    observer.observe(node)
  }
  return () => observer.disconnect()
}
