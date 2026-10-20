<script lang="ts">
  import { untrack } from 'svelte'
  import type { GraphSnapshot } from '../lib/generated/core'
  import { GraphRequest } from '../lib/graph-request'
  import GraphCanvas from './GraphCanvas.svelte'

  let {
    projectId,
    projectName,
    watchSeq = 0,
    onopen,
  }: {
    projectId: string | null
    projectName: string
    watchSeq?: number
    onopen: (path: string) => void
  } = $props()
  let snapshot = $state<GraphSnapshot | null>(null)
  let search = $state(''),
    focus = $state<string | null>(null),
    hideIsolated = $state(false)
  let selectedPath = $state<string | null>(null),
    error = $state(''),
    busy = $state(false)
  let refresh = $state(0),
    scrollTop = $state(0)
  let map = $state<GraphCanvas>()
  let noteList: HTMLDivElement
  let rebuild = false
  let previousProject: string | null = null
  const request = new GraphRequest()
  const selected = $derived(
    snapshot?.nodes.find((node) => node.path === selectedPath) ?? null,
  )
  const start = $derived(Math.max(0, Math.floor(scrollTop / 48) - 3))
  const visible = $derived(snapshot?.nodes.slice(start, start + 30) ?? [])

  $effect(() => {
    const id = projectId
    if (id !== previousProject) {
      previousProject = id
      untrack(() => {
        snapshot = null
        selectedPath = null
        focus = null
        search = ''
        error = ''
      })
    }
    const query = { search, focus, hideIsolated, rebuild }
    rebuild = false
    void watchSeq
    void refresh
    request.invalidate()
    if (!id) {
      busy = false
      return
    }
    busy = true
    const timer = setTimeout(() => {
      void request.load(
        id,
        query,
        (result) => {
          snapshot = result
          busy = false
          error = ''
          scrollTop = 0
          if (noteList) noteList.scrollTop = 0
          if (!result.nodes.some((node) => node.path === selectedPath))
            selectedPath = null
        },
        (message) => {
          error = message
          busy = false
        },
      )
    }, 180)
    return () => {
      clearTimeout(timer)
      request.invalidate()
    }
  })
</script>

