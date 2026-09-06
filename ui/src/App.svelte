<script lang="ts">
  import { onMount } from 'svelte'

  import {
    configGet,
    configSet,
    copyConflicts,
    docOpen,
    docSave,
    docSource,
    docStat,
    errorMessage,
    fsCopy,
    fsCreateUntitled,
    fsImport,
    fsMove,
    fsTransfer,
    fsTrash,
    getAppVersion,
    openDroppedPaths,
    openExternal,
    openUrl,
    pickAndOpen,
    projectsList,
    revealInFinder,
    setWindowTitle,
    startWindowDrag,
    themesCss,
    themesList,
    treeExpandedGet,
    treeExpandedSet,
    updatesCheck,
    watchStart,
    type Config,
    type ConflictStrategy,
    type DocumentMeta,
    type DocumentSource,
    type FsChangedEvent,
    type Project,
    type ThemeInfo,
    type TreeNode,
    type UntitledKind,
    type ViewMode,
    type AgentClientEvent,
    type AgentChoice,
    type AgentPermission,
    type PromptHistoryEntry,
    agentCancel,
    agentPermissionReply,
    agentPrompt,
    agentPromptHistory,
    agentPromptHistoryClear,
    agentPromptHistoryRemove,
    agentSetModel,
    agentStart,
    dashboardGet,
    type DashboardSnapshot,
  } from './lib/ipc'
  import type { MarkdownEditor } from './editor/types'
  import { applyMarkdownCommand } from './lib/markdown'
  import {
    pathsFromDataTransfer,
    recentProjects,
    isExternalFileDrag,
  } from './lib/open'
  import { clampPanelWidth } from './lib/panel-width'
  import {
    closeWorkspaceTab,
    nextAfterClose,
    openWorkspaceTab,
    removeTab,
    retitleTab,
    tabTitle,
    upsertTab,
    type DocTab,
    type WorkspacePage,
    type WorkspaceTab,
  } from './lib/tabs'
  import { recordMessage, TOAST_MS, type AppMessage } from './lib/messages'
  import { exportDocumentPdf } from './lib/print'
  import { windowTitle } from './lib/text'
  import { nextPreviewZoom } from './lib/zoom'
  import {
    dirsToReload,
    dropDirAtPoint,
    isDraftDirty,
    isHttpHref,
    isMarkdownPath,
    parseAssetHref,
    peekTreeDrag,
    peekTreeDragCopy,
    projectIdAtPoint,
    claimTreeDrop,
    targetDir,
    watchTouchesOpenFile,
  } from './lib/tree'
  import About from './panes/About.svelte'
  import Assistant from './panes/Assistant.svelte'
  import ChromeToolbar from './panes/ChromeToolbar.svelte'
  import Conflict from './panes/Conflict.svelte'
  import Dashboard from './panes/Dashboard.svelte'
  import DocTabs from './panes/DocTabs.svelte'
  import Editor from './panes/Editor.svelte'
  import FindBar from './panes/FindBar.svelte'
  import MessageToasts from './panes/MessageToasts.svelte'
  import Preview from './panes/Preview.svelte'
  import Projects from './panes/Projects.svelte'
  import QuickOpen from './panes/QuickOpen.svelte'
  import QuickSwitch from './panes/QuickSwitch.svelte'
  import ReloadDisk from './panes/ReloadDisk.svelte'
  import Settings from './panes/Settings.svelte'
  import Tree from './panes/Tree.svelte'

  let projects = $state<Project[]>([])
  let active = $state<Project | null>(null)
  let selectedNode = $state<TreeNode | null>(null)
  let selectedRelPaths = $state<string[]>([])
  let selectedNodes = $state<TreeNode[]>([])
  let finderDropRel = $state<string | null>(null)
  let projectDropId = $state<string | null>(null)
  let docMissing = $state(false)
  let projectsHidden = $state(false)
  let revealRelPath = $state<string | null>(null)
  let html = $state('')
  let docMeta = $state<DocumentMeta | null>(null)
  let openMeta = $state<{ projectId: string; relPath: string } | null>(null)
  let error = $state('')
  let errorSeq = $state(0)
  let messageHistory = $state<AppMessage[]>([])
  let historyOpen = $state(false)
  let dragging = $state(false)
  let sidebarWidth = $state(220)
  let treeWidth = $state(260)
  let tocWidth = $state(224)
  let editorWidth = $state(480)
  let workspaceEl = $state<HTMLDivElement | undefined>()
  let treeHidden = $state(false)
  let viewMode = $state<ViewMode>('preview')
  let editorOpened = $state(false)
  let draftText = $state('')
  let docSourceMeta = $state<DocumentSource | null>(null)
  let editorApi = $state<MarkdownEditor | null>(null)
  let tabs = $state<DocTab[]>([])
  let switchOpen = $state(false)
  let themeInfos = $state<ThemeInfo[]>([])
  let forcedThemeId = $state<string | null>(null)
  let activeThemeId = $state('paper-light')
  let treeReload = $state(0)
  let watchSeq = $state(0)
  let watchDirs = $state<string[]>([])
  let destMode = $state<'copy' | 'move' | null>(null)
  let transferFrom = $state<string[]>([])
  let conflictNames = $state<string[]>([])
  let pendingTransfer = $state<{
    mode: 'copy' | 'move' | 'import'
    from: string[]
    toDir: string
    fromProjectId?: string
    toProjectId?: string
  } | null>(null)
  let externalPrompt = $state<{
    relPath: string
    name: string
    dirty: boolean
    missing: boolean
    diskHash: string | null
  } | null>(null)
  let ignoredExternal = $state<{ relPath: string; hash: string } | null>(null)
  let settingsOpen = $state(false)
  let settingsFocusAgents = $state(false)
  let aboutOpen = $state(false)
  let workspacePage = $state<WorkspacePage>('document')
  let workspaceTabs = $state<WorkspaceTab[]>([])
  let dashboardSnapshot = $state<DashboardSnapshot | null>(null)
  let dashboardError = $state('')
  let assistantBusy = $state(false)
  let assistantError = $state('')
  let assistantTranscript = $state('')
  let assistantModels = $state<AgentChoice[]>([])
  let assistantModelId = $state('')
  let assistantComposer = $state('')
  let assistantHistory = $state<PromptHistoryEntry[]>([])
  let assistantPermission = $state<AgentPermission>('allowance')
  let assistantServerId = $state('')
  let assistantPermit = $state<{
    id: number
    title: string
    options: AgentChoice[]
  } | null>(null)
  let aboutAutocheck = $state(false)
  let aboutCheckSeq = $state(0)
  let projectsReload = $state(0)
  let appVersion = $state('0.1.0')
  let findOpen = $state(false)
  let quickOpen = $state(false)
  let articleEl = $state<HTMLElement | undefined>()
  let appConfig = $state<Config | null>(null)
  let expandedSeed = $state<string[]>([])
  let confirmDelete = $state(true)
  let showToc = $state(true)
  let trashConfirm = $state<TreeNode | null>(null)
  let fontSize = $state(16)
  let previewZoom = $state(1)
  let lineHeight = $state(1.65)
  let measureCh = $state(72)
  let bodyFont = $state('New York')
  let monoFont = $state('JetBrains Mono')
  let resizeStart = $state<{
    kind: 'sidebar' | 'tree' | 'toc' | 'editor'
    x: number
    width: number
  } | null>(null)

  function showError(message: string) {
    const text = message.trim()
    if (!text) {
      error = ''
      return
    }
    messageHistory = recordMessage(messageHistory, text)
    error = text
    errorSeq += 1
  }

  $effect(() => {
    const seq = errorSeq
    if (!error) {
      return
    }
    const handle = setTimeout(() => {
      if (errorSeq === seq) {
        error = ''
      }
    }, TOAST_MS)
    return () => clearTimeout(handle)
  })

  const documentTitle = $derived(
    workspacePage === 'assistant'
      ? 'Assistant'
      : workspacePage === 'dashboard'
        ? 'Dashboard'
        : windowTitle(active?.path, openMeta?.relPath),
  )

  $effect(() => {
    const title = documentTitle
    void setWindowTitle(title).catch((cause) => {
      showError(errorMessage(cause))
    })
  })

  const emptyMessage = $derived(
    destMode
      ? 'Choose a destination folder in the tree.'
      : docMissing
        ? 'This file is no longer in the project.'
        : docMeta?.sourceOnly
          ? (docMeta.readonlyReason ??
            'This file cannot be previewed as Markdown.')
          : active
            ? 'Select a Markdown file in the tree, or drop one onto the window.'
            : 'Your Markdown projects will appear here. Drop a Markdown file or a folder to open it.',
  )

  function applyConfig(config: Config) {
    forcedThemeId = null
    appConfig = config
    sidebarWidth = config.window.sidebar_w
    treeWidth = config.window.tree_w
    tocWidth = config.window.toc_w
    editorWidth = config.window.editor_w
    fontSize = config.typography.font_size
    lineHeight = config.typography.line_height
    measureCh = config.typography.measure_ch
    bodyFont = config.typography.body_font
    monoFont = config.typography.mono_font
    confirmDelete = config.files.confirm_delete
    showToc = config.viewer.show_toc
    assistantPermission = config.agents?.permission ?? 'allowance'
    const servers = config.agents?.servers ?? []
    if (
      !assistantServerId ||
      !servers.some((server) => server.id === assistantServerId)
    ) {
      assistantServerId =
        config.agents?.default_server_id ?? servers[0]?.id ?? ''
    }
    applyTheme(config)
  }

  function applyTheme(config: Config) {
    if (typeof document === 'undefined') {
      return
    }
    const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    const id =
      forcedThemeId ??
      (config.appearance.follow_system
        ? systemDark
          ? config.appearance.theme_dark
          : config.appearance.theme
        : config.appearance.theme)
    document.documentElement.dataset.theme = id
    activeThemeId = id
  }

  function menuActionId(payload: unknown): string {
    if (typeof payload === 'string') {
      return payload
    }
    if (payload && typeof payload === 'object' && 'id' in payload) {
      const id = (payload as { id: unknown }).id
      if (typeof id === 'string') {
        return id
      }
    }
    return ''
  }

  async function openSettings(opts?: { agents?: boolean }) {
    settingsFocusAgents = opts?.agents ?? false
    settingsOpen = true
    if (appConfig) {
      return
    }
    try {
      applyConfig(await configGet())
    } catch (cause) {
      showError(errorMessage(cause))
    }
  }

  async function openAssistant() {
    workspaceTabs = openWorkspaceTab(workspaceTabs, 'assistant')
    workspacePage = 'assistant'
    assistantError = ''
    try {
      assistantHistory = await agentPromptHistory()
      if (appConfig) {
        applyConfig(appConfig)
      }
    } catch (cause) {
      assistantError = errorMessage(cause)
    }
  }

  async function openDashboard() {
    workspaceTabs = openWorkspaceTab(workspaceTabs, 'dashboard')
    workspacePage = 'dashboard'
    dashboardError = ''
    try {
      dashboardSnapshot = await dashboardGet(active?.id ?? null)
    } catch (cause) {
      dashboardError = errorMessage(cause)
    }
  }

  function applyAgentEvent(event: AgentClientEvent) {
    switch (event.kind) {
      case 'ready':
        assistantModels = event.models
        assistantModelId = event.models[0]?.id ?? ''
        assistantBusy = false
        break
      case 'message':
        assistantTranscript += event.text
        break
      case 'tool':
        assistantTranscript += `\n[${event.title}: ${event.status}]`
        break
      case 'permission':
        assistantPermit = {
          id: event.id,
          title: event.title,
          options: event.options,
        }
        break
      case 'done':
        assistantBusy = false
        assistantTranscript += assistantTranscript.endsWith('\n') ? '' : '\n'
        break
      case 'error':
        assistantBusy = false
        assistantError = event.message
        break
    }
    if (event.kind !== 'message' && workspacePage === 'dashboard') {
      void openDashboard()
    }
  }

  async function startAssistantChat(): Promise<boolean> {
    if (!active) {
      assistantError = 'Open a folder first.'
      return false
    }
    if (!assistantServerId) {
      assistantError = 'Add an agent in Settings.'
      return false
    }
    assistantError = ''
    assistantTranscript = ''
    assistantPermit = null
    assistantBusy = true
    try {
      const ready = await agentStart(
        assistantServerId,
        active.id,
        assistantPermission,
      )
      applyAgentEvent(ready)
      return true
    } catch (cause) {
      assistantBusy = false
      assistantError = errorMessage(cause)
      return false
    }
  }

  async function sendAssistantPrompt(text: string) {
    assistantComposer = ''
    assistantTranscript +=
      (assistantTranscript ? '\n\n' : '') + `You: ${text}\n\n`
    assistantBusy = true
    assistantError = ''
    try {
      await agentPrompt(assistantServerId, text)
      assistantHistory = await agentPromptHistory()
    } catch (cause) {
      assistantBusy = false
      assistantError = errorMessage(cause)
    }
  }

  async function adjustReadingFont(delta: number) {
    const config = await configGet()
    if (config.viewer.preview_font_size !== 0) {
      config.viewer.preview_font_size = Math.min(
        32,
        Math.max(10, config.viewer.preview_font_size + delta),
      )
    } else {
      config.typography.font_size = Math.min(
        32,
        Math.max(10, config.typography.font_size + delta),
      )
    }
    await configSet(config)
    applyConfig(config)
  }

  async function persistPanelWidth(
    kind: 'sidebar' | 'tree' | 'toc' | 'editor',
    width: number,
  ) {
    const config = await configGet()
    const pixels = Math.round(width)
    if (kind === 'sidebar') {
      config.window.sidebar_w = pixels
    } else if (kind === 'tree') {
      config.window.tree_w = pixels
    } else if (kind === 'toc') {
      config.window.toc_w = pixels
    } else {
      config.window.editor_w = pixels
    }
    await configSet(config)
  }

  function applyPanelDrag(
    kind: 'sidebar' | 'tree' | 'toc' | 'editor',
    requested: number,
  ) {
    const next = clampPanelWidth(
      kind,
      requested,
      workspaceEl?.clientWidth || 1600,
    )
    if (kind === 'sidebar') {
      sidebarWidth = next
    } else if (kind === 'tree') {
      treeWidth = next
    } else if (kind === 'toc') {
      tocWidth = next
    } else {
      editorWidth = next
    }
    return next
  }

  async function activateProject(
    project: Project,
    openRelPath?: string | null,
  ) {
    if (active?.id !== project.id) {
      tabs = []
    }
    active = project
    expandedSeed = await treeExpandedGet(project.id)
    void watchStart(project.id).catch((cause) => {
      showError(errorMessage(cause))
    })
    const focus = openRelPath ?? project.last_file
    if (focus) {
      revealRelPath = focus
      await openDocument(focus)
    } else {
      selectedNode = null
      selectedRelPaths = []
      selectedNodes = []
      revealRelPath = null
      html = ''
      docMeta = null
      openMeta = null
      docSourceMeta = null
      draftText = ''
      docMissing = false
    }
  }

  async function pickOpen(kind: 'file' | 'folder') {
    try {
      const opened = await pickAndOpen(kind)
      if (!opened) {
        return
      }
      await applyOpened(opened.project, opened.openRelPath)
    } catch (cause) {
      showError(errorMessage(cause))
    }
  }

  async function applyOpened(project: Project, openRelPath: string | null) {
    await loadProjects(project.id)
    projectsReload += 1
    await activateProject(project, openRelPath)
    error = ''
  }

  async function handleMenu(id: string) {
    try {
      if (id === 'file-open-file') {
        await pickOpen('file')
        return
      }
      if (id === 'file-open-folder') {
        await pickOpen('folder')
        return
      }
      if (id === 'file-new') {
        await createUntitled('file')
        return
      }
      if (id === 'file-new-folder') {
        await createUntitled('folder')
        return
      }
      if (id === 'file-trash') {
        requestTrash()
        return
      }
      if (id === 'go-reveal') {
        await revealSelected()
        return
      }
      if (id === 'go-external-editor') {
        await openSelectedExternal()
        return
      }
      if (id === 'view-toggle-tree') {
        treeHidden = !treeHidden
        return
      }
      if (id === 'view-toggle-projects') {
        projectsHidden = !projectsHidden
        return
      }
      if (id === 'go-switch-project') {
        switchOpen = true
        return
      }
      if (id === 'go-open-file') {
        if (!active) {
          showError('Open a folder first.')
          return
        }
        quickOpen = true
        return
      }
      if (id === 'edit-find') {
        if (!html) {
          showError('Open a document first.')
          return
        }
        if (viewMode === 'editor') {
          await setViewMode('split')
        }
        findOpen = true
        return
      }
      if (id === 'edit-find-replace') {
        showError('Find and replace arrives with the editor.')
        return
      }
      if (id === 'file-save') {
        await saveDocument()
        return
      }
      if (id === 'file-export') {
        if (!openMeta) {
          showError('Open a document first.')
          return
        }
        const articleHtml = articleEl?.innerHTML || html
        if (!articleHtml.trim()) {
          showError('There is nothing to export yet.')
          return
        }
        const themeCss =
          document.getElementById('theme-tokens')?.textContent ?? ''
        try {
          await exportDocumentPdf({
            relPath: openMeta.relPath,
            articleHtml,
            themeId: activeThemeId,
            themeCss,
            fontSize,
            lineHeight,
            measureCh,
            bodyFont,
            monoFont,
          })
        } catch (cause) {
          showError(errorMessage(cause))
        }
        return
      }
      if (id === 'view-toggle-editor') {
        await setViewMode(viewMode === 'editor' ? 'preview' : 'editor')
        return
      }
      if (id === 'view-toggle-split') {
        await setViewMode(viewMode === 'split' ? 'preview' : 'split')
        return
      }
      if (
        id === 'edit-bold' ||
        id === 'edit-italic' ||
        id === 'edit-link' ||
        id === 'edit-image' ||
        id === 'edit-inline-code' ||
        id === 'edit-list' ||
        id === 'edit-quote' ||
        id.startsWith('edit-heading-')
      ) {
        await applyEditorCommand(id)
        return
      }
      if (id === 'app-settings' || id === 'file-settings') {
        await openSettings()
        return
      }
      if (id === 'view-assistant') {
        await openAssistant()
        return
      }
      if (id === 'view-dashboard') {
        await openDashboard()
        return
      }
      if (id === 'app-about' || id === 'file-about') {
        aboutAutocheck = false
        aboutOpen = true
        return
      }
      if (id === 'app-check-updates' || id === 'file-check-updates') {
        aboutAutocheck = true
        aboutCheckSeq += 1
        aboutOpen = true
        return
      }
      if (id === 'view-toggle-theme' && appConfig) {
        const light = appConfig.appearance.theme
        const dark = appConfig.appearance.theme_dark
        const current = document.documentElement.dataset.theme
        forcedThemeId = current === dark ? light : dark
        applyTheme(appConfig)
        return
      }
      if (
        id === 'view-font-larger' ||
        id === 'view-font-smaller' ||
        id === 'view-font-reset'
      ) {
        applyConfig(await configGet())
        return
      }
      if (id === 'reading-font-larger') {
        await adjustReadingFont(1)
        return
      }
      if (id === 'reading-font-smaller') {
        await adjustReadingFont(-1)
        return
      }
      if (id === 'view-zoom-in') {
        previewZoom = nextPreviewZoom(previewZoom, 0.1)
        return
      }
      if (id === 'view-zoom-out') {
        previewZoom = nextPreviewZoom(previewZoom, -0.1)
        return
      }
      if (id === 'view-zoom-reset') {
        previewZoom = 1
        return
      }
    } catch (cause) {
      showError(errorMessage(cause))
    }
  }

  function fileNode(relPath: string): TreeNode {
    const name = relPath.split('/').filter(Boolean).at(-1) ?? relPath
    return { name, relPath, kind: 'file' }
  }

  async function createUntitled(kind: UntitledKind) {
    if (!active) {
      showError('Open a folder first.')
      return
    }
    const created = await fsCreateUntitled(
      active.id,
      targetDir(selectedNode),
      kind,
    )
    setSelection([created])
    revealRelPath = created.relPath
    treeReload += 1
    if (kind === 'file') {
      await openDocument(created.relPath)
    }
  }

  function setSelection(nodes: TreeNode[]) {
    selectedNodes = nodes
    selectedRelPaths = nodes.map((node) => node.relPath)
    selectedNode = nodes.at(-1) ?? null
  }

  function requestTrash() {
    if (!active || selectedNodes.length === 0) {
      showError('Select a file or folder first.')
      return
    }
    if (confirmDelete) {
      trashConfirm = selectedNode
      return
    }
    void trashNodes(selectedNodes)
  }

  async function trashNodes(nodes: TreeNode[]) {
    if (!active || nodes.length === 0) {
      return
    }
    try {
      const paths = nodes.map((node) => node.relPath)
      await fsTrash(active.id, paths)
      trashConfirm = null
      treeReload += 1
      if (paths.some((path) => selectedRelPaths.includes(path))) {
        setSelection([])
      }
      if (openMeta && paths.includes(openMeta.relPath)) {
        html = ''
        docMeta = null
        openMeta = null
        docMissing = true
      }
    } catch (cause) {
      showError(errorMessage(cause))
    }
  }

  async function revealSelected() {
    if (!active) {
      showError('Open a folder first.')
      return
    }
    await revealInFinder(active.id, selectedNode?.relPath ?? '')
  }

  async function openSelectedExternal() {
    if (!active || !selectedNode || selectedNode.kind === 'directory') {
      showError('Select a file first.')
      return
    }
    await openExternal(active.id, selectedNode.relPath)
  }

  async function transfer(
    mode: 'copy' | 'move',
    from: string[],
    toDir: string,
  ) {
    if (!active || (from.length === 0 && transferFrom.length === 0)) {
      return
    }
    const sources = from.length > 0 ? from : transferFrom
    if (toDir === '' && destMode === null) {
      destMode = mode
      transferFrom = sources
      showError('Choose a destination folder.')
      return
    }
    destMode = null
    const destProjectId = active.id
    const sourceProjectId = active.id
    const conflicts = await copyConflicts(destProjectId, sources, toDir)
    if (conflicts.length > 0) {
      pendingTransfer = {
        mode,
        from: sources,
        toDir,
        fromProjectId: sourceProjectId,
        toProjectId: destProjectId,
      }
      conflictNames = conflicts
      return
    }
    await finishTransfer(
      mode,
      sources,
      toDir,
      'keepBoth',
      sourceProjectId,
      destProjectId,
    )
  }

  async function finishTransfer(
    mode: 'copy' | 'move' | 'import',
    from: string[],
    toDir: string,
    conflict: ConflictStrategy,
    fromProjectId?: string,
    toProjectId?: string,
  ) {
    if (!active) {
      return
    }
    const destProjectId =
      toProjectId ?? pendingTransfer?.toProjectId ?? active.id
    const sourceProjectId =
      fromProjectId ?? pendingTransfer?.fromProjectId ?? active.id
    if (mode === 'import') {
      await fsImport(active.id, from, toDir, conflict)
    } else if (sourceProjectId !== destProjectId) {
      await fsTransfer(
        sourceProjectId,
        from,
        destProjectId,
        toDir,
        mode === 'copy',
        conflict,
      )
      if (mode === 'move' && sourceProjectId === active.id) {
        const openPath = openMeta?.relPath
        if (
          openPath &&
          from.some(
            (path) => openPath === path || openPath.startsWith(`${path}/`),
          )
        ) {
          closeTab(openPath)
        }
      }
    } else if (mode === 'copy') {
      await fsCopy(active.id, from, toDir, conflict)
    } else {
      await fsMove(active.id, from, toDir, conflict)
    }
    pendingTransfer = null
    conflictNames = []
    treeReload += 1
  }

  async function transferAcross(
    toProject: Project,
    fromProjectId: string,
    from: string[],
    copy: boolean,
  ) {
    if (from.length === 0) {
      return
    }
    if (!claimTreeDrop(toProject.id, from)) {
      return
    }
    if (fromProjectId === toProject.id) {
      destMode = copy ? 'copy' : 'move'
      await transfer(copy ? 'copy' : 'move', from, '')
      return
    }
    const mode = copy ? 'copy' : 'move'
    const conflicts = await copyConflicts(toProject.id, from, '')
    if (conflicts.length > 0) {
      pendingTransfer = {
        mode,
        from,
        toDir: '',
        fromProjectId,
        toProjectId: toProject.id,
      }
      conflictNames = conflicts
      return
    }
    await finishTransfer(
      mode,
      from,
      '',
      'keepBoth',
      fromProjectId,
      toProject.id,
    )
  }

  async function importInto(toDir: string, sources: string[]) {
    if (!active || sources.length === 0) {
      return
    }
    const conflicts = await copyConflicts(active.id, sources, toDir)
    if (conflicts.length > 0) {
      pendingTransfer = { mode: 'import', from: sources, toDir }
      conflictNames = conflicts
      return
    }
    await finishTransfer('import', sources, toDir, 'keepBoth')
  }

  async function navigate(href: string) {
    if (href.startsWith('#')) {
      articleEl
        ?.querySelector(`#${CSS.escape(href.slice(1))}`)
        ?.scrollIntoView({ block: 'start', behavior: 'smooth' })
      return
    }
    if (isHttpHref(href)) {
      await openUrl(href)
      return
    }
    const asset = parseAssetHref(href)
    if (!asset || !active || asset.projectId !== active.id) {
      return
    }
    if (isMarkdownPath(asset.relPath)) {
      revealRelPath = asset.relPath
      await openDocument(asset.relPath)
      if (asset.hash) {
        requestAnimationFrame(() => {
          articleEl
            ?.querySelector(`#${CSS.escape(asset.hash)}`)
            ?.scrollIntoView({ block: 'start', behavior: 'smooth' })
        })
      }
    }
  }

  function applyWatch(event: FsChangedEvent) {
    if (!active || event.projectId !== active.id) {
      return
    }
    if ('rescanExpanded' in event.update) {
      watchDirs = ['', ...event.update.rescanExpanded.paths.map((path) => path)]
    } else {
      watchDirs = dirsToReload(event.update.pathsChanged.paths)
    }
    watchSeq += 1
    if (
      openMeta &&
      event.projectId === openMeta.projectId &&
      watchTouchesOpenFile(event.update, openMeta.relPath)
    ) {
      void checkExternalChange()
    }
  }

  async function checkExternalChange() {
    if (!active || !openMeta || !docMeta || externalPrompt) {
      return
    }
    const projectId = active.id
    const relPath = openMeta.relPath
    const knownHash = docMeta.hash
    try {
      const stat = await docStat(projectId, relPath)
      if (
        !openMeta ||
        openMeta.relPath !== relPath ||
        active?.id !== projectId
      ) {
        return
      }
      if (stat.hash === knownHash) {
        return
      }
      if (
        ignoredExternal?.relPath === relPath &&
        ignoredExternal.hash === stat.hash
      ) {
        return
      }
      externalPrompt = {
        relPath,
        name: tabTitle(relPath),
        dirty: isDraftDirty(editorOpened, docSourceMeta, draftText),
        missing: false,
        diskHash: stat.hash,
      }
    } catch {
      if (!openMeta || openMeta.relPath !== relPath) {
        return
      }
      externalPrompt = {
        relPath,
        name: tabTitle(relPath),
        dirty: isDraftDirty(editorOpened, docSourceMeta, draftText),
        missing: true,
        diskHash: null,
      }
    }
  }

  async function reloadFromDisk() {
    const prompt = externalPrompt
    externalPrompt = null
    if (!prompt || !active) {
      return
    }
    ignoredExternal = null
    if (prompt.missing) {
      closeTab(prompt.relPath)
      return
    }
    await openDocument(prompt.relPath, true)
  }

  function keepDiskVersion() {
    if (externalPrompt && !externalPrompt.missing && externalPrompt.diskHash) {
      ignoredExternal = {
        relPath: externalPrompt.relPath,
        hash: externalPrompt.diskHash,
      }
    }
    externalPrompt = null
  }

  async function loadProjects(preferredId?: string) {
    const listed = await projectsList({ query: null, limit: 200, offset: 0 })
    projects = listed.items
    const currentId = active?.id
    const preferred =
      (preferredId && projects.find((project) => project.id === preferredId)) ||
      (currentId && projects.find((project) => project.id === currentId)) ||
      recentProjects(projects, 1)[0]
    active = preferred ?? null
  }

  async function loadSource(relPath: string) {
    if (!active) {
      return
    }
    const loaded = await docSource(active.id, relPath)
    docSourceMeta = loaded
    draftText = loaded.text
  }

  async function setViewMode(next: ViewMode) {
    viewMode = next
    if (next !== 'preview') {
      editorOpened = true
    }
    if (next !== 'preview' && openMeta && !docSourceMeta) {
      await loadSource(openMeta.relPath)
    }
  }

  async function saveDocument() {
    if (!active || !openMeta || !docSourceMeta) {
      showError('Open a document in the editor first.')
      return
    }
    if (!docSourceMeta.writable) {
      showError(docSourceMeta.readonlyReason ?? 'This file cannot be saved.')
      return
    }
    const written = await docSave(
      active.id,
      openMeta.relPath,
      draftText,
      docMeta?.hash ?? '',
      {
        eol: docSourceMeta.eol,
        bom: docSourceMeta.bom,
        trailingNewline: docSourceMeta.trailingNewline,
      },
    )
    if (docMeta) {
      docMeta = { ...docMeta, hash: written.hash, size: written.size }
    }
    if (viewMode !== 'editor') {
      await openDocument(openMeta.relPath)
    }
  }

  async function applyEditorCommand(id: string) {
    if (viewMode === 'preview') {
      await setViewMode('editor')
    }
    if (!openMeta) {
      showError('Open a document first.')
      return
    }
    if (!docSourceMeta) {
      await loadSource(openMeta.relPath)
    }
    const range = editorApi?.selection()
    const start = range?.start ?? draftText.length
    const end = range?.end ?? draftText.length
    const next = applyMarkdownCommand(draftText, start, end, id)
    draftText = next.text
    editorApi?.setTextAndSelection(next.text, next.start, next.end)
    requestAnimationFrame(() => {
      editorApi?.focus()
    })
  }

  async function openDocument(relPath: string, forceReload = false) {
    if (!active) {
      return
    }
    workspacePage = 'document'
    const leaving = snapshotCurrentTab()
    if (leaving && leaving.relPath !== relPath) {
      tabs = upsertTab(tabs, leaving)
      const cached = tabs.find((tab) => tab.relPath === relPath)
      if (!forceReload && cached?.docMeta) {
        restoreTab(cached)
        return
      }
    }
    if (forceReload) {
      ignoredExternal = null
      externalPrompt = null
    }
    setSelection([fileNode(relPath)])
    try {
      const opened = await docOpen(active.id, relPath)
      docMissing = false
      docMeta = opened.meta
      openMeta = {
        projectId: opened.meta.projectId,
        relPath: opened.meta.relPath,
      }
      html = opened.firstChunk ?? ''
      if (viewMode !== 'preview') {
        await loadSource(opened.meta.relPath)
      } else {
        docSourceMeta = null
        draftText = ''
      }
      const snap = snapshotCurrentTab()
      if (snap) {
        tabs = upsertTab(tabs, snap)
      }
    } catch (cause) {
      html = ''
      docMeta = null
      openMeta = null
      docSourceMeta = null
      draftText = ''
      docMissing = true
      throw cause
    }
  }

  function snapshotCurrentTab(): DocTab | null {
    if (!openMeta) {
      return null
    }
    return {
      relPath: openMeta.relPath,
      title: tabTitle(openMeta.relPath),
      html,
      docMeta,
      docSourceMeta,
      draftText,
    }
  }

  function restoreTab(tab: DocTab) {
    if (!active) {
      return
    }
    workspacePage = 'document'
    openMeta = { projectId: active.id, relPath: tab.relPath }
    html = tab.html
    docMeta = tab.docMeta
    docSourceMeta = tab.docSourceMeta
    draftText = tab.draftText
    docMissing = false
    revealRelPath = tab.relPath
    setSelection([fileNode(tab.relPath)])
  }

  function closeTab(relPath: string) {
    const next = nextAfterClose(tabs, relPath)
    tabs = removeTab(tabs, relPath)
    if (openMeta?.relPath !== relPath) {
      return
    }
    if (next) {
      void openDocument(next).catch((cause) => {
        showError(errorMessage(cause))
      })
      return
    }
    html = ''
    docMeta = null
    openMeta = null
    docSourceMeta = null
    draftText = ''
    docMissing = false
  }

  function closeWorkspacePage(page: WorkspaceTab) {
    workspaceTabs = closeWorkspaceTab(workspaceTabs, page)
    if (workspacePage === page) {
      workspacePage = workspaceTabs.at(-1) ?? 'document'
    }
  }

  async function handleDrop(
    paths: string[],
    position?: { x: number; y: number },
  ) {
    dragging = false
    finderDropRel = null
    projectDropId = null
    try {
      const dest = active
        ? dropDirAtPoint(position?.x ?? -1, position?.y ?? -1)
        : null
      if (active && dest !== null) {
        await importInto(dest, paths)
        return
      }
      const opened = await openDroppedPaths(paths)
      await applyOpened(opened.project, opened.openRelPath)
    } catch (cause) {
      showError(errorMessage(cause))
    }
  }

  function dropTreeOnProject(destId: string | null, copy: boolean): boolean {
    const drag = peekTreeDrag()
    if (!drag || !destId) {
      return false
    }
    const dest = projects.find((project) => project.id === destId)
    if (!dest || dest.available === false) {
      return false
    }
    void transferAcross(dest, drag.projectId, drag.paths, copy).catch(
      (cause) => {
        showError(errorMessage(cause))
      },
    )
    return true
  }

  function appendChunk(payload: {
    projectId: string
    relPath: string
    html: string
  }) {
    if (
      openMeta &&
      payload.projectId === openMeta.projectId &&
      payload.relPath === openMeta.relPath
    ) {
      html += payload.html
    }
  }

  onMount(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
      return
    }

    const stops: Array<() => void> = []

    void (async () => {
      try {
        const [config, css, infos, version] = await Promise.all([
          configGet(),
          themesCss(),
          themesList(),
          getAppVersion(),
        ])
        let style = document.getElementById('theme-tokens')
        if (!style) {
          style = document.createElement('style')
          style.id = 'theme-tokens'
          document.head.appendChild(style)
        }
        style.textContent = css
        themeInfos = infos
        appVersion = version
        applyConfig(config)
        viewMode = config.viewer.default_mode
        if (viewMode !== 'preview') {
          editorOpened = true
        }
        await loadProjects()
        if (active) {
          await activateProject(active)
        }
      } catch (cause) {
        showError(errorMessage(cause))
      }
    })()

    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const onScheme = () => {
      if (appConfig) {
        applyTheme(appConfig)
      }
    }
    media.addEventListener('change', onScheme)
    stops.push(() => media.removeEventListener('change', onScheme))

    void import('@tauri-apps/api/event')
      .then(({ listen }) => {
        void listen('menu://action', (event) => {
          void handleMenu(menuActionId(event.payload)).catch((cause) => {
            showError(errorMessage(cause))
          })
        }).then((unlisten) => stops.push(unlisten))

        void listen<{ projectId: string; relPath: string; html: string }>(
          'doc://chunk',
          (event) => {
            appendChunk(event.payload)
          },
        ).then((unlisten) => stops.push(unlisten))

        void listen<FsChangedEvent>('fs://changed', (event) => {
          applyWatch(event.payload)
        }).then((unlisten) => stops.push(unlisten))

        void listen<AgentClientEvent>('agent://event', (event) => {
          applyAgentEvent(event.payload)
        }).then((unlisten) => stops.push(unlisten))
      })
      .catch((cause) => {
        showError(errorMessage(cause))
      })

    void import('@tauri-apps/api/webview')
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          const position =
            'position' in event.payload ? event.payload.position : undefined
          const treeDrag = peekTreeDrag()
          switch (event.payload.type) {
            case 'enter':
            case 'over':
              if (treeDrag) {
                dragging = false
                finderDropRel = null
                projectDropId = projectIdAtPoint(
                  position?.x ?? -1,
                  position?.y ?? -1,
                )
                break
              }
              dragging = true
              projectDropId = null
              finderDropRel = active
                ? dropDirAtPoint(position?.x ?? -1, position?.y ?? -1)
                : null
              break
            case 'leave':
              dragging = false
              finderDropRel = null
              projectDropId = null
              break
            case 'drop':
              if (treeDrag) {
                dragging = false
                finderDropRel = null
                const destId =
                  projectIdAtPoint(position?.x ?? -1, position?.y ?? -1) ??
                  projectDropId
                projectDropId = null
                dropTreeOnProject(destId, peekTreeDragCopy())
                break
              }
              void handleDrop(event.payload.paths, position)
              break
            default:
              dragging = false
              finderDropRel = null
              projectDropId = null
          }
        }),
      )
      .then((unlisten) => stops.push(unlisten))
      .catch((cause) => {
        showError(errorMessage(cause))
      })

    return () => {
      for (const stop of stops) {
        stop()
      }
    }
  })
