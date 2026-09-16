/** Smallest Preview/Split zoom. */
export const PREVIEW_ZOOM_MIN = 0.5
/** Largest Preview/Split zoom. */
export const PREVIEW_ZOOM_MAX = 2

/** Next zoom after a ±0.1 step, clamped to the supported range. */
export function nextPreviewZoom(current: number, delta: number): number {
  const stepped = Math.round((current + delta) * 10) / 10
  return Math.min(PREVIEW_ZOOM_MAX, Math.max(PREVIEW_ZOOM_MIN, stepped))
}

/** Formats a zoom factor for the toolbar, e.g. `100%`. */
export function previewZoomPercent(zoom: number): string {
  return `${Math.round(zoom * 100)}%`
}

/** Smallest Mermaid diagram lightbox zoom. */
export const DIAGRAM_ZOOM_MIN = 0.25
/** Largest Mermaid diagram lightbox zoom. */
export const DIAGRAM_ZOOM_MAX = 32
const DIAGRAM_ZOOM_FACTOR = 1.25

/** Multiplicative zoom step for the diagram lightbox, clamped. */
export function nextDiagramZoom(current: number, direction: 1 | -1): number {
  const next =
    direction > 0
      ? current * DIAGRAM_ZOOM_FACTOR
      : current / DIAGRAM_ZOOM_FACTOR
  return clampDiagramZoom(next)
}

/** Wheel zoom for the diagram lightbox. Negative deltaY zooms in. */
export function wheelDiagramZoom(current: number, deltaY: number): number {
  if (deltaY === 0) {
    return clampDiagramZoom(current)
  }
  return nextDiagramZoom(current, deltaY < 0 ? 1 : -1)
}

function clampDiagramZoom(value: number): number {
  const clamped = Math.min(DIAGRAM_ZOOM_MAX, Math.max(DIAGRAM_ZOOM_MIN, value))
  return Math.round(clamped * 1000) / 1000
}
