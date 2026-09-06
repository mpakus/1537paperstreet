<script lang="ts">
  import { formatMessageTime, type AppMessage } from '../lib/messages'

  let {
    toast = '',
    history = [],
    open = false,
    ondismiss,
    ontoggle,
    onclose,
  }: {
    toast?: string
    history?: AppMessage[]
    open?: boolean
    ondismiss: () => void
    ontoggle: () => void
    onclose: () => void
  } = $props()

  const badge = $derived(
    history.length === 0
      ? ''
      : history.length > 9
        ? '9+'
        : String(history.length),
  )
  const newestFirst = $derived([...history].reverse())
  let dock = $state<HTMLDivElement | undefined>()
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key !== 'Escape') {
      return
    }
    if (open) {
      event.stopPropagation()
      onclose()
      return
    }
    if (toast) {
      ondismiss()
    }
  }}
  onpointerdown={(event) => {
    if (!open) {
      return
    }
    const target = event.target
    if (target instanceof Node && dock?.contains(target)) {
      return
    }
    onclose()
  }}
/>

<div class="dock" bind:this={dock}>
  <div class="anchor">
    {#if open}
      <div class="panel" role="dialog" aria-label="Message history list">
        {#if newestFirst.length === 0}
          <p class="empty">No messages yet.</p>
        {:else}
          <ul>
            {#each newestFirst as entry (entry.id)}
              <li>
                <time datetime={new Date(entry.at).toISOString()}
                  >{formatMessageTime(entry.at)}</time
                >
                <span>{entry.text}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <button
      type="button"
      class="history"
      title="Message history"
      aria-label="Message history"
      aria-expanded={open}
      aria-haspopup="dialog"
      onclick={ontoggle}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M3 3.5h10M3 8h10M3 12.5h7"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        />
      </svg>
      {#if badge}
        <span class="badge">{badge}</span>
      {/if}
    </button>
  </div>

  {#if toast}
    <div class="toast" role="status">
      <p>{toast}</p>
      <button type="button" aria-label="Dismiss" onclick={ondismiss}>×</button>
    </div>
  {/if}
</div>

<style>
  .dock {
    position: absolute;
    z-index: 50;
    inset-inline-start: var(--space-3);
    inset-block-end: var(--space-3);
    display: flex;
    align-items: end;
    gap: var(--space-2);
    max-width: calc(100% - var(--space-6));
    pointer-events: none;
  }

  .history,
  .panel,
  .toast {
    pointer-events: auto;
  }

  .anchor {
    position: relative;
    flex: none;
  }

  .history {
    position: relative;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev);
    color: var(--fg-muted);
    transition-property: background-color, color, transform;
    transition-duration: var(--duration);
  }

  .history:hover {
    color: var(--fg);
    background: var(--selection);
  }

  .history:active {
    transform: scale(0.96);
  }

  .history svg {
    width: 14px;
    height: 14px;
  }

  .badge {
    position: absolute;
    top: -5px;
    right: -5px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--bg);
    font-size: 0.625rem;
    font-weight: 700;
    line-height: 16px;
    text-align: center;
  }

  .panel {
    position: absolute;
    bottom: calc(100% + var(--space-2));
    left: 0;
    width: min(22rem, 70vw);
    max-height: min(16rem, 40vh);
    overflow: auto;
    padding: var(--space-2);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .empty {
    margin: 0;
    padding: var(--space-2);
    color: var(--fg-muted);
    font-size: 0.8125rem;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
  }

  li + li {
    border-block-start: 1px solid var(--border);
  }

  time {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    font-size: 0.75rem;
  }

  .toast {
    display: flex;
    align-items: start;
    gap: var(--space-2);
    min-width: 0;
    max-width: min(36rem, 100%);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    background: var(--bg-elev);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .toast p {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 0.8125rem;
  }

  .toast button {
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    font-size: 1.125rem;
    line-height: 1;
  }

  .toast button:hover {
    color: var(--fg);
    background: var(--selection);
  }

  @media (prefers-reduced-motion: reduce) {
    .history:active {
      transform: none;
    }
  }
</style>
