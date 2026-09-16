<script lang="ts">
  import { copySvg, savePng } from '../lib/diagrams'
  import { errorMessage } from '../lib/ipc'
  import {
    DIAGRAM_ZOOM_MAX,
    DIAGRAM_ZOOM_MIN,
    nextDiagramZoom,
    wheelDiagramZoom,
  } from '../lib/zoom'

  let {
    svg,
    onclose,
    onerror,
  }: {
    svg: string
    onclose: () => void
    onerror?: (message: string) => void
  } = $props()

  const minWidth = 360
  const minHeight = 220
  const edges = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'] as const
  type Edge = (typeof edges)[number]

  let zoom = $state(1)
  let panX = $state(0)
  let panY = $state(0)
  let panDrag = $state<{
    x: number
    y: number
    panX: number
    panY: number
  } | null>(null)
  let moveDrag = $state<{
    x: number
    y: number
    left: number
    top: number
  } | null>(null)
  let sizeDrag = $state<{
    x: number
    y: number
    edge: Edge
    left: number
    top: number
    width: number
    height: number
  } | null>(null)
  let frame = $state(defaultFrame())

  function viewport() {
    if (typeof window === 'undefined') {
      return { width: 1200, height: 800 }
    }
    return { width: window.innerWidth, height: window.innerHeight }
  }

  function defaultFrame() {
    const { width: vw, height: vh } = viewport()
    const width = Math.min(896, Math.max(minWidth, vw - 48))
    const height = Math.min(576, Math.max(minHeight, vh - 48))
    return {
      left: Math.max(16, (vw - width) / 2),
      top: Math.max(16, (vh - height) / 2),
      width,
      height,
    }
  }

  function clampFrame(next: {
    left: number
    top: number
    width: number
    height: number
  }) {
    const { width: vw, height: vh } = viewport()
    const width = Math.min(vw - 16, Math.max(minWidth, next.width))
    const height = Math.min(vh - 16, Math.max(minHeight, next.height))
    const left = Math.min(vw - 72, Math.max(72 - width, next.left))
    const top = Math.min(vh - 40, Math.max(8, next.top))
    return { left, top, width, height }
  }

  function resizedFrame(
    edge: Edge,
    dx: number,
    dy: number,
    start: {
      left: number
      top: number
      width: number
      height: number
    },
  ) {
    let { left, top, width, height } = start
    if (edge.includes('e')) {
      width = start.width + dx
    }
    if (edge.includes('s')) {
      height = start.height + dy
    }
    if (edge.includes('w')) {
      width = start.width - dx
      left = start.left + start.width - Math.max(minWidth, width)
    }
    if (edge.includes('n')) {
      height = start.height - dy
      top = start.top + start.height - Math.max(minHeight, height)
    }
    return clampFrame({ left, top, width, height })
  }

  function onTitlePointerDown(event: PointerEvent) {
    if (event.button !== 0) {
      return
    }
    const target = event.target as HTMLElement | null
    if (target?.closest('button')) {
      return
    }
    event.preventDefault()
    const host = event.currentTarget
    if (!(host instanceof HTMLElement)) {
      return
    }
    host.setPointerCapture(event.pointerId)
    moveDrag = {
      x: event.clientX,
      y: event.clientY,
      left: frame.left,
      top: frame.top,
    }
  }

  function onTitlePointerMove(event: PointerEvent) {
    if (!moveDrag) {
      return
    }
    frame = clampFrame({
      left: moveDrag.left + event.clientX - moveDrag.x,
      top: moveDrag.top + event.clientY - moveDrag.y,
      width: frame.width,
      height: frame.height,
    })
  }

  function onResizePointerDown(event: PointerEvent, edge: Edge) {
    if (event.button !== 0) {
      return
    }
    event.preventDefault()
    event.stopPropagation()
    const host = event.currentTarget
    if (!(host instanceof HTMLElement)) {
      return
    }
    host.setPointerCapture(event.pointerId)
    sizeDrag = {
      x: event.clientX,
      y: event.clientY,
      edge,
      left: frame.left,
      top: frame.top,
      width: frame.width,
      height: frame.height,
    }
  }

  function onResizePointerMove(event: PointerEvent) {
    if (!sizeDrag) {
      return
    }
    frame = resizedFrame(
      sizeDrag.edge,
      event.clientX - sizeDrag.x,
      event.clientY - sizeDrag.y,
      sizeDrag,
    )
  }
