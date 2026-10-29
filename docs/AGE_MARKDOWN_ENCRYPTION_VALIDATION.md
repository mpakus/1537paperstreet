# Encrypted Markdown implementation and validation

Date: 2026-10-18. Task: T-218, branch `task/T-218-age-markdown`, based on
`10d165e`. This is local implementation evidence, not a production release or
an independent cryptographic audit. T-218 remains `[~]` until release gates and
its closing PR are complete.

## Version 0.9.0 merge verification — 2026-10-19

Rust, npm, Tauri and lockfile versions were synchronized to 0.9.0, with a dated
changelog entry and matching comparison links. Locked, offline Cargo metadata
and changelog extraction passed.

The final version passed all 360 Rust tests (45 suites), 197 UI tests (55 files),
Rust/UI formatting, Clippy, documentation with warnings denied, UI type checks,
lint, the frontend build and the optimized workspace build. Cargo audit, Cargo
deny and npm audit passed; the existing Rust dependency warnings remain open.
A temporary inversion of the encrypted Mermaid-cache guard failed its regression
test; restoring the original source made it pass. Native macOS, interoperability,
coverage and performance evidence below comes from the earlier implementation
checks. The open release gates at the end of this report still apply.

## Dependency maintenance exception — 2026-10-20

The new exception in `deny.toml` applies only to RUSTSEC-2026-0374, which marks
`basic-toml` 0.1.10 as unmaintained and describes no vulnerability. It is pulled in by
`i18n-config` 0.4.8 through the translation procedural macros used to build
age 0.12.1. No published replacement for `i18n-config` removes this dependency
yet. Remove the exception when the upstream release is available and adopted.
Existing exceptions are unchanged; advisory, license, source and dependency
checks remain enabled. After this change, local `cargo deny check`, `cargo audit`
and `npm audit --audit-level=high` passed. Cargo audit still reports the maintenance
advisory alongside the existing dependency warnings; npm reports zero vulnerabilities.

## Implemented

- Standard binary age v1/scrypt, now pinned age 0.11.5 by T-226; complete authentication and
  strict UTF-8 before source enters the editor. X25519 and armor are rejected.
- Protected create, unlock, Preview/Edit/Split, Save, Lock, conversion, password
  changes and explicit no-clobber plaintext export. Conversion keeps its source
  until a separate Keep/Delete choice after the protected preview opens.
- Direct ciphertext streaming to same-directory private temp files, finish,
  verification, sync, base-hash recheck and atomic replacement. No-op Save keeps
  exact ciphertext; ordinary writes cannot overwrite age files as plaintext.
- Recovery copies before Discard/dirty Reload, password replacement and
  confirmed deletion of a converted original. Failed
  writes preserve saved content and the UI buffer. Window hide/quit and update
  relaunch use the dirty-note guard before clearing sessions.
- Content-search exclusion, no protected Mermaid caches, no source in global
  chunk broadcasts, no plaintext session restore or stored passwords.
- Generated IPC bindings, guide, ADR-013, changelog, and Windows crypto test lane.

The 2026-10-29 dependency change removes the affected P-256 dependency chain
from age 0.12.1 while retaining the same scrypt implementation. Its historical
ciphertext fixture, bidirectional compatibility checks, audit results and
maintenance tradeoff are documented in
[T-226 validation](SECURITY_DEPENDENCIES_2026-10.md). Earlier measurements below
describe the original dependency version.

## Executed locally

Host: Apple M4 Max, macOS 27.0.1, Rust 1.97.1, Node 24.13.0.

| Check | Result |
| --- | --- |
| `cargo test --workspace` | 360 tests passed, 45 suites, after the conversion/deletion fixes |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed; existing ts-rs macro diagnostics still print |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Passed |
| Generated TypeScript equality test | Passed; generated file retains ts-rs formatting |
| `npm run check` | 0 errors, 0 warnings |
| `npm run lint` | Passed |
| `npm run test` | 197 tests passed, 55 files, including encrypted-note reopening |
| `npm run format:check` | Passed |
| Rust coverage gate | Passed: core 86.53%, render 91.35%, app 67.33%; docio file gate passed |
| `npm run test:coverage` | Passed; configured tree/text logic 94.44% lines |
| `npm run build` | Passed; existing large vendor-chunk warning |
| macOS `cargo build --workspace --release` | Passed after preserving host build-macro symbols |
| `cargo audit` | Passed: zero vulnerabilities; existing maintenance, soundness and yanked-package warnings remain |
| `cargo deny check` | Passed: advisories, licenses, sources and bans; existing duplicate-version warnings remain |

The release profile keeps `strip = "symbols"` for shipped artifacts and adds
`[profile.release.build-override] strip = "none"`. Without this, the local Rust
toolchain could not load the stripped `rustversion` procedural macro through
arc-swap. The same failure reproduced in a fresh target directory; the scoped
override fixed the release build.

