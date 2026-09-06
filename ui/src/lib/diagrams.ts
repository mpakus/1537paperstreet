import { mermaidCacheGet, mermaidCachePut, saveUserFile } from './ipc'

export type MermaidThemeVariables = {
  background: string
  primaryColor: string
  primaryTextColor: string
  primaryBorderColor: string
  lineColor: string
  secondaryColor: string
  tertiaryColor: string
  fontFamily: string
}

const memory = new Map<string, string>()
let renderSeq = 0
let mermaidReady: Promise<
  typeof import('../../vendor/mermaid.esm.min.mjs')
> | null = null

/** Reads CSS variables used to theme Mermaid. */
export function mermaidThemeVariables(
  style: CSSStyleDeclaration,
): MermaidThemeVariables {
  return {
    background: style.getPropertyValue('--bg').trim() || '#fbfaf7',
    primaryColor: style.getPropertyValue('--selection').trim() || '#f0dfd8',
    primaryTextColor: style.getPropertyValue('--fg').trim() || '#1e1c1a',
    primaryBorderColor: style.getPropertyValue('--border').trim() || '#e3dfd8',
    lineColor: style.getPropertyValue('--fg-muted').trim() || '#6b6763',
    secondaryColor: style.getPropertyValue('--bg-elev').trim() || '#ffffff',
    tertiaryColor: style.getPropertyValue('--code-bg').trim() || '#f4f2ed',
    fontFamily:
      style.getPropertyValue('--font-ui').trim() || 'system-ui, sans-serif',
  }
}

/** Source text stored in the renderer’s `<template>` placeholder. */
export function diagramSource(figure: Element): string {
  const template = figure.querySelector('template')
  if (!template) {
    return ''
  }
  // WebKit keeps <template> children on `content`, so `textContent` is empty.
  const fragment = (template as { content?: { textContent?: string | null } })
    .content?.textContent
  return (fragment || template.textContent || '').replace(/\u00a0/g, ' ')
}

/** Memory + disk cache key: blake3(source) already sits on `data-hash`. */
export function cacheKey(sourceHash: string, themeId: string): string {
  return `${sourceHash}:${themeId}`
}

function loadMermaid() {
  mermaidReady ??= import('../../vendor/mermaid.esm.min.mjs')
  return mermaidReady
}

/** Dynamically loads Mermaid the first time a document contains a diagram. */
export async function renderMermaidFigure(
  figure: HTMLElement,
  themeId: string,
  style: CSSStyleDeclaration,
  onerror?: (message: string) => void,
): Promise<void> {
  if (
    figure.dataset.rendered === 'svg' ||
    figure.dataset.rendered === 'pending'
  ) {
    return
  }

  const sourceHash = figure.dataset.hash ?? ''
  const source = diagramSource(figure)
  if (!sourceHash || !source.trim()) {
    return
  }

  figure.dataset.rendered = 'pending'
  const key = cacheKey(sourceHash, themeId)
  let cached = memory.get(key)
  if (!cached) {
    try {
      cached = (await mermaidCacheGet(sourceHash, themeId)) ?? undefined
    } catch (cause) {
      figure.dataset.cacheError =
        cause instanceof Error ? cause.message : String(cause)
    }
  }
  if (cached) {
    memory.set(key, cached)
    showSvg(figure, cached)
    return
  }

  const mod = await loadMermaid()
  const mermaid = mod.default
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    suppressErrorRendering: true,
    theme: 'base',
    themeVariables: mermaidThemeVariables(style),
    flowchart: { htmlLabels: false },
  })
  renderSeq += 1
  const id = `mermaid-${sourceHash.slice(0, 12)}-${renderSeq}`
  try {
    const { svg } = await mermaid.render(id, source)
    memory.set(key, svg)
    try {
      await mermaidCachePut(sourceHash, themeId, svg)
    } catch (cause) {
      figure.dataset.cacheError =
        cause instanceof Error ? cause.message : String(cause)
    }
    showSvg(figure, svg)
  } catch (cause) {
    removeMermaidOrphans(id)
    onerror?.(showDiagramError(figure, source, cause))
  }
}

function showSvg(figure: HTMLElement, svg: string): void {
  figure.classList.remove('mermaid-error')
  figure.dataset.rendered = 'svg'
  figure.replaceChildren()
  const frame = document.createElement('div')
  frame.className = 'mermaid-svg'
  frame.innerHTML = svg
  figure.append(frame)
}

