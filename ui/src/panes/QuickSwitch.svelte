<script lang="ts">
  import { errorMessage, projectsList, type Project } from '../lib/ipc'
  import { highlightQuery } from '../lib/text'

  let {
    title = 'Switch project',
    onopen,
    onclose,
    onerror,
  }: {
    title?: string
    onopen: (project: Project) => void
    onclose: () => void
    onerror: (message: string) => void
  } = $props()

  let query = $state('')
  let items = $state<Project[]>([])
  let index = $state(0)
  let inputEl = $state<HTMLInputElement | undefined>()

  $effect(() => {
    inputEl?.focus()
  })

  $effect(() => {
    const needle = query
    const handle = setTimeout(() => {
      void (async () => {
        try {
          const page = await projectsList({
            query: needle.trim() ? needle : null,
            limit: 40,
            offset: 0,
          })
          items = page.items
          index = 0
        } catch (cause) {
          onerror(errorMessage(cause))
        }
      })()
    }, 30)
    return () => clearTimeout(handle)
  })

  function confirm() {
    const item = items[index]
    if (item) {
      onopen(item)
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
      placeholder={title}
      aria-label="Search projects"
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
      {#each items as item, itemIndex (item.id)}
        <li>
          <button
            type="button"
            class:active={itemIndex === index}
            onclick={() => {
              onopen(item)
              onclose()
            }}
          >
            <span>
              {#each highlightQuery(item.name, query) as part, partIndex (`${item.id}-${partIndex}`)}
                {#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}
              {/each}
            </span>
            <small>{item.path}</small>
          </button>
        </li>
      {/each}
    </ul>
    {#if items.length === 0}
      <p>No projects match.</p>
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
    width: min(36rem, 100%);
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
    max-height: 16rem;
    overflow: auto;
  }

  button {
    display: grid;
    width: 100%;
    padding: var(--space-2);
    text-align: start;
    border-radius: var(--radius-sm);
    color: var(--fg);
  }

  small {
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  mark {
    background: var(--selection);
    color: var(--accent);
    padding: 0;
  }

  button.active,
  button:hover {
    background: var(--selection);
  }

  p {
    margin: 0;
    padding: var(--space-3);
    color: var(--fg-muted);
  }
</style>
