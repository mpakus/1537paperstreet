//! Fetches, verifies, and installs the latest GitHub Release.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ps_core::updates::{UpdateCheck, UpdateInstall, install_ready_message};
use sha2::{Digest, Sha256};

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/mpakus/1537paperstreet/releases/latest";
const USER_AGENT: &str = concat!(
    "1537paperstreet/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mpakus/1537paperstreet)"
);
const MAX_JSON_BYTES: u64 = 256 * 1024;
const MAX_ZIP_BYTES: u64 = 80 * 1024 * 1024;
const APP_BUNDLE_NAME: &str = "1537paperstreet.app";
const APP_EXECUTABLE_NAME: &str = "1537paperstreet";
const APP_BUNDLE_ID: &str = "org.paperstreet1537.reader";

/// Downloads the GitHub `releases/latest` JSON body.
pub(crate) fn fetch_latest_release_json() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .redirects(0)
        .build();
    let response = agent
        .get(LATEST_RELEASE_URL)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|_| unreachable_message())?;
    if response.status() != 200 {
        return Err(unreachable_message());
    }
    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_JSON_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|_| unreachable_message())?;
    if body.len() as u64 > MAX_JSON_BYTES {
        return Err("GitHub did not return a release.".to_owned());
    }
    Ok(body)
}

/// Downloads the latest macOS zip, verifies it, and replaces the running `.app`.
pub(crate) fn install_latest_update() -> Result<UpdateInstall, String> {
    let body = fetch_latest_release_json()?;
    let check = ps_core::updates::from_github_json(env!("CARGO_PKG_VERSION"), &body)
        .map_err(|error| error.to_string())?;
    install_checked_update(&check)
}

fn install_checked_update(check: &UpdateCheck) -> Result<UpdateInstall, String> {
    if !check.available {
        return Err(check.message.clone());
    }
    if !check.can_install {
        return Err(
            "This release cannot be installed from the app. Open the download page instead."
                .to_owned(),
        );
    }
    let current_app = bundled_app_path(&current_exe()?)?;
    let work = work_dir()?;
    let _cleanup = RemovePath(work.clone());
    let zip_path = work.join("update.zip");
    download_release_zip(&check.asset_url, &zip_path)?;
    verify_sha256(&zip_path, &check.asset_sha256)?;
    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted)
        .map_err(|source| io_message("prepare the update", &extracted, source))?;
    extract_zip(&zip_path, &extracted)?;
    let new_app = find_app_bundle(&extracted)?;
    verify_app_bundle(&new_app, true)?;
    replace_app_bundle(&new_app, &current_app)?;
    Ok(UpdateInstall {
        version: check.latest.clone(),
        message: install_ready_message(&check.latest),
    })
}

fn current_exe() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|source| io_message("find the running app", Path::new(APP_BUNDLE_NAME), source))
}

fn bundled_app_path(exe: &Path) -> Result<PathBuf, String> {
    let macos = exe
        .parent()
        .filter(|path| path.file_name() == Some("MacOS".as_ref()));
    let contents = macos.and_then(Path::parent);
    let app = contents.and_then(Path::parent);
    match app {
        Some(path)
            if path.extension() == Some("app".as_ref())
                && path.file_name() == Some(APP_BUNDLE_NAME.as_ref()) =>
        {
            Ok(path.to_path_buf())
        }
        _ => Err(
            "Install updates from the released app in Applications, not from a development build."
                .to_owned(),
        ),
    }
}

fn work_dir() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("1537paperstreet-update-{}", std::process::id()));
    if path.exists() {
        fs::remove_dir_all(&path)
            .map_err(|source| io_message("clear the update folder", &path, source))?;
    }
    fs::create_dir_all(&path)
        .map_err(|source| io_message("create the update folder", &path, source))?;
    Ok(path)
}

