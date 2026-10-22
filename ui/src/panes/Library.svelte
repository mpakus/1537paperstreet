<script lang="ts">
  import { untrack } from 'svelte'
  import type {
    LibraryNote,
    LibraryQuery,
    LibrarySnapshot,
    LibrarySource,
    NoteInput,
    PackPreview,
    SavedSet,
  } from '../lib/generated/core'
  import {
    errorMessage,
    libraryGet,
    libraryNoteCreate,
    libraryPreview,
    libraryUpdate,
  } from '../lib/ipc'
  import LibraryNotes from './LibraryNotes.svelte'
  import LibrarySelection from './LibrarySelection.svelte'
  import StructuredNote from './StructuredNote.svelte'
  let {
    projectId,
    projectName,
    active = true,
    watchSeq = 0,
    onopen,
    oncontext,
    onclose = () => {},
  }: {
    projectId: string | null
    projectName: string
    active?: boolean
    watchSeq?: number
    onopen: (projectId: string, path: string) => void | Promise<void>
    oncontext: (text: string) => void | Promise<void>
    onclose?: () => void
  } = $props()
  let snapshot = $state<LibrarySnapshot | null>(null)
  let search = $state(''),
    noteType = $state(''),
    status = $state(''),
    freshness = $state(''),
    tag = $state(''),
    allProjects = $state(false)
  let error = $state(''),
    message = $state(''),
    loading = $state(false),
    working = $state(false),
    refresh = $state(0)
  let createOpen = $state(false),
    edited = $state(false),
    confirmDelete = $state(false)
  let selection = $state<SavedSet>({
    id: '',
    name: '',
    kind: 'collection',
    sources: [],
  })
  let replacement = $state<SavedSet | null>(null)
  let preview = $state<PackPreview | null>(null)
  let createTarget = $state<{ id: string; name: string } | null>(null)
  let closeConfirm = $state(false)
  export function requestClose(): boolean {
    if (working) return false
    if (edited || createOpen) {
      closeConfirm = true
      return false
    }
    return true
  }
  let generation = 0
  const busy = $derived(loading || working)
  const query = $derived<LibraryQuery>({
    projectIds: allProjects || !projectId ? [] : [projectId],
    search,
    noteType,
    status,
    freshness,
    tag,
  })

  $effect(() => {
    const next = query
    const enabled = active
    void watchSeq
    void refresh
    const current = ++generation
    if (!enabled) return
    untrack(() => {
      loading = true
      preview = null
    })
    const timer = setTimeout(() => {
      void libraryGet(next)
        .then((value) => {
          if (generation === current) {
            snapshot = value
            error = ''
            loading = false
          }
        })
        .catch((cause) => {
          if (generation === current) {
            error = errorMessage(cause)
            loading = false
          }
        })
    }, 180)
    return () => {
      clearTimeout(timer)
      generation++
    }
  })

  async function run(action: () => Promise<void>) {
    working = true
    error = ''
    message = ''
    try {
      await action()
    } catch (cause) {
      error = errorMessage(cause)
    } finally {
      working = false
    }
  }
  function edit(value: SavedSet) {
    selection = value
    edited = true
    preview = null
    confirmDelete = false
  }
  function replace(value: SavedSet) {
    if (edited) {
      replacement = value
      return
    }
    select(value)
  }
  function select(value: SavedSet) {
    selection = {
      ...value,
      sources: value.sources.map((source) => ({ ...source })),
    }
    edited = false
    preview = null
    replacement = null
    confirmDelete = false
  }
  function sourceOf(
    note: LibraryNote,
    section: string | null = null,
  ): LibrarySource {
    return {
      projectId: note.projectId,
      path: note.path,
      section,
      hash: note.hash,
    }
  }
  function add(note: LibraryNote, section: string | null) {
    if (
      selection.sources.some(
        (item) =>
          item.projectId === note.projectId &&
          item.path === note.path &&
          item.section === section,
      )
    ) {
      message = 'This source is already in the selection.'
      return
    }
    if (selection.sources.length >= 100) {
      error = 'A selection supports at most 100 sources.'
      return
    }
    edit({
      ...selection,
      sources: [...selection.sources, sourceOf(note, section)],
    })
  }
  async function save() {
    if (!snapshot) return
    const name = selection.name.trim(),
      kind = selection.kind
    await libraryUpdate(snapshot.revision, { action: 'save', set: selection })
    snapshot = await libraryGet(query)
    const saved = snapshot.sets.find(
      (set) => set.name === name && set.kind === kind,
    )
    if (saved) select(saved)
    message = 'Selection saved.'
  }
  async function review(note: LibraryNote) {
    if (!snapshot) return
    await libraryUpdate(snapshot.revision, {
      action: 'review',
      source: sourceOf(note),
    })
    snapshot = await libraryGet(query)
    message = 'Saved version marked reviewed.'
  }
  async function create(input: NoteInput) {
    if (!createTarget) return
    const id = createTarget.id
    const path = await libraryNoteCreate(id, input)
    createOpen = false
    snapshot = await libraryGet(query)
    await onopen(id, path)
  }
  async function deliver(destination: 'clipboard' | 'assistant') {
    const next = await libraryPreview(selection.sources)
    if (!next.complete || next.text !== preview?.text) {
      preview = next
      message = 'Sources changed. Review the refreshed preview before using it.'
      return
    }
    if (destination === 'clipboard') {
      await navigator.clipboard.writeText(next.text)
      message = 'Context copied.'
    } else {
      await oncontext(next.text)
      message = 'Context added to the Assistant prompt.'
    }
  }
  async function remove() {
    if (!snapshot) return
    await libraryUpdate(snapshot.revision, {
      action: 'delete',
      id: selection.id,
    })
    snapshot = await libraryGet(query)
    select({ id: '', name: '', kind: 'collection', sources: [] })
    message = 'Saved selection removed. Source files were kept.'
  }
