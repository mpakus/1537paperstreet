// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { expect, it, vi } from 'vitest'
import EncryptionDialog from './EncryptionDialog.svelte'

it('requires matching passwords and clears secret inputs after a failed request', async () => {
  const target = document.createElement('div')
  document.body.append(target)
  const submit = vi
    .fn()
    .mockRejectedValue(new Error('Could not encrypt this note.'))
  const instance = mount(EncryptionDialog, {
    target,
    props: {
      action: 'create',
      path: 'note.md.age',
      onsubmit: submit,
      oncancel: () => {},
    },
  })
  flushSync()
  const input = (label: string, value: string) => {
    const field = target.querySelector<HTMLInputElement>(
      `[aria-label="${label}"]`,
    )!
    field.value = value
    field.dispatchEvent(new Event('input', { bubbles: true }))
    flushSync()
  }
  input('Password', 'test passphrase')
  input('Confirm password', 'different')
  target
    .querySelector('form')!
    .dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  flushSync()
  expect(submit).not.toHaveBeenCalled()
  expect(target.textContent).toContain('Passwords do not match')
  input('Confirm password', 'test passphrase')
  target
    .querySelector('form')!
    .dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  await vi.waitFor(() => {
    flushSync()
    expect(target.textContent).toContain('Could not encrypt')
  })
  expect(submit).toHaveBeenCalledWith('note.md.age', 'test passphrase')
  expect(
    target.querySelector<HTMLInputElement>('[aria-label="Password"]')!.value,
  ).toBe('')
  expect(
    target.querySelector<HTMLInputElement>('[aria-label="Confirm password"]')!
      .value,
  ).toBe('')
  await unmount(instance)
  target.remove()
})
