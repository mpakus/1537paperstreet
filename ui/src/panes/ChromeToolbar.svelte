<script lang="ts">
  import type { ViewMode } from '../lib/generated/core'
  import type { WorkspacePage } from '../lib/tabs'
  import {
    PREVIEW_ZOOM_MAX,
    PREVIEW_ZOOM_MIN,
    previewZoomPercent,
  } from '../lib/zoom'
  import EditorToolbar from './EditorToolbar.svelte'

  let {
    mode,
    page = 'document',
    canSave = false,
    canFormat = false,
    hasDocument = false,
    readingZoom = 1,
    onmode,
    oncommand,
  }: {
    mode: ViewMode
    page?: WorkspacePage
    canSave?: boolean
    canFormat?: boolean
    hasDocument?: boolean
    readingZoom?: number
    onmode: (mode: ViewMode) => void
    oncommand: (id: string) => void
  } = $props()

  const views: { id: ViewMode; label: string; title: string }[] = [
    { id: 'preview', label: 'Preview', title: 'Preview (⌘E)' },
    { id: 'editor', label: 'Edit', title: 'Edit (⌘E)' },
    { id: 'split', label: 'Split', title: 'Split (⌘⇧E)' },
  ]
</script>

<div class="chrome" role="toolbar" aria-label="Document">
  <div class="cluster" role="group" aria-label="View">
    <div class="segment">
      {#each views as view (view.id)}
        <button
          type="button"
          class="seg"
          title={view.title}
          aria-pressed={page === 'document' && mode === view.id}
          onclick={() => onmode(view.id)}>{view.label}</button
        >
      {/each}
    </div>
  </div>

  <div class="rule" aria-hidden="true"></div>

  <div class="cluster" role="group" aria-label="File">
    <button
      type="button"
      disabled={!canSave}
      title="Save (⌘S)"
      onclick={() => oncommand('file-save')}>Save</button
    >
    <button
      type="button"
      title="Export PDF (⌘⌥E)"
      disabled={!hasDocument}
      onclick={() => oncommand('file-export')}>Export</button
    >
    <button
      type="button"
      title="Board"
      aria-pressed={page === 'dashboard'}
      onclick={() => oncommand('view-dashboard')}>Board</button
    >
    <button
      type="button"
      title="Assistant (⌘⌥A)"
      aria-pressed={page === 'assistant'}
      onclick={() => oncommand('view-assistant')}>AI</button
    >
  </div>

  {#if mode !== 'preview'}
    <div class="rule" aria-hidden="true"></div>
    <div class="format">
      <EditorToolbar disabled={!canFormat} {oncommand} />
    </div>
  {:else if !hasDocument}
    <p class="hint">Open a Markdown file to preview, edit, or export.</p>
  {/if}

  {#if mode !== 'editor'}
    <div class="reading" role="group" aria-label="Reading">
      <div class="tiny" role="group" aria-label="Text size">
        <button
          type="button"
          title="Smaller text (⌘−)"
          aria-label="Smaller text"
          onclick={() => oncommand('reading-font-smaller')}>A−</button
        >
        <button
          type="button"
          title="Larger text (⌘+)"
          aria-label="Larger text"
          onclick={() => oncommand('reading-font-larger')}>A+</button
        >
      </div>
      <div class="tiny" role="group" aria-label="Zoom">
        <button
          type="button"
          title="Zoom out"
          aria-label="Zoom out"
          disabled={readingZoom <= PREVIEW_ZOOM_MIN}
          onclick={() => oncommand('view-zoom-out')}>−</button
        >
        <button
          type="button"
          class="percent"
          title="Reset zoom"
          aria-label="Reset zoom"
          onclick={() => oncommand('view-zoom-reset')}
          >{previewZoomPercent(readingZoom)}</button
        >
        <button
          type="button"
          title="Zoom in"
          aria-label="Zoom in"
          disabled={readingZoom >= PREVIEW_ZOOM_MAX}
          onclick={() => oncommand('view-zoom-in')}>+</button
        >
      </div>
    </div>
  {/if}
</div>

<style>
  .chrome {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: 44px;
    flex: none;
    min-width: 0;
    padding-inline: var(--space-3);
    border-block-end: 1px solid var(--border);
    background: color-mix(in srgb, var(--sidebar) 55%, var(--bg));
    -webkit-app-region: no-drag;
  }

  .cluster,
  .format {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
    min-width: 0;
  }

  .format {
    flex: 1;
    overflow: auto;
  }

  .reading {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: none;
    margin-inline-start: auto;
  }

  .tiny {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 1px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg) 70%, transparent);
  }

  .tiny button {
    min-width: 22px;
    height: 22px;
    padding: 0 var(--space-1);
    border-radius: 3px;
    color: var(--fg-muted);
    font-size: 0.6875rem;
    font-weight: 600;
    line-height: 1;
    transition-property: background-color, color, transform, opacity;
    transition-duration: var(--duration);
  }

  .tiny .percent {
    min-width: 2.5rem;
    font-variant-numeric: tabular-nums;
  }

  .tiny button:hover:not(:disabled) {
    color: var(--fg);
    background: var(--selection);
  }

  .tiny button:active:not(:disabled) {
    transform: scale(0.96);
  }

  .tiny button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .rule {
    width: 1px;
    height: 18px;
    flex: none;
    background: var(--border);
  }

  .hint {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-muted);
    font-size: 0.75rem;
  }

  .segment {
    display: flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--bg) 70%, transparent);
  }

  .seg {
    min-height: 28px;
    padding: 0 var(--space-3);
    border-radius: calc(var(--radius) - 2px);
    color: var(--fg-muted);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    transition-property: background-color, color, transform;
    transition-duration: var(--duration);
  }

  .seg[aria-pressed='true'] {
    color: var(--fg);
    background: var(--bg-elev);
    box-shadow: 0 1px 0 color-mix(in srgb, var(--fg) 8%, transparent);
  }

  .cluster > button {
    min-height: 28px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    color: var(--fg);
    font-size: 0.75rem;
    font-weight: 600;
    transition-property: background-color, color, transform, opacity;
    transition-duration: var(--duration);
  }

  .seg:hover:not([aria-pressed='true']),
  .cluster > button:hover:not(:disabled) {
    background: var(--selection);
  }

  .seg:active,
  .cluster > button:active:not(:disabled) {
    transform: scale(0.96);
  }

  .cluster > button[aria-pressed='true'] {
    color: var(--fg);
    background: var(--selection);
  }

  .cluster > button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    .seg:active,
    .cluster > button:active:not(:disabled),
    .tiny button:active:not(:disabled) {
      transform: none;
    }
  }
</style>