Tests cover Unicode/BOM/CRLF and mixed-EOL preservation, empty input, random
ciphertext, wrong passwords, corruption/truncation, invalid decrypted UTF-8,
unsupported recipients, excessive scrypt work, size limits, symlinks, Unix
0600/read-only permissions, stale independent sessions, injected partial-write
and pre-replace failures, recovery, password change and export collisions.
App tests cover authenticated rendering, content-search exclusion and no
plaintext chunk broadcast. UI tests cover wrong-password cleanup, dirty Save
failure, Cancel, recovery failure, Discard, native-close events, reload recovery,
encrypted creation, and Mermaid cache isolation.

With user approval, ammonia was updated from 4.1.4 to 4.1.8, restricted to the
4.1.x patch series. This resolves RUSTSEC-2021-0157, RUSTSEC-2021-0158 and
RUSTSEC-2026-0373 without adding advisory suppressions. New renderer tests
exercise escaped CSS URL payloads, mixed-case CSS URL functions, all three SVG
animation tags, and preservation of safe marker colors. They also pass with
4.1.4: the application's existing strict style and tag filters already rejected
these payloads. The full Rust suite was rerun with 4.1.8, including encryption,
authenticated rendering, and failure/recovery tests. Formatting, Clippy, the
release build, audit and deny checks were also rerun. UI, coverage, browser and
CLI-interoperability results above are from the encryption implementation;
those checks were not repeated for this Rust sanitizer dependency update.

Browser smoke used the real Svelte App and CodeMirror with synthetic mocked IPC:
locked view → password dialog → unlocked preview → edit → Save/Discard/Cancel
dialog → Save → locked view without source. Focus enters the password field;
the close dialog defaults to Cancel. This verifies browser behavior, not native
Tauri filesystem IPC or a signed installed application.

## Conversion and reopening follow-up

The default conversion path now preserves the complete original name and adds
`.age`, including `.markdown`, `.mdown`, `.mdwn`, and case variants. Conversion
opens the protected copy in Preview, then offers **Keep original / Delete
original**. Deletion uses a session-bound conversion receipt and rejects changes
to either file. An encrypted `pre_encrypt_delete` recovery copy preserves the
exact original bytes even when the converted buffer contains unsaved edits.
Tests cover recovery failure and source/ciphertext changes during the snapshot.
Failed conversion cannot fall through to creating an empty encrypted file.

The reported empty view was reproduced in the native app after closing the last
tab: its selected tree row survived, so a single filename click initiated Rename
instead of opening the document. Closing the last tab now clears that selection.
A regression test reproduces the old failure and verifies one-click reopening,
unlock, and rendered content. Truly empty notes now have an empty-note message.

An unsigned macOS debug app with a separate test bundle identifier and temporary
`PAPERSTREET_HOME` exercised real Tauri IPC and filesystem operations:

- Convert `Original.MARKDOWN` to `Original.MARKDOWN.age`; preview shows its text.
- Delete the original only after confirmation; the original disappears and an
  encrypted recovery file is created.
- Convert `Keep.md`, choose Keep, and verify the original remains byte-identical.
- Close all tabs, single-click `Keep.md.age`, unlock, and read the original text.
- Quit the isolated app and verify its process exits.

The native checks used synthetic files and a public test passphrase. They do not
close the Windows/Linux, minimum-device, crash/power-loss or independent-review
gates. Full Rust/UI tests, type checks, lint, formatting, Clippy, the unsigned
native build and the optimized workspace build passed. Earlier coverage and CLI interoperability evidence remains
recorded above and below; those checks were not rerun for this follow-up.

## Windows file-lock follow-up — 2026-10-19

