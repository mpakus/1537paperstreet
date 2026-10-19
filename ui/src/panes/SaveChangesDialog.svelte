<script lang="ts">
  import { onMount } from 'svelte'
  let {
    path,
    onchoose,
  }: {
    path: string
    onchoose: (choice: 'save' | 'discard' | 'cancel') => void
  } = $props()
  let dialog: HTMLDialogElement
  onMount(() => {
    if (typeof dialog.showModal === 'function') dialog.showModal()
    else dialog.setAttribute('open', '')
    dialog.querySelector('button')?.focus()
  })
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="save-changes-title"
  oncancel={(event) => {
    event.preventDefault()
    onchoose('cancel')
  }}
>
  <h2 id="save-changes-title">Save changes before closing?</h2>
  <p>
    {path} has unsaved changes. Discard saves an encrypted recovery copy, then removes
    the edits from memory.
  </p>
  <div class="actions">
    <button onclick={() => onchoose('cancel')}>Cancel</button>
    <button onclick={() => onchoose('discard')}>Discard</button>
    <button onclick={() => onchoose('save')}>Save</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(28rem, calc(100vw - 2rem));
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev);
    color: var(--fg);
  }
  dialog::backdrop {
    background: color-mix(in srgb, var(--fg) 25%, transparent);
  }
  h2 {
    margin-top: 0;
    font-size: 1rem;
  }
  p {
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    justify-content: end;
  }
  button {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
  }
  button:hover {
    background: var(--selection);
  }
</style>
