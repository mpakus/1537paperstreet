import { describe, expect, it } from 'vitest'

import { classifyPreviewHref, linkOpensEditor } from './preview-nav'

describe('classifyPreviewHref', () => {
  const project = {
    projectId: 'proj',
    projectPath: '/Users/me/Notes',
    currentRelPath: 'chapters/intro.md',
  }

  it('opens an asset Markdown link in the current project', () => {
    expect(
      classifyPreviewHref(
        'asset://localhost/proj/notes/guide.md#intro',
        project,
      ),
    ).toEqual({
      kind: 'document',
      relPath: 'notes/guide.md',
      hash: 'intro',
    })
  })

  it('opens a relative Markdown link next to the current file in one hop', () => {
    expect(classifyPreviewHref('guide.md', project)).toEqual({
      kind: 'document',
      relPath: 'chapters/guide.md',
      hash: '',
    })
    expect(classifyPreviewHref('./setup.md#install', project)).toEqual({
      kind: 'document',
      relPath: 'chapters/setup.md',
      hash: 'install',
    })
  })

  it('opens an in-project file:// link and ignores paths outside the project', () => {
    expect(
      classifyPreviewHref('file:///Users/me/Notes/chapters/guide.md', project),
    ).toEqual({
      kind: 'document',
      relPath: 'chapters/guide.md',
      hash: '',
    })
    expect(classifyPreviewHref('file:///etc/passwd', project)).toEqual({
      kind: 'ignore',
    })
  })

  it('scrolls hashes, opens http(s) in the browser, and ignores javascript', () => {
    expect(classifyPreviewHref('#intro', project)).toEqual({
      kind: 'hash',
      id: 'intro',
    })
    expect(classifyPreviewHref('https://example.com/docs', project)).toEqual({
      kind: 'http',
      url: 'https://example.com/docs',
    })
    expect(classifyPreviewHref('javascript:alert(1)', project)).toEqual({
      kind: 'ignore',
    })
    expect(classifyPreviewHref('../secret.md', project)).toEqual({
      kind: 'ignore',
    })
  })

  it('ignores asset URLs from another project', () => {
    expect(
      classifyPreviewHref('asset://localhost/other/notes/guide.md', project),
    ).toEqual({ kind: 'ignore' })
  })

  it('opens linked source files in the editor and leaves Markdown in preview', () => {
    expect(linkOpensEditor('lib/app.rb', false)).toBe(true)
    expect(linkOpensEditor('web/app.js', false)).toBe(true)
    expect(linkOpensEditor('lib/app.ex', false)).toBe(true)
    expect(linkOpensEditor('notes/guide.md', false)).toBe(false)
    expect(linkOpensEditor('assets/cover.png', false)).toBe(false)
    expect(linkOpensEditor('shots/photo.jpeg', false)).toBe(false)
    expect(linkOpensEditor('anim.gif', false)).toBe(false)
    expect(linkOpensEditor('assets/cover.png', true)).toBe(false)
  })
})
