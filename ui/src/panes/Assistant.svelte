<script lang="ts">
  import type {
    AgentChoice,
    AgentPermission,
    AgentServer,
    PromptHistoryEntry,
  } from '../lib/ipc'

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
    onpermit: (id: number, optionId: string) => void
  } = $props()
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
        <h3>History</h3>
        <ul>
          {#each history as entry (entry.id)}
            <li>
              <button type="button" onclick={() => onhistory(entry.text)}>
                {entry.text}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
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
    <form
      class="composer"
      onsubmit={(event) => {
        event.preventDefault()
        const text = composer.trim()
        if (!text || busy) {
          return
        }
        onsend(text)
        composer = ''
      }}
    >
      <textarea
        rows="4"
        bind:value={composer}
        placeholder="Ask the agent…"
        disabled={!projectOpen || !selectedServerId}></textarea>
      <div class="actions">
        <button type="button" onclick={onnewchat}>New chat</button>
        {#if busy}
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
  {/if}
</div>

<style>
  .pane {
    display: grid;
    grid-template-rows: auto auto auto 1fr auto auto auto;
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: var(--space-5);
    color: var(--fg);
    background: var(--bg);
  }

  header {
    display: flex;
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
    min-height: 28px;
    padding: 0 var(--space-3);
    font-size: 0.75rem;
    font-weight: 600;
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

  .history button {
    width: 100%;
    text-align: start;
    font-weight: 400;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chat {
    min-height: 12rem;
    overflow: auto;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    white-space: pre-wrap;
    font-size: 0.875rem;
  }

  .permit {
    padding: var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
  }

  .composer {
    display: grid;
    gap: var(--space-2);
  }

  button:disabled {
    opacity: 0.6;
  }
</style>