</script>

<svelte:head>
  <title>{documentTitle}</title>
</svelte:head>

<svelte:window
  onpointermove={(event) => {
    if (!resizeStart) {
      return
    }
    applyPanelDrag(
      resizeStart.kind,
      resizeStart.width + event.clientX - resizeStart.x,
    )
  }}
  onpointerup={() => {
    if (!resizeStart) {
      return
    }
    const kind = resizeStart.kind
    const width =
      kind === 'sidebar'
        ? sidebarWidth
        : kind === 'tree'
          ? treeWidth
          : kind === 'toc'
            ? tocWidth
            : editorWidth
    resizeStart = null
    void persistPanelWidth(kind, width).catch((cause) => {
      showError(errorMessage(cause))
    })
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      destMode = null
    }
    if ((event.metaKey || event.ctrlKey) && event.key === ',') {
      event.preventDefault()
      void openSettings()
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'g') {
      event.preventDefault()
      if (!html) {
        return
      }
      if (viewMode === 'editor') {
        void setViewMode('split')
      }
      findOpen = true
    }
  }}
  ondragover={(event) => {
    event.preventDefault()
    if (peekTreeDrag()) {
      projectDropId = projectIdAtPoint(event.clientX, event.clientY)
      return
    }
    if (isExternalFileDrag(event.dataTransfer)) {
      dragging = true
    }
  }}
  ondragleave={(event) => {
    if (event.relatedTarget === null) {
      dragging = false
    }
  }}
  ondrop={(event) => {
    event.preventDefault()
    if (peekTreeDrag()) {
      dragging = false
      const destId =
        projectIdAtPoint(event.clientX, event.clientY) ?? projectDropId
      projectDropId = null
      dropTreeOnProject(destId, event.altKey)
      return
    }
    const paths = pathsFromDataTransfer(event.dataTransfer)
    if (paths.length === 0) {
      dragging = false
      return
    }
    void handleDrop(paths, { x: event.clientX, y: event.clientY })
  }}
