<script lang="ts">
  let {
    name,
    dirty = false,
    missing = false,
    onreload,
    onkeep,
  }: {
    name: string
    dirty?: boolean
    missing?: boolean
    onreload: () => void
    onkeep: () => void
  } = $props()
</script>

<div class="scrim" role="presentation" onclick={onkeep}>
  <div
    class="sheet"
    role="dialog"
    tabindex="-1"
    aria-labelledby="reload-title"
    aria-describedby="reload-body"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    {#if missing}
      <h2 id="reload-title">File removed from disk</h2>
      <p id="reload-body">
        {name} is no longer in this folder.
      </p>
      <div class="actions">
        <button type="button" onclick={onkeep}>Keep open</button>
        <button type="button" class="primary" onclick={onreload}>Close</button>
      </div>
    {:else}
      <h2 id="reload-title">File changed on disk</h2>
      <p id="reload-body">
        {name} was updated in another program.
        {#if dirty}
          Reloading discards unsaved edits.
        {/if}
      </p>
      <div class="actions">
        <button type="button" onclick={onkeep}>Keep this version</button>
        <button type="button" class="primary" onclick={onreload}>Reload</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .sheet {
    width: min(28rem, 100%);
    padding: var(--space-4);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h2 {
    margin: 0 0 var(--space-3);
    font-size: 1rem;
  }

  p {
    margin: 0 0 var(--space-3);
    color: var(--fg);
  }

  .actions {
    display: flex;
    justify-content: end;
    gap: var(--space-2);
  }

  button {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--bg);
    border: 1px solid var(--border);
  }

  button:hover,
  .primary {
    background: var(--selection);
  }
</style>