/** Drops leftover parse-error nodes Mermaid inserts outside the figure. */
function removeMermaidOrphans(id: string): void {
  document.getElementById(id)?.remove()
  document.getElementById(`d${id}`)?.remove()
}

/** Replaces a figure with the error message and the diagram source. */
export function showDiagramError(
  figure: HTMLElement,
  source: string,
  cause: unknown,
): string {
  figure.classList.add('mermaid-error')
  figure.dataset.rendered = 'error'
  const message =
    cause instanceof Error ? cause.message : 'This diagram could not be drawn.'
  const status = document.createElement('p')
  status.className = 'mermaid-error-msg'
  status.setAttribute('role', 'status')
  status.textContent = message
  const pre = document.createElement('pre')
  pre.textContent = source
  figure.replaceChildren(status, pre)
  return message
}

/** Observes mermaid figures and renders them just before they enter view. */
export function observeMermaid(
  root: HTMLElement,
  themeId: string,
  enabled: boolean,
  onerror?: (message: string) => void,
): () => void {
  const figures = [...root.querySelectorAll<HTMLElement>('figure.mermaid')]
  if (figures.length === 0) {
    return () => {}
  }
  if (!enabled) {
    for (const figure of figures) {
      const message = showDiagramError(
        figure,
        diagramSource(figure),
        new Error('Mermaid is turned off in Settings.'),
      )
      onerror?.(message)
    }
    return () => {}
  }

  const style = getComputedStyle(document.documentElement)
  let cancelled = false
  const start = (figure: HTMLElement) => {
    if (cancelled) {
      return
    }
    void renderMermaidFigure(figure, themeId, style, onerror).catch((cause) => {
      onerror?.(showDiagramError(figure, diagramSource(figure), cause))
    })
  }

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) {
          continue
        }
        const figure = entry.target
        if (!(figure instanceof HTMLElement)) {
          continue
        }
        observer.unobserve(figure)
        start(figure)
      }
    },
    { root, rootMargin: '400px', threshold: 0 },
  )
  for (const figure of figures) {
    observer.observe(figure)
  }

  // WKWebView often skips the first IntersectionObserver callback, and a
  // <template>-only figure can report an empty intersection rect until drawn.
  const flushVisible = () => {
    const rootRect = root.getBoundingClientRect()
    for (const figure of figures) {
      if (figure.dataset.rendered) {
        continue
      }
      const rect = figure.getBoundingClientRect()
      const near =
        rect.height === 0 ||
        (rect.bottom >= rootRect.top - 400 && rect.top <= rootRect.bottom + 400)
      if (near) {
        observer.unobserve(figure)
        start(figure)
      }
    }
  }
  let frame = 0
  if (typeof requestAnimationFrame === 'function') {
    frame = requestAnimationFrame(() => requestAnimationFrame(flushVisible))
  } else {
    flushVisible()
  }

  return () => {
    cancelled = true
    observer.disconnect()
    if (typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(frame)
    }
  }
}

/** Turns an SVG string into a PNG blob for the Save dialog. */
export async function svgToPng(svg: string): Promise<Blob> {
  const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  try {
    const image = new Image()
    const loaded = new Promise<void>((resolve, reject) => {
      image.onload = () => resolve()
      image.onerror = () =>
        reject(new Error('Could not rasterize the diagram.'))
    })
    image.src = url
    await loaded
    const canvas = document.createElement('canvas')
    canvas.width = Math.max(1, image.naturalWidth)
    canvas.height = Math.max(1, image.naturalHeight)
    const context = canvas.getContext('2d')
    if (!context) {
      throw new Error('Could not rasterize the diagram.')
    }
    context.drawImage(image, 0, 0)
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((png) => {
        if (png) {
          resolve(png)
        } else {
          reject(new Error('Could not rasterize the diagram.'))
        }
      }, 'image/png')
    })
  } finally {
    URL.revokeObjectURL(url)
  }
}

/** Native save of a PNG chosen by the user. */
export async function savePng(svg: string): Promise<void> {
  const { save } = await import('@tauri-apps/plugin-dialog')
  const path = await save({
    defaultPath: 'diagram.png',
    filters: [{ name: 'PNG', extensions: ['png'] }],
  })
  if (!path) {
    return
  }
  const png = await svgToPng(svg)
  const bytes = Array.from(new Uint8Array(await png.arrayBuffer()))
  await saveUserFile(path, bytes)
}

/** Copies SVG markup to the clipboard. */
export async function copySvg(svg: string): Promise<void> {
  await navigator.clipboard.writeText(svg)
}
