# ADR-011: Linux and Windows release packages

- Status: accepted
- Date: 2026-09-28

## Context

The app shell is Tauri 2, which can target Linux (WebKitGTK) and Windows
(WebView2). v1 shipped only a signed universal macOS app. WKWebView PDF export,
the overlay title bar, and in-app replacement of a `.app` bundle do not exist
on the other hosts.

## Decision

A version tag builds three artifacts: the existing notarized macOS `.app` and
DMG, an x86_64 `.deb`, and an unsigned Windows NSIS installer. Linux and
Windows use a normal decorated window. Reveal, open-in-editor, and http links
go through the host (`open`, `xdg-open` / FileManager1, Explorer / `rundll32`).
PDF export and in-app install stay macOS-only (ADR-009). On Linux and Windows,
Check for Updates opens the GitHub release page.

## Alternatives

- Keep shipping macOS only. Rejected: the same tag should offer the other
  desktops.
- Reimplement PDF with WebKitGTK and WebView2 in this change. Deferred; the
  command returns a clear error instead of compiling WKWebView into those builds.
- Code-sign the Windows installer in the same workflow. Deferred until an
  Authenticode certificate is available. The macOS job still refuses an
  unsigned Apple build.

## Consequences

`ps-app` compiles on Linux and Windows. The custom toolbar remains; traffic-light
padding is only applied on macOS. App data stays in `~/.1537paperstreet`, using
`HOME` or `USERPROFILE`.
