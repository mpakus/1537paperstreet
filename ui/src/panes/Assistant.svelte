<script lang="ts">
  import { isComposerSubmitKey } from '../lib/keys'
  import type {
    AgentChoice,
    AgentPermission,
    AgentServer,
    PromptHistoryEntry,
  } from '../lib/ipc'

  const COMPOSER_MIN = 72
  const COMPOSER_DEFAULT = 140

  let {
    servers,
    history,
    projectOpen,
    busy = false,
    models = [],
    transcript = '',
    permissionPrompt = null,
    selectedServerId,
    permission,
    selectedModelId = '',
    composer = $bindable(''),
    error = '',
    onconfigure,
    onserver,
    onpermission,
    onmodel,
    onnewchat,
    onsend,
    oncancel,
    onhistory,
    onforget,
    onclearhistory,
    onpermit,
  }: {
    servers: AgentServer[]
    history: PromptHistoryEntry[]
    projectOpen: boolean
    busy?: boolean
    models?: AgentChoice[]
    transcript?: string
    permissionPrompt?: {
      id: number
      title: string
      options: AgentChoice[]
    } | null
    selectedServerId: string
    permission: AgentPermission
    selectedModelId?: string
    composer?: string
    error?: string
    onconfigure: () => void
    onserver: (id: string) => void
    onpermission: (value: AgentPermission) => void
    onmodel: (id: string) => void
    onnewchat: () => void
    onsend: (text: string) => void
    oncancel: () => void
    onhistory: (text: string) => void
    onforget: (id: string) => void
    onclearhistory: () => void
    onpermit: (id: number, optionId: string) => void
  } = $props()

  let composerHeight = $state(COMPOSER_DEFAULT)
  let stageEl = $state<HTMLDivElement | undefined>(undefined)
  let splitDrag = $state<{ y: number; height: number } | null>(null)

  function clampComposer(next: number): number {
    const stage = stageEl?.clientHeight ?? 480
    const max = Math.max(COMPOSER_MIN, Math.floor(stage * 0.55))
    return Math.min(max, Math.max(COMPOSER_MIN, Math.round(next)))
  }

  function submitComposer(event: Event) {
    event.preventDefault()
    const text = composer.trim()
    if (!text || busy) {
      return
    }
    onsend(text)
    composer = ''
  }
</script>

