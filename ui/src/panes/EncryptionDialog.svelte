<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import type { DocumentSecurity } from '../lib/generated/core'
  export type EncryptionAction =
    | 'manage'
    | 'create'
    | 'encrypt'
    | 'unlock'
    | 'password'
    | 'export'
    | 'original'
  let {
    action,
    path,
    security = 'plain',
    canEncrypt = false,
    onsubmit,
    oncancel,
    onaction,
  }: {
    action: EncryptionAction
    path: string
    security?: DocumentSecurity
    canEncrypt?: boolean
    onsubmit: (path: string, password: string) => Promise<void>
    oncancel: () => void
    onaction?: (action: EncryptionAction | 'lock') => void
  } = $props()
  let destination = $state(untrack(() => path))
  let password = $state('')
  let confirmation = $state('')
  let error = $state('')
  let busy = $state(false)
  let dialog: HTMLDialogElement
  const titles = {
    manage: 'Protect notes',
    create: 'New encrypted note',
    encrypt: 'Encrypt note',
    unlock: 'Unlock note',
    password: 'Change password',
    export: 'Export decrypted note',
    original: 'Delete the original?',
  }
  const needsPassword = $derived(
    action !== 'manage' && action !== 'export' && action !== 'original',
  )
  const needsConfirmation = $derived(needsPassword && action !== 'unlock')
  const needsPath = $derived(
    action === 'create' || action === 'encrypt' || action === 'export',
  )
  onMount(() => {
    const previous = document.activeElement
    if (typeof dialog.showModal === 'function') dialog.showModal()
    else dialog.setAttribute('open', '')
    dialog.querySelector<HTMLElement>('input, [data-safe]')?.focus()
    return () => {
      password = ''
      confirmation = ''
      if (previous instanceof HTMLElement) previous.focus()
    }
  })
  async function submit(event: SubmitEvent) {
    event.preventDefault()
    if (busy) return
    if (needsPassword && !password) {
      error = 'Enter a password.'
      return
    }
    if (needsConfirmation && password !== confirmation) {
      error = 'Passwords do not match.'
      return
    }
    busy = true
    error = ''
    const secret = password
    password = ''
    confirmation = ''
    try {
      await onsubmit(destination, secret)
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause)
    } finally {
      busy = false
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="encryption-dialog"
  aria-labelledby="encryption-title"
  oncancel={(event) => {
    event.preventDefault()
    if (!busy) oncancel()
  }}
>
  <h2 id="encryption-title">{titles[action]}</h2>
  {#if action === 'manage'}
    <div class="choices">
      <button onclick={() => onaction?.('create')}>New encrypted note</button>
      {#if security === 'locked'}
        <button onclick={() => onaction?.('unlock')}>Unlock note</button>
      {:else if security === 'unlocked'}
        <p>🔒 Encrypted · unlocked in this session</p>
        <button onclick={() => onaction?.('lock')}>Lock note</button>
        <button onclick={() => onaction?.('password')}>Change password</button>
        <button onclick={() => onaction?.('export')}
          >Export decrypted .md</button
        >
      {:else if canEncrypt}
        <button onclick={() => onaction?.('encrypt')}>Encrypt this note</button>
      {/if}
    </div>
    <div class="actions"><button onclick={oncancel}>Cancel</button></div>
  {:else}
    <form onsubmit={submit} aria-busy={busy}>
      {#if needsPath}
        <label
          >File path in this project<input
            aria-label="File path"
            bind:value={destination}
            required
            disabled={busy}
            spellcheck="false"
          /></label
        >
      {:else}<p class="filename">{path}</p>{/if}
      {#if needsPassword}
        <label
          >Password<input
            aria-label="Password"
            type="password"
            bind:value={password}
            disabled={busy}
            autocomplete="off"
            maxlength="1024"
            spellcheck="false"
          /></label
        >
      {/if}
      {#if needsConfirmation}
        <label
          >Confirm password<input
            aria-label="Confirm password"
            type="password"
            bind:value={confirmation}
            disabled={busy}
            autocomplete="off"
            maxlength="1024"
            spellcheck="false"
          /></label
        >
        <p>Keep this password safe. There is no password recovery.</p>
      {/if}
      {#if action === 'encrypt'}
        <p>
          After the encrypted copy is ready, choose whether to keep or delete
          the unencrypted original.
        </p>
      {:else if action === 'original'}
        <p>
          The encrypted copy is ready. Keep the unencrypted original, or delete
          it from this folder. An encrypted recovery copy will be kept before
          deletion. Existing backups and filesystem snapshots may still contain
          unencrypted text.
        </p>
      {:else if action === 'export'}
        <p>
          This creates an unencrypted copy that anyone with access to the folder
          can read. The encrypted note is kept. Existing files will not be
          overwritten.
        </p>
      {:else if action === 'password'}
        <p>
          The previous encrypted version is kept in a hidden backup, protected
          by the old password.
        </p>
      {:else if action === 'unlock'}
        <p>
          The password stays in memory until you lock or close the note. A lost
          password cannot be recovered.
        </p>
      {/if}
      {#if error}<p role="alert">{error}</p>{/if}
      <div class="actions">
        <button data-safe type="button" disabled={busy} onclick={oncancel}>
          {action === 'original' ? 'Keep original' : 'Cancel'}
        </button>
        <button type="submit" disabled={busy}
          >{busy
            ? 'Working…'
            : action === 'original'
              ? 'Delete original'
              : titles[action]}</button
        >
      </div>
    </form>
  {/if}
</dialog>

<style>
  .encryption-dialog {
    width: min(28rem, calc(100vw - 2rem));
    max-height: calc(100vh - 2rem);
    overflow: auto;
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev);
    color: var(--fg);
  }
  .encryption-dialog::backdrop {
    background: color-mix(in srgb, var(--fg) 25%, transparent);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: 1rem;
  }
  p {
    color: var(--fg-muted);
    margin-block: var(--space-3);
  }
  .filename {
    overflow-wrap: anywhere;
  }
  label {
    display: grid;
    gap: var(--space-1);
    margin-block: var(--space-3);
  }
  input {
    width: 100%;
    box-sizing: border-box;
    padding: var(--space-2);
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  button {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
  }
  button:hover:not(:disabled) {
    background: var(--selection);
  }
  button:disabled {
    opacity: 0.55;
  }
  .actions {
    display: flex;
    justify-content: end;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
  .choices {
    display: grid;
    gap: var(--space-2);
  }
</style>
