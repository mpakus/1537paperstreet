<script lang="ts">
  import { onMount } from 'svelte'

  let {
    left,
    top,
    current = '',
    busy = false,
    onapply,
    onclose,
  }: {
    left: number
    top: number
    current?: string
    busy?: boolean
    onapply: (color: string | null) => void
    onclose: () => void
  } = $props()
  let palette: HTMLDivElement
  const presets = [
    'default',
    'red',
    'orange',
    'yellow',
    'green',
    'blue',
    'purple',
  ]
  let hex = $derived(
    current.startsWith('#')
      ? current
      : getComputedStyle(document.documentElement)
          .getPropertyValue(
            `--marker-${presets.includes(current) ? current : 'yellow'}`,
          )
          .trim(),
  )
  onMount(() => {
    palette.showPopover?.()
  })
</script>

<div
  bind:this={palette}
  popover="manual"
  class="highlight-palette"
  role="dialog"
  aria-label="Highlight text"
  aria-busy={busy}
  style:left="{left}px"
  style:top="{top}px"
>
  <div class="heading">
    <strong>Highlight</strong><button
      type="button"
      aria-label="Close highlight palette"
      onclick={onclose}>×</button
    >
  </div>
  <div class="presets" role="group" aria-label="Highlight colors">
    {#each presets as preset (preset)}
      <button
        type="button"
        class="swatch"
        class:chosen={current === preset}
        style:background="var(--marker-{preset})"
        aria-label="{preset} highlight"
        title="{preset} highlight"
        aria-pressed={current === preset}
        disabled={busy}
        onclick={() => onapply(preset)}
        >{current === preset ? '✓' : preset === 'default' ? 'A' : ''}</button
      >
    {/each}
  </div>
  <form
    onsubmit={(event) => {
      event.preventDefault()
      onapply(hex)
    }}
  >
    <label class="hex"
      >Hex color<input
        aria-label="Hex color"
        type="text"
        bind:value={hex}
        required
        pattern={'#[0-9a-fA-F]{3}([0-9a-fA-F]{3})?'}
        placeholder="#ffcc00"
        spellcheck="false"
        disabled={busy}
      /></label
    >
    <input
      type="color"
      aria-label="Choose custom highlight color"
      value={hex}
      oninput={(event) => {
        hex = event.currentTarget.value
      }}
      disabled={busy}
    />
    <button type="submit" disabled={busy}>Apply</button>
  </form>
  <button
    type="button"
    class="remove"
    disabled={busy}
    onclick={() => onapply(null)}>Remove highlight</button
  >
</div>

<style>
  .highlight-palette {
    position: fixed;
    inset: auto;
    margin: 0;
    width: min(300px, calc(100vw - 16px));
    box-sizing: border-box;
    max-height: calc(100vh - 16px);
    overflow: auto;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev);
    color: var(--fg);
    font: 13px/1.4 var(--font-ui);
    z-index: 50;
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-block-end: var(--space-2);
  }
  button {
    min-height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  button:hover {
    border-color: var(--accent);
  }
  .heading button {
    width: 30px;
    font-size: 18px;
  }
  .presets {
    display: flex;
    gap: var(--space-2);
    margin-block-end: var(--space-3);
  }
  .swatch {
    flex: 1;
    min-width: 24px;
    color: var(--marker-fg);
  }
  .chosen {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  form {
    display: flex;
    align-items: end;
    gap: var(--space-2);
  }
  .hex {
    display: flex;
    flex: 1;
    min-width: 0;
    flex-direction: column;
    gap: var(--space-1);
  }
  input[type='text'] {
    width: 100%;
    box-sizing: border-box;
    min-height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--fg);
    font: 13px var(--font-mono);
    padding-inline: var(--space-2);
  }
  input[type='color'] {
    width: 32px;
    height: 30px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    cursor: pointer;
  }
  .remove {
    margin-block-start: var(--space-3);
    width: 100%;
  }
  :is(button, input):focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
