<script lang="ts">
  import Preview from './Preview.svelte'

  let html = $state('')

  export function setHtml(next: string) {
    html = next
  }
</script>

<Preview {html} emptyMessage="empty" onnavigate={() => {}} />
