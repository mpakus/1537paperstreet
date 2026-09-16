//! GitHub Releases lookup used by the optional in-app update check.

use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

use crate::{Error, Result};

const RELEASES_HOME: &str = "https://github.com/mpakus/1537paperstreet/releases";
const RELEASE_URL_PREFIX: &str = "https://github.com/mpakus/1537paperstreet/releases/";
const DOWNLOAD_URL_PREFIX: &str = "https://github.com/mpakus/1537paperstreet/releases/download/";

/// Result of comparing the running app to the latest GitHub Release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
pub struct UpdateCheck {
    /// Whether `latest` is newer than `current`.
    pub available: bool,
    /// Whether the latest release includes a trusted macOS zip the app can install.
    pub can_install: bool,
    /// Version of the running app, without a `v` prefix.
    pub current: String,
    /// Latest GitHub Release version, without a `v` prefix.
    pub latest: String,
    /// HTTPS GitHub Releases URL to open when an update is available.
    pub release_url: String,
    /// HTTPS GitHub asset URL for the macOS universal zip; empty when install is not possible.
    pub asset_url: String,
    /// Lowercase hex SHA-256 of that zip; empty when GitHub omitted a digest.
    pub asset_sha256: String,
    /// User-facing status line; the UI displays this without further formatting.
    pub message: String,
}

/// Result of replacing the running app with a downloaded GitHub Release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
pub struct UpdateInstall {
    /// Installed GitHub Release version, without a `v` prefix.
    pub version: String,
    /// User-facing status line; the UI displays this without further formatting.
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

/// Parses a GitHub Releases JSON payload and compares it to `current`.
pub fn from_github_json(current: &str, json: &str) -> Result<UpdateCheck> {
    let release: GithubRelease =
        serde_json::from_str(json).map_err(|_| Error::InvalidUpdateRelease)?;
    let current_version = parse_version(current)?;
    let latest_version = parse_version(&release.tag_name)?;
    let latest = display_version(&release.tag_name).to_owned();
    let available = latest_version > current_version;
    let release_url = if available {
        trusted_release_url(&release.html_url)
    } else {
        String::new()
    };
    let (asset_url, asset_sha256) = if available {
        macos_zip_asset(&latest, &release.assets)
    } else {
        (String::new(), String::new())
    };
    let can_install = available && !asset_url.is_empty() && !asset_sha256.is_empty();
    let message = if available {
        format!("Version {latest} is available.")
    } else {
        format!("You're up to date ({current}).")
    };
    Ok(UpdateCheck {
        available,
        can_install,
        current: current.to_owned(),
        latest,
        release_url,
        asset_url,
        asset_sha256,
        message,
    })
}

/// Status line after the downloaded app has replaced the running bundle.
pub fn install_ready_message(version: &str) -> String {
    format!("Version {version} is installed. Restart to finish the update.")
}

fn macos_zip_asset(version: &str, assets: &[GithubAsset]) -> (String, String) {
    let expected_name = macos_zip_name(version);
    for asset in assets {
        if asset.name != expected_name {
            continue;
        }
        let Some(sha256) = parse_sha256_digest(asset.digest.as_deref().unwrap_or("")) else {
            return (String::new(), String::new());
        };
        if !is_trusted_asset_url(&asset.browser_download_url, version, &expected_name) {
            return (String::new(), String::new());
        }
        return (asset.browser_download_url.clone(), sha256);
    }
    (String::new(), String::new())
}

fn macos_zip_name(version: &str) -> String {
    format!("1537paperstreet-{version}-macos-universal.zip")
}

fn parse_sha256_digest(digest: &str) -> Option<String> {
    let hex = digest
        .strip_prefix("sha256:")
        .or_else(|| digest.strip_prefix("SHA256:"))
        .unwrap_or(digest)
        .trim();
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(hex.to_ascii_lowercase())
}

fn display_version(tag: &str) -> &str {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
}

fn parse_version(tag: &str) -> Result<(u64, u64, u64)> {
    let trimmed = display_version(tag.trim());
    let mut parts = trimmed.split('.');
    let major = parse_numeric_prefix(parts.next())?;
    let minor = match parts.next() {
        Some(part) => parse_numeric_prefix(Some(part))?,
        None => 0,
    };
    let patch = match parts.next() {
        Some(part) => parse_numeric_prefix(Some(part))?,
        None => 0,
    };
    Ok((major, minor, patch))
}

fn parse_numeric_prefix(part: Option<&str>) -> Result<u64> {
    let Some(part) = part.filter(|value| !value.is_empty()) else {
        return Err(Error::InvalidUpdateVersion);
    };
    let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return Err(Error::InvalidUpdateVersion);
    }
    digits.parse().map_err(|_| Error::InvalidUpdateVersion)
}

fn trusted_release_url(url: &str) -> String {
    if is_trusted_release_url(url) {
        url.to_owned()
    } else {
        RELEASES_HOME.to_owned()
    }
}

fn is_trusted_release_url(url: &str) -> bool {
    if url.is_empty()
        || url
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == b'\0')
    {
        return false;
    }
    url.starts_with(RELEASE_URL_PREFIX) && !url[RELEASE_URL_PREFIX.len()..].contains("..")
}

fn is_trusted_asset_url(url: &str, version: &str, file_name: &str) -> bool {
    if url.is_empty()
        || url
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == b'\0')
        || url.contains("..")
    {
        return false;
    }
    let tagged = format!("{DOWNLOAD_URL_PREFIX}v{version}/{file_name}");
    let plain = format!("{DOWNLOAD_URL_PREFIX}{version}/{file_name}");
    url == tagged || url == plain
}
