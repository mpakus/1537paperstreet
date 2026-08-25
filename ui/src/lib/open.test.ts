import { describe, expect, it } from 'vitest'

import type { Project } from './generated/core'
import {
  isExternalFileDrag,
  pathsFromDataTransfer,
  recentProjects,
} from './open'

function project(
  partial: Partial<Project> & Pick<Project, 'id' | 'name'>,
): Project {
  return {
    path: `/tmp/${partial.name}`,
    added_at: '2026-01-01T00:00:00Z',
    last_opened_at: null,
    pinned: false,
    accent: null,
    last_file: null,
    ...partial,
  }
}

describe('recentProjects', () => {
  it('orders by last opened time and caps the list', () => {
    const older = project({
      id: 'a',
      name: 'Alpha',
      last_opened_at: '2026-01-01T00:00:00Z',
    })
    const newer = project({
      id: 'b',
      name: 'Beta',
      last_opened_at: '2026-08-01T00:00:00Z',
    })
    const never = project({ id: 'c', name: 'Gamma' })

    expect(
      recentProjects([older, never, newer], 2).map((item) => item.id),
    ).toEqual(['b', 'a'])
  })
})

describe('pathsFromDataTransfer', () => {
  it('returns an empty list when the transfer has no files', () => {
    expect(pathsFromDataTransfer(null)).toEqual([])
  })

  it('reads file:// URIs from a uri-list', () => {
    const transfer = {
      files: [],
      getData(type: string) {
        return type === 'text/uri-list'
          ? 'file:///Users/me/Notes/hello.md\n# comment\nfile:///Users/me/Notes'
          : ''
      },
    } as unknown as DataTransfer

    expect(pathsFromDataTransfer(transfer)).toEqual([
      '/Users/me/Notes/hello.md',
      '/Users/me/Notes',
    ])
  })
})

describe('isExternalFileDrag', () => {
  it('ignores in-app text drags and empty transfers', () => {
    expect(isExternalFileDrag(null)).toBe(false)
    const internal = {
      types: ['text/plain'],
    } as unknown as DataTransfer
    expect(isExternalFileDrag(internal)).toBe(false)
  })

  it('recognizes Finder file lists', () => {
    const files = { types: ['Files'] } as unknown as DataTransfer
    expect(isExternalFileDrag(files)).toBe(true)
  })
})
