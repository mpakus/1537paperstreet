# Dependency security validation — T-226

Date: 2026-10-29. User-approved remediation of the CI blockers found while
preparing Tags Cloud 0.12.0. No user documents were modified.

## Markdown parser

[RUSTSEC-2026-0414](https://rustsec.org/advisories/RUSTSEC-2026-0414) affects
pulldown-cmark 0.13.4 when wiki links are enabled, as they are in this app.
The regression renders ten `[[a|]]` links followed by unique text: the old
parser repeats that text 1,024 times, while patched 0.13.5 emits it once.
The regression failed before the update and passed afterward. Graph cache
filenames move from v1 to v2 to rebuild links parsed with the old behavior.

Criterion measurements on local arm64 macOS, 10 samples with 1 second warm-up
and 2 seconds measurement, using the existing render benchmark:

| Document | Before 0.13.4 | After 0.13.5 |
| --- | --- | --- |
| 100 KiB | 1.3477 ms | 1.3524 ms |
| 1 MiB | 14.203 ms | 14.296 ms |

Criterion reported the first difference within its noise threshold and no
significant change for the second. This is local comparative evidence, not the
PLAN M1 acceptance rig. Command: `cargo bench -p ps-render --bench render --
'render markdown/document/(100 KiB|1 MiB)' --sample-size 10 --warm-up-time 1
--measurement-time 2`, with saved before/after baselines.

## Encryption dependency

[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387) covers timing
variability in crypto-bigint. age 0.12.1 unconditionally brings the affected
0.5.5 version through P-256/HPKE. The app only invokes binary v1/scrypt;
review found no route from those operations to the affected P-256 arithmetic.
Nevertheless, T-226 removes that dependency entirely using published age 0.11.5,
with default features still disabled. No cryptographic code is forked or patched.

The [upstream changelog](https://github.com/str4d/rage/blob/main/age/CHANGELOG.md)
records the same July 14, 2026 armor fixes in 0.11.5 and 0.12.1. Source comparison
found the scrypt implementation identical apart from constant visibility;
production protocol and primitive encryption code are unchanged. The binary
file format, passwords, resource limits, authentication and atomic writer remain
unchanged. No conversion of existing notes is required.

- A committed public fixture was encrypted through the app with age 0.12.1
  before changing dependencies. A regression decrypts it byte-for-byte with
  0.11.5, preserving BOM, Unicode and mixed line endings.
- A temporary app helper linked to 0.11.5 encrypted the same synthetic text;
  the saved helper linked to 0.12.1 decrypted it byte-for-byte. The temporary
  generator was removed from the repository.
- Existing crypto and encrypted-document tests pass, including no-op ciphertext
  preservation, conflicts, failed saves, recovery, password changes and export.

## Maintenance tradeoff

age 0.11.5 uses i18n-embed-fl 0.9.4, whose build-only proc-macro-error2 2.0.1
has [RUSTSEC-2026-0173](https://rustsec.org/advisories/RUSTSEC-2026-0173).
This is an unmaintained-status advisory with no patched release or reported
vulnerability. `deny.toml` has an explicit exception for that advisory only,
review due **2026-11-29**. Remove it when a published age version avoids both
this build macro and the affected crypto-bigint chain. Cargo also reports its
future-compiler incompatibility warning; current pinned-toolchain checks pass.
An unpublished or custom cryptography patch was rejected as a larger risk.

Neither vulnerability is suppressed. The existing basic-toml maintenance
exception remains applicable. Duplicate-version policy removes obsolete
const-oid/hybrid-array entries and precisely allows rustc-hash 1.1.0, required
by older Fluent alongside the 2.x version used by Tauri/intl-memoizer.

## Verification scope

### npm audit follow-up

After the Rust gate passed, CI reached npm audit and reported previously
unreached JavaScript findings. Each update was isolated and checked:

- Mermaid keeps its existing version; only its resolved DOMPurify dependency
  moves from 3.4.16 to [3.4.17](https://github.com/cure53/DOMPurify/releases/tag/3.4.17).
  The patched sanitizer fixes the template/shadow-root, in-place and XHTML SMIL
  advisories. Browser checks used the real local Mermaid import in strict mode:
  normal diagrams rendered, hostile labels lost event handlers, and the XHTML
  SVG animation guard passed. The console had no errors/warnings. The temporary
  fixture and dev server were removed afterward.
- [Happy DOM 21](https://github.com/capricorn86/happy-dom/releases/tag/v21.0.0)
  disables iframe/CSS loading by default; those safer defaults remain enabled.
  The test dependency is pinned to patched 21.0.4 after reviewing the major
  release notes and passing the component tests and type checks.
- Vitest and its matching coverage provider move together from 4.1.11 to
  [4.1.12](https://github.com/vitest-dev/vitest/releases/tag/v4.1.12), which
  hardens manual/redirect mocks. No production dependency is added.

Install scripts were disabled during each targeted update. npm audit now reports
**zero critical/high/moderate** findings, with the existing high-severity CI gate
unchanged. Two low findings remain for `deepmerge` and its build-only Svelte
plugin consumer ([GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4)).
No fixed release is available. The merge inputs here are repository-controlled
build configuration, not Markdown or user document data. Review by **2026-11-29**;
adopt a fixed upstream release when available. No advisory suppression was added
for npm. UI checks, coverage, lint and build passed with the patched tooling.

### Completed checks

Local checks cover the repository's Rust/UI Definition of Done, documentation,
dependency audits, targeted regressions, bidirectional format compatibility and
render performance: **390 Rust tests passed** (two existing ignored) and
**209 UI tests passed**, with formatting, Clippy, documentation, UI type checks,
lint and build passing. Cargo audit reports zero vulnerabilities; Cargo deny
passes advisories, bans, licenses and sources with the documented exceptions.
CI separately checks Linux, Windows, coverage, optimized
builds and security. No new native GUI, signing or installer validation is claimed.
