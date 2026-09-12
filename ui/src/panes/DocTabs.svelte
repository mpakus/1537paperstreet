<script lang="ts">
  import {
    tabTitle,
    type DocTab,
    type WorkspacePage,
    type WorkspaceTab,
  } from '../lib/tabs'

  let {
    tabs,
    workspaceTabs = [],
    page = 'document',
    activeRelPath = null,
    onpage,
    onclosepage,
    onselect,
    onclose,
  }: {
    tabs: DocTab[]
    workspaceTabs?: WorkspaceTab[]
    page?: WorkspacePage
    activeRelPath?: string | null
    onpage: (page: WorkspaceTab) => void
    onclosepage: (page: WorkspaceTab) => void
    onselect: (relPath: string) => void
    onclose: (relPath: string) => void
  } = $props()
</script>

<div class="tabs" role="tablist" aria-label="Workspace">
  {#each workspaceTabs as workspaceTab (workspaceTab)}
    {@const title = workspaceTab === 'dashboard' ? 'Dashboard' : 'Assistant'}
    <div class="tab" class:selected={page === workspaceTab}>
      <button
        type="button"
        role="tab"
        aria-selected={page === workspaceTab}
        onclick={() => onpage(workspaceTab)}>{title}</button
      >
      <button
        type="button"
        class="close"
        title="Close"
        aria-label="Close {title}"
        onclick={() => onclosepage(workspaceTab)}>×</button
      >
    </div>
  {/each}
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
    background: var(--bg-elev);
    box-shadow: inset 0 -2px 0 var(--accent);
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
    font-weight: 500;
  }

  .tab.selected > button[role='tab'] {
    color: var(--fg);
    font-weight: 700;
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
