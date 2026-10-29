<script lang="ts">
  import { untrack } from 'svelte'
  import type { TagNote, TagQuery, TagSnapshot } from '../lib/generated/core'
  import { errorMessage, tagsGet } from '../lib/ipc'
  import TagNotes from './TagNotes.svelte'
  import QuickSwitch from './QuickSwitch.svelte'
  let {
    active = true,
    watchSeq = 0,
    projectsRevision = '',
    onopen,
  }: {
    active?: boolean
    watchSeq?: number
    projectsRevision?: string
    onopen: (projectId: string, path: string) => void | Promise<void>
  } = $props()
  let chooseFolder = $state(false)
  let snapshot = $state<TagSnapshot | null>(null)
  let selected = $state<{ key: string; label: string }[]>([])
  let folder = $state<{ id: string; name: string } | null>(null)
  let search = $state(''),
    error = $state(''),
    loading = $state(false)
  let tagOffset = $state(0),
    refresh = $state(0),
    sourceRevision = $state(0)
  let generation = 0,
    loadedRevision = -1
  const query = $derived<TagQuery>({
    tags: selected.map((t) => t.key),
    projectId: folder?.id ?? '',
    search,
    tagOffset,
  })
  $effect(() => {
    void active
    void watchSeq
    void projectsRevision
    void refresh
    untrack(() => sourceRevision++)
  })
  $effect(() => {
    const next = query,
      revision = sourceRevision,
      enabled = active
    const current = ++generation
    if (!enabled) return
    untrack(() => {
      loading = true
      error = ''
    })
    const timer = setTimeout(() => {
      void tagsGet(next, loadedRevision !== revision)
        .then((value) => {
          if (current !== generation) return
          snapshot = value
          loadedRevision = revision
          loading = false
        })
        .catch((cause) => {
          if (current !== generation) return
          error = errorMessage(cause)
          loading = false
          snapshot = null
        })
    }, 180)
    return () => {
      clearTimeout(timer)
      generation++
    }
  })
  function toggle(key: string, label: string) {
    selected = selected.some((t) => t.key === key)
      ? selected.filter((t) => t.key !== key)
      : [...selected, { key, label }]
    tagOffset = 0
  }
  async function open(note: TagNote) {
    try {
      await onopen(note.projectId, note.path)
    } catch (cause) {
      error = errorMessage(cause)
    }
  }
</script>

