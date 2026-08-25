<script lang="ts">
  import {
    errorMessage,
    pickFolder,
    projectsList,
    projectsRelocate,
    projectsRemove,
    projectsRename,
    revealInFinder,
    type Project,
  } from '../lib/ipc'
  import { listboxFocusIndex } from '../lib/list-focus'
  import { highlightQuery } from '../lib/text'
  import {
    isTreeDrag,
    resolveTreeDrag,
    setTreeDragCopy,
    visibleWindow,
  } from '../lib/tree'

  let {
    activeId = null,
    reloadSeq = 0,
    onopen,
    onerror,
    onadd,
    onremoved,
    oncollapse,
    onfilesdrop,
    externalDropId = null,
  }: {
    activeId?: string | null
    reloadSeq?: number
    onopen: (project: Project) => void
    onerror: (message: string) => void
    onadd: () => void
    onremoved: (id: string) => void
    oncollapse?: () => void
    onfilesdrop?: (
      project: Project,
      payload: { projectId: string; paths: string[]; copy: boolean },
    ) => void
    externalDropId?: string | null
  } = $props()

  const ROW = 44
  const BUFFER = 20

  let query = $state('')
  let items = $state<Project[]>([])
  let total = $state(0)
  let scrollTop = $state(0)
  let viewportHeight = $state(0)
  let menu = $state<{ x: number; y: number; project: Project } | null>(null)
  let focused = $state(0)
  let renaming = $state<string | null>(null)
  let renameValue = $state('')
  let removeTarget = $state<Project | null>(null)
  let renameInput = $state<HTMLInputElement | undefined>()
  let dropTargetId = $state<string | null>(null)

  function projectFromEvent(event: DragEvent): Project | null {
    const el = event.target
    if (!(el instanceof Element)) {
      return null
    }
    const id = el.closest('[data-project-id]')?.getAttribute('data-project-id')
    if (!id) {
      return null
    }
    return items.find((project) => project.id === id) ?? null
  }

  function allowTreeDrop(event: DragEvent) {
    if (!isTreeDrag(event.dataTransfer)) {
      return false
    }
    event.preventDefault()
    event.stopPropagation()
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = event.altKey ? 'copy' : 'move'
    }
    const hovered = projectFromEvent(event)
    dropTargetId = hovered && hovered.available !== false ? hovered.id : null
    setTreeDragCopy(event.altKey)
    return true
  }

  function dropOnProject(event: DragEvent) {
    if (!isTreeDrag(event.dataTransfer)) {
      return
    }
    event.preventDefault()
    event.stopPropagation()
    const project =
      projectFromEvent(event) ??
      items.find((item) => item.id === dropTargetId) ??
      null
    dropTargetId = null
    if (!project || project.available === false) {
      return
    }
    const drag = resolveTreeDrag(event.dataTransfer)
    if (!drag || drag.paths.length === 0) {
      return
    }
    onfilesdrop?.(project, {
      projectId: drag.projectId,
      paths: drag.paths,
      copy: event.altKey,
    })
  }

  $effect(() => {
    const needle = query
    const generation = reloadSeq
    let cancelled = false
    const handle = setTimeout(() => {
      void (async () => {
        try {
          const page = await projectsList({
            query: needle.trim() ? needle : null,
            limit: 10_000,
            offset: 0,
          })
          if (cancelled || generation < 0) {
            return
          }
          items = page.items
          total = page.total
          focused = listboxFocusIndex(page.items.length, -1, focused)
        } catch (cause) {
          if (cancelled) {
            return
          }
          onerror(errorMessage(cause))
        }
      })()
    }, 30)
    return () => {
      cancelled = true
      clearTimeout(handle)
    }
  })

  $effect(() => {
    if (!activeId) {
      return
    }
    const activeIndex = items.findIndex((project) => project.id === activeId)
    if (activeIndex >= 0) {
      focused = activeIndex
    }
  })

  const range = $derived(
    visibleWindow(items.length, scrollTop, viewportHeight, ROW, BUFFER),
  )
  const visible = $derived(items.slice(range.start, range.end))

  $effect(() => {
    if (renaming && renameInput) {
      renameInput.focus()
      renameInput.select()
    }
  })

  async function commitRename(project: Project) {
    const next = renameValue.trim()
    renaming = null
    if (!next || next === project.name) {
      return
    }
    try {
      await projectsRename(project.id, next)
      items = items.map((item) =>
        item.id === project.id ? { ...item, name: next } : item,
      )
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function relocate(project: Project) {
    try {
      const path = await pickFolder()
      if (!path) {
        return
      }
      const updated = await projectsRelocate(project.id, path)
      items = items.map((item) => (item.id === project.id ? updated : item))
      if (activeId === project.id) {
        onopen(updated)
      }
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function remove(project: Project) {
    try {
      await projectsRemove(project.id)
      removeTarget = null
      items = items.filter((item) => item.id !== project.id)
      total = Math.max(0, total - 1)
      onremoved(project.id)
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (event.button === 0) {
      menu = null
    }
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      menu = null
      renaming = null
      removeTarget = null
    }
  }}
/>

<div class="pane" data-reload-seq={reloadSeq}>
  <div class="head">
    <h2 class="heading">Projects</h2>
    {#if oncollapse}
      <button
        type="button"
        class="collapse"
        title="Hide projects (⌘1)"
        aria-label="Hide projects"
        aria-expanded="true"
        onclick={oncollapse}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M10 3.5 5.5 8 10 12.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
  </div>
  <div class="toolbar">
    <input
      bind:value={query}
      placeholder="Search projects"
      aria-label="Search projects"
    />
    <button type="button" class="add" onclick={onadd}>Open Folder…</button>
  </div>
  <div
    class="list"
    role="listbox"
    tabindex="0"
    aria-label="Projects"
    bind:clientHeight={viewportHeight}
    onscroll={(event) => {
      scrollTop = event.currentTarget.scrollTop
    }}
    ondragover={allowTreeDrop}
    ondragenter={allowTreeDrop}
    ondragleave={(event) => {
      const next = event.relatedTarget
      if (next instanceof Node && event.currentTarget.contains(next)) {
        return
      }
      dropTargetId = null
    }}
    ondrop={dropOnProject}
    onkeydown={(event) => {
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        focused = Math.min(items.length - 1, focused + 1)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        focused = Math.max(0, focused - 1)
      } else if (event.key === 'Enter' || event.key === ' ') {
        const project = items[focused]
        if (project) {
          event.preventDefault()
          onopen(project)
        }
      }
    }}
  >
    {#if items.length === 0}
      <p class="empty">
        {#if query.trim()}
          No projects match.
        {:else}
          Your Markdown projects will appear here. Drop a Markdown file or a
          folder to open it.
        {/if}
      </p>
    {:else}
      <div class="spacer" style:height="{items.length * ROW}px">
        <div class="window" style:top="{range.start * ROW}px">
          {#each visible as project, offset (project.id)}
            {@const unavailable = project.available === false}
            <div
              class="row"
              class:active={project.id === activeId}
              class:focused={range.start + offset === focused}
              class:unavailable
              class:drop={dropTargetId === project.id ||
                externalDropId === project.id}
              data-project-id={project.id}
              role="option"
              aria-selected={project.id === activeId}
              onclick={() => {
                focused = range.start + offset
                onopen(project)
              }}
              oncontextmenu={(event) => {
                event.preventDefault()
                menu = { x: event.clientX, y: event.clientY, project }
              }}
              onkeydown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onopen(project)
                }
              }}
              tabindex="0"
            >
              {#if renaming === project.id}
                <input
                  class="rename"
                  bind:this={renameInput}
                  value={renameValue}
                  aria-label="Rename project"
                  onclick={(event) => event.stopPropagation()}
                  onpointerdown={(event) => event.stopPropagation()}
                  oninput={(event) => {
                    renameValue = event.currentTarget.value
                  }}
                  onkeydown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault()
                      void commitRename(project)
                    }
                    if (event.key === 'Escape') {
                      event.preventDefault()
                      renaming = null
                    }
                  }}
                  onblur={() => void commitRename(project)}
                />
              {:else}
                <span class="name">
                  {#each highlightQuery(project.name, query) as part, index (`${project.id}-n-${index}`)}
                    {#if part.hit}<mark>{part.text}</mark
                      >{:else}{part.text}{/if}
                  {/each}
                </span>
                <span class="path">
                  {#each highlightQuery(project.path, query) as part, index (`${project.id}-p-${index}`)}
                    {#if part.hit}<mark>{part.text}</mark
                      >{:else}{part.text}{/if}
                  {/each}
                </span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
  {#if total > items.length}
    <p class="count">{items.length} of {total}</p>
  {/if}
</div>

{#if menu}
  <div
    class="menu"
    style:left="{menu.x}px"
    style:top="{menu.y}px"
    role="menu"
    tabindex="-1"
    onpointerdown={(event) => event.stopPropagation()}
  >
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        const project = menu?.project
        menu = null
        if (project) {
          onopen(project)
        }
      }}>Open</button
    >
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        const project = menu?.project
        menu = null
        if (project) {
          renaming = project.id
          renameValue = project.name
        }
      }}>Rename</button
    >
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        const project = menu?.project
        menu = null
        if (project) {
          void revealInFinder(project.id, '').catch((cause) => {
            onerror(errorMessage(cause))
          })
        }
      }}>Reveal in Finder</button
    >
    {#if menu.project.available === false}
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          const project = menu?.project
          menu = null
          if (project) {
            void relocate(project)
          }
        }}>Find Folder…</button
      >
    {/if}
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        menu = null
        onerror('Export arrives in a later version.')
      }}>Export…</button
    >
    <button
      type="button"
      role="menuitem"
      class="danger"
      onclick={() => {
        const project = menu?.project
        menu = null
        if (project) {
          removeTarget = project
        }
      }}>Remove from List</button
    >
  </div>
{/if}

