# Library workspace validation

T-222 is implemented on `task/T-222-library-workspace` and remains in progress.
The dependency gate below must be resolved before considering it complete.

## Verified locally

- Rust tests cover portable note creation, no-overwrite behavior, byte-preserving
  open/save, YAML filters and malformed metadata, source hashes, optimistic
  revisions, persisted selections, invalid metadata limits, unavailable projects,
  encrypted/symlink exclusion, heading selection, source ordering and fenced text.
- Application-state tests reopen saved selections from a temporary app root and
  assemble a selected section through the same parser used by IPC. Library tabs
  round-trip through session normalization.
- UI tests cover virtualization and keyboard navigation, failed-save retention,
  refreshed review hashes, unavailable source blocking, stale-preview rechecks,
  explicit selection replacement/close, and removed-heading scope protection.
- `cargo test --workspace`: 385 passed, 2 ignored across 48 suites.
- `npm test`: 209 tests across 58 files passed.
- `npm run check`, `npm run lint`, `npm run format:check`, and
  `cargo fmt --all --check` passed. Svelte checking reported no warnings.
- `cargo clippy --workspace --all-targets -- -D warnings` exited successfully.
  Existing ts-rs diagnostics about `deserialize_with` attributes remain.
- Production UI build passed. The existing Mermaid-related large-chunk advisory
  remains; Library is emitted as a separate lazy-loaded chunk.
- Browser fixture used the actual Svelte components with in-memory IPC: selected
  a note heading, saved a pack, previewed context and opened the note form.
  At 390 × 844 there was no horizontal overflow. Browser warning/error logs were
  empty. This was a component browser check, not a packaged Tauri smoke test.

## Performance baseline

The ignored `measure_library_scan_and_pack` test creates 1,000 notes with YAML and
headings, then scans them and assembles 20 selected sections. On the local macOS
host, in the debug profile before replacing the initial YAML dependency:

| Operation | Five runs | Median |
| --- | --- | --- |
| Scan 1,000 notes | 186.65, 181.64, 186.11, 183.92, 180.83 ms | 183.92 ms |
| Assemble 20 sections | 2.95, 2.85, 3.11, 2.91, 3.01 ms | 2.95 ms |

Output was 55,631 bytes. These are local diagnostic timings, not release-profile
budgets or cross-platform measurements. There is no pre-feature Library latency;
this is the initial baseline. Repeat it after changing the YAML parser:

```sh
cargo test -p ps-render --test library measure_library -- --ignored --nocapture
```

## Open dependency gate

`cargo deny --locked check --deny warnings` rejected the initial
`serde_yaml_ng 0.10.0` dependency because it pulls in unmaintained
`unsafe-libyaml` (RUSTSEC-2024-0449). No audit exception was added.

A replacement with `serde-saphyr 2.0.0` is proposed. Its 720,173-byte crate package
exceeds the approximately 500 KB threshold in AGENTS.md §8, so user approval is
pending. Use only serialization/deserialization features, with file inclusion and
property expansion disabled; enforce tight parser budgets for front matter.
After approval, replace the dependency, rerun tests and the dependency audit,
and record the updated performance measurements here.

Native macOS interaction, Linux/Windows runtime checks, packaged installer builds,
and release publication have not been performed for this change.
