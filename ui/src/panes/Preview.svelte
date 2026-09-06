<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity'

  import type { TocEntry } from '../lib/generated/core'
  import { enhanceCodeBlocks } from '../lib/code'
  import { copySvg, observeMermaid, savePng } from '../lib/diagrams'
  import { errorMessage } from '../lib/ipc'
  import { observeMath } from '../lib/math'

  let {
    html,
    emptyMessage,
    toc = [],
    tocWidth = 224,
    banner = null,
    themeId = 'paper-light',
    mermaidEnabled = true,
    mathEnabled = true,
    previewFont = '',
    previewFontSize = 0,
    previewBg = '',
    previewFg = '',
    readingZoom = 1,
    articleEl = $bindable(),
    onnavigate,
    onerror,
    ontocresize,
  }: {
    html: string
    emptyMessage: string
    toc?: TocEntry[]
    tocWidth?: number
    banner?: string | null
    themeId?: string
    mermaidEnabled?: boolean
    mathEnabled?: boolean
    previewFont?: string
    previewFontSize?: number
    previewBg?: string
    previewFg?: string
    readingZoom?: number
    articleEl?: HTMLElement | undefined
    onnavigate: (href: string) => void
    onerror?: (message: string) => void
    ontocresize?: (event: PointerEvent) => void
  } = $props()

  let activeId = $state<string | null>(null)
  let modalSvg = $state<string | null>(null)
  let zoom = $state(1)
  let panX = $state(0)
  let panY = $state(0)
  let dragging = $state<{
    x: number
    y: number
    panX: number
    panY: number
  } | null>(null)
  const collapsed = new SvelteSet<string>()
  let expanded = $state(false)

  function jump(event: MouseEvent, id: string) {
    event.preventDefault()
    const heading = articleEl?.querySelector(`#${CSS.escape(id)}`)
    heading?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  }

  function hasChildren(index: number): boolean {
    const current = toc[index]
    const next = toc[index + 1]
    return Boolean(current && next && next.level > current.level)
  }

  function hiddenByCollapse(index: number): boolean {
    const entry = toc[index]
    if (!entry) {
      return false
    }
    for (let previous = index - 1; previous >= 0; previous -= 1) {
      const ancestor = toc[previous]
      if (!ancestor || ancestor.level >= entry.level) {
        continue
      }
      if (collapsed.has(ancestor.id)) {
        return true
      }
    }
    return false
  }

  $effect(() => {
    const host = articleEl
    const markup = html
    const theme = themeId
    if (!host || !markup) {
      return
    }
    for (const image of host.querySelectorAll('img')) {
      image.loading = 'lazy'
      image.decoding = 'async'
    }
    const stopCode = enhanceCodeBlocks(host, (message) => {
      onerror?.(message)
    })
    const preview = host.parentElement
    const stopMermaid = preview
      ? observeMermaid(preview, theme, mermaidEnabled, onerror)
      : () => {}
    const stopMath = observeMath(host, mathEnabled)
    const headings = [...host.querySelectorAll('h1, h2, h3, h4, h5, h6')]
    if (headings.length === 0) {
      return () => {
        stopCode()
        stopMermaid()
        stopMath()
      }
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort(
            (left, right) =>
              left.boundingClientRect.top - right.boundingClientRect.top,
          )
        const id = visible[0]?.target.id
        if (id) {
          activeId = id
        }
      },
      { root: host, rootMargin: '-10% 0px -70% 0px', threshold: 0 },
    )
    for (const heading of headings) {
      observer.observe(heading)
    }
    return () => {
      observer.disconnect()
      stopCode()
      stopMermaid()
      stopMath()
    }
  })

  $effect(() => {
    if (!expanded && !modalSvg) {
      return
    }
    function onKey(event: KeyboardEvent) {
      if (event.key !== 'Escape') {
        return
      }
      if (modalSvg) {
        modalSvg = null
        event.preventDefault()
        return
      }
      expanded = false
      event.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })
</script>

<div
  class="pane"
  class:is-full={expanded}
  style:--read-font={previewFont
    ? `"${previewFont}", "Iowan Old Style", Palatino, serif`
    : undefined}
  style:--read-size={previewFontSize ? `${previewFontSize}px` : undefined}
  style:--read-bg={previewBg || undefined}
  style:--read-fg={previewFg || undefined}
>
  <button
    type="button"
    class="fullsize"
    title={expanded ? 'Exit full size' : 'Full size'}
    aria-label={expanded ? 'Exit full size' : 'Full size'}
    aria-pressed={expanded}
    onclick={() => (expanded = !expanded)}
  >
    {#if expanded}
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M5 3H3v2M11 3h2v2M5 13H3v-2M11 13h2v-2"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {:else}
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M3 6V3h3M13 6V3h-3M3 10v3h3M13 10v3h-3"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </button>
  {#if banner}
    <p class="banner" role="status">{banner}</p>
  {/if}

  <div class="body">
    {#if toc.length > 0}
      <nav
        class="toc"
        aria-label="Table of contents"
        style:width="{tocWidth}px"
      >
        {#each toc as entry, index (entry.id)}
          {#if !hiddenByCollapse(index)}
            <div class="toc-row">
              {#if hasChildren(index)}
                <button
                  type="button"
                  class="twist"
                  aria-expanded={!collapsed.has(entry.id)}
                  onclick={() => {
                    if (collapsed.has(entry.id)) {
                      collapsed.delete(entry.id)
                    } else {
                      collapsed.add(entry.id)
                    }
                  }}>{collapsed.has(entry.id) ? '▸' : '▾'}</button
                >
              {:else}
                <span class="twist"></span>
              {/if}
              <a
                href="#{entry.id}"
                class:active={entry.id === activeId}
                style:padding-inline-start="calc({entry.level - 1} * var(--space-3))"
                onclick={(event) => jump(event, entry.id)}>{entry.title}</a
              >
            </div>
          {/if}
        {/each}
      </nav>
      <div
        class="toc-resize"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize table of contents"
        onpointerdown={(event) => ontocresize?.(event)}
      ></div>
    {/if}

    {#if html}
      <div
        class="preview"
        role="presentation"
        onclick={(event) => {
          const figure = (event.target as HTMLElement | null)?.closest(
            'figure.mermaid',
          )
          if (
            figure instanceof HTMLElement &&
            figure.dataset.rendered === 'svg'
          ) {
            event.preventDefault()
            const svg = figure.querySelector('svg')?.outerHTML
            if (svg) {
              modalSvg = svg
              zoom = 1
              panX = 0
              panY = 0
            }
            return
          }
          const link = (event.target as HTMLElement | null)?.closest('a')
          if (!link) {
            return
          }
          const href = link.getAttribute('href')
          if (!href) {
            return
          }
          event.preventDefault()
          onnavigate(href)
        }}
      >
        <!-- HTML is sanitized by ps-render before it crosses IPC. -->
        <article bind:this={articleEl} style:zoom={readingZoom}>
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html html}
        </article>
      </div>
    {:else}
      <p class="empty" style:zoom={readingZoom}>{emptyMessage}</p>
    {/if}
  </div>
</div>

{#if modalSvg}
  <div
    class="diagram-scrim"
    role="dialog"
    tabindex="-1"
    aria-label="Diagram"
    onclick={(event) => {
      if (event.target === event.currentTarget) {
        modalSvg = null
      }
    }}
    onkeydown={(event) => {
      if (event.key === 'Escape') {
        modalSvg = null
      }
    }}
  >
    <div class="diagram-sheet" role="document">
      <div class="diagram-toolbar">
        <button
          type="button"
          onclick={() => {
            void copySvg(modalSvg ?? '').catch((cause) => {
              onerror?.(errorMessage(cause))
            })
          }}>Copy SVG</button
        >
        <button
          type="button"
          onclick={() => {
            void savePng(modalSvg ?? '').catch((cause) => {
              onerror?.(errorMessage(cause))
            })
          }}>Save PNG</button
        >
        <button type="button" onclick={() => (zoom = Math.min(4, zoom + 0.25))}
          >Zoom in</button
        >
        <button
          type="button"
          onclick={() => (zoom = Math.max(0.25, zoom - 0.25))}>Zoom out</button
        >
        <button type="button" onclick={() => (modalSvg = null)}>Close</button>
      </div>
      <div
        class="diagram-stage"
        role="presentation"
        onwheel={(event) => {
          event.preventDefault()
          zoom = Math.min(
            4,
            Math.max(0.25, zoom + (event.deltaY < 0 ? 0.1 : -0.1)),
          )
        }}
        onpointerdown={(event) => {
          dragging = { x: event.clientX, y: event.clientY, panX, panY }
        }}
        onpointermove={(event) => {
          if (!dragging) {
            return
          }
          panX = dragging.panX + event.clientX - dragging.x
          panY = dragging.panY + event.clientY - dragging.y
        }}
        onpointerup={() => {
          dragging = null
        }}
        onpointerleave={() => {
          dragging = null
        }}
      >
        <div
          class="diagram-pan"
          style:transform="translate({panX}px, {panY}px) scale({zoom})"
        >
          <!-- SVG was produced by Mermaid with securityLevel: strict. -->
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html modalSvg}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    background: var(--read-bg, var(--bg));
    color: var(--read-fg, var(--fg));
  }

  .pane.is-full {
    position: fixed;
    inset: var(--titlebar-h) 0 0 0;
    z-index: 25;
  }

  .fullsize {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    z-index: 2;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: var(--fg-muted);
    background: color-mix(in srgb, var(--bg-elev) 88%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    -webkit-app-region: no-drag;
  }

  .fullsize svg {
    width: 12px;
    height: 12px;
  }

  .fullsize:hover {
    color: var(--fg);
    background: var(--bg-elev);
  }

  .banner {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    color: var(--fg);
    background: var(--selection);
    border-bottom: 1px solid var(--border);
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .toc {
    display: flex;
    flex-direction: column;
    flex: none;
    width: var(--toc-w, 14rem);
    max-height: 100%;
    overflow: auto;
    padding: var(--space-4) var(--space-3);
    background: var(--sidebar);
  }

  .toc-resize {
    width: var(--space-1);
    flex: none;
    cursor: col-resize;
    background: var(--border);
  }

  .toc-resize:hover {
    background: var(--accent);
  }

  .toc-row {
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .twist {
    width: var(--space-3);
    flex: none;
    color: var(--fg-muted);
  }

  .toc a {
    flex: 1;
    min-width: 0;
    padding-block: var(--space-1);
    color: var(--fg-muted);
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toc a:hover,
  .toc a.active {
    color: var(--fg);
  }

  .preview {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }

  .preview article {
    font-family: var(--read-font, var(--font-body));
    font-size: var(--read-size, var(--font-size));
    line-height: var(--line-height);
    max-width: calc(var(--measure-ch) * 1ch);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    color: inherit;
  }

  .preview :global(a) {
    color: var(--accent);
  }

  .preview :global(img),
  .preview :global(video) {
    max-width: 100%;
    height: auto;
  }

  .preview :global(code),
  .preview :global(pre) {
    font-family: var(--font-mono);
    background: var(--code-bg);
  }

  .preview :global(pre) {
    padding: var(--space-3);
    overflow: auto;
    border-radius: var(--radius);
  }

  .empty {
    flex: 1;
    overflow: auto;
    max-width: 36rem;
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    color: var(--fg-muted);
  }

  .diagram-scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .diagram-sheet {
    display: grid;
    grid-template-rows: auto 1fr;
    width: min(56rem, 100%);
    height: min(36rem, 100%);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .diagram-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2);
    border-bottom: 1px solid var(--border);
  }

  .diagram-toolbar button {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
  }

  .diagram-stage {
    overflow: hidden;
    cursor: grab;
    background: var(--bg);
  }

  .diagram-pan {
    transform-origin: 0 0;
    width: max-content;
    padding: var(--space-4);
  }
</style>