<section class="tags-page" aria-label="Tags Cloud" aria-busy={loading}>
  <header>
    <div>
      <p class="eyebrow">CONNECTED FOLDERS</p>
      <h1>Tags Cloud</h1>
      <p class="intro">Follow a topic across your projects.</p>
    </div>
    <button class="refresh" onclick={() => refresh++}>Refresh</button>
  </header>
  <div class="filters">
    <input
      aria-label="Find tags or notes"
      placeholder="Find tags or notes…"
      bind:value={search}
      oninput={() => (tagOffset = 0)}
    />
    <button onclick={() => (chooseFolder = true)}>Choose folder…</button>
    {#if folder}<button
        onclick={() => {
          folder = null
          tagOffset = 0
        }}>All projects</button
      ><span class="scope">Folder: {folder.name}</span>{:else}<span
        class="scope">All registered project folders</span
      >{/if}
  </div>
  {#if selected.length}<div class="selection" aria-label="Selected tags">
      <span>Workspace</span>{#each selected as tag (tag.key)}<button
          class="selected"
          onclick={() => toggle(tag.key, tag.label)}
          aria-label={`Remove ${tag.label} filter`}
          >{tag.label} <span aria-hidden="true">×</span></button
        >{/each}
      <button
        onclick={() => {
          selected = []
          tagOffset = 0
        }}>Clear tags</button
      ><small>Notes must match every selected tag.</small>
    </div>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#each snapshot?.notices ?? [] as notice, index (index)}<p
      class="notice"
      role="status"
    >
      {notice}
    </p>{/each}
  <div class="network">
    <section class="topics" aria-label="Related tags">
      <div class="heading">
        <h2>{selected.length ? 'Related tags' : 'Explore tags'}</h2>
        <span aria-live="polite"
          >{loading
            ? 'Updating…'
            : `Tags: ${snapshot?.totalTags ?? 0} · Folders: ${snapshot?.projects ?? 0}`}</span
        >
      </div>
      <div class="cloud">
        {#each snapshot?.tags ?? [] as tag (tag.key)}
          <button
            class="tag"
            class:chosen={selected.some((t) => t.key === tag.key)}
            data-tag={tag.key}
            data-weight={tag.weight}
            disabled={loading}
            aria-pressed={selected.some((t) => t.key === tag.key)}
            title={`Documents: ${tag.documents} · Folders: ${tag.projects}. Includes nested tags.`}
            onclick={() => toggle(tag.key, tag.label)}
            ><span>{tag.label}</span><small
              >{tag.documents}<span class="folders">
                / {tag.projects}</span
              ></small
            ></button
          >
        {/each}
        {#if !loading && !snapshot?.tags.length}<p class="empty">
            Add <code>#topic</code>, <code>#project/design</code> or
            <code>#project alpha#</code> to a Markdown note, then save it. Shared
            tags connect your folders here.
          </p>{/if}
      </div>
      {#if snapshot && snapshot.totalTags > 80}<nav
          class="pages"
          aria-label="Tag pages"
        >
          <button
            disabled={loading || snapshot.tagOffset === 0}
            onclick={() => (tagOffset = Math.max(0, snapshot!.tagOffset - 80))}
            >Previous tags</button
          >
          <span
            >{snapshot.tagOffset + 1}–{snapshot.tagOffset +
              snapshot.tags.length} of {snapshot.totalTags}</span
          >
          <button
            disabled={loading || snapshot.tagOffset + 80 >= snapshot.totalTags}
            onclick={() => (tagOffset = snapshot!.tagOffset + 80)}
            >Next tags</button
          >
        </nav>{/if}
      <footer>
        <span>Count = documents / folders</span>
        <p>
          Choose a tag to focus. Choose another to find their shared notes.
          Parent tags include their nested topics.
        </p>
        <p>
          Edit tags in a note and Save. Refresh picks up changes in other
          folders.
        </p>
      </footer>
    </section>
    <TagNotes
      notes={snapshot?.notes ?? []}
      busy={loading}
      onopen={open}
      onproject={(note) => {
        folder = { id: note.projectId, name: note.projectName }
        tagOffset = 0
      }}
    />
  </div>
</section>

{#if chooseFolder}
  <QuickSwitch
    title="Choose tag folder"
    onopen={(project) => {
      folder = { id: project.id, name: project.name }
      tagOffset = 0
    }}
    onclose={() => (chooseFolder = false)}
    onerror={(message) => (error = message)}
  />
{/if}

<style>
  .tags-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
    min-width: 0;
    padding: var(--space-5);
    overflow: auto;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .eyebrow {
    font-size: 0.625rem;
    letter-spacing: 0.14em;
    color: var(--fg-muted);
    margin: 0 0 var(--space-2);
    font-weight: 600;
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 650;
    letter-spacing: -0.025em;
  }
  .intro {
    color: var(--fg-muted);
    font-size: 0.8125rem;
    margin: var(--space-2) 0 0;
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
  .filters,
  .selection {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  input {
    min-width: 180px;
    max-width: 360px;
    flex: 1;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font-size: 0.8125rem;
  }
  .scope,
  .selection > span,
  .selection small {
    font-size: 0.75rem;
    color: var(--fg-muted);
  }
  .selected {
    color: var(--accent);
    border-color: var(--accent);
  }
  .network {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(280px, 1fr);
    gap: var(--space-4);
    flex: 1;
    min-height: 360px;
  }
  .topics {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3);
    gap: var(--space-2);
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  h2 {
    font-size: 0.875rem;
    margin: 0;
  }
  .heading > span {
    font-size: 0.6875rem;
    color: var(--fg-muted);
  }
  .cloud {
    padding: var(--space-4);
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    align-items: baseline;
    gap: var(--space-3);
    flex: 1;
    overflow: auto;
    min-height: 180px;
  }
  .tag {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    max-width: 100%;
    border-color: transparent;
    background: var(--bg-elev);
    font-size: 0.875rem;
    text-align: left;
  }
  .tag > span {
    overflow-wrap: anywhere;
  }
  .tag[data-weight='1'] {
    font-size: 1rem;
  }
  .tag[data-weight='2'] {
    font-size: 1.125rem;
  }
  .tag[data-weight='3'] {
    font-size: 1.25rem;
  }
  .tag[data-weight='4'] {
    font-size: 1.5rem;
  }
  .tag small {
    font-size: 0.6875rem;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  .tag.chosen {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--selection);
  }
  .folders {
    opacity: 0.8;
  }
  footer {
    border-top: 1px solid var(--border);
    padding: var(--space-3);
    color: var(--fg-muted);
    font-size: 0.6875rem;
    line-height: 1.6;
  }
  footer p {
    margin: var(--space-1) 0 0;
  }
  footer > span {
    font-weight: 600;
  }
  .pages {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-3);
    font-size: 0.6875rem;
    flex-wrap: wrap;
  }
  .empty {
    color: var(--fg-muted);
    font-size: 0.8125rem;
    line-height: 1.8;
  }
  .error {
    color: var(--accent);
  }
  .notice,
  .error {
    font-size: 0.75rem;
    margin: 0;
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  @media (max-width: 1000px) {
    .network {
      grid-template-columns: minmax(0, 1fr);
    }
    .tags-page {
      padding: var(--space-3);
    }
    .cloud {
      max-height: 320px;
    }
  }
</style>
