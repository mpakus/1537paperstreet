# Install

Minimum macOS version is 12.0 (Monterey), Apple Silicon or Intel.

**From a GitHub Release** (the usual way to try the app): download the
universal DMG, the Linux `.deb`, or the Windows NSIS installer from
[Releases](https://github.com/mpakus/1537paperstreet/releases).
Release 0.3.0 and earlier are not Developer ID–signed, so Gatekeeper will block
a normal double-click of those macOS builds. First launch for one of those
versions: Right-click → Open, or after copying to Applications:

```sh
xattr -cr /Applications/1537paperstreet.app
```

New macOS releases are published only after Developer ID signing, Apple
notarization, and Gatekeeper verification succeed for both the app and DMG.
The same tag also publishes an unsigned Linux `.deb` (x86_64, WebKitGTK 4.1)
and an unsigned Windows NSIS installer (WebView2). On Linux, install with
`sudo apt install ./1537paperstreet_*_amd64.deb`. On Windows, run the
`*-setup.exe`. PDF export and installing an update over the running app stay
on macOS; Check for Updates on Linux and Windows opens the release page.

**From source** (development):

1. Install Rust (rustup, stable), Node.js 22+, and Xcode Command Line Tools.
2. Clone the repository.
3. `npm install`
4. `./bin/test.dev` — development window (`cargo tauri dev` still works).
   `./bin/build host` — `.app` for the current Mac under
   `target/release/bundle`. `./bin/build` (or `./bin/build universal`) —
   universal arm64 + x86_64, macOS 12+, `.app` and DMG under
   `target/universal-apple-darwin/release/bundle`. On Linux, `./bin/build deb`.
   On Windows, `./bin/build nsis`. Also `npm run tauri:build:universal`. The
   app icon is `icon.png` at the repo root; `npx tauri icon icon.png` writes
   PNG/ICNS into `crates/ps-app/icons/`. A `v*` tag on GitHub builds the
   universal DMG, the `.deb`, and the NSIS installer, and publishes a Release.

Closing the window (red traffic light) does not quit: a menu-bar icon
(`icon-system.png`) stays on the right of the macOS menu bar. Click it to show
the window; Quit in that menu or ⌘Q exits fully.

File → About 1537paperstreet (and About in the application menu) opens a sheet
with the logo, version, and a link to [aomega.co](https://aomega.co).
Check for Updates in that sheet and File → Check for Updates… ask GitHub
Releases. On macOS, if a newer version exists, the app downloads the universal
zip, installs it over the running `.app`, and restarts. On Linux and Windows
the same check opens the release page. Settings → Updates can skip the launch
check. Homebrew cask is not available yet.
