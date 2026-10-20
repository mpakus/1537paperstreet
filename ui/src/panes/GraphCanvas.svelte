<script lang="ts">
  import { onMount } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import type { GraphSnapshot } from '../lib/generated/core'

  let {
    snapshot,
    selectedPath,
    onselect,
    onopen,
  }: {
    snapshot: GraphSnapshot
    selectedPath: string | null
    onselect: (path: string) => void
    onopen: (path: string) => void
  } = $props()

  let canvas: HTMLCanvasElement
  let width = 1,
    height = 1,
    scale = 1,
    panX = 0,
    panY = 0
  let mounted = false
  let hovered = -1
  let drag: { x: number; y: number; moved: boolean } | null = null
  let lastSnapshot: GraphSnapshot | null = null

  export function fit() {
    const extent = snapshot.nodes.reduce(
      (extent, node) => ({
        x: Math.max(extent.x, Math.abs(node.x)),
        y: Math.max(extent.y, Math.abs(node.y)),
      }),
      { x: 80, y: 80 },
    )
    scale = Math.min(
      2,
      Math.max(
        0.08,
        Math.min(
          (width - 100) / (extent.x * 2),
          (height - 120) / (extent.y * 2),
        ),
      ),
    )
    panX = 0
    panY = 0
    draw()
  }

  export function zoom(factor: number, x = width / 2, y = height / 2) {
    const next = Math.min(6, Math.max(0.06, scale * factor))
    const ratio = next / scale
    panX = (panX + width / 2 - x) * ratio + x - width / 2
    panY = (panY + height / 2 - y) * ratio + y - height / 2
    scale = next
    draw()
  }

  function project(x: number, y: number) {
    return [width / 2 + panX + x * scale, height / 2 + panY + y * scale]
  }
  function hit(x: number, y: number) {
    let nearest = -1,
      distance = 18
    snapshot.nodes.forEach((node, i) => {
      const [nx, ny] = project(node.x, node.y)
      const d = Math.hypot(x - nx, y - ny)
      if (d < distance) {
        distance = d
        nearest = i
      }
    })
    return nearest
  }

  function draw() {
    if (!mounted) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const styles = getComputedStyle(canvas)
    const token = (name: string) => styles.getPropertyValue(name).trim()
    const accent = token('--accent'),
      muted = token('--fg-muted'),
      foreground = token('--fg')
    const colors = [
      accent,
      token('--hl-type'),
      token('--hl-fn'),
      token('--hl-kw'),
      token('--hl-str'),
    ]
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    if (canvas.width !== Math.round(width * dpr))
      canvas.width = Math.round(width * dpr)
    if (canvas.height !== Math.round(height * dpr))
      canvas.height = Math.round(height * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, width, height)
    const cx = width / 2,
      cy = height / 2
    // Fine orbital guides anchor the map without moving the camera.
    ctx.strokeStyle = muted
    ctx.lineWidth = 0.6
    ctx.globalAlpha = 0.13
    for (let radius = 100; radius < Math.max(width, height); radius += 100) {
      ctx.beginPath()
      ctx.ellipse(cx, cy, radius, radius * 0.78, -0.16, 0, Math.PI * 2)
      ctx.stroke()
    }
    ctx.setLineDash([2, 8])
    ctx.beginPath()
    ctx.moveTo(0, cy)
    ctx.lineTo(width, cy)
    ctx.moveTo(cx, 0)
    ctx.lineTo(cx, height)
    ctx.stroke()
    ctx.setLineDash([])
    ctx.fillStyle = muted
    for (let i = 0; i < 100; i++) {
      const x = (((i * 997 + 31) % 1013) / 1013) * width
      const y = (((i * 487 + 61) % 991) / 991) * height
      ctx.globalAlpha = i % 3 === 0 ? 0.3 : 0.12
      ctx.fillRect(x, y, i % 7 === 0 ? 2 : 1, 1)
    }
    const selected = snapshot.nodes.findIndex(
      (node) => node.path === selectedPath,
    )
    const active = hovered >= 0 ? hovered : selected
    const adjacent = new SvelteSet<number>([active])
    snapshot.edges.forEach((edge) => {
      if (edge.source === active) adjacent.add(edge.target)
      if (edge.target === active) adjacent.add(edge.source)
    })
    for (const edge of snapshot.edges) {
      const source = snapshot.nodes[edge.source],
        target = snapshot.nodes[edge.target]
      if (!source || !target) continue
      const [ax, ay] = project(source.x, source.y),
        [bx, by] = project(target.x, target.y)
      const lit = edge.source === active || edge.target === active
      ctx.globalAlpha = lit ? 0.85 : active >= 0 ? 0.07 : 0.2
      ctx.strokeStyle = lit ? accent : muted
      ctx.lineWidth = lit ? 1.5 : 0.7
      ctx.beginPath()
      ctx.moveTo(ax, ay)
      ctx.lineTo(bx, by)
      ctx.stroke()
      if (lit) {
        const angle = Math.atan2(by - ay, bx - ax),
          x = ax + (bx - ax) * 0.7,
          y = ay + (by - ay) * 0.7
        ctx.beginPath()
        ctx.moveTo(x - Math.cos(angle - 0.5) * 7, y - Math.sin(angle - 0.5) * 7)
        ctx.lineTo(x, y)
        ctx.lineTo(x - Math.cos(angle + 0.5) * 7, y - Math.sin(angle + 0.5) * 7)
        ctx.stroke()
      }
    }
    const occupied: { x: number; y: number; w: number }[] = []
    // Active labels get first choice of space; all other labels avoid collisions.
    const indices = snapshot.nodes.map((_, i) => i)
    if (active >= 0) {
      indices.splice(active, 1)
      indices.unshift(active)
    }
    for (const i of indices) {
      const node = snapshot.nodes[i],
        [x, y] = project(node.x, node.y)
      if (x < -40 || y < -40 || x > width + 40 || y > height + 40) continue
      let hash = 0
      for (const char of node.folder)
        hash = (hash * 31 + char.charCodeAt(0)) >>> 0
      const color = colors[hash % colors.length]
      const lit = adjacent.has(i),
        focused = i === active
      const radius =
        Math.min(9, 3.2 + Math.sqrt(node.incoming + node.outgoing) * 0.75) *
        Math.min(1.3, Math.max(0.8, scale))
      ctx.globalAlpha = active >= 0 && !lit ? 0.28 : 1
      ctx.shadowColor = color
      ctx.shadowBlur = focused ? 24 : lit ? 12 : 5
      ctx.fillStyle = color
      ctx.beginPath()
      ctx.arc(x, y, radius, 0, Math.PI * 2)
      ctx.fill()
      ctx.shadowBlur = 0
      if (focused || i === selected) {
        ctx.strokeStyle = color
        ctx.lineWidth = 1
        ctx.beginPath()
        ctx.arc(x, y, radius + 5, 0, Math.PI * 2)
        ctx.stroke()
        ctx.globalAlpha = 0.2
        ctx.beginPath()
        ctx.arc(x, y, radius + 13, 0, Math.PI * 2)
        ctx.stroke()
      }
      if (focused || lit || snapshot.nodes.length < 45 || scale > 0.7) {
        ctx.font = `${focused ? '600' : '400'} 11px ${styles.fontFamily}`
        const title =
          node.title.length > 34 ? `${node.title.slice(0, 32)}…` : node.title
        const tw = ctx.measureText(title).width,
          tx = x + radius + 8,
          ty = y + 4
        if (
          focused ||
          !occupied.some(
            (box) =>
              Math.abs(box.y - ty) < 16 &&
              tx < box.x + box.w + 8 &&
              tx + tw + 8 > box.x,
          )
        ) {
          occupied.push({ x: tx, y: ty, w: tw })
          ctx.globalAlpha = active >= 0 && !lit ? 0.25 : focused ? 1 : 0.8
          ctx.lineWidth = 3
          ctx.strokeStyle = token('--bg')
          ctx.strokeText(title, tx, ty)
          ctx.fillStyle = focused ? foreground : muted
          ctx.fillText(title, tx, ty)
        }
      }
    }
    ctx.globalAlpha = 1
  }

  onMount(() => {
    mounted = true
    const resize = new ResizeObserver(([entry]) => {
      width = entry.contentRect.width
      height = entry.contentRect.height
      fit()
    })
    resize.observe(canvas)
    const theme = new MutationObserver(draw)
    theme.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme', 'style'],
    })
    return () => {
      mounted = false
      resize.disconnect()
      theme.disconnect()
    }
  })

  $effect(() => {
    const next = snapshot
    void selectedPath
    const frame = requestAnimationFrame(() => {
      const changed =
        next.nodes.map((node) => node.path).join('\0') !==
        lastSnapshot?.nodes.map((node) => node.path).join('\0')
      lastSnapshot = next
      if (changed) {
        hovered = -1
        fit()
      } else draw()
    })
    return () => cancelAnimationFrame(frame)
  })
