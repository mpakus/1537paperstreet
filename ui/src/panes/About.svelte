<script lang="ts">
  import { onMount } from 'svelte'

  import { errorMessage } from '../lib/ipc'
  import { isBackdropEvent } from '../lib/overlay'
  import type { UpdateCheck, UpdateInstall } from '../lib/ipc'

  let {
    version,
    autocheck = false,
    onclose,
    onopen,
    oncheck,
    oninstall,
    onrelaunch,
  }: {
    version: string
    autocheck?: boolean
    onclose: () => void
    onopen: (url: string) => void
    oncheck: () => Promise<UpdateCheck>
    oninstall?: () => Promise<UpdateInstall>
    onrelaunch?: () => Promise<void>
  } = $props()

  const site = 'https://aomega.co'

  let checking = $state(false)
  let installing = $state(false)
  let installed = $state(false)
  let installFailed = $state(false)
  let result = $state<UpdateCheck | null>(null)
  let checkError = $state('')

  onMount(() => {
    if (autocheck) {
      void runCheck()
    }
  })

  async function runCheck() {
    checking = true
    checkError = ''
    result = null
    installFailed = false
    try {
      result = await oncheck()
      if (result.available && result.can_install && !installed) {
        await runInstall()
        if (installed) {
          await runRelaunch()
        }
      }
    } catch (cause) {
      checkError = errorMessage(cause)
    } finally {
      checking = false
    }
  }

  async function runInstall() {
    if (!oninstall) {
      return
    }
    installing = true
    checkError = ''
    installFailed = false
    try {
      const done = await oninstall()
      installed = true
      if (result) {
        result = { ...result, message: done.message, can_install: false }
      }
    } catch (cause) {
      installFailed = true
      checkError = errorMessage(cause)
    } finally {
      installing = false
    }
  }

  async function runRelaunch() {
    if (!onrelaunch) {
      return
    }
    try {
      await onrelaunch()
    } catch (cause) {
      checkError = errorMessage(cause)
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      onclose()
    }
  }}
/>

<div
  class="scrim"
  role="presentation"
  onclick={(event) => {
    if (isBackdropEvent(event)) {
      onclose()
    }
  }}
>
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    aria-label="About 1537paperstreet"
    aria-busy={checking || installing}
    onpointerdown={(event) => event.stopPropagation()}
  >
    <div class="logo-stripe">
      <img class="logo" src="/logo.png" alt="1537paperstreet" />
    </div>
    <p class="version">Version {version}</p>
    <p class="blurb">
      A local Markdown reader for macOS. It works with folders on disk, does not
      require an account, and does not send your documents over the network.
    </p>
    <p class="credit">Made in Austin ✩ Texas</p>
    <a
      href={site}
      onclick={(event) => {
        event.preventDefault()
        onopen(site)
      }}>{site.replace('https://', '')}</a
    >
    {#if installing}
      <p class="status" role="status" aria-live="polite">
        Downloading and installing the update…
      </p>
    {:else if checking}
      <p class="status" role="status" aria-live="polite">Checking for updates…</p>
    {:else if checkError}
      <p class="status is-error" role="status" aria-live="polite">{checkError}</p>
    {:else if result}
      <p class="status" role="status" aria-live="polite">{result.message}</p>
    {/if}
    <div class="actions">
      <button
        type="button"
        disabled={checking || installing}
        onclick={() => void runCheck()}
      >
        Check for Updates
      </button>
      {#if installed}
        <button
          type="button"
          disabled={installing}
          onclick={() => void runRelaunch()}
        >
          Restart to Update
        </button>
      {:else if result?.available && result.release_url && (!result.can_install || installFailed)}
        <button
          type="button"
          onclick={() => {
            const url = result?.release_url
            if (url) {
              onopen(url)
            }
          }}
        >
          Open Download
        </button>
      {/if}
      <button type="button" onclick={onclose}>Close</button>
    </div>
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
    display: grid;
    justify-items: stretch;
    gap: var(--space-3);
    width: min(26rem, 100%);
    padding: 0 0 var(--space-5);
    overflow: hidden;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-align: center;
  }

  .logo-stripe {
    display: grid;
    place-items: center;
    padding: var(--space-4) var(--space-5);
    background: white;
    border-block-end: 1px solid var(--border);
  }

  .logo {
    width: min(18rem, 100%);
    height: auto;
  }

  .version,
  .blurb,
  .credit,
  a,
  .status,
  .actions {
    justify-self: center;
    padding-inline: var(--space-5);
  }

  .version,
  .credit {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .blurb {
    margin: 0;
    color: var(--fg);
    font-size: 0.875rem;
  }

  a {
    color: var(--accent);
    font-size: 0.875rem;
    font-weight: 600;
  }

  .status {
    margin: 0;
    color: var(--fg);
    font-size: 0.8125rem;
  }

  .status.is-error {
    color: var(--accent);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
  }

  button {
    min-height: 28px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    font-size: 0.75rem;
    font-weight: 600;
  }

  button:hover:not(:disabled) {
    background: var(--selection);
  }

  button:disabled {
    opacity: 0.6;
  }
</style>
