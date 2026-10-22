<script lang="ts">
  import type { LibraryNote } from '../lib/generated/core'
  let {
    notes,
    busy,
    onopen,
    onadd,
    onreview,
  }: {
    notes: LibraryNote[]
    busy: boolean
    onopen: (note: LibraryNote) => void
    onadd: (note: LibraryNote, section: string | null) => void
    onreview: (note: LibraryNote) => void
  } = $props()
  let selectedKey = $state('')
  const selected = $derived(
    notes.find((note) => `${note.projectId}/${note.path}` === selectedKey) ??
      null,
  )
  const selectedIndex = $derived(selected ? notes.indexOf(selected) : -1)
  let section = $state('')
  const sectionAvailable = $derived(
    !section ||
      Boolean(selected?.sections.some((heading) => heading.id === section)),
  )
  let scrollTop = $state(0)
  let list: HTMLDivElement
  const start = $derived(
    Math.max(0, Math.min(Math.floor(scrollTop / 64) - 3, notes.length - 1)),
  )
  const visible = $derived(notes.slice(start, start + 30))
  function choose(note: LibraryNote) {
    selectedKey = `${note.projectId}/${note.path}`
    section = ''
  }
  function key(event: KeyboardEvent) {
    if (!notes.length) return
    const index = notes.findIndex(
      (n) => n.projectId === selected?.projectId && n.path === selected?.path,
    )
    let next: number
    if (event.key === 'ArrowDown') next = Math.min(notes.length - 1, index + 1)
    else if (event.key === 'ArrowUp') next = Math.max(0, index - 1)
    else if (event.key === 'Home') next = 0
    else if (event.key === 'End') next = notes.length - 1
    else if (event.key === 'Enter' && selected) {
      event.preventDefault()
      onopen(selected)
      return
    } else return
    event.preventDefault()
    choose(notes[next])
    list.scrollTop = Math.max(0, next * 64 - 128)
    scrollTop = list.scrollTop
  }
</script>

<div class="documents">
  <div class="list-heading">
    <h2>Documents</h2>
    <span>{notes.length} results</span>
  </div>
  <div
    class="note-list"
    role="listbox"
    aria-label="Library documents"
    tabindex="0"
    bind:this={list}
    aria-activedescendant={selectedIndex >= start &&
    selectedIndex < start + visible.length
      ? `library-note-${selectedIndex}`
      : undefined}
    onkeydown={key}
    onscroll={() => (scrollTop = list.scrollTop)}
  >
    <div style:height={`${notes.length * 64}px`} class="rows">
      {#each visible as note, offset (`${note.projectId}/${note.path}`)}
        <div
          class="note"
          role="option"
          tabindex="-1"
          id={`library-note-${start + offset}`}
          aria-selected={selected?.path === note.path &&
            selected?.projectId === note.projectId}
          style:top={`${(start + offset) * 64}px`}
          onclick={() => {
            choose(note)
            list.focus({ preventScroll: true })
          }}
          onkeydown={(event) => {
            if (event.key === 'Enter') {
              event.stopPropagation()
              onopen(note)
            }
          }}
          ondblclick={() => onopen(note)}
        >
          <div class="title">
            <strong>{note.title}</strong><span
              class:changed={note.freshness === 'changed'}
              >{note.freshness}</span
            >
          </div>
          <div class="metadata">
            {note.noteType} · {note.status || 'No status'} · {note.projectName} /
            {note.path}
          </div>
        </div>
      {/each}
    </div>
    {#if notes.length === 0}<p class="empty">
        {busy
          ? 'Reading saved notes…'
          : 'No notes match. Adjust the filters or create a structured note.'}
      </p>{/if}
  </div>
  {#if selected}
    <div class="inspector">
      <strong>{selected.title}</strong>
      <p>{selected.projectName} / {selected.path}</p>
      {#if selected.source}<p>Source: {selected.source}</p>{/if}
      {#if selected.tags.length}<p>Topics: {selected.tags.join(', ')}</p>{/if}
      <label
        >Include <select
          value={section}
          onchange={(event) => {
            section = event.currentTarget.value
          }}
          ><option value="">Whole document</option
          >{#each selected.sections as heading (heading.id)}<option
              value={heading.id}>{heading.title}</option
            >{/each}</select
        ></label
      >
      {#if !sectionAvailable}<p role="status">
          The selected heading changed. Choose a section again.
        </p>{/if}
      <div class="actions">
        <button onclick={() => onopen(selected!)}>Open note</button>
        <button
          disabled={busy || !sectionAvailable}
          onclick={() => onadd(selected!, section || null)}
          >Add to selection</button
        >
        <button disabled={busy} onclick={() => onreview(selected!)}
          >Mark reviewed</button
        >
      </div>
      <small
        >Review records this saved version. A later file change will be marked
        “changed”.</small
      >
    </div>
  {/if}
</div>

<style>
  .documents {
    min-width: 0;
  }
  .list-heading,
  .title,
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
  }
  .list-heading {
    padding: 12px 0;
  }
  .list-heading span,
  small,
  .metadata,
  .inspector p {
    color: var(--fg-muted);
    font-size: 12px;
  }
  .note-list {
    height: 384px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .rows {
    position: relative;
  }
  .note {
    position: absolute;
    inset-inline: 0;
    height: 64px;
    box-sizing: border-box;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
  }
  .note[aria-selected='true'] {
    background: var(--selection);
  }
  .note:hover {
    background: var(--code-bg);
  }
  .title strong,
  .metadata {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title strong {
    font-weight: 600;
    font-size: 13px;
  }
  .title span {
    flex-shrink: 0;
    color: var(--fg-muted);
    font-size: 11px;
  }
  .title .changed {
    color: var(--accent);
  }
  .metadata {
    margin-top: 5px;
  }
  .empty {
    padding: 16px;
    color: var(--fg-muted);
  }
  .inspector {
    padding: 16px 0;
    display: grid;
    gap: 10px;
  }
  .inspector p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .actions {
    justify-content: flex-start;
    flex-wrap: wrap;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  select {
    flex: 1;
    min-width: 0;
  }
</style>