fn download_release_zip(url: &str, dest: &Path) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(180))
        .redirects(8)
        .build();
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/octet-stream")
        .call()
        .map_err(|_| unreachable_message())?;
    if response.status() != 200 {
        return Err(unreachable_message());
    }
    let mut file =
        File::create(dest).map_err(|source| io_message("save the update", dest, source))?;
    let mut reader = response.into_reader().take(MAX_ZIP_BYTES + 1);
    let copied = io::copy(&mut reader, &mut file)
        .map_err(|source| io_message("download the update", dest, source))?;
    if copied > MAX_ZIP_BYTES {
        return Err("The update download was larger than expected.".to_owned());
    }
    if copied == 0 {
        return Err("GitHub did not return a release.".to_owned());
    }
    file.sync_all()
        .map_err(|source| io_message("save the update", dest, source))?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let mut file =
        File::open(path).map_err(|source| io_message("read the update", path, source))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| io_message("read the update", path, source))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hex_lower(&hasher.finalize());
    if actual != expected {
        return Err("The downloaded update did not match the GitHub checksum.".to_owned());
    }
    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn extract_zip(zip: &Path, dest: &Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(zip)
        .arg(dest)
        .status()
        .map_err(|source| io_message("unpack the update", zip, source))?;
    if status.success() {
        return Ok(());
    }
    Err("Couldn't unpack the update.".to_owned())
}

fn find_app_bundle(root: &Path) -> Result<PathBuf, String> {
    let nested = root.join(APP_BUNDLE_NAME);
    if nested.is_dir() {
        return Ok(nested);
    }
    let mut found = Vec::new();
    collect_app_bundles(root, &mut found)?;
    match found.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err("The update archive did not contain 1537paperstreet.app.".to_owned()),
        _ => Err("The update archive contained more than one app.".to_owned()),
    }
}

fn collect_app_bundles(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|source| io_message("read the update", dir, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| io_message("read the update", dir, source))?;
        let path = entry.path();
        if path.file_name() == Some(APP_BUNDLE_NAME.as_ref()) && path.is_dir() {
            found.push(path);
        } else if path.is_dir() {
            collect_app_bundles(&path, found)?;
        }
    }
    Ok(())
}

fn verify_app_bundle(app: &Path, require_signature: bool) -> Result<(), String> {
    let executable = app.join("Contents").join("MacOS").join(APP_EXECUTABLE_NAME);
    if !executable.is_file() {
        return Err("The downloaded app is missing its executable.".to_owned());
    }
    let identifier = bundle_identifier(app)
        .ok_or_else(|| "The downloaded app is missing a bundle identifier.".to_owned())?;
    if identifier != APP_BUNDLE_ID {
        return Err("The downloaded app is not 1537paperstreet.".to_owned());
    }
    if require_signature {
        verify_code_signature(app)?;
    }
    Ok(())
}

fn bundle_identifier(app: &Path) -> Option<String> {
    let plist = fs::read_to_string(app.join("Contents/Info.plist")).ok()?;
    let key = plist.find("<key>CFBundleIdentifier</key>")?;
    let after = &plist[key..];
    let start = after.find("<string>")? + "<string>".len();
    let end = after[start..].find("</string>")?;
    let identifier = after[start..start + end].trim();
    if identifier.is_empty() {
        None
    } else {
        Some(identifier.to_owned())
    }
}

fn verify_code_signature(app: &Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app)
        .status()
        .map_err(|source| io_message("verify the update signature", app, source))?;
    if status.success() {
        return Ok(());
    }
    Err("The downloaded app is not a valid signed update.".to_owned())
}

fn replace_app_bundle(new_app: &Path, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().ok_or_else(|| {
        "Couldn't replace 1537paperstreet. Move it to Applications and try again.".to_owned()
    })?;
    let incoming = parent.join("1537paperstreet.app.incoming");
    let previous = parent.join("1537paperstreet.app.previous");
    if incoming.exists() {
        fs::remove_dir_all(&incoming)
            .map_err(|source| io_message("clear the incoming app", &incoming, source))?;
    }
    if previous.exists() {
        fs::remove_dir_all(&previous)
            .map_err(|source| io_message("clear the previous app", &previous, source))?;
    }
    copy_app_bundle(new_app, &incoming)?;
    if dest.exists() {
        fs::rename(dest, &previous)
            .map_err(|source| io_message("replace 1537paperstreet", dest, source))?;
    }
    if let Err(source) = fs::rename(&incoming, dest) {
        if previous.exists() {
            let _ = fs::rename(&previous, dest);
        }
        return Err(io_message("replace 1537paperstreet", dest, source));
    }
    let _ = fs::remove_dir_all(&previous);
    Ok(())
}

