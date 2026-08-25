# Assistant

The Assistant talks to **local** coding agents through the
[Agent Client Protocol](https://agentclientprotocol.com). It lives in its
**own tab** in the workspace strip (next to Dashboard). Open it with
**⌘⌥A**, View → Assistant, the Assistant tab, or the title-bar control.

The **Dashboard** tab (View → Dashboard) shows local library and agent
counts: projects, Markdown files in the open folder, configured agents, the
live session, and recent prompts. Nothing on that page is sent off the Mac.

This app does not call a model API and does not install agents from a
registry. You add a CLI that is already on your Mac.

## Settings

Settings → External Agents:

- **Add OpenCode / Claude / Codex** when that binary is on `PATH`
  (`opencode acp`, `claude --acp`, `codex acp` or `codex-acp`).
- **Add Custom Agent** — name, command, arguments (no shell).
- Remove a row with Remove.

Full permission can let the CLI change files in the open project. Allowance
asks each time. Plan prefers a plan/ask mode and rejects write-like tools.

## Chat

Configure opens Settings. Pick an agent, model (if the agent lists models),
and a permission. History lists recent **user** prompts stored in
`~/.1537paperstreet/agents/prompts.json`. New chat starts a fresh ACP session
in the current project folder.

The agent process may use the network. The reader still does not, except
Check for Updates.
