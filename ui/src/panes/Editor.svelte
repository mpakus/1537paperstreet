<script lang="ts">
  import { usesCodeEditor } from '../editor/language'
  import type { MarkdownEditor } from '../editor/types'

  let {
    value = $bindable(''),
    api = $bindable(null),
    fileName = '',
    writable = true,
    spellcheck = true,
    lineNumbers = false,
    softWrap = true,
    indentUnit = 2,
    hidden = false,
  }: {
    value: string
    api?: MarkdownEditor | null
    fileName?: string
    writable?: boolean
    spellcheck?: boolean
    lineNumbers?: boolean
    softWrap?: boolean
    indentUnit?: number
    hidden?: boolean
  } = $props()

  let host = $state<HTMLDivElement | undefined>()

  $effect(() => {
    const el = host
    if (!el) {
      return
    }
    let cancelled = false
    let instance: MarkdownEditor | undefined

    void import('../editor/setup').then(({ createMarkdownEditor }) => {
      if (cancelled) {
        return
      }
      instance = createMarkdownEditor(el, {
        doc: value,
        fileName,
        writable,
        spellcheck,
        lineNumbers,
        softWrap,
        indentUnit,
        onChange(text) {
          value = text
        },
      })
      api = instance
    })

    return () => {
      cancelled = true
      instance?.destroy()
      api = null
    }
  })

  $effect(() => {
    api?.setDoc(value)
  })
  $effect(() => {
    api?.setFileName(fileName)
  })
  $effect(() => {
    api?.setWritable(writable)
  })
  $effect(() => {
    api?.setSpellcheck(spellcheck)
  })
  $effect(() => {
    api?.setLineNumbers(lineNumbers)
  })
  $effect(() => {
    api?.setSoftWrap(softWrap)
  })
  $effect(() => {
    api?.setIndentUnit(indentUnit)
  })
  $effect(() => {
    if (!hidden) {
      api?.refresh()
    }
  })
</script>

<div
  class="editor"
  class:is-hidden={hidden}
  class:is-code={usesCodeEditor(fileName)}
>
  <div bind:this={host} class="cm-host"></div>
</div>

<style>
  .editor {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .editor.is-hidden {
    display: none;
  }

  .editor.is-code :global(.cm-editor),
  .editor.is-code :global(.cm-scroller) {
    font-family: var(--font-mono);
  }

  .cm-host {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .cm-host :global(.cm-editor) {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
</style>
