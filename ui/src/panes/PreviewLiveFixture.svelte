<script lang="ts">
  import type { TocEntry } from '../lib/generated/core'
  import Preview from './Preview.svelte'

  let html = $state('')
  let toc = $state<TocEntry[]>([])
  let sourceLine = $state<number | null>(null)

  export function setHtml(next: string) {
    html = next
  }

  export function setToc(next: TocEntry[]) {
    toc = next
  }

  export function line() {
    return sourceLine
  }
</script>

<Preview
  {html}
  {toc}
  emptyMessage="empty"
  onnavigate={() => {}}
  onsource={(next) => {
    sourceLine = next
  }}
/>