fn copy_app_bundle(from: &Path, to: &Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/ditto")
        .arg(from)
        .arg(to)
        .status()
        .map_err(|source| io_message("copy the update", from, source))?;
    if status.success() {
        return Ok(());
    }
    Err("Couldn't copy the update into place.".to_owned())
}

fn io_message(action: &str, path: &Path, source: io::Error) -> String {
    format!("Couldn't {action} at '{}': {source}", path.display())
}

fn unreachable_message() -> String {
    "Couldn't reach GitHub to check for updates.".to_owned()
}

struct RemovePath(PathBuf);

impl Drop for RemovePath {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn bundled_app_path_accepts_a_release_layout() {
        let exe = PathBuf::from("/Applications/1537paperstreet.app/Contents/MacOS/1537paperstreet");
        let app = bundled_app_path(&exe).expect("bundle");
        assert_eq!(app, PathBuf::from("/Applications/1537paperstreet.app"));
    }

    #[test]
    fn bundled_app_path_rejects_a_dev_binary() {
        let exe = PathBuf::from("/Users/me/downey.jr/target/debug/1537paperstreet");
        let error = bundled_app_path(&exe).expect_err("dev");
        assert!(error.contains("released app"));
    }

    #[test]
    fn hex_lower_encodes_sha256_bytes() {
        assert_eq!(hex_lower(&[0x01, 0xab, 0xff]), "01abff");
    }

    #[test]
    fn verify_app_bundle_accepts_a_matching_unsigned_fixture() {
        let root = tempfile::tempdir().expect("temp");
        let app = write_fake_app(root.path(), APP_BUNDLE_ID);
        verify_app_bundle(&app, false).expect("ok");
    }

    #[test]
    fn verify_app_bundle_rejects_the_wrong_identifier() {
        let root = tempfile::tempdir().expect("temp");
        let app = write_fake_app(root.path(), "com.example.other");
        let error = verify_app_bundle(&app, false).expect_err("id");
        assert!(error.contains("not 1537paperstreet"));
    }

    #[test]
    fn replace_app_bundle_swaps_the_destination() {
        let root = tempfile::tempdir().expect("temp");
        let dest = write_fake_app(root.path().join("current"), "org.paperstreet1537.reader");
        let incoming_src = write_fake_app(root.path().join("next"), "org.paperstreet1537.reader");
        fs::write(
            incoming_src
                .join("Contents")
                .join("MacOS")
                .join(APP_EXECUTABLE_NAME),
            b"new-binary",
        )
        .expect("payload");
        replace_app_bundle(&incoming_src, &dest).expect("replace");
        let installed = fs::read(
            dest.join("Contents")
                .join("MacOS")
                .join(APP_EXECUTABLE_NAME),
        )
        .expect("read");
        assert_eq!(installed, b"new-binary");
        assert!(
            !root
                .path()
                .join("current")
                .join("1537paperstreet.app.previous")
                .exists()
        );
    }

    #[test]
    fn find_app_bundle_reads_the_archive_root() {
        let root = tempfile::tempdir().expect("temp");
        let app = write_fake_app(root.path(), APP_BUNDLE_ID);
        assert_eq!(find_app_bundle(root.path()).expect("found"), app);
    }

    fn write_fake_app(parent: impl AsRef<Path>, bundle_id: &str) -> PathBuf {
        let app = parent.as_ref().join(APP_BUNDLE_NAME);
        let macos = app.join("Contents").join("MacOS");
        fs::create_dir_all(&macos).expect("macos");
        let executable = macos.join(APP_EXECUTABLE_NAME);
        fs::write(&executable, b"old-binary").expect("exe");
        let mut permissions = fs::metadata(&executable).expect("meta").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("chmod");
        fs::write(
            app.join("Contents").join("Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>{bundle_id}</string>
  <key>CFBundleExecutable</key>
  <string>1537paperstreet</string>
</dict>
</plist>
"#
            ),
        )
        .expect("plist");
        app
    }
}
