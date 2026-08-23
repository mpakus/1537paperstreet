# ADR-010: External agents via ACP, not a built-in LLM

- Status: accepted
- Date: 2026-08-23

## Context

PLAN § 18 listed plugins, cloud accounts, and extra network as out of v1. The
product now needs an Assistant that talks to **local** coding agents the way
Zed does: the [Agent Client Protocol](https://agentclientprotocol.com) over
stdio. The reader must still not call model APIs or the ACP Registry. Agent
filesystem methods (`fs/read`, `fs/write`) and terminals must stay off until a
later task; the subprocess still uses the open project as `cwd`.

## Decision

- Store `agents` in `config.json` with serde defaults and **no**
  `schema_version` bump.
- Discover OpenCode, Claude, and Codex only if their binaries are already on
  `PATH`. Never run `npx -y` or fetch the ACP Registry.
- Host ACP in `ps-app` as newline-delimited JSON-RPC. Do not implement client
  `fs` or terminal capabilities.
- Default permission is **allowance** (ask). Full auto-allows tool requests;
  Plan prefers a plan/ask session mode and rejects write-like kinds.
- Prompt history is `~/.1537paperstreet/agents/prompts.json` (user prompts
  only). Logs record spawn/exit, never prompt text or env values.

## Alternatives

- Official `agent-client-protocol` crate: full `Client` trait includes
  filesystem and terminal hooks we must not advertise in this slice.
- In-app OpenAI/Anthropic keys: the app would send documents itself, against
  the privacy boundary.
- ACP Registry install: a new network call from the app.

## Consequences

Billing, auth, and model catalogs stay with the CLI. Full permission can still
let that CLI change files in the project. Settings must say so. Check for
Updates remains the only network call the **app** makes.
