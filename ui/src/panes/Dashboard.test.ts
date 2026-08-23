import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'

import type { DashboardSnapshot } from '../lib/ipc'
import Dashboard from './Dashboard.svelte'

const snapshot: DashboardSnapshot = {
  title: 'Dashboard',
  lede: 'Local counts only. Nothing here is sent off this Mac.',
  sections: [
    {
      title: 'Library',
      metrics: [{ label: 'Projects', value: '2' }],
      rows: [{ title: 'Notes', detail: '2026-08-23 12:00' }],
    },
    {
      title: 'Agents',
      metrics: [{ label: 'Configured', value: '1' }],
      rows: [{ title: 'OpenCode', detail: 'OpenCode · on' }],
    },
  ],
}

describe('Dashboard', () => {
  it('renders formatted sections from Rust', () => {
    const { body } = render(Dashboard, {
      props: {
        snapshot,
        onrefresh() {},
        onassistant() {},
        onconfigure() {},
      },
    })
    expect(body).toContain('aria-label="Dashboard"')
    expect(body).toContain('Local counts only')
    expect(body).toContain('Projects')
    expect(body).toContain('OpenCode')
    expect(body).toContain('Configure agents')
  })
})
