<script lang="ts">
  import type { TagNote } from '../lib/generated/core'
  let {
    notes,
    busy,
    onopen,
    onproject,
  }: {
    notes: TagNote[]
    busy: boolean
    onopen: (note: TagNote) => void
    onproject: (note: TagNote) => void
  } = $props()
  let selectedKey = $state('')
  let scrollTop = $state(0)
  let list: HTMLDivElement
  const selected = $derived(
    notes.find((n) => `${n.projectId}/${n.path}` === selectedKey) ?? null,
  )
  const selectedIndex = $derived(selected ? notes.indexOf(selected) : -1)
  const start = $derived(
    Math.max(0, Math.min(Math.floor(scrollTop / 64) - 3, notes.length - 1)),
  )
  const visible = $derived(notes.slice(start, start + 30))
  function choose(note: TagNote) {
    selectedKey = `${note.projectId}/${note.path}`
  }
  function key(event: KeyboardEvent) {
    if (busy || !notes.length) return
    let next: number
    if (event.key === 'ArrowDown')
      next = Math.min(notes.length - 1, selectedIndex + 1)
    else if (event.key === 'ArrowUp') next = Math.max(0, selectedIndex - 1)
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

<section class="documents" aria-label="Matching documents">
  <div class="heading">
    <h2>Documents</h2>
    <span>{notes.length} matching</span>
  </div>
  <div
    class="list"
    role="listbox"
    aria-label="Tagged documents"
    aria-busy={busy}
    tabindex="0"
    bind:this={list}
    aria-activedescendant={selectedIndex >= start &&
    selectedIndex < start + visible.length
      ? `tag-note-${selectedIndex}`
      : undefined}
    onkeydown={key}
    onscroll={() => (scrollTop = list.scrollTop)}
  >
    <div class="rows" style:height={`${notes.length * 64}px`}>
      {#each visible as note, offset (`${note.projectId}/${note.path}`)}
        <div
          role="option"
          class="note"
          tabindex="-1"
          id={`tag-note-${start + offset}`}
          aria-selected={selected === note}
          style:top={`${(start + offset) * 64}px`}
          onclick={() => {
            if (!busy) {
              choose(note)
              list.focus({ preventScroll: true })
            }
          }}
          onkeydown={(event) => {
            if (event.key === 'Enter' && !busy) {
              event.stopPropagation()
              onopen(note)
            }
          }}
          ondblclick={() => {
            if (!busy) onopen(note)
          }}
        >
          <strong>{note.title}</strong><span
            >{note.projectName} / {note.path}</span
          >
        </div>
      {/each}
    </div>
    {#if !notes.length}<p class="empty">
        {busy
          ? 'Reading saved notes…'
          : 'No documents match. Clear a filter or add tags to a note and save it.'}
      </p>{/if}
  </div>
  {#if selected}<div class="details">
      <strong>{selected.title}</strong>
      <p>{selected.projectName} / {selected.path}</p>
      <div class="actions">
        <button disabled={busy} onclick={() => onopen(selected!)}
          >Open note</button
        ><button disabled={busy} onclick={() => onproject(selected!)}
          >Only this folder</button
        >
      </div>
    </div>{/if}
</section>

<style>
  .documents {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }
  .heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3);
    border-bottom: 1px solid var(--border);
    gap: var(--space-3);
  }
  h2 {
    font-size: 0.875rem;
    margin: 0;
  }
  .heading span,
  .note span,
  .details p {
    color: var(--fg-muted);
    font-size: 0.75rem;
  }
  .list {
    flex: 1;
    min-height: 240px;
    overflow: auto;
  }
  .rows {
    position: relative;
  }
  .note {
    position: absolute;
    inset-inline: 0;
    height: 64px;
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    border-bottom: 1px solid var(--border);
    cursor: pointer;
  }
  .note strong,
  .note span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note strong {
    font-size: 0.8125rem;
  }
  .note[aria-selected='true'] {
    background: var(--selection);
  }
  .note:hover {
    background: var(--bg-elev);
  }
  .details {
    border-top: 1px solid var(--border);
    padding: var(--space-3);
    overflow-wrap: anywhere;
  }
  .details strong {
    font-size: 0.875rem;
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  button {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-3);
    font-size: 0.75rem;
  }
  button:hover {
    background: var(--selection);
  }
  .empty {
    padding: var(--space-4);
    color: var(--fg-muted);
    font-size: 0.8125rem;
    line-height: 1.6;
  }
</style>
