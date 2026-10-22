<script lang="ts">
  import type {
    LibrarySource,
    PackPreview,
    SavedSet,
  } from '../lib/generated/core'
  let {
    selection,
    preview,
    busy,
    onchange,
    onsave,
    onpreview,
    oncopy,
    oncontext,
    onaccept,
    onopen,
  }: {
    selection: SavedSet
    preview: PackPreview | null
    busy: boolean
    onchange: (set: SavedSet) => void
    onsave: () => void
    onpreview: () => void
    onopen: (source: LibrarySource) => void
    oncopy: () => void
    oncontext: () => void
    onaccept: () => void
  } = $props()
  function move(index: number, direction: number) {
    const sources = [...selection.sources]
    const [source] = sources.splice(index, 1)
    sources.splice(index + direction, 0, source)
    onchange({ ...selection, sources })
  }
</script>

<aside aria-label="Saved selection editor">
  <div class="heading">
    <h2>{selection.id ? 'Edit selection' : 'Build a selection'}</h2>
    <span>{selection.sources.length} / 100</span>
  </div>
  <label
    >Name<input
      disabled={busy}
      aria-label="Selection name"
      value={selection.name}
      maxlength="200"
      placeholder="e.g. Authentication redesign"
      oninput={(event) =>
        onchange({ ...selection, name: event.currentTarget.value })}
    /></label
  >
  <label
    >Save as<select
      disabled={busy}
      value={selection.kind}
      onchange={(event) =>
        onchange({
          ...selection,
          kind: event.currentTarget.value as SavedSet['kind'],
        })}
    >
      <option value="collection">Collection</option><option value="pack"
        >Context pack</option
      >
    </select></label
  >
  <ol>
    {#each selection.sources as source, i (`${source.projectId}/${source.path}#${source.section}`)}
      <li>
        <div class="source">
          <button
            class="source-link"
            disabled={busy}
            title={`Open ${source.path} · project ${source.projectId}`}
            onclick={() => onopen(source)}>{source.path}</button
          ><small
            >{source.section
              ? `Section: ${source.section}`
              : 'Whole document'}</small
          >
        </div>
        <div class="order">
          <button
            aria-label={`Move source ${i + 1} up`}
            disabled={busy || i === 0}
            onclick={() => move(i, -1)}>↑</button
          >
          <button
            aria-label={`Move source ${i + 1} down`}
            disabled={busy || i === selection.sources.length - 1}
            onclick={() => move(i, 1)}>↓</button
          >
          <button
            aria-label={`Remove source ${i + 1}`}
            disabled={busy}
            onclick={() =>
              onchange({
                ...selection,
                sources: selection.sources.filter((_, index) => index !== i),
              })}>×</button
          >
        </div>
      </li>
    {/each}
  </ol>
  {#if !selection.sources.length}<p class="hint">
      Select a document, choose a section if needed, then add it here. Sources
      stay in their original folders.
    </p>{/if}
  <div class="actions">
    <button disabled={busy || !selection.name.trim()} onclick={onsave}
      >Save selection</button
    >
    <button disabled={busy || !selection.sources.length} onclick={onpreview}
      >Preview context</button
    >
  </div>
  <p class="hint">
    Context uses saved files. Save editor changes before refreshing. Estimated
    tokens vary by model.
  </p>
  {#if preview}
    <div class="preview">
      <h3>Context preview</h3>
      {#each preview.sources as item, i (i)}
        <div class="source-status">
          <strong>{item.source.path}</strong><span
            class:changed={item.status !== 'current'}>{item.status}</span
          >
          {#if item.message}<small>{item.message}</small>{/if}
        </div>
      {/each}
      {#if preview.complete}
        <p class="hint">
          Approximately {preview.estimatedTokens} tokens · saved source text
        </p>
        <textarea
          readonly
          aria-label="Context preview"
          value={preview.text}
          rows="12"></textarea>
        {#if preview.sources.some((item) => item.status === 'changed')}<button
            disabled={busy}
            onclick={onaccept}>Accept current source versions</button
          >{/if}
      {:else}<p role="status">
          Some sources are unavailable. Remove or repair them to create a
          complete pack.
        </p>{/if}
      <div class="actions">
        <button disabled={busy || !preview.complete} onclick={oncopy}
          >Copy context</button
        ><button disabled={busy || !preview.complete} onclick={oncontext}
          >Use in Assistant</button
        >
      </div>
      <small
        >Use in Assistant fills the prompt for your review. You choose when to
        send it.</small
      >
    </div>
  {/if}
</aside>

<style>
  aside {
    min-width: 0;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    border-radius: 6px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    font-size: 15px;
    margin: 0;
  }
  h3 {
    font-size: 14px;
    margin: 0;
  }
  .heading,
  .actions,
  .order {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .heading {
    justify-content: space-between;
  }
  .heading span,
  small,
  .hint {
    color: var(--fg-muted);
    font-size: 12px;
  }
  .actions {
    flex-wrap: wrap;
  }
  label {
    display: grid;
    gap: 5px;
    font-size: 12px;
  }
  input,
  select,
  textarea {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }
  ol {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 320px;
    overflow: auto;
  }
  li {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }
  .source {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 4px;
  }
  .source-link {
    overflow-wrap: anywhere;
    font-size: 12px;
  }
  .order {
    gap: 4px;
  }
  .order button {
    padding: 4px 8px;
  }
  .hint {
    margin: 0;
    line-height: 1.5;
  }
  .preview {
    display: grid;
    gap: 12px;
    border-top: 1px solid var(--border);
    padding-top: 16px;
  }
  textarea {
    font: 12px/1.5 var(--font-mono);
    resize: vertical;
  }
  .source-status {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12px;
  }
  .source-status strong {
    flex: 1;
    overflow-wrap: anywhere;
  }
  .source-status small {
    flex-basis: 100%;
  }
  .changed {
    color: var(--accent);
  }
</style>
