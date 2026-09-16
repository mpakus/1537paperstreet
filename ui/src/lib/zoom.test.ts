import { describe, expect, it } from 'vitest'

import {
  DIAGRAM_ZOOM_MAX,
  DIAGRAM_ZOOM_MIN,
  PREVIEW_ZOOM_MAX,
  PREVIEW_ZOOM_MIN,
  nextDiagramZoom,
  nextPreviewZoom,
  panAfterZoom,
  previewZoomPercent,
  wheelDiagramZoom,
} from './zoom'

describe('nextPreviewZoom', () => {
  it('steps by tenths and stays inside the reading range', () => {
    expect(nextPreviewZoom(1, 0.1)).toBe(1.1)
    expect(nextPreviewZoom(1, -0.1)).toBe(0.9)
    expect(nextPreviewZoom(PREVIEW_ZOOM_MIN, -0.1)).toBe(PREVIEW_ZOOM_MIN)
    expect(nextPreviewZoom(PREVIEW_ZOOM_MAX, 0.1)).toBe(PREVIEW_ZOOM_MAX)
  })

  it('formats a percent label', () => {
    expect(previewZoomPercent(1)).toBe('100%')
    expect(previewZoomPercent(1.2)).toBe('120%')
  })
})

describe('nextDiagramZoom', () => {
  it('scales by 1.25 and stays inside a deep range', () => {
    expect(nextDiagramZoom(1, 1)).toBe(1.25)
    expect(nextDiagramZoom(1.25, 1)).toBe(1.563)
    expect(nextDiagramZoom(1.25, -1)).toBe(1)
    expect(nextDiagramZoom(DIAGRAM_ZOOM_MIN, -1)).toBe(DIAGRAM_ZOOM_MIN)
    expect(nextDiagramZoom(DIAGRAM_ZOOM_MAX, 1)).toBe(DIAGRAM_ZOOM_MAX)
    expect(DIAGRAM_ZOOM_MAX).toBeGreaterThan(8)
  })

  it('zooms in on wheel up and out on wheel down', () => {
    expect(wheelDiagramZoom(1, -10)).toBeGreaterThan(1)
    expect(wheelDiagramZoom(1, -10)).toBeLessThan(1.05)
    expect(wheelDiagramZoom(1, -100)).toBeCloseTo(1.174, 3)
    expect(wheelDiagramZoom(1, -100)).toBeLessThan(nextDiagramZoom(1, 1))
    expect(wheelDiagramZoom(1, 100)).toBeCloseTo(0.852, 3)
    expect(wheelDiagramZoom(1, 0)).toBe(1)
    expect(wheelDiagramZoom(DIAGRAM_ZOOM_MAX, -100)).toBe(DIAGRAM_ZOOM_MAX)
    expect(wheelDiagramZoom(DIAGRAM_ZOOM_MIN, 100)).toBe(DIAGRAM_ZOOM_MIN)
  })

  it('treats line-mode wheel ticks as larger than a few pixels', () => {
    expect(wheelDiagramZoom(1, -1, 1)).toBeGreaterThan(
      wheelDiagramZoom(1, -1, 0),
    )
  })

  it('keeps the pointer over the same content when zooming', () => {
    expect(panAfterZoom(0, 0, 100, 50, 1, 2)).toEqual({ x: -100, y: -50 })
    expect(panAfterZoom(0, 0, 0, 0, 1, 2)).toEqual({ x: 0, y: 0 })
    expect(panAfterZoom(20, 10, 100, 50, 2, 2)).toEqual({ x: 20, y: 10 })
  })
})