</script>

<canvas
  bind:this={canvas}
  tabindex="0"
  aria-label="Document constellation. Drag to pan, scroll to zoom. Arrow keys pan, plus and minus zoom, zero fits the map. Select a note in the list, then Enter opens it."
  onwheel={(event) => {
    event.preventDefault()
    zoom(Math.exp(-event.deltaY * 0.0015), event.offsetX, event.offsetY)
  }}
  onpointerdown={(event) => {
    if (event.button !== 0) return
    canvas.focus()
    canvas.setPointerCapture(event.pointerId)
    drag = { x: event.clientX, y: event.clientY, moved: false }
  }}
  onpointermove={(event) => {
    if (drag) {
      const dx = event.clientX - drag.x,
        dy = event.clientY - drag.y
      if (drag.moved || Math.hypot(dx, dy) > 3) {
        panX += dx
        panY += dy
        drag = { x: event.clientX, y: event.clientY, moved: true }
        draw()
      }
    } else {
      const next = hit(event.offsetX, event.offsetY)
      if (next !== hovered) {
        hovered = next
        canvas.style.cursor = next < 0 ? 'grab' : 'pointer'
        draw()
      }
    }
  }}
  onpointerup={(event) => {
    if (drag && !drag.moved) {
      const node = snapshot.nodes[hit(event.offsetX, event.offsetY)]
      if (node) onselect(node.path)
    }
    drag = null
    if (canvas.hasPointerCapture(event.pointerId))
      canvas.releasePointerCapture(event.pointerId)
  }}
  onpointercancel={() => {
    drag = null
  }}
  onpointerleave={() => {
    hovered = -1
    draw()
  }}
  ondblclick={(event) => {
    const node = snapshot.nodes[hit(event.offsetX, event.offsetY)]
    if (node) onopen(node.path)
  }}
  onkeydown={(event) => {
    const amount = event.shiftKey ? 80 : 30
    if (event.key === 'ArrowLeft') panX += amount
    else if (event.key === 'ArrowRight') panX -= amount
    else if (event.key === 'ArrowUp') panY += amount
    else if (event.key === 'ArrowDown') panY -= amount
    else if (event.key === '+' || event.key === '=') zoom(1.2)
    else if (event.key === '-') zoom(1 / 1.2)
    else if (event.key === '0') fit()
    else if (event.key === 'Enter' && selectedPath) onopen(selectedPath)
    else return
    event.preventDefault()
    draw()
  }}
></canvas>

<style>
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    touch-action: none;
    cursor: grab;
    font-family: var(--font-ui);
  }
  canvas:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -3px;
  }
</style>