/>

<div
  class="shell"
  class:dragging
  class:resizing={resizeStart !== null}
  style:--sidebar-w="{sidebarWidth}px"
  style:--tree-w="{treeWidth}px"
  style:--toc-w="{tocWidth}px"
  style:--editor-w="{editorWidth}px"
  style:--font-size="{fontSize}px"
  style:--line-height={lineHeight}
  style:--measure-ch={measureCh}
  style:--font-body={`"${bodyFont}", "Iowan Old Style", Palatino, serif`}
  style:--font-mono={`"${monoFont}", ui-monospace, monospace`}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header
    class="titlebar"
    onpointerdown={(event) => {
      if (event.button !== 0 || event.detail > 1) {
        return
      }
      const target = event.target
      if (
        target instanceof Element &&
        target.closest('button, input, select, textarea, a')
      ) {
        return
      }
      void startWindowDrag().catch((cause) => {
        showError(errorMessage(cause))
      })
    }}
  >
    <div class="titlebar-drag" data-tauri-drag-region></div>
    <ChromeToolbar
      mode={viewMode}
      page={workspacePage}
      hasDocument={Boolean(openMeta)}
      canSave={Boolean(openMeta && docSourceMeta?.writable)}
      canFormat={Boolean(docSourceMeta?.writable)}
      readingZoom={previewZoom}
      onmode={(mode) => {
        workspacePage = 'document'
        void setViewMode(mode).catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
      oncommand={(id) => {
        void handleMenu(id).catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
    />
  </header>
  <div class="columns">
    {#if projectsHidden}
      <aside class="projects-rail">
        <button
          type="button"
          title="Show projects (⌘1)"
          aria-label="Show projects"
          aria-expanded="false"
          onclick={() => (projectsHidden = false)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M6 3.5 10.5 8 6 12.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span>Projects</span>
        </button>
      </aside>
    {:else}
      <aside class="projects-pane" aria-label="Projects">
        <Projects
          activeId={active?.id ?? null}
          reloadSeq={projectsReload}
          externalDropId={projectDropId}
          oncollapse={() => (projectsHidden = true)}
          onopen={(project) => {
            void activateProject(project).catch((cause) => {
              showError(errorMessage(cause))
            })
          }}
          onerror={(message) => {
            showError(message)
          }}
          onadd={() => void pickOpen('folder')}
          onfilesdrop={(project, payload) => {
            void transferAcross(
              project,
              payload.projectId,
              payload.paths,
              payload.copy,
            ).catch((cause) => {
              showError(errorMessage(cause))
            })
          }}
          onremoved={() => {
            const previous = active?.id
            projectsReload += 1
            void loadProjects()
              .then(() => {
                if (!active) {
                  html = ''
                  docMeta = null
                  openMeta = null
                  return
                }
                if (active.id !== previous) {
                  return activateProject(active)
                }
              })
              .catch((cause) => {
                showError(errorMessage(cause))
              })
          }}
        />
      </aside>
      <div
        class="resize"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize projects"
        onpointerdown={(event) => {
          event.preventDefault()
          resizeStart = {
            kind: 'sidebar',
            x: event.clientX,
            width: sidebarWidth,
          }
        }}
      ></div>
    {/if}

    {#if !treeHidden}
      <aside class="tree-pane" aria-label="File tree">
        {#if active}
          <h1 class="project-name">{active.name}</h1>
        {:else if projectsHidden}
          <h1 class="project-name">1537paperstreet</h1>
        {/if}

        {#if active}
          <Tree
            project={active}
            {selectedRelPaths}
            {revealRelPath}
            initialExpanded={expandedSeed}
            {confirmDelete}
            reloadToken={treeReload}
            {destMode}
            {watchSeq}
            {watchDirs}
            externalDropRel={finderDropRel}
            onerror={(message) => {
              showError(message)
            }}
            onselect={(nodes) => {
              setSelection(nodes)
            }}
            ontrashed={(relPaths) => {
              for (const relPath of relPaths) {
                closeTab(relPath)
              }
            }}
            onopen={(relPath) => {
              void openDocument(relPath).catch((cause) => {
                showError(errorMessage(cause))
              })
            }}
            onrenamed={(from, to) => {
              tabs = retitleTab(tabs, from, to)
              if (openMeta?.relPath === from) {
                openMeta = { ...openMeta, relPath: to }
              }
            }}
            onexpanded={(paths) => {
              if (!active) {
                return
              }
              void treeExpandedSet(active.id, paths).catch((cause) => {
                showError(errorMessage(cause))
              })
            }}
            ontransfer={(mode, from, toDir) => {
              void transfer(mode, from, toDir).catch((cause) => {
                showError(errorMessage(cause))
              })
            }}
          />
        {:else}
          <p class="hint">
            Select a project, or drop a Markdown file or a folder onto the
            window.
          </p>
        {/if}
      </aside>
      <div
        class="resize"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize file tree"
        onpointerdown={(event) => {
          event.preventDefault()
          resizeStart = { kind: 'tree', x: event.clientX, width: treeWidth }
        }}
      ></div>
    {/if}

    <main>
      <DocTabs
        {tabs}
        {workspaceTabs}
        page={workspacePage}
        activeRelPath={openMeta?.relPath ?? null}
        onpage={(page) => {
          if (page === 'assistant') {
            void openAssistant().catch((cause) => {
              showError(errorMessage(cause))
            })
            return
          }
          if (page === 'dashboard') {
            void openDashboard().catch((cause) => {
              showError(errorMessage(cause))
            })
          }
        }}
        onclosepage={closeWorkspacePage}
        onselect={(relPath) => {
          void openDocument(relPath).catch((cause) => {
            showError(errorMessage(cause))
          })
        }}
        onclose={closeTab}
      />
      {#if workspacePage === 'assistant'}
        <Assistant
          servers={appConfig?.agents.servers ?? []}
          history={assistantHistory}
          projectOpen={Boolean(active)}
          busy={assistantBusy}
          models={assistantModels}
          transcript={assistantTranscript}
          permissionPrompt={assistantPermit}
          selectedServerId={assistantServerId}
          permission={assistantPermission}
          selectedModelId={assistantModelId}
          bind:composer={assistantComposer}
          error={assistantError}
          onconfigure={() => {
            void openSettings({ agents: true })
          }}
          onserver={(id) => {
            assistantServerId = id
            if (!appConfig) {
              return
            }
            appConfig.agents.default_server_id = id
            void configSet(appConfig).catch((cause) => {
              showError(errorMessage(cause))
            })
          }}
          onpermission={(value) => {
            assistantPermission = value
            if (!appConfig) {
              return
            }
            appConfig.agents.permission = value
            void configSet(appConfig).catch((cause) => {
              showError(errorMessage(cause))
            })
          }}
          onmodel={(id) => {
            assistantModelId = id
            void agentSetModel(id).catch((cause) => {
              assistantError = errorMessage(cause)
            })
          }}
          onnewchat={() => {
            void startAssistantChat()
          }}
          onsend={(text) => {
            void (async () => {
              if (!assistantTranscript) {
                const ok = await startAssistantChat()
                if (!ok) {
                  return
                }
              }
              await sendAssistantPrompt(text)
            })()
          }}
          oncancel={() => {
            void agentCancel().catch((cause) => {
              assistantError = errorMessage(cause)
            })
            assistantBusy = false
          }}
          onhistory={(text) => {
            assistantComposer = text
          }}
          onforget={(id) => {
            void (async () => {
              try {
                await agentPromptHistoryRemove(id)
                assistantHistory = await agentPromptHistory()
              } catch (cause) {
                assistantError = errorMessage(cause)
              }
            })()
          }}
          onclearhistory={() => {
            void (async () => {
              try {
                await agentPromptHistoryClear()
                assistantHistory = await agentPromptHistory()
              } catch (cause) {
                assistantError = errorMessage(cause)
              }
            })()
          }}
          onpermit={(id, optionId) => {
            assistantPermit = null
            void agentPermissionReply(id, optionId).catch((cause) => {
              assistantError = errorMessage(cause)
            })
          }}
        />
      {:else if workspacePage === 'dashboard'}
        <Dashboard
          snapshot={dashboardSnapshot}
          error={dashboardError}
          onrefresh={() => {
            void openDashboard()
          }}
          onassistant={() => {
            void openAssistant()
          }}
          onconfigure={() => {
            void openSettings({ agents: true })
          }}
        />
      {:else}
        <div
          class="workspace"
          class:split={viewMode === 'split'}
          bind:this={workspaceEl}
        >
          {#if findOpen}
            <FindBar
              root={articleEl ?? null}
              revision={html}
              onclose={() => (findOpen = false)}
            />
          {/if}
          {#if editorOpened}
            <Editor
              bind:value={draftText}
              bind:api={editorApi}
              writable={docSourceMeta?.writable ?? false}
              spellcheck={appConfig?.editor.spellcheck ?? true}
              lineNumbers={appConfig?.editor.line_numbers ?? false}
              softWrap={appConfig?.editor.soft_wrap ?? true}
              indentUnit={appConfig?.editor.indent_unit ?? 2}
              hidden={viewMode === 'preview'}
            />
          {/if}
          {#if viewMode === 'split'}
            <div
              class="resize"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize editor"
              onpointerdown={(event) => {
                event.preventDefault()
                resizeStart = {
                  kind: 'editor',
                  x: event.clientX,
                  width: editorWidth,
                }
              }}
            ></div>
          {/if}
          {#if viewMode !== 'editor'}
            <Preview
              {html}
              {emptyMessage}
              toc={showToc ? (docMeta?.toc ?? []) : []}
              {tocWidth}
              banner={docMeta?.readonlyReason ?? null}
              themeId={activeThemeId}
              mermaidEnabled={appConfig?.viewer.mermaid_enabled ?? true}
              mathEnabled={appConfig?.viewer.math_enabled ?? true}
              previewFont={appConfig?.viewer.preview_font ?? ''}
              previewFontSize={appConfig?.viewer.preview_font_size ?? 0}
              previewBg={appConfig?.viewer.preview_bg ?? ''}
              previewFg={appConfig?.viewer.preview_fg ?? ''}
              readingZoom={previewZoom}
              bind:articleEl
              onnavigate={(href) => {
                void navigate(href).catch((cause) => {
                  showError(errorMessage(cause))
                })
              }}
              onerror={(message) => {
                showError(message)
              }}
              ontocresize={(event) => {
                event.preventDefault()
                resizeStart = { kind: 'toc', x: event.clientX, width: tocWidth }
              }}
            />
          {/if}
        </div>
      {/if}
    </main>
  </div>

  {#if trashConfirm}
    <div class="confirm" role="dialog" aria-labelledby="app-trash-title">
      <div class="confirm-card">
        <p id="app-trash-title">
          {#if selectedNodes.length > 1}
            Move {selectedNodes.length} items to Trash?
          {:else}
            Move “{trashConfirm.name}” to Trash?
          {/if}
        </p>
        <div class="confirm-actions">
          <button type="button" onclick={() => (trashConfirm = null)}
            >Cancel</button
          >
          <button
            type="button"
            class="danger"
            onclick={() => {
              if (selectedNodes.length > 0) {
                void trashNodes(selectedNodes)
              } else if (trashConfirm) {
                void trashNodes([trashConfirm])
              }
            }}>Move to Trash</button
          >
        </div>
      </div>
    </div>
  {/if}

  {#if switchOpen}
    <QuickSwitch
      onopen={(project) => {
        void activateProject(project).catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
      onclose={() => (switchOpen = false)}
      onerror={(message) => {
        showError(message)
      }}
    />
  {/if}

  {#if quickOpen && active}
    <QuickOpen
      projectId={active.id}
      onopen={(relPath) => {
        revealRelPath = relPath
        void openDocument(relPath).catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
      onclose={() => (quickOpen = false)}
      onerror={(message) => {
        showError(message)
      }}
    />
  {/if}

  {#if settingsOpen}
    {#if appConfig}
      <Settings
        config={appConfig}
        themes={themeInfos}
        onsave={(next) => {
          void configSet(next)
            .then(() => {
              applyConfig(next)
              settingsOpen = false
              treeReload += 1
            })
            .catch((cause) => {
              showError(errorMessage(cause))
            })
        }}
        onclose={() => (settingsOpen = false)}
        onlive={(next) => {
          void configSet(next)
            .then(() => {
              applyConfig(next)
            })
            .catch((cause) => {
              showError(errorMessage(cause))
            })
        }}
        focusAgents={settingsFocusAgents}
      />
    {:else}
      <div class="settings-loading" role="status">Loading settings…</div>
    {/if}
  {/if}

  {#if aboutOpen}
    {#key aboutCheckSeq}
      <About
        version={appVersion}
        autocheck={aboutAutocheck}
        onclose={() => {
          aboutOpen = false
          aboutAutocheck = false
        }}
        onopen={(url) => {
          void openUrl(url).catch((cause) => {
            showError(errorMessage(cause))
          })
        }}
        oncheck={() => updatesCheck()}
      />
    {/key}
  {/if}

  {#if conflictNames.length > 0 && pendingTransfer}
    <Conflict
      names={conflictNames}
      onchoose={(strategy) => {
        const pending = pendingTransfer
        if (!pending) {
          return
        }
        void finishTransfer(
          pending.mode,
          pending.from,
          pending.toDir,
          strategy,
          pending.fromProjectId,
          pending.toProjectId,
        ).catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
      oncancel={() => {
        pendingTransfer = null
        conflictNames = []
      }}
    />
  {/if}

  {#if externalPrompt}
    <ReloadDisk
      name={externalPrompt.name}
      dirty={externalPrompt.dirty}
      missing={externalPrompt.missing}
      onreload={() => {
        void reloadFromDisk().catch((cause) => {
          showError(errorMessage(cause))
        })
      }}
      onkeep={keepDiskVersion}
    />
  {/if}

  {#if dragging && finderDropRel === null}
    <div class="drop-overlay" role="status">
      Drop a Markdown file or a folder to open it
    </div>
  {/if}

  <MessageToasts
    toast={error}
    history={messageHistory}
    open={historyOpen}
    ondismiss={() => (error = '')}
    ontoggle={() => (historyOpen = !historyOpen)}
    onclose={() => (historyOpen = false)}
  />
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }

  .titlebar {
    position: relative;
    height: var(--titlebar-h);
    min-height: var(--titlebar-h);
    flex: none;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding-inline: var(--traffic-pad) var(--space-3);
    background: color-mix(in srgb, var(--sidebar) 78%, var(--bg));
    border-block-end: 1px solid var(--border);
    z-index: 2;
  }

  .titlebar-drag {
    position: absolute;
    inset: 0;
    -webkit-app-region: drag;
  }

  .columns {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .projects-pane,
  .tree-pane {
    display: flex;
    flex-direction: column;
    min-width: 160px;
    max-width: 480px;
    flex: none;
    overflow: hidden;
    background: color-mix(in srgb, var(--sidebar) 78%, transparent);
  }

  .projects-pane {
    width: var(--sidebar-w);
    -webkit-app-region: no-drag;
  }

  .projects-rail {
    display: flex;
    flex: none;
    width: 36px;
    min-width: 36px;
    overflow: hidden;
    background: color-mix(in srgb, var(--sidebar) 78%, transparent);
    border-inline-end: 1px solid var(--border);
  }

  .projects-rail button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 44px;
    padding-block: var(--space-3);
    color: var(--fg-muted);
    transition-property: background-color, color, transform;
    transition-duration: var(--duration);
  }

  .projects-rail button:hover {
    color: var(--fg);
    background: var(--selection);
  }

  .projects-rail button:active {
    transform: scale(0.96);
  }

  @media (prefers-reduced-motion: reduce) {
    .projects-rail button:active {
      transform: none;
    }
  }

  .projects-rail svg {
    width: 14px;
    height: 14px;
    flex: none;
  }

  .projects-rail span {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .tree-pane {
    width: var(--tree-w);
  }

  .projects-pane :global(button),
  .projects-pane :global(input),
  .tree-pane :global(button),
  .resize {
    -webkit-app-region: no-drag;
  }

  .resize {
    width: var(--space-1);
    flex: none;
    cursor: col-resize;
    background: var(--border);
  }

  .resize:hover,
  .shell.resizing .resize {
    background: var(--accent);
  }

  .shell.resizing {
    cursor: col-resize;
    user-select: none;
  }

  .project-name,
  .hint {
    margin: 0;
    padding: var(--space-3);
    font-size: 0.8125rem;
    font-weight: 600;
  }

  .hint {
    font-weight: 400;
    color: var(--fg-muted);
  }

  main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    background: var(--bg);
  }

  main > :global([role='tabpanel']) {
    flex: 1;
    min-height: 0;
  }

  .workspace {
    display: flex;
    flex: 1;
    min-height: 0;
    min-width: 0;
    flex-direction: column;
  }

  .workspace.split {
    flex-direction: row;
  }

  .workspace.split :global(.editor) {
    flex: none;
    width: var(--editor-w);
    min-width: 0;
  }

  .workspace.split :global(.pane) {
    flex: 1;
    min-width: 0;
  }

  .settings-loading {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding-top: var(--titlebar-h);
    background: var(--bg);
    color: var(--fg-muted);
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

  .confirm-actions button:hover {
    background: var(--selection);
  }

  .danger {
    color: var(--accent);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--selection) 80%, transparent);
    border: 2px dashed var(--accent);
    color: var(--fg);
    font-weight: 600;
    pointer-events: none;
  }
</style>
