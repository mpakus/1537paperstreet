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
/** Default diagram-window width in logical pixels. */
export const DIAGRAM_FRAME_DEFAULT_WIDTH = 896
/** Default diagram-window height in logical pixels. */
export const DIAGRAM_FRAME_DEFAULT_HEIGHT = 576
const DIAGRAM_ZOOM_FACTOR = 1.25
/** Pixel-wheel scale so a ~100 px notch is about 17%, not a full 1.25× step. */
const DIAGRAM_WHEEL_PIXEL_SCALE = 0.0016
const DOM_DELTA_LINE = 1
const DOM_DELTA_PAGE = 2
const WHEEL_LINE_PX = 16
const WHEEL_PAGE_PX = 800

/** Multiplicative zoom step for the diagram lightbox, clamped. */
export function nextDiagramZoom(current: number, direction: 1 | -1): number {
  const next =
    direction > 0
      ? current * DIAGRAM_ZOOM_FACTOR
      : current / DIAGRAM_ZOOM_FACTOR
  return clampDiagramZoom(next)
}

/** Wheel zoom for the diagram lightbox. Negative deltaY zooms in. */
export function wheelDiagramZoom(
  current: number,
  deltaY: number,
  deltaMode = 0,
): number {
  if (deltaY === 0) {
    return clampDiagramZoom(current)
  }
  let pixels = deltaY
  if (deltaMode === DOM_DELTA_LINE) {
    pixels *= WHEEL_LINE_PX
  } else if (deltaMode === DOM_DELTA_PAGE) {
    pixels *= WHEEL_PAGE_PX
  }
  return clampDiagramZoom(
    current * Math.exp(-pixels * DIAGRAM_WHEEL_PIXEL_SCALE),
  )
}

/** Pan offset that keeps `pointer` over the same content after a zoom change. */
export function panAfterZoom(
  panX: number,
  panY: number,
  pointerX: number,
  pointerY: number,
  fromZoom: number,
  toZoom: number,
): { x: number; y: number } {
  if (fromZoom <= 0 || fromZoom === toZoom) {
    return { x: panX, y: panY }
  }
  const scale = toZoom / fromZoom
  return {
    x: pointerX - (pointerX - panX) * scale,
    y: pointerY - (pointerY - panY) * scale,
  }
}

/** Clamps a diagram zoom factor to the supported range. */
export function clampDiagramZoom(value: number): number {
  if (!Number.isFinite(value)) {
    return 1
  }
  const clamped = Math.min(DIAGRAM_ZOOM_MAX, Math.max(DIAGRAM_ZOOM_MIN, value))
  return Math.round(clamped * 1000) / 1000
}