<div class="pane" role="tabpanel" aria-label="Assistant">
  <header>
    <div>
      <h2>Assistant</h2>
      <p class="lede">
        Talk to a local agent over ACP in this tab. Configure adds or edits
        agents in Settings. Default permission asks before tools.
      </p>
    </div>
    <button type="button" onclick={onconfigure}>Configure</button>
  </header>
  {#if servers.length === 0}
    <p class="empty">
      No agents yet. Configure opens Settings so you can add OpenCode, Claude,
      Codex, or a custom command.
    </p>
  {:else}
    <div class="controls">
      <label>
        Agent
        <select
          value={selectedServerId}
          onchange={(event) => onserver(event.currentTarget.value)}
        >
          {#each servers.filter((server) => server.enabled) as server (server.id)}
            <option value={server.id}>{server.name}</option>
          {/each}
        </select>
      </label>
      {#if models.length > 0}
        <label>
          Model
          <select
            value={selectedModelId}
            onchange={(event) => onmodel(event.currentTarget.value)}
          >
            {#each models as model (model.id)}
              <option value={model.id}>{model.name}</option>
            {/each}
          </select>
        </label>
      {/if}
      <label>
        Permissions
        <select
          value={permission}
          onchange={(event) =>
            onpermission(event.currentTarget.value as AgentPermission)}
        >
          <option value="allowance">Allowance — ask each time</option>
          <option value="plan">Plan — no writes</option>
          <option value="full">Full — auto-allow tools</option>
        </select>
      </label>
    </div>
    {#if history.length > 0}
      <section class="history" aria-label="Prompt history">
        <div class="history-head">
          <h3>History</h3>
          <button type="button" onclick={onclearhistory}>Clear History</button>
        </div>
        <ul>
          {#each history as entry (entry.id)}
            <li>
              <button
                type="button"
                class="recall"
                onclick={() => onhistory(entry.text)}
              >
                {entry.text}
              </button>
              <button
                type="button"
                class="forget"
                aria-label="Remove prompt"
                onclick={() => onforget(entry.id)}>×</button
              >
            </li>
          {/each}
        </ul>
      </section>
    {/if}
    <div class="stage" bind:this={stageEl}>
      <div class="chat" aria-live="polite">{transcript}</div>
      {#if permissionPrompt}
        <div
          class="permit"
          role="alertdialog"
          tabindex="-1"
          aria-label="Tool permission"
        >
          <p>{permissionPrompt.title}</p>
          <div class="actions">
            {#each permissionPrompt.options as option (option.id)}
              <button
                type="button"
                onclick={() => onpermit(permissionPrompt.id, option.id)}
                >{option.name}</button
              >
            {/each}
          </div>
        </div>
      {/if}
      {#if error}
        <p class="status" role="status">{error}</p>
      {/if}
      {#if !projectOpen}
        <p class="status">Open a folder before starting a chat.</p>
      {/if}
      <div
        class="split"
        class:dragging={splitDrag !== null}
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize prompt"
        onpointerdown={(event) => {
          event.preventDefault()
          splitDrag = { y: event.clientY, height: composerHeight }
          event.currentTarget.setPointerCapture(event.pointerId)
        }}
        onpointermove={(event) => {
          if (!splitDrag) {
            return
          }
          composerHeight = clampComposer(
            splitDrag.height - (event.clientY - splitDrag.y),
          )
        }}
        onpointerup={() => {
          splitDrag = null
        }}
        onpointercancel={() => {
          splitDrag = null
        }}
      ></div>
      <form class="composer" onsubmit={submitComposer}>
        <textarea
          bind:value={composer}
          style:height="{composerHeight}px"
          placeholder="Ask the agent…"
          title="Shift+Enter to send"
          disabled={!projectOpen || !selectedServerId}
          onkeydown={(event) => {
            if (!isComposerSubmitKey(event)) {
              return
            }
            event.preventDefault()
            event.currentTarget.form?.requestSubmit()
          }}></textarea>
        <div class="actions">
          <button type="button" onclick={onnewchat}>New chat</button>
          {#if busy}
            <span
              class="busy"
              role="status"
              aria-live="polite"
              aria-label="Working"
            >
              <span class="busy-ring" aria-hidden="true"></span>
            </span>
            <button type="button" onclick={oncancel}>Stop</button>
          {:else}
            <button
              type="submit"
              disabled={!projectOpen || !selectedServerId || !composer.trim()}
              >Send</button
            >
          {/if}
        </div>
      </form>
    </div>
  {/if}
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: var(--space-5);
    color: var(--fg);
    background: var(--bg);
  }

  header {
    display: flex;
    flex: none;
    align-items: start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  h2,
  h3 {
    margin: 0;
  }

  h3 {
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }

  .lede,
  .empty,
  .status {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.8125rem;
    line-height: 1.45;
  }

  .lede {
    max-width: 40rem;
    margin-top: var(--space-2);
  }

  .controls,
  .actions {
    display: flex;
    flex-wrap: wrap;
    flex: none;
    align-items: center;
    gap: var(--space-2);
  }

  label {
    display: grid;
    gap: var(--space-1);
    font-size: 0.8125rem;
  }

  select,
  textarea,
  button {
    font: inherit;
    color: var(--fg);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  select,
  textarea {
    padding: var(--space-1) var(--space-2);
  }

  button {
    flex: none;
    height: 28px;
    min-height: 28px;
    padding: 0 var(--space-3);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .history {
    flex: none;
    display: grid;
    gap: var(--space-2);
  }

  .history-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .history ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-1);
    max-height: 8rem;
    overflow: auto;
  }

  .history li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .history .recall {
    flex: 1;
    min-width: 0;
    text-align: start;
    font-weight: 400;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .history .forget {
    width: 28px;
    padding: 0;
    color: var(--fg-muted);
    font-size: 1rem;
    line-height: 1;
  }

  .history .forget:hover {
    color: var(--fg);
  }

  .stage {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }

  .chat {
    flex: 1;
    min-height: 4rem;
    overflow: auto;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    white-space: pre-wrap;
    font-size: 0.875rem;
  }

  .split {
    flex: none;
    height: var(--space-1);
    margin: var(--space-2) 0;
    cursor: row-resize;
    touch-action: none;
    background: var(--border);
  }

  .split:hover,
  .split.dragging {
    background: var(--accent);
  }

  .permit {
    padding: var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
  }

  .composer {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: var(--space-2);
  }

  textarea {
    flex: none;
    width: 100%;
    min-height: 4.5rem;
    resize: none;
    box-sizing: border-box;
  }

  button:disabled {
    opacity: 0.6;
  }

  .busy {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
  }

  .busy-ring {
    width: 14px;
    height: 14px;
    box-sizing: border-box;
    border: 2px solid color-mix(in srgb, var(--accent) 28%, var(--border));
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: assistant-busy 0.8s linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .busy-ring {
      animation: none;
      border-color: var(--accent);
    }
  }

  @keyframes assistant-busy {
    to {
      transform: rotate(360deg);
    }
  }
</style>