</script>

<div
  class="diagram-scrim"
  role="presentation"
  tabindex="-1"
  onclick={(event) => {
    if (event.target === event.currentTarget) {
      onclose()
    }
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      onclose()
    }
  }}
>
  <div
    class="diagram-sheet"
    class:is-moving={moveDrag !== null}
    class:is-sizing={sizeDrag !== null}
    role="dialog"
    aria-modal="true"
    aria-labelledby="diagram-modal-title"
    tabindex="-1"
    style:left="{frame.left}px"
    style:top="{frame.top}px"
    style:width="{frame.width}px"
    style:height="{frame.height}px"
  >
    <div class="diagram-titlebar">
      <h2
        id="diagram-modal-title"
        class="diagram-title"
        onpointerdown={onTitlePointerDown}
        onpointermove={onTitlePointerMove}
        onpointerup={() => {
          moveDrag = null
        }}
        onpointercancel={() => {
          moveDrag = null
        }}
      >
        Diagram
      </h2>
      <div class="diagram-actions">
        <button
          type="button"
          class="diagram-icon"
          title="Copy SVG"
          aria-label="Copy SVG"
          onclick={() => {
            void copySvg(svg).catch((cause) => {
              onerror?.(errorMessage(cause))
            })
          }}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect
              x="5.5"
              y="4.5"
              width="7"
              height="9"
              rx="1"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M3.5 11.5H3a1 1 0 0 1-1-1V3.5A1 1 0 0 1 3 2.5h6.5a1 1 0 0 1 1 1V4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
        <button
          type="button"
          class="diagram-icon"
          title="Save PNG"
          aria-label="Save PNG"
          onclick={() => {
            void savePng(svg).catch((cause) => {
              onerror?.(errorMessage(cause))
            })
          }}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M8 3v7M5.25 7.5 8 10.25 10.75 7.5M3.5 13h9"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
        <button
          type="button"
          class="diagram-icon"
          title="Zoom out"
          aria-label="Zoom out"
          disabled={zoom <= DIAGRAM_ZOOM_MIN}
          onclick={() => {
            zoom = nextDiagramZoom(zoom, -1)
          }}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle
              cx="6.75"
              cy="6.75"
              r="4.25"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M10 10.25 13.25 13.5M5 6.75h3.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
        <button
          type="button"
          class="diagram-icon"
          title="Zoom in"
          aria-label="Zoom in"
          disabled={zoom >= DIAGRAM_ZOOM_MAX}
          onclick={() => {
            zoom = nextDiagramZoom(zoom, 1)
          }}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle
              cx="6.75"
              cy="6.75"
              r="4.25"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M10 10.25 13.25 13.5M6.75 5v3.5M5 6.75h3.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
      <button
        type="button"
        class="diagram-icon diagram-close"
        title="Close"
        aria-label="Close"
        onclick={onclose}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M4 4l8 8M12 4l-8 8"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
    <div
      class="diagram-stage"
      role="presentation"
      onwheel={(event) => {
        event.preventDefault()
        zoom = wheelDiagramZoom(zoom, event.deltaY)
      }}
      onpointerdown={(event) => {
        if (event.button !== 0) {
          return
        }
        event.currentTarget.setPointerCapture(event.pointerId)
        panDrag = { x: event.clientX, y: event.clientY, panX, panY }
      }}
      onpointermove={(event) => {
        if (!panDrag) {
          return
        }
        panX = panDrag.panX + event.clientX - panDrag.x
        panY = panDrag.panY + event.clientY - panDrag.y
      }}
      onpointerup={() => {
        panDrag = null
      }}
      onpointercancel={() => {
        panDrag = null
      }}
    >
      <div
        class="diagram-pan"
        style:transform="translate({panX}px, {panY}px) scale({zoom})"
      >
        <!-- SVG was produced by Mermaid with securityLevel: strict. -->
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html svg}
      </div>
    </div>
    {#each edges as edge (edge)}
      <div
        class="diagram-resize diagram-resize-{edge}"
        class:is-grip={edge === 'se'}
        role="separator"
        aria-label={edge === 'se' ? 'Resize diagram' : undefined}
        aria-hidden={edge === 'se' ? undefined : true}
        onpointerdown={(event) => onResizePointerDown(event, edge)}
        onpointermove={onResizePointerMove}
        onpointerup={() => {
          sizeDrag = null
        }}
        onpointercancel={() => {
          sizeDrag = null
        }}
      ></div>
    {/each}
  </div>
</div>

<style>
  .diagram-scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .diagram-sheet {
    position: absolute;
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 360px;
    min-height: 220px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: 0 16px 40px color-mix(in srgb, var(--fg) 18%, transparent);
  }

  .diagram-sheet.is-moving,
  .diagram-sheet.is-sizing {
    user-select: none;
  }

  .diagram-titlebar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 40px;
    padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
    border-bottom: 1px solid var(--border);
  }

  .diagram-title {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: grab;
    user-select: none;
    touch-action: none;
  }

  .diagram-sheet.is-moving .diagram-title {
    cursor: grabbing;
  }

  .diagram-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
    cursor: default;
  }

  .diagram-icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    color: var(--fg-muted);
    border-radius: var(--radius-sm);
  }

  .diagram-icon svg {
    width: 16px;
    height: 16px;
  }

  .diagram-icon:hover:not(:disabled) {
    color: var(--fg);
    background: var(--selection);
  }

  .diagram-icon:active:not(:disabled) {
    transform: scale(0.96);
  }

  .diagram-icon:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .diagram-close {
    flex: none;
  }

  .diagram-stage {
    overflow: hidden;
    cursor: grab;
    background: var(--bg);
    touch-action: none;
  }

  .diagram-stage:active {
    cursor: grabbing;
  }

  .diagram-pan {
    transform-origin: 0 0;
    width: max-content;
    padding: var(--space-4);
  }

  .diagram-resize {
    position: absolute;
    z-index: 2;
  }

  .diagram-resize-n,
  .diagram-resize-s {
    left: 10px;
    right: 10px;
    height: 6px;
    cursor: ns-resize;
  }

  .diagram-resize-e,
  .diagram-resize-w {
    top: 10px;
    bottom: 10px;
    width: 6px;
    cursor: ew-resize;
  }

  .diagram-resize-n {
    top: 0;
  }

  .diagram-resize-s {
    bottom: 0;
  }

  .diagram-resize-e {
    right: 0;
  }

  .diagram-resize-w {
    left: 0;
  }

  .diagram-resize-ne,
  .diagram-resize-nw,
  .diagram-resize-se,
  .diagram-resize-sw {
    width: 12px;
    height: 12px;
  }

  .diagram-resize-ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  .diagram-resize-nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  .diagram-resize-se {
    right: 0;
    bottom: 0;
    cursor: nwse-resize;
  }

  .diagram-resize-sw {
    left: 0;
    bottom: 0;
    cursor: nesw-resize;
  }

  .diagram-resize-se.is-grip::after {
    content: '';
    position: absolute;
    right: 3px;
    bottom: 3px;
    width: 9px;
    height: 9px;
    background:
      linear-gradient(
        135deg,
        transparent 45%,
        var(--fg-muted) 45%,
        var(--fg-muted) 55%,
        transparent 55%
      ),
      linear-gradient(
        135deg,
        transparent 20%,
        var(--fg-muted) 20%,
        var(--fg-muted) 30%,
        transparent 30%
      );
    pointer-events: none;
  }
</style>
