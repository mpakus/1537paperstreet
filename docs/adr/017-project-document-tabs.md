# ADR-017: Document tabs belong to projects

Status: Accepted for T-232.

## Context

The workspace discarded all document tabs when the active project changed.
Relative paths alone could also collide when two folders contained the same
filename. File drops need to identify the owning folder before adding it to
Projects.

## Decision

The tab strip identifies a document by its registered project ID and relative
path. Selecting a tab activates its project in the sidebar. The app snapshots
the outgoing editor buffer before switching so saving and closing a background
tab use its own project and base hash. A project switch still locks unlocked
encrypted sessions through the existing confirmation flow.

The session records an optional project ID per tab. Older tabs inherit the
session's project ID. On startup, the app validates each tab against its own
registered root before reopening it. Session records contain paths and view
modes.

For an external drop, Rust resolves canonical paths and reports which project
folders would be new. The native confirmation passes the approved roots back to
Rust, which rechecks the paths before registration and opens each accepted file.

YAML formatting uses Pretty YAML to preserve comments and anchors.
`toml_edit` keeps TOML comments and value spelling while the app normalizes
spacing. Configuration formatting targets the editor buffer. Save uses the
existing atomic, hash-checked write.
