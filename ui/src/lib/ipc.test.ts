import { afterEach, describe, expect, it } from 'vitest'

import {
  docOpen,
  docSave,
  docSource,
  docStat,
  errorMessage,
  fsTrash,
  mermaidCacheGet,
  mermaidCachePut,
  saveUserFile,
  exportPdf,
  themesCss,
  themesList,
  treeExpandedGet,
  treeReadDir,
  watchStart,
} from './ipc'
import { mockIpc, resetIpc } from './ipc.mock'

afterEach(() => {
  resetIpc()
})

describe('ipc helpers', () => {
  it('turns failures into a user-visible string', () => {
    expect(errorMessage('disk full')).toBe('disk full')
    expect(errorMessage(new Error('not found'))).toBe('not found')
    expect(errorMessage({ message: "Couldn't reach GitHub to check for updates." })).toBe(
      "Couldn't reach GitHub to check for updates.",
    )
    expect(errorMessage(12)).toBe('12')
  })

  it('forwards mermaid cache and theme commands', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = []
    mockIpc({
      mermaid_cache_get: (args) => {
        calls.push({ cmd: 'mermaid_cache_get', args })
        return '<svg></svg>'
      },
      mermaid_cache_put: (args) => {
        calls.push({ cmd: 'mermaid_cache_put', args })
      },
      save_user_file: (args) => {
        calls.push({ cmd: 'save_user_file', args })
      },
      export_pdf: (args) => {
        calls.push({ cmd: 'export_pdf', args })
      },
      themes_list: () => [{ id: 'paper-light', name: 'Paper Light' }],
      themes_css: () => "[data-theme='paper-light']{--bg:#fff;}",
    })

    await expect(mermaidCacheGet('abc', 'paper-light')).resolves.toBe(
      '<svg></svg>',
    )
    await mermaidCachePut('abc', 'paper-light', '<svg></svg>')
    await saveUserFile('/tmp/diagram.png', [1, 2])
    await exportPdf('/tmp/note.pdf', '<html></html>')
    await expect(themesList()).resolves.toEqual([
      { id: 'paper-light', name: 'Paper Light' },
    ])
    await expect(themesCss()).resolves.toContain('data-theme')

    expect(calls[0]).toEqual({
      cmd: 'mermaid_cache_get',
      args: { source_hash: 'abc', theme_id: 'paper-light' },
    })
    expect(calls[1]?.args).toMatchObject({
      source_hash: 'abc',
      theme_id: 'paper-light',
      svg: '<svg></svg>',
    })
    expect(calls[2]?.args).toEqual({
      path: '/tmp/diagram.png',
      bytes: [1, 2],
    })
    expect(calls[3]?.args).toEqual({
      path: '/tmp/note.pdf',
      html: '<html></html>',
    })
  })

  it('sends tree and document command arguments in snake_case', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = []
    mockIpc({
      tree_read_dir: (args) => {
        calls.push({ cmd: 'tree_read_dir', args })
        return [{ name: 'note.md', relPath: 'note.md', kind: 'file' }]
      },
      tree_expanded_get: (args) => {
        calls.push({ cmd: 'tree_expanded_get', args })
        return []
      },
      doc_open: (args) => {
        calls.push({ cmd: 'doc_open', args })
        return { firstChunk: '', meta: null }
      },
      doc_source: (args) => {
        calls.push({ cmd: 'doc_source', args })
        return { text: '', eol: 'lf', bom: false, trailingNewline: true }
      },
      doc_stat: (args) => {
        calls.push({ cmd: 'doc_stat', args })
        return { hash: 'abc', size: 1 }
      },
      doc_save: (args) => {
        calls.push({ cmd: 'doc_save', args })
        return { hash: 'abc', size: 1, skipped: true }
      },
      watch_start: (args) => {
        calls.push({ cmd: 'watch_start', args })
      },
      fs_trash: (args) => {
        calls.push({ cmd: 'fs_trash', args })
      },
    })

    await treeReadDir('proj', '')
    await treeExpandedGet('proj')
    await docOpen('proj', 'note.md')
    await docSource('proj', 'note.md')
    await docStat('proj', 'note.md')
    await docSave('proj', 'note.md', 'hi\n', 'hash', {
      eol: 'lf',
      bom: false,
      trailingNewline: true,
    })
    await watchStart('proj')
    await fsTrash('proj', ['note.md'])

    expect(calls[0]).toEqual({
      cmd: 'tree_read_dir',
      args: { project_id: 'proj', rel_path: '' },
    })
    expect(calls[1]?.args).toEqual({ project_id: 'proj' })
    expect(calls[2]?.args).toEqual({ project_id: 'proj', rel_path: 'note.md' })
    expect(calls[3]?.args).toEqual({ project_id: 'proj', rel_path: 'note.md' })
    expect(calls[4]?.args).toEqual({ project_id: 'proj', rel_path: 'note.md' })
    expect(calls[5]?.args).toEqual({
      project_id: 'proj',
      rel_path: 'note.md',
      text: 'hi\n',
      base_hash: 'hash',
      traits: { eol: 'lf', bom: false, trailingNewline: true },
    })
    expect(calls[6]?.args).toEqual({ project_id: 'proj' })
    expect(calls[7]?.args).toEqual({
      project_id: 'proj',
      rel_paths: ['note.md'],
    })
  })
})
