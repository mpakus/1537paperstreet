import { describe, expect, it } from 'vitest'

import {
  DIAGRAM_ZOOM_MAX,
  DIAGRAM_ZOOM_MIN,
  PREVIEW_ZOOM_MAX,
  PREVIEW_ZOOM_MIN,
  nextDiagramZoom,
  nextPreviewZoom,
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
    expect(wheelDiagramZoom(1, -100)).toBe(1.25)
    expect(wheelDiagramZoom(1.25, 100)).toBe(1)
    expect(wheelDiagramZoom(1, 0)).toBe(1)
  })
})
