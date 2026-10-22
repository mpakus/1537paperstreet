<script lang="ts">
  import type { NoteInput } from '../lib/generated/core'
  let {
    types,
    projectName = '',
    busy,
    oncreate,
    oncancel,
  }: {
    projectName?: string
    types: string[]
    busy: boolean
    oncreate: (input: NoteInput) => void
    oncancel: () => void
  } = $props()
  let title = $state(''),
    path = $state(''),
    noteType = $state('note'),
    status = $state('draft'),
    tags = $state(''),
    source = $state('')
</script>

<form
  onsubmit={(event) => {
    event.preventDefault()
    oncreate({ title, path, noteType, status, tags, source })
  }}
>
  <h2>New structured note</h2>
  <p>
    Create a Markdown file in {projectName || 'the current project'}. Its
    properties stay editable in the source.
  </p>
  <div class="fields">
    <label
      >Title<input
        disabled={busy}
        required
        maxlength="300"
        bind:value={title}
        placeholder="What are you working on?"
      /></label
    >
    <label
      >File path<input
        disabled={busy}
        required
        bind:value={path}
        placeholder="research/question.md"
      /></label
    >
    <label
      >Type<select disabled={busy} bind:value={noteType}
        >{#each types as type (type)}<option value={type}>{type}</option
          >{/each}</select
      ></label
    >
    <label
      >Status<input
        disabled={busy}
        maxlength="100"
        bind:value={status}
        placeholder="draft, unread, reviewed…"
      /></label
    >
    <label
      >Topics<input
        disabled={busy}
        maxlength="1000"
        bind:value={tags}
        placeholder="Comma-separated topics"
      /></label
    >
    <label
      >Source reference<input
        disabled={busy}
        maxlength="2000"
        bind:value={source}
        placeholder="Optional URL, DOI, or reference"
      /></label
    >
  </div>
  <p class="hint">
    The destination folder must exist. Existing files are never replaced.
  </p>
  <div class="actions">
    <button type="submit" disabled={busy}>Create note</button><button
      type="button"
      disabled={busy}
      onclick={oncancel}>Cancel</button
    >
  </div>
</form>

<style>
  form {
    border: 1px solid var(--border);
    padding: 16px;
    border-radius: 6px;
    margin: 16px 0;
    background: var(--bg);
  }
  h2 {
    margin: 0;
    font-size: 16px;
  }
  p {
    font-size: 12px;
    color: var(--fg-muted);
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  label {
    display: grid;
    gap: 5px;
    font-size: 12px;
  }
  input,
  select {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  @media (max-width: 700px) {
    .fields {
      grid-template-columns: 1fr;
    }
  }
</style>