</script>

<section class="library" aria-label="Library" aria-busy={busy}>
  <header>
    <div>
      <h1>Library</h1>
      <p>Structured notes, saved collections, and context you can trace.</p>
    </div>
    <div class="actions">
      <button disabled={busy} onclick={() => refresh++}>Refresh</button><button
        disabled={!projectId || busy}
        onclick={() => {
          if (!createOpen && projectId) {
            createTarget = { id: projectId, name: projectName }
            createOpen = true
          }
        }}>New structured note</button
      >
    </div>
  </header>
  {#if closeConfirm}<div class="notice" role="alert">
      Close Library and discard the unsaved selection or note form? <button
        disabled={busy}
        onclick={onclose}>Discard and close</button
      ><button onclick={() => (closeConfirm = false)}>Keep editing</button>
    </div>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if message}<p class="notice" role="status">{message}</p>{/if}
  {#each snapshot?.notices ?? [] as notice, i (i)}<p class="notice">
      {notice}
    </p>{/each}
  {#if createOpen}<StructuredNote
      projectName={createTarget?.name ?? projectName}
      types={snapshot?.noteTypes ?? ['note']}
      {busy}
      oncreate={(input) => void run(() => create(input))}
      oncancel={() => (createOpen = false)}
    />{/if}
  <div class="filters">
    <label class="search"
      >Find notes<input
        type="search"
        bind:value={search}
        placeholder="Title, path, or project"
      /></label
    >
    <label
      >Type<select bind:value={noteType}
        ><option value="">All types</option
        >{#each snapshot?.noteTypes ?? [] as type (type)}<option value={type}
            >{type}</option
          >{/each}</select
      ></label
    >
    <label
      >Freshness<select bind:value={freshness}
        ><option value="">All states</option><option value="unreviewed"
          >Unreviewed</option
        ><option value="reviewed">Reviewed</option><option value="changed"
          >Changed</option
        ></select
      ></label
    >
    <label>Status<input bind:value={status} placeholder="Any status" /></label>
    <label>Topic<input bind:value={tag} placeholder="Any topic" /></label>
  </div>
  <div class="scope">
    <label
      ><input type="checkbox" bind:checked={allProjects} /> All registered projects</label
    ><span
      >{allProjects
        ? 'Across your projects'
        : projectName || 'Choose a project to create notes'}</span
    >
  </div>
  <div class="saved">
    <label
      >Saved selections<select
        aria-label="Saved selections"
        value={selection.id}
        disabled={busy}
        onchange={(event) => {
          const saved = snapshot?.sets.find(
            (set) => set.id === event.currentTarget.value,
          )
          if (saved) replace(saved)
        }}
        ><option value="">Choose a collection or context pack</option
        >{#each snapshot?.sets ?? [] as set (set.id)}<option value={set.id}
            >{set.kind === 'pack' ? 'Pack' : 'Collection'} · {set.name}</option
          >{/each}</select
      ></label
    >
    <button
      disabled={busy}
      onclick={() =>
        replace({ id: '', name: '', kind: 'collection', sources: [] })}
      >New selection</button
    >
    {#if selection.id}<button
        disabled={busy}
        onclick={() => (confirmDelete = true)}>Delete saved selection</button
      >{/if}
  </div>
  {#if replacement}<div class="notice" role="status">
      Replace the unsaved selection? <button
        onclick={() => select(replacement!)}>Discard selection edits</button
      ><button onclick={() => (replacement = null)}>Keep editing</button>
    </div>{/if}
  {#if confirmDelete}<div class="notice" role="status">
      Delete this saved selection? Source documents will remain. <button
        disabled={busy}
        onclick={() => void run(remove)}>Delete selection</button
      ><button onclick={() => (confirmDelete = false)}>Cancel</button>
    </div>{/if}
  {#if edited}<p class="selection-state" role="status">
      Unsaved selection · Save selection to keep it after restarting.
    </p>{/if}
  <div class="workspace">
    <LibraryNotes
      notes={snapshot?.notes ?? []}
      {busy}
      onadd={add}
      onopen={(note) =>
        void run(async () => {
          await onopen(note.projectId, note.path)
        })}
      onreview={(note) => void run(() => review(note))}
    />
    <LibrarySelection
      onopen={(source) =>
        void run(async () => {
          await onopen(source.projectId, source.path)
        })}
      {selection}
      {preview}
      {busy}
      onchange={edit}
      onsave={() => void run(save)}
      onpreview={() =>
        void run(async () => {
          preview = await libraryPreview(selection.sources)
        })}
      oncopy={() => void run(() => deliver('clipboard'))}
      oncontext={() => void run(() => deliver('assistant'))}
      onaccept={() => {
        if (preview?.complete) {
          const sources = preview.sources.map((item) => ({
            ...item.source,
            hash: item.currentHash ?? item.source.hash,
          }))
          edit({ ...selection, sources })
          message =
            'Current source versions accepted. Save the selection to keep these baselines.'
        }
      }}
    />
  </div>
</section>

<style>
  .library :global(button) {
    font: inherit;
    font-size: 12px;
    min-height: 30px;
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    color: var(--fg);
    cursor: pointer;
  }
  .library :global(button:hover:not(:disabled)) {
    background: var(--selection);
  }
  .library :global(button:disabled) {
    opacity: 0.45;
    cursor: default;
  }
  .library :global(input:not([type='checkbox'])),
  .library :global(select),
  .library :global(textarea) {
    min-height: 32px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    font-size: 12px;
  }
  .library :global(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .selection-state {
    font-size: 12px;
    color: var(--fg-muted);
    margin: 12px 0 0;
  }

  .library {
    height: 100%;
    box-sizing: border-box;
    overflow: auto;
    padding: 24px;
    background: var(--bg);
    color: var(--fg);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 20px;
  }
  h1 {
    margin: 0 0 5px;
    font-size: 24px;
    letter-spacing: -0.02em;
  }
  header p {
    margin: 0;
    color: var(--fg-muted);
    font-size: 13px;
  }
  .actions,
  .saved,
  .scope {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .filters {
    display: grid;
    grid-template-columns: minmax(160px, 2fr) repeat(4, minmax(75px, 1fr));
    gap: 12px;
  }
  .filters label,
  .saved label {
    display: grid;
    gap: 5px;
    font-size: 12px;
    min-width: 0;
  }
  input,
  select {
    min-width: 0;
    width: 100%;
    box-sizing: border-box;
  }
  .scope {
    justify-content: space-between;
    margin: 12px 0 20px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .scope label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .scope input {
    width: auto;
  }
  .saved {
    padding: 14px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    align-items: end;
  }
  .saved label {
    flex: 1;
    min-width: 180px;
  }
  .workspace {
    display: grid;
    grid-template-columns: minmax(240px, 1.2fr) minmax(280px, 1fr);
    gap: 24px;
    padding-top: 20px;
  }
  .notice {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .error {
    border-color: var(--accent);
  }
  .notice button {
    margin-left: 8px;
  }
  @media (max-width: 1050px) {
    .workspace {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 700px) {
    .library {
      padding: 16px;
    }
    .filters {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 400px) {
    .filters {
      grid-template-columns: 1fr;
    }
  }
</style>
