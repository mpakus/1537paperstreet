# Library workspace validation

T-222 is implemented on `task/T-222-library-workspace`. The initially rejected
YAML dependency has been replaced with the approved, bounded parser described
in ADR-015. This record distinguishes local checks from platform/release checks.

## Verified locally

- Rust tests cover portable note creation, no-overwrite behavior, byte-preserving
  open/save, YAML filters and malformed metadata, source hashes, optimistic
  revisions, persisted selections, invalid metadata and parser complexity limits,
  literal properties and disabled file inclusion, unavailable projects,
  encrypted/symlink exclusion, heading selection, source ordering and fenced text.
- Application-state tests reopen saved selections from a temporary app root and
  assemble a selected section through the same parser used by IPC. Library tabs
  round-trip through session normalization.
- UI tests cover virtualization and keyboard navigation, failed-save retention,
  refreshed review hashes, unavailable source blocking, stale-preview rechecks,
  explicit selection replacement/close, and removed-heading scope protection.
- `cargo test --workspace`: 388 passed, 2 ignored across 48 suites.
- `npm test`: 209 tests across 58 files passed.
- `npm run check`, `npm run lint`, `npm run format:check`, and
  `cargo fmt --all --check` passed. Svelte checking reported no warnings.
- `cargo clippy --workspace --all-targets -- -D warnings` exited successfully.
  Existing ts-rs diagnostics about `deserialize_with` attributes remain.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passed, with the
  same existing ts-rs diagnostics. Changelog extraction and site release tests
  also passed.
- Production UI build passed. The existing Mermaid-related large-chunk advisory
  remains; Library is emitted as a separate lazy-loaded chunk.
- Browser fixture used the actual Svelte components with in-memory IPC: selected
  a note heading, saved a pack, previewed context and opened the note form.
  At 390 × 844 there was no horizontal overflow. Browser warning/error logs were
  empty. This was a component browser check, not a packaged Tauri smoke test.

## Performance comparison

The ignored `measure_library_scan_and_pack` test creates 1,000 notes with YAML and
headings, then scans them and assembles 20 selected sections. Measurements below
use the local macOS host and debug profile. The initial YAML parser had five
passes; the final parser had five invocations with five passes each (25 total):

| Operation | Initial parser, five passes | Median |
| --- | --- | --- |
| Scan 1,000 notes | 186.65, 181.64, 186.11, 183.92, 180.83 ms | 183.92 ms |
| Assemble 20 sections | 2.95, 2.85, 3.11, 2.91, 3.01 ms | 2.95 ms |

| Operation | Final parser, 25 passes: range | Median | Median change |
| --- | --- | --- | --- |
| Scan 1,000 notes | 180.70–196.32 ms | 189.81 ms | +3.2% |
| Assemble 20 sections | 2.46–2.81 ms | 2.69 ms | −8.8% |

Output remained 55,631 bytes. These are local diagnostic timings, not
release-profile budgets or cross-platform measurements. There is no pre-feature
Library latency; this compares the original implementation with the replacement
YAML parser. Reproduce with:

```sh
cargo test -p ps-render --test library measure_library -- --ignored --nocapture
```

## Dependency verification

`cargo deny --locked check --deny warnings` rejected the initial
`serde_yaml_ng 0.10.0` dependency because it pulls in unmaintained
`unsafe-libyaml` (RUSTSEC-2024-0449). No audit exception was added.

The approved replacement is `serde-saphyr 2.0.0`, whose crate archive is 720,173
bytes. Only serialization/deserialization features are enabled, with file
inclusion and property expansion disabled. Parser and alias-replay budgets bound
front-matter processing. `serde_yaml_ng` and `unsafe-libyaml` are no longer in
the lockfile; no existing dependency was upgraded as part of this replacement.

`cargo deny --locked check --deny warnings` passed advisories, bans, licenses and
sources. No new policy exception was added. `cargo audit` exited successfully
with six existing allowed transitive warnings (basic-toml, proc-macro-error,
zbus_names, zvariant, zvariant_derive and glib). All six package versions are
unchanged from the base commit; the glib warning concerns `VariantStrIter`
unsoundness. These warnings are not evidence of a completely clean dependency
tree across all platforms. `npm audit --audit-level=high` also exited successfully,
with three existing moderate findings in the unchanged Vitest development-tool
dependency chain; no high or critical findings were reported.

Native macOS interaction, Linux/Windows runtime checks, packaged installer builds,
and release publication have not been performed for this change.
