import { describe, expect, it } from 'vitest'

import type { TreeNode } from './generated/core'
import {
  ancestorDirs,
  fileIconKind,
  flattenTree,
  isMarkdownPath,
  joinRel,
  parentDir,
  sortDirsByDepth,
  targetDir,
  visibleWindow,
  parseAssetHref,
  isHttpHref,
  dirsToReload,
  rangeRelPaths,
  dropDirAtPoint,
  encodeTreeDrag,
  decodeTreeDrag,
  beginTreeDrag,
  peekTreeDrag,
  peekTreeDragCopy,
  setTreeDragCopy,
  clearTreeDrag,
  claimTreeDrop,
  isTreeDrag,
  resolveTreeDrag,
  dataTransferHasType,
  projectIdAtPoint,
  canDropInto,
  acceptTreeDrop,
  logicalDragPoint,
  clipboardNames,
  clipboardPaths,
  dragGhostPreview,
  watchTouchesOpenFile,
  isDraftDirty,
  TREE_DRAG_PREFIX,
} from './tree'

function node(relPath: string, kind: TreeNode['kind']): TreeNode {
  const name = relPath.split('/').at(-1) ?? relPath
  return { name, relPath, kind }
}

describe('tree helpers', () => {
  it('recognizes Markdown file names', () => {
    expect(isMarkdownPath('readme.md')).toBe(true)
    expect(isMarkdownPath('Note.MARKDOWN')).toBe(true)
    expect(isMarkdownPath('cover.png')).toBe(false)
  })

  it('picks distinct icons for folders, Markdown, and other files', () => {
    expect(fileIconKind(node('notes', 'directory'))).toBe('directory')
    expect(fileIconKind(node('readme.md', 'file'))).toBe('markdown')
    expect(fileIconKind(node('photo.png', 'file'))).toBe('file')
    expect(fileIconKind(node('link', 'symlink'))).toBe('file')
  })

  it('lists ancestor directories for a nested file', () => {
    expect(ancestorDirs('01.md')).toEqual([])
    expect(ancestorDirs('chapters/01.md')).toEqual(['chapters'])
    expect(ancestorDirs('a/b/c.md')).toEqual(['a', 'a/b'])
  })

  it('sorts directories from the root downward', () => {
    expect(sortDirsByDepth(['a/b', 'a', 'z'])).toEqual(['a', 'z', 'a/b'])
  })

  it('resolves parent and creation directories', () => {
    expect(parentDir('chapters/01.md')).toBe('chapters')
    expect(parentDir('readme.md')).toBe('')
    expect(joinRel('chapters', 'untitled.md')).toBe('chapters/untitled.md')
    expect(joinRel('', 'untitled.md')).toBe('untitled.md')
    expect(targetDir(null)).toBe('')
    expect(targetDir(node('chapters', 'directory'))).toBe('chapters')
    expect(targetDir(node('chapters/01.md', 'file'))).toBe('chapters')
  })

  it('allows dropping into another folder, not onto self or a descendant', () => {
    expect(canDropInto('notes/a.md', 'docs')).toBe(true)
    expect(canDropInto('a.md', '')).toBe(false)
    expect(canDropInto('notes/a.md', 'notes')).toBe(false)
    expect(canDropInto('docs', 'docs')).toBe(false)
    expect(canDropInto('docs', 'docs/sub')).toBe(false)
    expect(canDropInto('docs', 'other')).toBe(true)
    expect(
      acceptTreeDrop(['notes/a.md'], { kind: 'tree', dir: 'docs' }),
    ).toEqual({ kind: 'tree', dir: 'docs' })
    expect(acceptTreeDrop(['a.md'], { kind: 'tree', dir: '' })).toBeNull()
    expect(acceptTreeDrop(['a.md'], { kind: 'project', id: 'p2' })).toEqual({
      kind: 'project',
      id: 'p2',
    })
  })

  it('converts Tauri physical drag points to CSS pixels', () => {
    expect(logicalDragPoint({ x: 200, y: 400 }, 2)).toEqual({ x: 100, y: 200 })
    expect(logicalDragPoint(undefined, 2)).toEqual({ x: -1, y: -1 })
  })

  it('formats clipboard name and absolute path text', () => {
    expect(clipboardNames([node('notes/a.md', 'file')])).toBe('a.md')
    expect(
      clipboardNames([node('notes/a.md', 'file'), node('docs', 'directory')]),
    ).toBe('a.md\ndocs')
    expect(clipboardPaths('/Users/me/notes', ['a.md'])).toBe(
      '/Users/me/notes/a.md',
    )
    expect(clipboardPaths('/Users/me/notes/', ['', 'docs/b.md'])).toBe(
      '/Users/me/notes\n/Users/me/notes/docs/b.md',
    )
  })

  it('stacks a few drag-ghost names and a leftover count', () => {
    expect(dragGhostPreview([node('a.md', 'file')])).toEqual({
      items: [{ name: 'a.md', kind: 'file', relPath: 'a.md' }],
      extra: 0,
    })
    expect(
      dragGhostPreview(
        [
          node('a.md', 'file'),
          node('b.md', 'file'),
          node('docs', 'directory'),
          node('c.md', 'file'),
        ],
        3,
      ),
    ).toEqual({
      items: [
        { name: 'a.md', kind: 'file', relPath: 'a.md' },
        { name: 'b.md', kind: 'file', relPath: 'b.md' },
        { name: 'docs', kind: 'directory', relPath: 'docs' },
      ],
      extra: 1,
    })
  })

  it('flattens only expanded directories', () => {
    const root = [node('chapters', 'directory'), node('readme.md', 'file')]
    const children = {
      chapters: [node('chapters/01.md', 'file')],
    }

    expect(
      flattenTree(root, children, new Set()).map((row) => row.node.relPath),
    ).toEqual(['chapters', 'readme.md'])
    expect(
      flattenTree(root, children, new Set(['chapters'])).map(
        (row) => row.node.relPath,
      ),
    ).toEqual(['chapters', 'chapters/01.md', 'readme.md'])
    expect(
      flattenTree(root, {}, new Set(['chapters'])).map(
        (row) => row.node.relPath,
      ),
    ).toEqual(['chapters', 'readme.md'])
  })

  it('windows long lists with a buffer of 20', () => {
    expect(visibleWindow(500, 0, 280, 28, 20)).toEqual({ start: 0, end: 30 })
    expect(visibleWindow(500, 2800, 280, 28, 20)).toEqual({
      start: 80,
      end: 130,
    })
    expect(visibleWindow(0, 0, 280, 28, 20)).toEqual({ start: 0, end: 0 })
    expect(visibleWindow(10, 0, 0, 28, 20)).toEqual({ start: 0, end: 10 })
  })

  it('parses renderer asset URLs and http links', () => {
    expect(
      parseAssetHref('asset://localhost/proj/notes/guide.md#intro'),
    ).toEqual({
      projectId: 'proj',
      relPath: 'notes/guide.md',
      hash: 'intro',
    })
    expect(parseAssetHref('asset://localhost/proj/a%20b.png')).toEqual({
      projectId: 'proj',
      relPath: 'a b.png',
      hash: '',
    })
    expect(parseAssetHref('../escape.md')).toBeNull()
    expect(parseAssetHref('asset://localhost/proj/a%2F..%2Fb.md')).toBeNull()
    expect(parseAssetHref('asset://localhost/proj/%E0%A4%A')).toBeNull()
    expect(isHttpHref('https://example.com')).toBe(true)
    expect(isHttpHref('javascript:alert(1)')).toBe(false)
  })

  it('collects parent directories for watch reloads', () => {
    expect(dirsToReload(['chapters/01.md', 'readme.md'])).toEqual([
      '',
      'chapters',
    ])
  })

  it('selects an inclusive range of visible rows', () => {
    const rows = flattenTree(
      [
        node('a.md', 'file'),
        node('b.md', 'file'),
        node('c.md', 'file'),
        node('d.md', 'file'),
      ],
      {},
      new Set(),
    )
    expect(rangeRelPaths(rows, 'a.md', 'c.md')).toEqual([
      'a.md',
      'b.md',
      'c.md',
    ])
    expect(rangeRelPaths(rows, 'c.md', 'b.md')).toEqual(['b.md', 'c.md'])
    expect(rangeRelPaths(rows, null, 'b.md')).toEqual(['b.md'])
    expect(rangeRelPaths(rows, 'missing.md', 'b.md')).toEqual(['b.md'])
    expect(rangeRelPaths(rows, 'a.md', 'missing.md')).toEqual(['missing.md'])
  })

  it('returns null for drop targeting when there is no document', () => {
    expect(dropDirAtPoint(0, 0)).toBeNull()
    expect(projectIdAtPoint(0, 0)).toBeNull()
  })

  it('encodes and decodes a tree drag that names its project', () => {
    const raw = encodeTreeDrag('proj-a', ['notes/a.md', 'b.md'])
    expect(raw.startsWith(TREE_DRAG_PREFIX)).toBe(true)
    expect(decodeTreeDrag(raw)).toEqual({
      projectId: 'proj-a',
      paths: ['notes/a.md', 'b.md'],
    })
    expect(decodeTreeDrag('notes/a.md\nb.md')).toEqual({
      projectId: null,
      paths: ['notes/a.md', 'b.md'],
    })
    expect(decodeTreeDrag('')).toBeNull()
  })

  it('keeps an in-memory tree drag when dataTransfer is empty', () => {
    clearTreeDrag()
    beginTreeDrag('proj-a', ['notes/a.md'])
    expect(peekTreeDrag()).toEqual({
      projectId: 'proj-a',
      paths: ['notes/a.md'],
    })
    expect(isTreeDrag(null)).toBe(true)
    expect(resolveTreeDrag(null)).toEqual({
      projectId: 'proj-a',
      paths: ['notes/a.md'],
    })
    clearTreeDrag()
    expect(peekTreeDrag()).toBeNull()
    expect(isTreeDrag(null)).toBe(false)
    const plain = {
      getData: () => 'not-a-tree-drag',
      types: ['text/plain'],
    } as unknown as DataTransfer
    expect(isTreeDrag(plain)).toBe(false)
  })

  it('records copy vs move and ignores a duplicate drop', () => {
    clearTreeDrag()
    beginTreeDrag('proj-a', ['a.md'])
    setTreeDragCopy(true)
    expect(peekTreeDragCopy()).toBe(true)
    expect(claimTreeDrop('proj-b', ['a.md'])).toBe(true)
    expect(claimTreeDrop('proj-b', ['a.md'])).toBe(false)
    clearTreeDrag()
  })

  it('reads text/plain from either includes or contains type lists', () => {
    const asArray = {
      types: ['text/plain'],
    } as unknown as DataTransfer
    expect(dataTransferHasType(asArray, 'text/plain')).toBe(true)
    const asDomList = {
      types: { contains: (name: string) => name === 'text/plain', length: 1 },
    } as unknown as DataTransfer
    expect(dataTransferHasType(asDomList, 'text/plain')).toBe(true)
    expect(dataTransferHasType(null, 'text/plain')).toBe(false)
  })

  it('detects watch updates that touch the open file', () => {
    expect(
      watchTouchesOpenFile(
        { pathsChanged: { paths: ['notes/a.md'] } },
        'notes/a.md',
      ),
    ).toBe(true)
    expect(
      watchTouchesOpenFile(
        { pathsChanged: { paths: ['notes'] } },
        'notes/a.md',
      ),
    ).toBe(true)
    expect(
      watchTouchesOpenFile(
        { pathsChanged: { paths: ['other.md'] } },
        'notes/a.md',
      ),
    ).toBe(false)
    expect(
      watchTouchesOpenFile({ rescanExpanded: { paths: ['notes'] } }, 'a.md'),
    ).toBe(true)
  })

  it('treats an editor buffer as dirty only after the source was loaded', () => {
    expect(isDraftDirty(false, { text: 'a' }, 'b')).toBe(false)
    expect(isDraftDirty(true, null, 'b')).toBe(false)
    expect(isDraftDirty(true, { text: 'a' }, 'a')).toBe(false)
    expect(isDraftDirty(true, { text: 'a' }, 'b')).toBe(true)
  })
})
