# Contributing

Read [`AGENTS.md`](AGENTS.md), [`docs/PLAN.md`](docs/PLAN.md), and
[`docs/CHECKLIST.md`](docs/CHECKLIST.md) before you start.

One checklist task per branch `task/T-xxx-short-name`, closed by its own PR.
Write the test first, then the code. Commits use Conventional Commits. Code,
comments, and commit messages are English. User-facing docs (README, this file,
CHANGELOG, the mdBook in `docs/src/`) are English. `PLAN.md`, `CHECKLIST.md`,
and `AGENTS.md` stay Russian for now.

Every Definition of Done item in `AGENTS.md` is required, including
`npm run check`. IPC types are generated from Rust. After changing a struct:

```sh
UPDATE_TS_BINDINGS=1 cargo test -p ps-core --test typescript
```

Commit `ui/src/lib/generated/core.ts`. UI tests mock IPC through
`ui/src/lib/ipc.mock.ts`. Filesystem tests use `tempfile` only — never the
real `$HOME`.

Dependency checks use `cargo deny --locked check --deny warnings` in CI.
Yanked releases and unreviewed duplicate versions fail the check. Prefer a
compatible dependency update; `deny.toml` lists exact-version exceptions only
where upstream crates require incompatible release lines. Each exception names
the dependency that still needs it. Remove it when upstream versions converge,
and review the graph again when an excepted version changes. These exceptions
apply only to duplicate detection; advisory, license, and source checks still run.
See the [cargo-deny documentation](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html#skip).

History (P10) and ZIP export (P12) are out of scope unless someone explicitly
asks. Do not add them “while you’re here.”

Release: tag `vX.Y.Z` (same version as `package.json`) runs
`.github/workflows/release.yml`. That publishes a universal `.app` and DMG
from `macos-14`, a `.deb` from `ubuntu-24.04`, and an NSIS installer from
`windows-latest`. The workflow refuses to publish an unsigned macOS build. It
needs the same Developer ID certificate and App Store Connect API key used by
GitRonimo in these repository secrets:

- `DEVELOPER_ID_APPLICATION`
- `MACOS_CERTIFICATE_BASE64`
- `MACOS_CERTIFICATE_PASSWORD`
- `APPLE_API_KEY_BASE64`
- `APPLE_API_KEY_ID`
- `APPLE_API_ISSUER`

Tauri signs the universal app with hardened runtime and notarizes it; the
workflow also notarizes and staples the final DMG. Both macOS artifacts must
pass `codesign`, `stapler`, and Gatekeeper verification before publication.
The Linux `.deb` and Windows installer are attached to the same release
without Authenticode or Apple signing.
