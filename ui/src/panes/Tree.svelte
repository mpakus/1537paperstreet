<script lang="ts">
  import { untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'

  import {
    errorMessage,
    fsCopy,
    fsCreateUntitled,
    fsRename,
    fsTrash,
    openExternal,
    revealInFinder,
    treeReadDir,
    type Project,
    type TreeNode,
  } from '../lib/ipc'
  import {
    ancestorDirs,
    beginTreeDrag,
    claimTreeDrop,
    clearTreeDrag,
    clipboardNames,
    clipboardPaths,
    fileIconKind,
    flattenTree,
    isEditablePath,
    isMarkdownPath,
    joinRel,
    parentDir,
    peekTreeDragCopy,
    rangeRelPaths,
    setTreeDragCopy,
    sortDirsByDepth,
    targetDir,
    treeClickIntent,
    treeDropSiteAt,
    acceptTreeDrop,
    dragGhostPreview,
    type DragGhostPreview,
    visibleWindow,
  } from '../lib/tree'

  let {
    project,
    selectedRelPaths = [],
    revealRelPath = null,
    initialExpanded = [],
    confirmDelete = true,
    reloadToken = 0,
    destMode = null,
    watchSeq = 0,
    watchDirs = [],
    externalDropRel = null,
    onerror,
    onopen,
    onselect,
    onexpanded,
    ontrashed = () => {},
    ontransfer,
    onprojectdrop = () => {},
    onrenamed,
  }: {
    project: Project | null
    selectedRelPaths?: string[]
    revealRelPath?: string | null
    initialExpanded?: string[]
    confirmDelete?: boolean
    reloadToken?: number
    destMode?: 'copy' | 'move' | null
    watchSeq?: number
    watchDirs?: string[]
    externalDropRel?: string | null
    onerror: (message: string) => void
    onopen: (relPath: string) => void
    onselect: (nodes: TreeNode[]) => void
    onexpanded: (paths: string[]) => void
    ontrashed?: (relPaths: string[]) => void
    ontransfer: (mode: 'copy' | 'move', from: string[], toDir: string) => void
    onprojectdrop?: (projectId: string, copy: boolean) => void
    onrenamed?: (from: string, to: string) => void
  } = $props()

  const ROW = 28
  const BUFFER = 20

  let rootNodes = $state<TreeNode[]>([])
  let children = $state<Record<string, TreeNode[]>>({})
  let loaded = $state(false)
  let loadError = $state('')
  const expanded = new SvelteSet<string>()
  let scrollTop = $state(0)
  let viewportHeight = $state(0)
  let menu = $state<{ x: number; y: number; node: TreeNode | null } | null>(
    null,
  )
  let renaming = $state<string | null>(null)
  let renameValue = $state('')
  let renameInput = $state<HTMLInputElement | undefined>()
  let dropTarget = $state<string | null>(null)
  let draggingPaths = $state<string[]>([])
  let ghost = $state<
    (DragGhostPreview & { x: number; y: number; copy: boolean }) | null
  >(null)
  let ghostEl = $state<HTMLDivElement | undefined>()
  let ghostLeaving = $state(false)
  let anchorRel = $state<string | null>(null)
  let trashTargets = $state<TreeNode[]>([])
  let renameTimer: ReturnType<typeof setTimeout> | null = null
  let expandTimer: ReturnType<typeof setTimeout> | null = null
  let expandDest: string | null = null
  let dragged = false
  let press: { x: number; y: number; node: TreeNode } | null = null
  const RENAME_CLICK_MS = 550
  const EXPAND_ON_DRAG_MS = 500
  const DRAG_PX = 6
  const GHOST_X = 14
  const GHOST_Y = 12
  const GHOST_DROP_MS = 200
  const GHOST_EASE = 'cubic-bezier(0.23, 1, 0.32, 1)'

  async function loadDir(relPath: string) {
    if (!project) {
      return
    }
    try {
      const nodes = await treeReadDir(project.id, relPath)
      if (relPath === '') {
        rootNodes = nodes
        loaded = true
        loadError = ''
      } else {
        children = { ...children, [relPath]: nodes }
      }
    } catch (cause) {
      const message = errorMessage(cause)
      if (relPath === '') {
        loaded = true
        loadError = message
      }
      onerror(message)
    }
  }

  function resetExpanded(dirs: Iterable<string> = []) {
    expanded.clear()
    for (const dir of dirs) {
      expanded.add(dir)
    }
  }

  function persistExpanded() {
    onexpanded([...expanded])
  }

  $effect(() => {
    const id = project?.id
    const saved = initialExpanded
    if (!id) {
      rootNodes = []
      children = {}
      loaded = false
      loadError = ''
      resetExpanded()
      return
    }
    const dirs = sortDirsByDepth(saved)
    children = {}
    loaded = false
    loadError = ''
    resetExpanded(dirs)
    void (async () => {
      await loadDir('')
      for (const dir of dirs) {
        await loadDir(dir)
      }
    })()
  })

  $effect(() => {
    const id = project?.id
    const reveal = revealRelPath
    if (!id || !reveal) {
      return
    }
    const dirs = ancestorDirs(reveal)
    for (const dir of dirs) {
      expanded.add(dir)
    }
    void (async () => {
      for (const dir of sortDirsByDepth(dirs)) {
        await loadDir(dir)
      }
    })()
  })

  const rows = $derived.by(() => {
    void expanded.size
    return flattenTree(rootNodes, children, expanded)
  })
  const range = $derived(
    visibleWindow(rows.length, scrollTop, viewportHeight, ROW, BUFFER),
  )
  const visible = $derived(rows.slice(range.start, range.end))

  function toggle(node: TreeNode) {
    if (node.kind !== 'directory') {
      return
    }
    if (expanded.has(node.relPath)) {
      expanded.delete(node.relPath)
    } else {
      expanded.add(node.relPath)
      if (!children[node.relPath]) {
        void loadDir(node.relPath)
      }
    }
    persistExpanded()
  }

  $effect(() => {
    const token = reloadToken
    const id = project?.id
    if (!id || token === 0) {
      return
    }
    const dirs = untrack(() => [...expanded])
    void (async () => {
      await loadDir('')
      for (const dir of dirs) {
        await loadDir(dir)
      }
    })()
  })

  $effect(() => {
    const token = watchSeq
    const id = project?.id
    if (!id || token === 0) {
      return
    }
    const dirs = watchDirs
    void (async () => {
      for (const dir of dirs) {
        await loadDir(dir)
      }
    })()
  })

  function nodesFor(paths: string[]): TreeNode[] {
    const byPath = new Map(rows.map((row) => [row.node.relPath, row.node]))
    return paths.map((path) => {
      const existing = byPath.get(path)
      if (existing) {
        return existing
      }
      const name = path.split('/').filter(Boolean).at(-1) ?? path
      return { name, relPath: path, kind: 'file' }
    })
  }

  function applySelection(paths: string[], anchor = paths.at(-1) ?? null) {
    if (anchor !== null) {
      anchorRel = anchor
    }
    onselect(nodesFor(paths))
  }

  function actionNodes(node: TreeNode): TreeNode[] {
    if (
      selectedRelPaths.includes(node.relPath) &&
      selectedRelPaths.length > 1
    ) {
      return nodesFor(selectedRelPaths)
    }
    return [node]
  }

  function activate(node: TreeNode) {
    applySelection([node.relPath], node.relPath)
    if (destMode) {
      ontransfer(destMode, [], targetDir(node))
      return
    }
    if (node.kind === 'directory') {
      toggle(node)
      return
    }
    onopen(node.relPath)
  }

  function clearRenameTimer() {
    if (renameTimer) {
      clearTimeout(renameTimer)
      renameTimer = null
    }
  }

  function clearExpandTimer() {
    if (expandTimer) {
      clearTimeout(expandTimer)
      expandTimer = null
    }
    expandDest = null
  }

  function beginRename(node: TreeNode) {
    clearRenameTimer()
    renaming = node.relPath
    renameValue = node.name
  }

  function clickRow(event: MouseEvent, node: TreeNode) {
    if (dragged) {
      dragged = false
      return
    }
    if (destMode) {
      applySelection([node.relPath], node.relPath)
      ontransfer(destMode, [], targetDir(node))
      return
    }
    const already =
      selectedRelPaths.length === 1 && selectedRelPaths[0] === node.relPath
    const onName =
      event.target instanceof Element && Boolean(event.target.closest('.name'))
    const intent = treeClickIntent({
      detail: event.detail,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
      ctrlKey: event.ctrlKey,
      alreadySelected: already,
      onName,
    })
    if (intent === 'range') {
      clearRenameTimer()
      applySelection(rangeRelPaths(rows, anchorRel, node.relPath), node.relPath)
      return
    }
    if (intent === 'toggle') {
      clearRenameTimer()
      const next = selectedRelPaths.includes(node.relPath)
        ? selectedRelPaths.filter((path) => path !== node.relPath)
        : [...selectedRelPaths, node.relPath]
      applySelection(next, node.relPath)
      return
    }
    if (intent === 'open') {
      clearRenameTimer()
      activate(node)
      return
    }
    if (intent === 'rename') {
      clearRenameTimer()
      renameTimer = setTimeout(() => beginRename(node), RENAME_CLICK_MS)
      return
    }
    clearRenameTimer()
    applySelection([node.relPath], node.relPath)
  }

  function prefersReducedMotion(): boolean {
    return (
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches
    )
  }

  function destGhostPoint(
    site: { kind: 'project'; id: string } | { kind: 'tree'; dir: string },
  ): { x: number; y: number } | null {
    if (typeof document === 'undefined') {
      return null
    }
    const selector =
      site.kind === 'project'
        ? `[data-project-id="${CSS.escape(site.id)}"]`
        : site.dir === ''
          ? '.tree-scroll'
          : `[data-rel="${CSS.escape(site.dir)}"][data-kind="directory"]`
    const el = document.querySelector(selector)
    if (!(el instanceof HTMLElement)) {
      return null
    }
    const box = el.getBoundingClientRect()
    return { x: box.left + 12, y: box.top + Math.min(8, box.height / 4) }
  }

  function placeGhost(x: number, y: number, copy: boolean) {
    if (!ghost || ghostLeaving) {
      return
    }
    const next = { ...ghost, x: x + GHOST_X, y: y + GHOST_Y, copy }
    ghost = next
    if (ghostEl) {
      ghostEl.style.transform = `translate3d(${next.x}px, ${next.y}px, 0)`
    }
  }

  function hideGhost() {
    ghost = null
    ghostLeaving = false
  }

  function exitGhost(to: { x: number; y: number } | null, ondone: () => void) {
    const current = ghost
    const el = ghostEl
    if (!current || !el || prefersReducedMotion()) {
      hideGhost()
      ondone()
      return
    }
    const dest = to ?? { x: current.x, y: current.y }
    const scale = to ? 0.94 : 0.96
    const anim = el.animate(
      [
        {
          transform: `translate3d(${current.x}px, ${current.y}px, 0) scale(1)`,
          opacity: 1,
        },
        {
          transform: `translate3d(${dest.x}px, ${dest.y}px, 0) scale(${scale})`,
          opacity: 0,
        },
      ],
      { duration: GHOST_DROP_MS, easing: GHOST_EASE, fill: 'forwards' },
    )
    ghostLeaving = true
    void anim.finished.finally(() => {
      hideGhost()
      ondone()
    })
  }

  function hoverPointerDrop(x: number, y: number) {
    const site = acceptTreeDrop(draggingPaths, treeDropSiteAt(x, y))
    dropTarget = site?.kind === 'tree' ? site.dir : null
    if (typeof document === 'undefined') {
      return
    }
    const el = document.elementFromPoint(x, y)
    const row = el instanceof Element ? el.closest('[data-rel]') : null
    const dest =
      row instanceof HTMLElement && row.dataset.kind === 'directory'
        ? (row.dataset.rel ?? null)
        : null
    if (!dest || expanded.has(dest)) {
      if (expandDest !== dest) {
        clearExpandTimer()
      }
      return
    }
    if (expandDest === dest) {
      return
    }
    clearExpandTimer()
    expandDest = dest
    expandTimer = setTimeout(() => {
      expanded.add(dest)
      void loadDir(dest)
      persistExpanded()
    }, EXPAND_ON_DRAG_MS)
  }

  function finishPointerDrop(event: PointerEvent) {
    const paths = draggingPaths
    const copy = event.altKey || peekTreeDragCopy()
    dropTarget = null
    clearExpandTimer()
    press = null
    if (paths.length === 0 || !project) {
      exitGhost(null, () => {
        draggingPaths = []
        clearTreeDrag()
      })
      return
    }
    const site = acceptTreeDrop(
      paths,
      treeDropSiteAt(event.clientX, event.clientY),
    )
    if (!site) {
      exitGhost(null, () => {
        draggingPaths = []
        clearTreeDrag()
      })
      return
    }
    const flyTo = destGhostPoint(site)
    if (site.kind === 'project') {
      onprojectdrop(site.id, copy)
    } else if (claimTreeDrop(project.id, paths)) {
      ontransfer(copy ? 'copy' : 'move', paths, site.dir)
    }
    exitGhost(flyTo, () => {
      draggingPaths = []
      clearTreeDrag()
    })
  }

  function movePointerDrag(event: PointerEvent) {
    if (!press || !project) {
      return
    }
    if (draggingPaths.length === 0) {
      if (
        Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_PX
      ) {
        return
      }
      if (event.buttons === 0) {
        press = null
        return
      }
      dragged = true
      const paths = actionNodes(press.node).map((item) => item.relPath)
      draggingPaths = paths
      beginTreeDrag(project.id, paths)
      ghost = {
        ...dragGhostPreview(nodesFor(paths)),
        x: event.clientX + GHOST_X,
        y: event.clientY + GHOST_Y,
        copy: event.altKey,
      }
    } else {
      placeGhost(event.clientX, event.clientY, event.altKey)
    }
    event.preventDefault()
    setTreeDragCopy(event.altKey)
    hoverPointerDrop(event.clientX, event.clientY)
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function createUntitled(
    kind: 'file' | 'folder',
    node: TreeNode | null,
  ) {
    if (!project) {
      return
    }
    try {
      const dir = targetDir(node)
      const created = await fsCreateUntitled(project.id, dir, kind)
      if (dir) {
        expanded.add(dir)
      }
      if (kind === 'folder') {
        expanded.add(created.relPath)
      }
      persistExpanded()
      await loadDir(dir)
      applySelection([created.relPath], created.relPath)
      beginRename(created)
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function duplicateNode(node: TreeNode) {
    if (!project) {
      return
    }
    try {
      const dir = parentDir(node.relPath)
      await fsCopy(project.id, [node.relPath], dir, 'keepBoth')
      await loadDir(dir)
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function commitRename(node: TreeNode) {
    if (!project || renaming !== node.relPath) {
      return
    }
    const next = renameValue.trim()
    renaming = null
    if (!next || next === node.name) {
      return
    }
    try {
      const to = joinRel(parentDir(node.relPath), next)
      const renamed = await fsRename(project.id, node.relPath, to)
      await loadDir(parentDir(node.relPath))
      applySelection([renamed.relPath], renamed.relPath)
      onrenamed?.(node.relPath, renamed.relPath)
      if (isEditablePath(renamed.name)) {
        onopen(renamed.relPath)
      }
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function trashNodes(nodes: TreeNode[]) {
    if (!project || nodes.length === 0) {
      return
    }
    try {
      const paths = nodes.map((node) => node.relPath)
      await fsTrash(project.id, paths)
      trashTargets = []
      const parents = new Set(paths.map((path) => parentDir(path)))
      for (const dir of sortDirsByDepth(parents)) {
        await loadDir(dir)
      }
      ontrashed(paths)
      if (paths.some((path) => selectedRelPaths.includes(path))) {
        onselect([])
      }
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  function requestTrash(nodes: TreeNode[]) {
    if (nodes.length === 0) {
      return
    }
    if (confirmDelete) {
      trashTargets = nodes
      return
    }
    void trashNodes(nodes)
  }

  async function reveal(node: TreeNode | null) {
    if (!project) {
      return
    }
    try {
      await revealInFinder(project.id, node?.relPath ?? '')
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  async function openOutside(node: TreeNode) {
    if (!project) {
      return
    }
    try {
      await openExternal(project.id, node.relPath)
    } catch (cause) {
      onerror(errorMessage(cause))
    }
  }

  function openMenu(event: MouseEvent, node: TreeNode | null) {
    event.preventDefault()
    menu = { x: event.clientX, y: event.clientY, node }
    if (node && !selectedRelPaths.includes(node.relPath)) {
      applySelection([node.relPath], node.relPath)
    }
  }

  function closeMenu() {
    menu = null
  }

  function withMenu(action: (node: TreeNode | null) => void) {
    const node = menu?.node ?? null
    closeMenu()
    action(node)
  }

  $effect(() => {
    if (ghostEl && ghost && !ghostLeaving) {
      ghostEl.style.transform = `translate3d(${ghost.x}px, ${ghost.y}px, 0)`
    }
  })

  $effect(() => {
    if (renaming && renameInput) {
      renameInput.focus()
      renameInput.select()
    }
  })
</script>

<svelte:window
  onpointerdown={(event) => {
    if (event.button === 0) {
      closeMenu()
    }
  }}
  onpointermove={(event) => {
    movePointerDrag(event)
  }}
  onpointerup={(event) => {
    if (press && event.button === 0) {
      finishPointerDrop(event)
    }
  }}
  onpointercancel={() => {
    dropTarget = null
    draggingPaths = []
    press = null
    clearExpandTimer()
    hideGhost()
    clearTreeDrag()
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      closeMenu()
      clearRenameTimer()
      renaming = null
      trashTargets = []
      if (press || draggingPaths.length > 0 || ghost) {
        dropTarget = null
        draggingPaths = []
        press = null
        clearExpandTimer()
        hideGhost()
        clearTreeDrag()
      }
    }
  }}
/>

<div
  class="tree-scroll"
  class:drop-root={externalDropRel === '' || dropTarget === ''}
  class:is-dragging={draggingPaths.length > 0}
  role="region"
  aria-label="Files"
  bind:clientHeight={viewportHeight}
  onscroll={(event) => {
    scrollTop = event.currentTarget.scrollTop
  }}
  oncontextmenu={(event) => openMenu(event, null)}
>
  {#if !loaded}
    <p class="empty">Loading…</p>
  {:else if loadError}
    <p class="empty">{loadError}</p>
  {:else if rows.length === 0}
    <p class="empty">This folder is empty.</p>
  {:else}
    <div class="tree-spacer" style:height="{rows.length * ROW}px">
      <div
        class="tree-window"
        style:top="{range.start * ROW}px"
        role="tree"
        aria-label="Project files"
      >
        {#each visible as row (row.node.relPath)}
          {@const selected = selectedRelPaths.includes(row.node.relPath)}
          {@const isDir = row.node.kind === 'directory'}
          {@const open = isDir && expanded.has(row.node.relPath)}
          {@const markdown = fileIconKind(row.node) === 'markdown'}
          {@const editing = renaming === row.node.relPath}
          {@const highlighted =
            dropTarget === row.node.relPath ||
            (isDir && externalDropRel === row.node.relPath)}
          <div
            class="row"
            class:selected
            class:markdown
            class:drop={highlighted}
            class:source={draggingPaths.includes(row.node.relPath)}
            data-rel={row.node.relPath}
            data-kind={row.node.kind}
            style:padding-inline-start={`${12 + row.depth * 12}px`}
            role="treeitem"
            aria-selected={selected}
            aria-expanded={isDir ? open : undefined}
            onclick={(event) => {
              if (!editing) {
                clickRow(event, row.node)
              }
            }}
            onpointerdown={(event) => {
              if (editing || destMode || event.button !== 0) {
                return
              }
              press = {
                x: event.clientX,
                y: event.clientY,
                node: row.node,
              }
            }}
            oncontextmenu={(event) => {
              event.stopPropagation()
              openMenu(event, row.node)
            }}
            onkeydown={(event) => {
              if (editing) {
                return
              }
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault()
                activate(row.node)
              }
              if (event.key === 'F2') {
                event.preventDefault()
                beginRename(row.node)
              }
            }}
            tabindex="0"
          >
            <span class="twist" aria-hidden="true">
              {#if isDir}{open ? '▾' : '▸'}{/if}
            </span>
            <span class="icon" aria-hidden="true">
              {#if isDir}
                <svg viewBox="0 0 16 16">
                  <path
                    d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1 1.5H12.5A1.5 1.5 0 0 1 14 6v6.5A1.5 1.5 0 0 1 12.5 14h-9A1.5 1.5 0 0 1 2 12.5z"
                  />
                </svg>
              {:else if markdown}
                <svg viewBox="0 0 16 16">
                  <path
                    d="M3.5 2h6l3.5 3.5V13.5A1.5 1.5 0 0 1 11.5 15h-8A1.5 1.5 0 0 1 2 13.5v-10A1.5 1.5 0 0 1 3.5 2zm.5 3h4v1.2L6.7 8.5 5.4 6.8 4 8.4V5zm5 6.2c.9 0 1.6-.6 1.6-1.5S10.4 8.2 9.5 8.2 7.9 8.8 7.9 9.7s.7 1.5 1.6 1.5z"
                  />
                </svg>
              {:else}
                <svg viewBox="0 0 16 16">
                  <path
                    d="M3.5 2h6L13 5.5V13.5A1.5 1.5 0 0 1 11.5 15h-8A1.5 1.5 0 0 1 2 13.5v-10A1.5 1.5 0 0 1 3.5 2z"
                  />
                </svg>
              {/if}
            </span>
            {#if editing}
              <input
                class="rename"
                bind:this={renameInput}
                value={renameValue}
                aria-label="Rename"
                onclick={(event) => event.stopPropagation()}
                onpointerdown={(event) => event.stopPropagation()}
                oninput={(event) => {
                  renameValue = event.currentTarget.value
                }}
                onkeydown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault()
                    void commitRename(row.node)
                  }
                  if (event.key === 'Escape') {
                    event.preventDefault()
                    renaming = null
                  }
                }}
                onblur={() => void commitRename(row.node)}
              />
            {:else}
              <span class="name">{row.node.name}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

{#if ghost}
  <div
    bind:this={ghostEl}
    class="drag-ghost"
    class:copy={ghost.copy}
    class:leaving={ghostLeaving}
    aria-hidden="true"
  >
    <div class="drag-ghost-inner">
      {#each ghost.items as item, index (item.relPath)}
        <div
          class="drag-ghost-card"
          class:folder={item.kind === 'directory'}
          class:markdown={item.kind !== 'directory' &&
            isMarkdownPath(item.name)}
          style:transform="translate({index * 3}px, {index * 3}px)"
          style:z-index={ghost.items.length - index}
        >
          <span class="icon" aria-hidden="true">
            {#if item.kind === 'directory'}
              <svg viewBox="0 0 16 16">
                <path
                  d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1 1.5H12.5A1.5 1.5 0 0 1 14 6v6.5A1.5 1.5 0 0 1 12.5 14h-9A1.5 1.5 0 0 1 2 12.5z"
                />
              </svg>
            {:else if isMarkdownPath(item.name)}
              <svg viewBox="0 0 16 16">
                <path
                  d="M3.5 2h6l3.5 3.5V13.5A1.5 1.5 0 0 1 11.5 15h-8A1.5 1.5 0 0 1 2 13.5v-10A1.5 1.5 0 0 1 3.5 2zm.5 3h4v1.2L6.7 8.5 5.4 6.8 4 8.4V5zm5 6.2c.9 0 1.6-.6 1.6-1.5S10.4 8.2 9.5 8.2 7.9 8.8 7.9 9.7s.7 1.5 1.6 1.5z"
                />
              </svg>
            {:else}
              <svg viewBox="0 0 16 16">
                <path
                  d="M3.5 2h6L13 5.5V13.5A1.5 1.5 0 0 1 11.5 15h-8A1.5 1.5 0 0 1 2 13.5v-10A1.5 1.5 0 0 1 3.5 2z"
                />
              </svg>
            {/if}
          </span>
          <span class="name">{item.name}</span>
        </div>
      {/each}
      {#if ghost.items.length + ghost.extra > 1}
        <span class="drag-ghost-count">
          {ghost.items.length + ghost.extra}
        </span>
      {/if}
    </div>
  </div>
{/if}

{#if menu}
  <div
    class="menu"
    style:left="{menu.x}px"
    style:top="{menu.y}px"
    role="menu"
    tabindex="-1"
    onpointerdown={(event) => event.stopPropagation()}
    oncontextmenu={(event) => event.preventDefault()}
  >
    <button
      type="button"
      role="menuitem"
      onclick={() => withMenu((node) => void createUntitled('file', node))}
      >New File</button
    >
    <button
      type="button"
      role="menuitem"
      onclick={() => withMenu((node) => void createUntitled('folder', node))}
      >New Folder</button
    >
    {#if menu.node}
      {#if actionNodes(menu.node).length === 1}
        <button
          type="button"
          role="menuitem"
          onclick={() =>
            withMenu((node) => {
              if (node) {
                beginRename(node)
              }
            })}>Rename</button
        >
      {/if}
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          withMenu((node) => {
            if (node) {
              for (const item of actionNodes(node)) {
                void duplicateNode(item)
              }
            }
          })}>Duplicate</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          withMenu((node) => {
            if (node) {
              void copyText(clipboardNames(actionNodes(node)))
            }
          })}>Copy name</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          withMenu((node) => {
            if (node && project) {
              void copyText(
                clipboardPaths(
                  project.path,
                  actionNodes(node).map((item) => item.relPath),
                ),
              )
            }
          })}>Copy path</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          withMenu((node) => {
            if (node) {
              ontransfer(
                'copy',
                actionNodes(node).map((item) => item.relPath),
                '',
              )
            }
          })}>Copy to…</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          withMenu((node) => {
            if (node) {
              ontransfer(
                'move',
                actionNodes(node).map((item) => item.relPath),
                '',
              )
            }
          })}>Move to…</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() => withMenu((node) => void reveal(node))}
        >Reveal in Finder</button
      >
      {#if actionNodes(menu.node).every((item) => item.kind !== 'directory')}
        <button
          type="button"
          role="menuitem"
          onclick={() =>
            withMenu((node) => {
              if (node) {
                for (const item of actionNodes(node)) {
                  void openOutside(item)
                }
              }
            })}>Open in External Editor</button
        >
      {/if}
      <button
        type="button"
        role="menuitem"
        class="danger"
        onclick={() =>
          withMenu((node) => {
            if (node) {
              requestTrash(actionNodes(node))
            }
          })}>Move to Trash</button
      >
    {:else}
      <button
        type="button"
        role="menuitem"
        onclick={() => withMenu(() => void reveal(null))}
        >Reveal in Finder</button
      >
    {/if}
  </div>
{/if}

{#if trashTargets.length > 0}
  <div class="confirm" role="dialog" aria-labelledby="trash-title">
    <div class="confirm-card">
      <p id="trash-title">
        {#if trashTargets.length === 1}
          Move “{trashTargets[0].name}” to Trash?
        {:else}
          Move {trashTargets.length} items to Trash?
        {/if}
      </p>
      <div class="confirm-actions">
        <button type="button" onclick={() => (trashTargets = [])}>Cancel</button
        >
        <button
          type="button"
          class="danger"
          onclick={() => {
            void trashNodes(trashTargets)
          }}>Move to Trash</button
        >
      </div>
    </div>
  </div>
{/if}

<style>
  .tree-scroll {
    position: relative;
    flex: 1;
    min-height: 0;
    height: 100%;
    overflow: auto;
  }

  .empty {
    margin: 0;
    padding: var(--space-3);
    color: var(--fg-muted);
    font-size: 0.875rem;
  }

  .tree-spacer {
    position: relative;
  }

  .tree-window {
    position: absolute;
    inset-inline: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    height: var(--tree-row);
    padding-inline-end: var(--space-2);
    text-align: start;
    color: var(--fg-muted);
    border-radius: var(--radius-sm);
    cursor: grab;
    user-select: none;
    transition-property: background-color, box-shadow, opacity, transform;
    transition-duration: var(--duration);
    transition-timing-function: var(--ease-out);
  }

  @media (prefers-reduced-motion: reduce) {
    .row {
      transition-duration: 0ms;
    }
  }

  .row:active {
    cursor: grabbing;
  }

  .tree-scroll.is-dragging,
  .tree-scroll.is-dragging .row {
    cursor: grabbing;
  }

  .row.markdown,
  .row[aria-expanded] {
    color: var(--fg);
  }

  .row.selected,
  .row:hover,
  .row.drop {
    background: var(--selection);
  }

  .row.source {
    opacity: 0.4;
    transform: scale(0.98);
  }

  .tree-scroll.drop-root,
  .row.drop {
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .row.drop[data-kind='directory'] {
    box-shadow: inset 0 0 0 2px var(--accent);
    transform: translateX(2px);
  }

  .twist {
    display: inline-flex;
    width: var(--space-3);
    flex: none;
    justify-content: center;
    color: var(--fg-muted);
  }

  .icon {
    display: inline-flex;
    width: var(--space-4);
    height: var(--space-4);
    flex: none;
    color: var(--fg-muted);
  }

  .row.markdown .icon,
  .row[aria-expanded] .icon {
    color: var(--accent);
  }

  .icon svg {
    width: 100%;
    height: 100%;
    fill: currentColor;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .drag-ghost {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 18;
    pointer-events: none;
    transform-origin: top left;
  }

  .drag-ghost-inner {
    position: relative;
    min-width: 8rem;
    max-width: 16rem;
    animation: drag-ghost-in var(--duration) var(--ease-out);
  }

  .drag-ghost.leaving .drag-ghost-inner {
    animation: none;
  }

  .drag-ghost.copy .drag-ghost-inner {
    outline: 1px dashed var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius);
  }

  .drag-ghost-card {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    height: var(--tree-row);
    padding: 0 var(--space-2);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg);
    box-shadow: 0 var(--space-1) var(--space-3)
      color-mix(in srgb, var(--fg) 16%, transparent);
  }

  .drag-ghost-card + .drag-ghost-card {
    position: absolute;
    inset-block-start: 0;
    inset-inline-start: 0;
    width: 100%;
  }

  .drag-ghost-card.folder .icon,
  .drag-ghost-card.markdown .icon {
    color: var(--accent);
  }

  .drag-ghost-count {
    position: absolute;
    inset-block-start: -6px;
    inset-inline-end: -6px;
    z-index: 4;
    min-width: 1.25rem;
    padding: 0 var(--space-1);
    border-radius: 999px;
    background: var(--accent);
    color: var(--bg);
    font-size: 0.7rem;
    line-height: 1.25rem;
    text-align: center;
  }

  @keyframes drag-ghost-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .drag-ghost-inner {
      animation: none;
    }

    .row.drop[data-kind='directory'],
    .row.source {
      transform: none;
    }
  }

  .rename {
    flex: 1;
    min-width: 0;
    height: calc(var(--tree-row) - 6px);
    padding: 0 var(--space-1);
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    font: inherit;
  }

  .menu {
    position: fixed;
    z-index: 20;
    min-width: 12rem;
    padding: var(--space-1);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .menu button {
    display: block;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    text-align: start;
    border-radius: var(--radius-sm);
    color: var(--fg);
  }

  .menu button:hover,
  .confirm-actions button:hover {
    background: var(--selection);
  }

  .danger {
    color: var(--accent);
  }

  .confirm {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--fg) 20%, transparent);
  }

  .confirm-card {
    width: min(22rem, 100%);
    padding: var(--space-4);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .confirm-card p {
    margin: 0 0 var(--space-3);
    color: var(--fg);
  }

  .confirm-actions {
    display: flex;
    justify-content: end;
    gap: var(--space-2);
  }

  .confirm-actions button {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--bg);
    border: 1px solid var(--border);
  }
</style>