The [0.9.0 CI run](https://github.com/mpakus/1537paperstreet/actions/runs/39322099820)
passed its macOS, Linux, coverage and security jobs but failed the Windows
encrypted-note lifecycle and original-deletion tests with OS error 33. An
exclusive Windows byte-range lock also blocks a second handle opened by the
same process; hashing by reopening the locked path caused the failure.

Save and Delete original now read through the lock-owning handle and rewind it
for repeated hash checks. A live `same-file` handle comparison rejects path
replacement before those reads. The lock remains held through atomic replacement
or deletion; no-op ciphertext preservation and encrypted recovery still apply.
`same-file` was already present transitively; its direct dependency adds no new
lockfile packages. Windows CI now runs encryption unit tests as well as the
crypto/lifecycle integration suites. Regressions cover lock contention and retry,
repeated reads, atomic replacement while locked, changed paths with identical
bytes, and recovery/write failures. Native desktop acceptance remains a separate
release gate.

Local validation: 364 Rust tests in 45 suites and 197 UI tests passed, along with
Rust/UI formatting, Clippy, UI type checks, lint and Cargo deny.

The first follow-up Windows run confirmed original deletion and no-op Save, then
exposed access denied at edited Save's replacement: `tempfile::persist` calls
`MoveFileExW`, which cannot replace the open destination. Replacement now uses
Rust's `std::fs::rename` fallback to `FileRenameInfoEx`/POSIX semantics, keeping
the destination lock held. The staged file's temporary attribute is cleared,
its synced handle is closed, and a cleanup guard removes it if rename fails.
A failure test verifies that a rejected rename preserves the saved bytes and
removes the staged file. An inverted identity comparison makes the identical-byte
replacement regression fail, confirming it detects a lost conflict guard.

## Independent format compatibility

Go `age` 1.2.1 and Rust `rage` 0.12.1 both passed:

1. App encrypts a BOM/Unicode/mixed-EOL fixture; CLI decrypts identical bytes.
2. CLI encrypts with `-p`; app fully authenticates and recovers identical bytes.

Fixtures and CLI interactions ran in temporary directories with a public test
password. The test driver never prints the password or decrypted content.
Reproduce on a POSIX host with independently installed binaries:

```sh
cargo build --release -p ps-core --example encryption_probe
python3 scripts/test-age-interop.py /absolute/path/to/age /absolute/path/to/rage
cargo run --release -p ps-core --example encryption_probe -- bench
```

## Performance

Ordinary Markdown `read_doc`, optimized builds, base `10d165e` versus this
implementation. Each run warms 20 reads and measures 200 reads of a temporary
file; values are the median of five alternating runs per build. The source is
`crates/ps-core/examples/plain_read_probe.rs`.

| Source | Before p50 | After p50 | Change | Before / after p95 |
| --- | ---: | ---: | ---: | ---: |
| 10 KiB | 46.834 µs | 47.500 µs | +1.42% | 52.042 / 55.708 µs |
| 1 MiB | 825.250 µs | 825.750 µs | +0.06% | 910.167 / 888.791 µs |

The existing Criterion tree benchmark (50,000 files, 10 samples, 1 s warmup,
1 s measurement) measured 35.982 ms before and 35.283 ms after. Its confidence
interval for change was −2.72% to −0.79%; both stayed below the 120 ms budget.
Neither measured ordinary-read nor tree work exceeded the 10% regression limit.

After the conversion and reopening follow-up, the same read probe was repeated
against the retained baseline executable and the final optimized build, with
five alternating runs per build:

| Source | Before p50 | After p50 | Change | Before / after p95 |
| --- | ---: | ---: | ---: | ---: |
| 10 KiB | 46.667 µs | 48.209 µs | +3.30% | 62.958 / 68.791 µs |
| 1 MiB | 867.250 µs | 872.375 µs | +0.59% | 1082.333 / 1101.292 µs |

Both median and p95 changes remained below 10% (p95: +9.26% and +1.75%).

The sanitizer update was also measured with identical application source and
release profiles, changing only ammonia 4.1.4 → 4.1.8. The synthetic fixture
contains raw HTML, safe marker colors, and rejected SVG/event attributes so
the sanitizer runs. Median of five alternating runs, each with 20 warmups and
200 measured renders, using `crates/ps-render/examples/sanitizer_probe.rs`:

| Raw-HTML Markdown | Before p50 | After p50 | Change | Before / after p95 |
| --- | ---: | ---: | ---: | ---: |
| 10 KiB | 0.792 ms | 0.778 ms | −1.79% | 0.820 / 0.808 ms |
| 100 KiB | 7.774 ms | 7.737 ms | −0.47% | 7.932 / 7.897 ms |

No measured sanitizer regression exceeded 10%. Reproduce a single current-build
run with `cargo run --release -p ps-render --example sanitizer_probe`.

These are core read, tree and render timings, not cold native-window startup measurements.
Illustrative optimized KDF measurements on this host: 10 KiB create 2.83 s,
unlock 1.34 s, unchanged Save 86 µs, edited Save 2.65 s; 1 MiB create 2.69 s,
unlock 1.33 s, unchanged Save 0.90 ms, edited Save 2.66 s. These are single
local samples, not minimum-device acceptance or a latency guarantee.

Encrypted KDF latency is intentionally much higher; create/edited Save also
decrypt-verifies the staged ciphertext. These operations run off the command
thread and the UI displays Working. Unchanged Save skips the KDF.

## Open release gates

- Review existing audit warnings: unmaintained proc-macro-error/zbus_names/
  zvariant/zvariant_derive, glib unsoundness, and yanked chacha20 0.10.1 through
  rand/ulid. This record does not claim those warnings are harmless.
- Run the updated CI on Windows/Linux, including Windows open-handle replacement,
  native permissions, actual disk-full/unavailable-volume failures, and native
  desktop create → edit → close → reopen → unlock. CI configuration is not
  evidence that a remote run passed.
- Measure scrypt on minimum supported devices, validate native close/tray/restart
  behavior, and complete independent review of secret handling and crash data.
  No destructive OS-crash or power-loss test was performed.

Advisory locks plus a final hash check protect cooperating writers and stale
editors. An unrelated process ignoring locks can still race the last check and
replacement. OS swap/crash dumps and JavaScript copies cannot be reliably
zeroized. Filesystem durability varies; backups and visible metadata remain
outside the per-file content-encryption boundary. See ADR-013 and the user guide.
