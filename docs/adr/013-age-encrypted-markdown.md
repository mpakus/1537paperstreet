# ADR-013: Per-document age passphrase encryption

Status: Accepted for T-218, explicitly requested through
`AGE_MARKDOWN_ENCRYPTION_PLAN.md`. Cross-platform release gates remain open.

## Decision

`ps-core::crypto` owns standard binary age v1 encryption and authenticated,
strict UTF-8 decryption. `age = "=0.12.1"` manages all keys, salts, nonces and
scrypt calibration. No custom format, vault, schema change, keychain entry,
network call, armor, or public-key mode is added.

Three dependency changes have separate purposes: pinned `age` provides the
standard cryptographic format; `zeroize` clears owned plaintext buffers;
existing test dependency `tempfile` becomes a runtime dependency for portable
private temporary files and replacement without unlinking the destination.
`age::secrecy::SecretString` owns session passwords. Default age CLI/SSH/plugin
features are disabled. The exact dependency graph is recorded in Cargo.lock.

The Windows lock fix promotes the existing transitive `same-file` 1.0.6 to a
direct dependency for portable file-identity checks on live handles. This adds
no packages to the lockfile and avoids custom unsafe platform code.

`ps-core::encrypted::EncryptedDocuments` serializes expensive operations and
keeps authenticated bytes, password and ciphertext BLAKE3 per canonical project
path. Every filesystem operation enters through `fsops::resolve`; encrypted
operations additionally reject symlink components and non-regular files.
Tauri wrappers run core calls through `spawn_blocking`.

## Persistence and conflicts

Create and edited Save stream age ciphertext directly into a private sibling
temporary file. Finish authentication, decrypt and compare the complete staged
file, flush, sync, re-resolve, recheck `base_hash`, then persist. Existing targets
are never unlinked first. New files/exports use no-clobber persistence. On Unix
files start with mode 0600; Windows uses the directory's inherited ACL. Directory
sync is attempted on Unix after replacement. Failure after commit does not
falsely report that the old file was retained; power-loss durability still
depends on the filesystem and OS.

The file handle's exclusive lock serializes cooperating processes during Save;
hash comparison detects stale editors and checks again immediately before
replacement. Windows byte-range locks deny reads through another handle, even
in the same process, so locked reads rewind and reuse the lock-owning handle.
Before each read, compare it with a fresh handle for the resolved path; an
atomic replacement of that path is a conflict even if its bytes are unchanged.
Keep the original lock alive through replacement or deletion. These rules also
apply to the plaintext source while creating `pre_encrypt_delete` recovery.
For replacement, clear the staged file's temporary attribute with `keep`, retain
a cleanup guard, close the synced staged handle, then call `std::fs::rename`.
Rust's Windows implementation can replace an open destination with
`FileRenameInfoEx`/POSIX semantics; `tempfile::persist` uses only `MoveFileExW`
and returns access denied while the destination remains open. A failed rename
removes only the staged file. New destinations still use no-clobber persistence.
An unrelated writer that ignores locks can still race the final
check/rename, as with ordinary document saves. This is not a filesystem-wide
compare-and-swap or protection against a hostile local process changing parent
directories. Windows replacement and open-handle behavior require native CI
and manual acceptance before release.

The lock behavior follows [Rust's file-lock documentation](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
and [Windows LockFileEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex).

Unchanged normalized text and restoration traits preserve the exact original
ciphertext (including mixed original line endings). Changed content restores
BOM, EOL and trailing-newline traits before encryption. Conversion appends `.age`
to the complete original filename, including supported Markdown extension aliases,
and retains the original until the user chooses Keep or Delete after opening the
protected preview. Only explicit decrypted export writes plaintext, and it
never overwrites an existing file.

Before changing a password, preserve old ciphertext as `pre_password_change`.
Before Discard or a dirty Reload, encrypt the current buffer as `pre_discard`.
Before explicitly deleting a converted original, preserve its exact saved bytes
as `pre_encrypt_delete`, using the conversion's password. A session-only receipt
binds the source path/hash and encrypted copy's hash. Both hashes are checked
again after recovery creation, immediately before deletion through `fsops`.
Deletion fails if either file changed, the session was closed, or recovery failed.
This removes the source file rather than moving plaintext into Trash; it makes
no secure-erasure guarantee. Keep, Escape, or closing the dialog retains the source.
Failure to preserve it cancels the destructive transition. These private sibling
backups add three recovery boundaries without starting P10's database/history UI.
Complete orphan `.recovery-….md.age` files can be opened manually; incomplete
ones fail authentication. No automatic recovery promotion or retention policy.

## Resource and privacy policy

Limit plaintext to 8 MiB and ciphertext to 9 MiB. Decryption caps the scrypt
exponent at 20 (about 1 GiB working memory); higher-cost untrusted inputs fail
before running the KDF. EOF/authentication and UTF-8 validation finish before
installing a session or returning plaintext. Existing ciphertext never falls
through to the ordinary text writer or content search.

IPC exposes only `DocumentSecurity` (plain/locked/unlocked), never a password
inside persisted types. Locked documents contain no source or HTML. Protected
HTML is returned directly to the requesting WebView, not broadcast in global
chunk events. Window titles use filenames, not decrypted headings. Mermaid
bypasses disk and shared memory caches; spellcheck is disabled for protected
notes. PDF export requires an explicit plaintext export first.

Lock/close/project switch/window hide/quit clear Rust sessions and UI tab,
editor/undo, preview, find and diagram references after Save/Discard/Cancel.
Generation checks ignore stale asynchronous results. Save may accept later
edits without clearing them. Plaintext remains in the WebView while unlocked;
zeroization cannot promise erasure of JS copies, OS swap or crash dumps. No
idle/sleep timeout or automatic encrypted draft is introduced.

## Validation

See [the validation record](../AGE_MARKDOWN_ENCRYPTION_VALIDATION.md) for the
executed tests, independent CLI checks, timings and remaining release gates.
