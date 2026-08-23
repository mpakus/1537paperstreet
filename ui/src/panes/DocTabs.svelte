<script lang="ts">
  import { tabTitle, type DocTab, type WorkspacePage } from '../lib/tabs'

  let {
    tabs,
    page = 'document',
    activeRelPath = null,
    onpage,
    onselect,
    onclose,
  }: {
    tabs: DocTab[]
    page?: WorkspacePage
    activeRelPath?: string | null
    onpage: (page: WorkspacePage) => void
    onselect: (relPath: string) => void
    onclose: (relPath: string) => void
  } = $props()
</script>

<div class="tabs" role="tablist" aria-label="Workspace">
  <div class="tab" class:selected={page === 'dashboard'}>
    <button
      type="button"
      role="tab"
      aria-selected={page === 'dashboard'}
      onclick={() => onpage('dashboard')}>Dashboard</button
    >
  </div>
  <div class="tab" class:selected={page === 'assistant'}>
    <button
      type="button"
      role="tab"
      aria-selected={page === 'assistant'}
      onclick={() => onpage('assistant')}>Assistant</button
    >
  </div>
  {#each tabs as tab (tab.relPath)}
    {@const selected = page === 'document' && tab.relPath === activeRelPath}
    <div class="tab" class:selected>
      <button
        type="button"
        role="tab"
        aria-selected={selected}
        title={tab.relPath}
        onclick={() => onselect(tab.relPath)}
        >{tab.title || tabTitle(tab.relPath)}</button
      >
      <button
        type="button"
        class="close"
        title="Close"
        aria-label="Close {tab.title}"
        onclick={() => onclose(tab.relPath)}>×</button
      >
    </div>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    flex: none;
    min-width: 0;
    overflow: auto;
    border-block-end: 1px solid var(--border);
    background: color-mix(in srgb, var(--sidebar) 40%, var(--bg));
  }

  .tab {
    display: flex;
    align-items: stretch;
    flex: none;
    min-width: 0;
    max-width: 16rem;
    border-inline-end: 1px solid var(--border);
  }

  .tab.selected {
    background: var(--bg);
  }

  .tab > button[role='tab'] {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: var(--space-2) var(--space-3);
    color: var(--fg-muted);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .tab.selected > button[role='tab'] {
    color: var(--fg);
  }

  .close {
    width: 28px;
    flex: none;
    color: var(--fg-muted);
    font-size: 1rem;
    line-height: 1;
  }

  .close:hover,
  .tab > button[role='tab']:hover {
    color: var(--fg);
    background: var(--selection);
  }
</style>
