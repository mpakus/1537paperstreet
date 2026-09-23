<script lang="ts">
  import { contentSearch, errorMessage, type TextSearchHit } from '../lib/ipc'

  let {
    projectId,
    scope = '',
    onopen,
    onclose,
    onerror,
  }: {
    projectId: string
    scope?: string
    onopen: (relPath: string) => void
    onclose: () => void
    onerror: (message: string) => void
  } = $props()

  let query = $state('')
  let items = $state<TextSearchHit[]>([])
  let literal = $state(false)
  let capped = $state(false)
  let index = $state(0)
  let inputEl = $state<HTMLInputElement | undefined>()

  const title = $derived(scope ? `Search in ${scope}` : 'Search in files')

  $effect(() => {
    inputEl?.focus()
  })

  $effect(() => {
    const needle = query.trim()
    const id = projectId
    const folder = scope
    if (!needle) {
      items = []
      literal = false
      capped = false
      index = 0
      return
    }
    let cancelled = false
    const timer = setTimeout(() => {
      void contentSearch(id, folder, needle, 40)
        .then((result) => {
          if (cancelled) {
            return
          }
          items = result.hits
          literal = result.literal
          capped = result.capped
          index = 0
        })
        .catch((cause: unknown) => {
          if (!cancelled) {
            onerror(errorMessage(cause))
          }
        })
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  })

  function confirm() {
    const item = items[index]
    if (item) {
      onopen(item.relPath)
    }
    onclose()
  }
</script>

<div
  class="scrim"
  role="presentation"
  onclick={onclose}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      onclose()
    }
  }}
>
  <div
    class="palette"
    role="dialog"
    tabindex="-1"
    aria-label={title}
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    <input
      bind:this={inputEl}
      bind:value={query}
      placeholder="Text or regular expression"
      aria-label={title}
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          onclose()
        }
        if (event.key === 'Enter') {
          event.preventDefault()
          confirm()
        }
        if (event.key === 'ArrowDown') {
          event.preventDefault()
          index = Math.min(items.length - 1, index + 1)
        }
        if (event.key === 'ArrowUp') {
          event.preventDefault()
          index = Math.max(0, index - 1)
        }
      }}
    />
    <ul>
      {#each items as item, itemIndex (item.relPath)}
        <li>
          <button
            type="button"
            class:active={itemIndex === index}
            onclick={() => {
              onopen(item.relPath)
              onclose()
            }}
          >
            <span class="path">{item.relPath}</span>
            {#if item.line > 0}
              <span class="snippet">{item.line}: {item.snippet}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if query.trim() && items.length === 0}
      <p>No files match.</p>
    {:else if !query.trim()}
      <p>Type to search file contents and paths.</p>
    {/if}
    {#if literal}
      <p>Not a regular expression, so this searches for the exact text.</p>
    {/if}
    {#if capped}
      <p>Search stopped after 10,000 files.</p>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: start center;
    padding: var(--space-6);
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .palette {
    width: min(40rem, 100%);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  input {
    width: 100%;
    padding: var(--space-3);
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--fg);
    font: inherit;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    max-height: 22rem;
    overflow: auto;
  }

  button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: var(--space-2);
    text-align: start;
    border-radius: var(--radius-sm);
    color: var(--fg);
  }

  button.active,
  button:hover {
    background: var(--selection);
  }

  .path {
    font-weight: 600;
  }

  .snippet {
    color: var(--fg-muted);
    font-size: 0.8125rem;
  }

  p {
    margin: 0;
    padding: var(--space-3);
    color: var(--fg-muted);
  }
</style>
