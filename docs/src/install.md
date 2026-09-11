# Install

Minimum macOS version is 12.0 (Monterey), Apple Silicon or Intel.

**From a GitHub Release** (the usual way to try the app): download the
universal DMG from [Releases](https://github.com/mpakus/1537paperstreet/releases).
Release 0.3.0 and earlier are not Developer ID–signed, so Gatekeeper will block
a normal double-click. First launch for one of those versions: Right-click →
Open, or after copying to Applications:

```sh
xattr -cr /Applications/1537paperstreet.app
```

New releases are published only after Developer ID signing, Apple
notarization, and Gatekeeper verification succeed for both the app and DMG.

**From source** (development):

1. Install Rust (rustup, stable), Node.js 22+, and Xcode Command Line Tools.
2. Clone the repository.
3. `npm install`
4. `./bin/test.dev` — development window (`cargo tauri dev` still works).
   `./bin/build host` — `.app` for the current architecture under
   `target/release/bundle`. `./bin/build` (or `./bin/build universal`) —
   universal arm64 + x86_64, macOS 12+, `.app` and DMG under
   `target/universal-apple-darwin/release/bundle`. Also
   `npm run tauri:build:universal`. The app icon is
   `icon.png` at the repo root; `npx tauri icon icon.png` writes PNG/ICNS into
   `crates/ps-app/icons/`. A `v*` tag on GitHub builds the same universal DMG
   and publishes a Release.

Closing the window (red traffic light) does not quit: a menu-bar icon
(`icon-system.png`) stays on the right of the macOS menu bar. Click it to show
the window; Quit in that menu or ⌘Q exits fully.

File → About 1537paperstreet (and About in the application menu) opens a sheet
with the logo, version, and a link to [aomega.co](https://aomega.co).
Check for Updates in that sheet and File → Check for Updates… ask GitHub
Releases and open the download page if a newer version exists.

Homebrew cask and in-app install of updates are not available yet.