{#if removeTarget}
  <div class="confirm" role="dialog" aria-labelledby="remove-title">
    <div class="card">
      <p id="remove-title">Remove “{removeTarget.name}” from the list?</p>
      <p class="hint">
        Files on disk will stay. Only this list entry is removed.
      </p>
      <div class="actions">
        <button type="button" onclick={() => (removeTarget = null)}
          >Cancel</button
        >
        <button
          type="button"
          class="danger"
          onclick={() => {
            if (removeTarget) {
              void remove(removeTarget)
            }
          }}>Remove</button
        >
      </div>
    </div>
  </div>
{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 32px;
    padding: var(--space-2) var(--space-2) 0;
  }

  .heading {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }

  .collapse {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    transition-property: background-color, color, transform;
    transition-duration: var(--duration);
  }

  .collapse:hover {
    background: var(--selection);
    color: var(--fg);
  }

  .collapse:active {
    transform: scale(0.96);
  }

  .collapse svg {
    width: 14px;
    height: 14px;
  }

  @media (prefers-reduced-motion: reduce) {
    .collapse:active {
      transform: none;
    }
  }

  .toolbar {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-2);
  }

  input {
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
  }

  .add {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    color: var(--fg);
  }

  .list {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: auto;
    -webkit-app-region: no-drag;
  }

  .empty,
  .count {
    margin: 0;
    padding: var(--space-3);
    color: var(--fg-muted);
    font-size: 0.8125rem;
  }

  .spacer {
    position: relative;
  }

  .window {
    position: absolute;
    inset-inline: 0;
  }

  .row {
    display: flex;
    flex-direction: column;
    justify-content: center;
    height: 44px;
    padding: 0 var(--space-3);
    min-width: 0;
    -webkit-app-region: no-drag;
  }

  .row.active,
  .row:hover {
    background: var(--selection);
  }

  .list:focus-visible .row.focused {
    background: var(--selection);
  }

  .row.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: var(--selection);
  }

  .row.unavailable .name,
  .row.unavailable .path {
    color: var(--fg-muted);
    opacity: 0.7;
  }

  .name,
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: 0.8125rem;
    color: var(--fg);
  }

  .path {
    font-size: 0.6875rem;
    color: var(--fg-muted);
  }

  mark {
    background: var(--selection);
    color: var(--accent);
    padding: 0;
  }

  .rename {
    font: inherit;
  }

  .menu {
    position: fixed;
    z-index: 20;
    min-width: 12rem;
    padding: var(--space-1);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .menu button {
    display: block;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    text-align: start;
    border-radius: var(--radius-sm);
    color: var(--fg);
  }

  .menu button:hover {
    background: var(--selection);
  }

  .danger {
    color: var(--accent);
  }

  .confirm {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .card {
    width: min(22rem, 100%);
    padding: var(--space-4);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .card p {
    margin: 0 0 var(--space-2);
  }

  .hint {
    color: var(--fg-muted);
    font-size: 0.8125rem;
  }

  .actions {
    display: flex;
    justify-content: end;
    gap: var(--space-2);
  }

  .actions button {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
  }
</style>
