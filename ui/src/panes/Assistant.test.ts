import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import Assistant from './Assistant.svelte'

describe('Assistant', () => {
  it('points an empty install at Settings', () => {
    const { body } = render(Assistant, {
      props: {
        servers: [],
        history: [],
        projectOpen: true,
        selectedServerId: '',
        permission: 'allowance',
        onconfigure() {},
        onclose() {},
        onserver() {},
        onpermission() {},
        onmodel() {},
        onnewchat() {},
        onsend() {},
        oncancel() {},
        onhistory() {},
        onpermit() {},
      },
    })
    expect(body).toContain('aria-label="Assistant"')
    expect(body).toContain('Configure')
    expect(body).toContain('No agents yet')
  })

  it('shows model, permissions, history, and composer when an agent exists', () => {
    const { body } = render(Assistant, {
      props: {
        servers: [
          {
            id: '01TEST',
            name: 'OpenCode',
            preset: 'opencode',
            command: 'opencode',
            args: ['acp'],
            env: [],
            enabled: true,
          },
        ],
        history: [
          {
            id: '01HIST',
            ts: '2026-08-23T12:00:00Z',
            text: 'Summarize FLOW.md',
            server_id: '01TEST',
          },
        ],
        projectOpen: true,
        models: [{ id: 'kimi', name: 'Kimi' }],
        selectedServerId: '01TEST',
        permission: 'allowance',
        selectedModelId: 'kimi',
        onconfigure() {},
        onclose() {},
        onserver() {},
        onpermission() {},
        onmodel() {},
        onnewchat() {},
        onsend() {},
        oncancel() {},
        onhistory() {},
        onpermit() {},
      },
    })
    expect(body).toContain('OpenCode')
    expect(body).toContain('Model')
    expect(body).toContain('Kimi')
    expect(body).toContain('Allowance')
    expect(body).toContain('Summarize FLOW.md')
    expect(body).toContain('New chat')
    expect(body).toContain('Send')
  })
})
