<script lang="ts">
  import type { DashboardSnapshot } from '../lib/ipc'

  let {
    snapshot = null,
    error = '',
    onrefresh,
    onassistant,
    onconfigure,
  }: {
    snapshot?: DashboardSnapshot | null
    error?: string
    onrefresh: () => void
    onassistant: () => void
    onconfigure: () => void
  } = $props()
</script>

<div class="dash" role="tabpanel" aria-label="Dashboard">
  <header>
    <div>
      <h2>{snapshot?.title ?? 'Dashboard'}</h2>
      <p class="lede">{snapshot?.lede ?? 'Loading local counts…'}</p>
    </div>
    <div class="actions">
      <button type="button" onclick={onassistant}>Assistant</button>
      <button type="button" onclick={onconfigure}>Configure agents</button>
      <button type="button" onclick={onrefresh}>Refresh</button>
    </div>
  </header>
  {#if error}
    <p class="status" role="status">{error}</p>
  {/if}
  {#if snapshot}
    {#each snapshot.sections as section (section.title)}
      <section>
        <h3>{section.title}</h3>
        {#if section.metrics.length > 0}
          <div class="metrics">
            {#each section.metrics as metric, metricIndex (`${section.title}-m-${metricIndex}`)}
              <article>
                <p class="label">{metric.label}</p>
                <p class="value">{metric.value}</p>
              </article>
            {/each}
          </div>
        {/if}
        {#if section.rows.length > 0}
          <ul>
            {#each section.rows as row, rowIndex (`${section.title}-r-${rowIndex}`)}
              <li>
                <span>{row.title}</span>
                <span class="detail">{row.detail}</span>
              </li>
            {/each}
          </ul>
        {:else if section.metrics.length === 0}
          <p class="empty">Nothing here yet.</p>
        {/if}
      </section>
    {/each}
  {/if}
</div>

<style>
  .dash {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: var(--space-5) var(--space-6);
    color: var(--fg);
    background: var(--bg);
  }

  header {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
  }

  h2,
  h3 {
    margin: 0;
  }

  h3 {
    margin-bottom: var(--space-3);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }

  .lede,
  .status,
  .empty,
  .detail {
    color: var(--fg-muted);
  }

  .lede {
    max-width: 40rem;
    margin: var(--space-2) 0 0;
    font-size: 0.8125rem;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  button {
    min-height: 28px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    color: var(--fg);
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
  }

  section + section {
    margin-top: var(--space-6);
  }

  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  article {
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
  }

  .label {
    margin: 0 0 var(--space-1);
    font-size: 0.6875rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }

  .value {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-1);
  }

  li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: 1px solid var(--border);
    font-size: 0.8125rem;
  }

  .detail {
    text-align: end;
  }

  .status,
  .empty {
    margin: 0;
    font-size: 0.8125rem;
  }
</style>
