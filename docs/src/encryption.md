# Password-protected notes

**Protect → New encrypted note** creates a standard binary `name.md.age` file.
Choose a path inside the current project, enter a password twice, and keep that
password somewhere safe. There is no password recovery or stored master key.
The app creates the encrypted file directly; it never creates a plain `.md`
first. Password-protected notes were introduced in 0.9.0.

**Windows 0.9.0:** encrypted Save and Delete original encounter file-lock errors.
The fix is on `main` and passes Windows CI, but is not in the published 0.9.0
packages. See the [unreleased changelog](https://github.com/mpakus/1537paperstreet/blob/main/CHANGELOG.md#unreleased).

Open a `.md.age` file and choose **Unlock note**. Only a completely authenticated,
valid UTF-8 document opens. Incorrect passwords and damaged files leave the
file unchanged. Public-key recipients, generic `.age` files, and ASCII armor
are not supported. Notes may contain up to 8 MiB of Markdown.
The other supported Markdown extensions (`.markdown`, `.mdown`, `.mdwn`) also
keep their spelling and take an `.age` suffix, such as `Readme.MARKDOWN.age`.

Once unlocked, use Preview, Edit, Split, Format, highlights, task checkboxes,
and Save as usual. The password stays in the Rust process only for this session.
An unchanged save keeps the encrypted file byte-for-byte identical. An edit
creates fresh ciphertext, checks for external changes, then atomically replaces
the old file. A failed save leaves your edits in memory. Autosave and automatic
drafts are not implemented; save before leaving the computer unattended.

## Locking and closing

**Encrypted → Lock note**, closing the tab, switching projects, closing the
window, and quitting clear the encrypted session and editor buffers. Closing
the main window still hides it to the tray, after these checks complete.
Dirty notes offer **Save / Discard / Cancel**. Discard first writes an encrypted
recovery copy; if that fails, the note stays open. Reloading an externally
changed dirty note also saves a recovery copy first. There is no idle timeout
or automatic lock on operating-system sleep in this version.

Lock or close encrypted tabs before renaming, moving, trashing, or replacing
their files. Other tabs retain unlocked text in memory until closed or locked.

## Converting, changing passwords, and exporting

For an ordinary Markdown note, choose **Protect → Encrypt this note**. The
current buffer becomes an encrypted file, including unsaved changes. The default
path is the complete original path with `.age` appended. The protected copy
opens in Preview, then **Keep original / Delete original** asks what to do with
the unencrypted source. Keeping it or dismissing the dialog leaves it unchanged.
Deleting removes it from the folder after saving an encrypted recovery copy of
its exact original bytes. If either file changed or recovery fails, deletion is
cancelled. Deletion is not secure erasure: snapshots, backups, cloud versions,
and filesystem remnants may keep the original.

**Encrypted → Change password** saves pending edits and re-encrypts the entire
document. A hidden `pre_password_change` copy retains the previous ciphertext
under the old password. Changing the password does not revoke old backups.

**Encrypted → Export decrypted .md** warns before creating an unencrypted copy.
It keeps the encrypted note and refuses to overwrite an existing destination.
Document PDF export is disabled for protected notes; explicitly export Markdown
first. Copying text or saving a diagram image also exposes that content outside
the encrypted note.

## Backups and recovery

Keep independent backups of the `.md.age` files and their passwords. Filenames,
sizes, timestamps, and directory structure remain visible.

Hidden sibling files named `.name.md.age.pre_discard-….md.age` contain discarded
edits. `.name.md.age.pre_password_change-….md.age` contains a previous encrypted
version. `.name.md.pre_encrypt_delete-….md.age` preserves the original before
deletion after conversion, using the new encrypted note's password.
Enable **Show hidden files** to open them; use the password active when
the copy was created. Keep or remove these backups deliberately. They are not
automatically pruned and are separate from the future Version History feature.

A crash can leave `.name.md.age.recovery-….md.age`. Try opening it with the same
password. A complete authenticated copy is readable; a partial one is rejected.
The app never automatically promotes it over your saved note. An abrupt crash
before Save or Discard can lose unsaved in-memory edits.

## Using other tools

The file format is standard age passphrase encryption. Both independent Go
`age` and Rust `rage` can decrypt it; they prompt for the password:

```sh
age -d -o recovered.md note.md.age
rage -d -o recovered.md note.md.age
```

To create an interoperable file, use `age -p -o note.md.age note.md` or
`rage -p -o note.md.age note.md`. Do not request ASCII armor or public-key
recipients. These commands deliberately create or consume plaintext files.

## Privacy boundaries

Content search skips encrypted files even when unlocked; filename search still
works. Protected Mermaid diagrams bypass both disk and shared memory caches.
Passwords and decrypted text are not written to app configuration, session
restore files, logs, prompt history, or automatic drafts. Explicitly copying
content to the Assistant or clipboard remains your choice.

Encryption protects saved content, not a compromised computer. Unlocked text
exists in the WebView and Rust memory. The app wipes its owned Rust secret
buffers when released, but cannot guarantee erasure of JavaScript strings,
swap, hibernation, OS crash dumps, clipboard history, screenshots, or backups.
Use full-disk encryption and a trustworthy device as well.