<section class="graph" aria-label="Document graph" aria-busy={busy}>
  <header>
    <div class="identity">
      <span class="eyebrow">LOCAL KNOWLEDGE / RELATIONSHIP MAP</span>
      <h1>Constellation <span>Graph</span></h1>
      <p>{projectName || 'Your documents, connected'}</p>
    </div>
    <div class="header-actions">
      <span class="local"><i></i> On your device</span>
      <button disabled={!projectId || busy} onclick={() => refresh++}
        >Refresh</button
      >
      <button
        disabled={!projectId || busy}
        onclick={() => {
          rebuild = true
          refresh++
        }}>Rebuild cache</button
      >
    </div>
  </header>
  <div class="controls">
    <label class="search"
      ><span>Find a note</span><input
        type="search"
        bind:value={search}
        placeholder="Search document paths…"
        disabled={!projectId}
      /></label
    >
    <div class="scope" aria-label="Graph scope">
      <button
        class:active={!focus}
        aria-pressed={!focus}
        onclick={() => (focus = null)}>Whole project</button
      ><button
        class:active={Boolean(focus)}
        aria-pressed={Boolean(focus)}
        disabled={!selected && !focus}
        onclick={() => {
          focus = selectedPath
          search = ''
        }}>Neighborhood</button
      >
    </div>
    <label class="isolated"
      ><input type="checkbox" bind:checked={hideIsolated} /> Hide isolated notes</label
    >
  </div>
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  {#each snapshot?.notices ?? [] as notice (notice)}<div class="notice">
      {notice}
    </div>{/each}
  <div class="body">
    <div class="map">
      {#if snapshot && snapshot.nodes.length > 0}
        <GraphCanvas
          bind:this={map}
          {snapshot}
          {selectedPath}
          onselect={(path) => (selectedPath = path)}
          {onopen}
        />
        <div class="map-caption">
          <span class="crosshair">⌖</span>
          {focus ? 'LOCAL ORBIT' : 'PROJECT FIELD'}
          <span>Links from saved Markdown</span>
        </div>
        <div class="navigation" aria-label="Map controls">
          <button
            aria-label="Zoom in"
            title="Zoom in (+)"
            onclick={() => map?.zoom(1.25)}>+</button
          ><button
            aria-label="Zoom out"
            title="Zoom out (−)"
            onclick={() => map?.zoom(0.8)}>−</button
          ><button onclick={() => map?.fit()}>Fit</button>
        </div>
        <div class="map-help">
          Drag to explore <span>·</span> Scroll to zoom <span>·</span> Double-click
          to open
        </div>
      {:else}
        <div class="empty">
          <div class="empty-orbit" aria-hidden="true">✦</div>
          <h2>
            {!projectId
              ? 'Choose a project to explore'
              : busy
                ? 'Mapping your documents…'
                : error
                  ? 'The graph could not load'
                  : snapshot?.totalNodes
                    ? 'No notes in this view'
                    : 'Your constellation starts with a note'}
          </h2>
          <p>
            {!projectId
              ? 'Select a folder in Projects, then return here.'
              : snapshot?.totalNodes
                ? 'Try another search or return to the whole project.'
                : 'Saved Markdown files become stars. Markdown and [[wiki links]] connect them.'}
          </p>
        </div>
      {/if}
      {#if busy && snapshot}<div class="updating" role="status">
          Updating connections…
        </div>{/if}
    </div>
    <aside aria-label="Graph notes">
      {#if selected}
        <div class="inspector">
          <span class="eyebrow">SELECTED NOTE</span>
          <h2>{selected.title}</h2>
          <p class="path">{selected.path}</p>
          <div class="connections">
            <span><strong>{selected.incoming}</strong> incoming</span><span
              ><strong>{selected.outgoing}</strong> outgoing</span
            >
          </div>
          <div class="note-actions">
            <button class="primary" onclick={() => onopen(selected.path)}
              >Open note ↗</button
            ><button
              onclick={() => {
                focus = selected.path
                search = ''
              }}>Focus here</button
            >
          </div>
        </div>
      {:else}<div class="inspector intro">
          <span class="eyebrow">FOLLOW A THREAD</span>
          <h2>See the connections.</h2>
          <p>Select a star or a note below to explore what links to it.</p>
        </div>{/if}
      <div class="list-heading">
        <span>NOTES IN VIEW</span><span>{snapshot?.nodes.length ?? 0}</span>
      </div>
      <div
        class="note-list"
        bind:this={noteList}
        role="listbox"
        tabindex="0"
        aria-label="Notes in the graph"
        aria-activedescendant={selectedPath
          ? `graph-note-${snapshot?.nodes.findIndex((node) => node.path === selectedPath)}`
          : undefined}
        onkeydown={(event) => {
          const nodes = snapshot?.nodes ?? []
          let index = nodes.findIndex((node) => node.path === selectedPath)
          if (event.key === 'ArrowDown')
            index = Math.min(nodes.length - 1, index + 1)
          else if (event.key === 'ArrowUp') index = Math.max(0, index - 1)
          else if (event.key === 'Home') index = 0
          else if (event.key === 'End') index = nodes.length - 1
          else if (event.key === 'Enter' && selectedPath) {
            onopen(selectedPath)
            event.preventDefault()
            return
          } else return
          event.preventDefault()
          if (nodes[index]) {
            selectedPath = nodes[index].path
            noteList.scrollTop = Math.max(
              0,
              index * 48 - noteList.clientHeight / 2,
            )
          }
        }}
        onscroll={(event) => (scrollTop = event.currentTarget.scrollTop)}
      >
        <div
          style:height="{(snapshot?.nodes.length ?? 0) * 48}px"
          class="virtual-list"
        >
          {#each visible as node, index (node.path)}
            <button
              class="note"
              role="option"
              tabindex="-1"
              id="graph-note-{start + index}"
              aria-selected={selectedPath === node.path}
              class:selected={selectedPath === node.path}
              style:top="{(start + index) * 48}px"
              title={node.path}
              onclick={() => {
                selectedPath = node.path
                noteList.focus()
              }}
              ondblclick={() => onopen(node.path)}
              ><span class="star" aria-hidden="true">✦</span><span
                class="note-label"
                ><strong>{node.title}</strong><small
                  >{node.folder || 'Project root'}</small
                ></span
              ><span class="degree">{node.incoming + node.outgoing}</span
              ></button
            >
          {/each}
        </div>
      </div>
    </aside>
  </div>
  <footer>
    <div>
      <strong>{snapshot?.totalNodes ?? 0}</strong> notes <span>/</span>
      <strong>{snapshot?.totalEdges ?? 0}</strong> connections
    </div>
    <p>Encrypted notes excluded <span>·</span> Saved files only</p>
    <span
      class="cache"
      title="Unchanged files reuse links stored in the local database"
      >{busy
        ? 'Indexing…'
        : `${snapshot?.cachedFiles ?? 0} cached · ${snapshot?.parsedFiles ?? 0} updated`}</span
    >
  </footer>
</section>

<style>
  .graph {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    background: var(--bg);
    color: var(--fg);
    container-type: inline-size;
    line-height: 1.4;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 24px 16px;
    border-bottom: 1px solid var(--border);
  }
  .eyebrow {
    font: 9px var(--font-mono);
    letter-spacing: 0.14em;
    color: var(--fg-muted);
  }
  h1 {
    margin: 6px 0 3px;
    font-size: 25px;
    font-weight: 500;
    letter-spacing: -0.04em;
  }
  h1 span {
    font-size: 11px;
    font-weight: 400;
    letter-spacing: 0;
    color: var(--fg-muted);
    margin-left: 8px;
  }
  p {
    margin: 0;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.6;
  }
  .header-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: end;
    align-items: center;
    gap: 8px;
  }
  button {
    border: 1px solid var(--border);
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    background: var(--bg);
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }
  button:hover {
    color: var(--fg);
    border-color: var(--fg-muted);
    background: var(--bg-elev);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .local {
    color: var(--fg-muted);
    font-size: 10px;
    margin-right: 8px;
    white-space: nowrap;
  }
  .local i {
    display: inline-block;
    width: 5px;
    height: 5px;
    margin-right: 5px;
    border-radius: 50%;
    background: var(--hl-str);
    box-shadow: 0 0 8px var(--hl-str);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 210px;
    font-size: 11px;
    color: var(--fg-muted);
  }
  .search input {
    width: 100%;
    min-width: 120px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    font-size: 12px;
  }
  .search span {
    white-space: nowrap;
  }
  .scope {
    display: flex;
  }
  .scope button {
    border-radius: 0;
  }
  .scope button:first-child {
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  }
  .scope button:last-child {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    margin-left: -1px;
  }
  .scope button.active {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, var(--bg));
  }
  .isolated {
    display: flex;
    gap: 5px;
    align-items: center;
    font-size: 11px;
    color: var(--fg-muted);
  }
  input[type='checkbox'] {
    accent-color: var(--accent);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .map {
    flex: 1;
    min-width: 0;
    position: relative;
    overflow: hidden;
    background: radial-gradient(
      ellipse at 45% 50%,
      color-mix(in srgb, var(--accent) 5%, transparent),
      transparent 65%
    );
  }
  .map-caption {
    position: absolute;
    top: 18px;
    left: 20px;
    font: 9px var(--font-mono);
    letter-spacing: 0.1em;
    color: var(--fg-muted);
    pointer-events: none;
  }
  .map-caption > span:last-child {
    display: block;
    font: 10px var(--font-ui);
    letter-spacing: 0;
    opacity: 0.75;
    margin: 6px 0 0 20px;
  }
  .crosshair {
    color: var(--accent);
    font-size: 14px;
  }
  .navigation {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    gap: 4px;
  }
  .navigation button {
    min-width: 30px;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--bg) 80%, transparent);
  }
  .map-help {
    position: absolute;
    left: 20px;
    bottom: 22px;
    color: var(--fg-muted);
    font-size: 10px;
    pointer-events: none;
  }
  .map-help span,
  footer p span {
    margin: 0 5px;
    opacity: 0.5;
  }
  aside {
    display: flex;
    flex-direction: column;
    width: 230px;
    flex: none;
    border-left: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-elev) 60%, var(--bg));
    min-height: 0;
  }
  .inspector {
    padding: 22px 16px 18px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    font-size: 17px;
    font-weight: 500;
    line-height: 1.3;
    margin: 10px 0 7px;
    overflow-wrap: anywhere;
  }
  .path {
    font: 10px/1.6 var(--font-mono);
    overflow-wrap: anywhere;
  }
  .intro p {
    font-size: 11px;
  }
  .connections {
    display: flex;
    gap: 18px;
    margin: 18px 0;
    color: var(--fg-muted);
    font-size: 10px;
  }
  .connections strong {
    display: block;
    color: var(--fg);
    font: 22px var(--font-mono);
    font-weight: 400;
    margin-bottom: 4px;
  }
  .note-actions {
    display: flex;
    gap: 6px;
  }
  .primary {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  }
  .list-heading {
    display: flex;
    justify-content: space-between;
    padding: 14px 16px 10px;
    color: var(--fg-muted);
    font: 9px var(--font-mono);
    letter-spacing: 0.08em;
  }
  .note-list {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .virtual-list {
    position: relative;
  }
  .note {
    position: absolute;
    left: 0;
    right: 0;
    width: 100%;
    height: 48px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 16px;
    border: 0;
    border-radius: 0;
    text-align: left;
    background: transparent;
  }
  .note.selected {
    background: var(--selection);
    box-shadow: inset 2px 0 var(--accent);
    color: var(--fg);
  }
  .star {
    color: var(--accent);
    font-size: 13px;
  }
  .note-label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .note strong {
    font-size: 11px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note small {
    color: var(--fg-muted);
    font-size: 9px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .degree {
    color: var(--fg-muted);
    font: 10px var(--font-mono);
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 10px 24px;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
    font: 10px var(--font-mono);
  }
  footer strong {
    color: var(--fg);
    font-weight: 500;
  }
  footer div > span {
    padding: 0 7px;
    opacity: 0.4;
  }
  footer p {
    flex: 1;
    font-size: 10px;
    text-align: center;
  }
  .cache {
    font-size: 9px;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px;
    text-align: center;
    box-sizing: border-box;
  }
  .empty p {
    max-width: 330px;
  }
  .empty-orbit {
    display: grid;
    place-items: center;
    width: 90px;
    height: 90px;
    border: 1px solid var(--border);
    border-radius: 50%;
    color: var(--accent);
    font-size: 26px;
    box-shadow:
      0 0 0 20px color-mix(in srgb, var(--border) 30%, transparent),
      0 0 50px color-mix(in srgb, var(--accent) 8%, transparent);
    margin-bottom: 28px;
  }
  .notice {
    padding: 8px 24px;
    background: var(--selection);
    color: var(--fg);
    font-size: 11px;
  }
  .error {
    border-left: 3px solid var(--dirty);
  }
  .updating {
    position: absolute;
    top: 18px;
    right: 18px;
    padding: 7px 10px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--fg-muted);
    border-radius: var(--radius-sm);
    font-size: 10px;
  }
  @container (max-width: 900px) {
    .local {
      display: none;
    }
    header,
    .controls {
      padding: 12px 16px;
    }
    aside {
      width: 190px;
    }
    .map-help {
      display: none;
    }
    footer p {
      display: none;
    }
    footer {
      justify-content: space-between;
      padding: 8px 16px;
    }
  }
  @container (max-width: 620px) {
    header {
      align-items: flex-start;
    }
    h1 {
      font-size: 21px;
    }
    .eyebrow {
      font-size: 8px;
    }
    .header-actions {
      max-width: 100px;
    }
    .header-actions button {
      font-size: 10px;
      padding: 4px 6px;
    }
    .body {
      flex-direction: column;
    }
    .map {
      min-height: 200px;
    }
    aside {
      width: auto;
      height: 150px;
      flex-direction: row;
      border-left: 0;
      border-top: 1px solid var(--border);
    }
    .inspector {
      width: 42%;
      padding: 10px 12px;
      border-bottom: 0;
      border-right: 1px solid var(--border);
      overflow: auto;
    }
    .connections {
      margin: 8px 0;
    }
    .connections strong {
      display: inline;
      font-size: 12px;
    }
    .list-heading {
      display: none;
    }
    .note-list {
      flex: 1;
    }
    .search {
      flex-basis: 100%;
    }
    .intro h2 {
      font-size: 14px;
    }
  }
</style>
