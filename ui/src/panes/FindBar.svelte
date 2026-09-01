<script lang="ts">
  import { onMount } from 'svelte'

  import { applyFindHits, clearFindHits } from '../lib/find'

  let {
    root = null,
    revision = '',
    onclose,
  }: {
    root?: HTMLElement | null
    revision?: string
    onclose: () => void
  } = $props()

  let query = $state('')
  let index = $state(0)
  let hits = $state<HTMLElement[]>([])
  let capped = $state(false)
  let inputEl = $state<HTMLInputElement | undefined>()

  onMount(() => {
    inputEl?.focus()
    const onKey = (event: KeyboardEvent) => {
      if (
        !(event.metaKey || event.ctrlKey) ||
        event.key.toLowerCase() !== 'g'
      ) {
        return
      }
      event.preventDefault()
      step(event.shiftKey ? -1 : 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  $effect(() => {
    const host = root
    return () => {
      if (host) {
        clearFindHits(host)
      }
    }
  })

  $effect(() => {
    const needle = query
    const host = root
    void revision
    const frame = requestAnimationFrame(() => {
      paint(host, needle)
    })
    return () => cancelAnimationFrame(frame)
  })

  function paint(host: HTMLElement | null, needle: string) {
    if (!host || !needle) {
      if (host) {
        clearFindHits(host)
      }
      hits = []
      capped = false
      index = 0
      return
    }
    const next = applyFindHits(host, needle)
    hits = next.marks
    capped = next.capped
    index = 0
    reveal(next.marks, 0)
  }

  function reveal(marks: HTMLElement[], next: number) {
    for (const mark of marks) {
      mark.classList.toggle('find-hit-current', false)
    }
    const current = marks[next]
    if (!current) {
      return
    }
    current.classList.add('find-hit-current')
    current.scrollIntoView({ block: 'center', inline: 'nearest' })
  }

  function step(delta: number) {
    if (hits.length === 0) {
      return
    }
    const next = (index + delta + hits.length) % hits.length
    index = next
    reveal(hits, next)
  }
</script>

<div class="find" role="search">
  <input
    bind:this={inputEl}
    bind:value={query}
    type="search"
    placeholder="Find in document"
    aria-label="Find in document"
    autocomplete="off"
    autocorrect="off"
    spellcheck="false"
    onkeydown={(event) => {
      if (event.key === 'Escape') {
        onclose()
      }
      if (event.key === 'Enter' && event.shiftKey) {
        event.preventDefault()
        step(-1)
      } else if (event.key === 'Enter') {
        event.preventDefault()
        step(1)
      }
    }}
  />
  <span
    >{hits.length === 0
      ? '0'
      : `${index + 1} of ${hits.length}${capped ? '+' : ''}`}</span
  >
  <button type="button" onclick={() => step(-1)}>Previous</button>
  <button type="button" onclick={() => step(1)}>Next</button>
  <button type="button" onclick={onclose}>Close</button>
</div>

<style>
  .find {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
  }

  input {
    flex: 1;
    min-width: 0;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
  }

  span {
    color: var(--fg-muted);
    font-size: 0.8125rem;
    white-space: nowrap;
  }

  button {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
  }

  :global(article mark.find-hit) {
    color: inherit;
    background: var(--selection);
    border-radius: 2px;
  }

  :global(article mark.find-hit-current) {
    background: color-mix(in srgb, var(--accent) 40%, var(--selection));
  }
</style>
