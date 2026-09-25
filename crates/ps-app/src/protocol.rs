//! `asset://` handler scoped to registered project roots.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::http::{StatusCode, header};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;

/// Loads bytes for a registered-project asset URL.
pub(crate) fn respond_asset(
    ctx: UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let uri = request.uri().clone();
    let state = ctx.app_handle().state::<AppState>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = match load_asset(&state, &uri) {
            Ok((bytes, mime)) => tauri::http::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "no-cache")
                .body(bytes)
                .unwrap_or_else(|_| forbidden()),
            Err(()) => forbidden(),
        };
        responder.respond(response);
    });
}

fn load_asset(state: &AppState, uri: &tauri::http::Uri) -> Result<(Vec<u8>, &'static str), ()> {
    let (project_id, rel_path) = parse_asset_uri(uri)?;
    let absolute = state
        .resolve_asset(&project_id, &rel_path)
        .map_err(|_| ())?;
    let bytes = fs::read(&absolute).map_err(|_| ())?;
    Ok((bytes, mime_for(&absolute)))
}

fn parse_asset_uri(uri: &tauri::http::Uri) -> Result<(String, PathBuf), ()> {
    if uri.scheme_str() != Some("asset") {
        return Err(());
    }
    let path = percent_decode(uri.path().trim_start_matches('/'))?;
    let mut parts = path.splitn(2, '/');
    let project_id = parts.next().filter(|id| !id.is_empty()).ok_or(())?;
    if !project_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(());
    }
    let relative = parts.next().unwrap_or("");
    if relative.is_empty() {
        return Err(());
    }
    Ok((project_id.to_owned(), PathBuf::from(relative)))
}

fn percent_decode(input: &str) -> Result<String, ()> {
    let mut bytes = Vec::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '%' if index + 2 < chars.len() => {
                let hex: String = chars[index + 1..index + 3].iter().collect();
                let value = u8::from_str_radix(&hex, 16).map_err(|_| ())?;
                bytes.push(value);
                index += 3;
            }
            '+' => {
                bytes.push(b' ');
                index += 1;
            }
            character => {
                let mut buffer = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                index += 1;
            }
        }
    }
    String::from_utf8(bytes).map_err(|_| ())
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("avif") => "image/avif",
        Some("tif" | "tiff") => "image/tiff",
        Some("pdf") => "application/pdf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("md" | "markdown" | "mdown" | "mdwn" | "txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn forbidden() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Vec::new())
        .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_asset_uri_reads_project_and_relative_path() {
        let uri: tauri::http::Uri = "asset://localhost/01ABC/notes/cover%20image.png"
            .parse()
            .expect("uri");
        let (project_id, rel) = parse_asset_uri(&uri).expect("parsed");
        assert_eq!(project_id, "01ABC");
        assert_eq!(rel, PathBuf::from("notes/cover image.png"));
    }

    #[test]
    fn parse_asset_uri_rejects_missing_project_or_file() {
        let root: tauri::http::Uri = "asset://localhost/".parse().expect("uri");
        assert!(parse_asset_uri(&root).is_err());
        let file: tauri::http::Uri = "https://example.com/x.png".parse().expect("uri");
        assert!(parse_asset_uri(&file).is_err());
        let plus: tauri::http::Uri = "asset://localhost/01ABC/a+b.png".parse().expect("uri");
        let (_, rel) = parse_asset_uri(&plus).expect("plus");
        assert_eq!(rel, PathBuf::from("a b.png"));
        let bad: tauri::http::Uri = "asset://localhost/01ABC/%zz.png".parse().expect("uri");
        assert!(parse_asset_uri(&bad).is_err());
    }

    #[test]
    fn mime_for_maps_common_extensions() {
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a.JPG")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.gif")), "image/gif");
        assert_eq!(mime_for(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(mime_for(Path::new("a.bmp")), "image/bmp");
        assert_eq!(mime_for(Path::new("a.avif")), "image/avif");
        assert_eq!(mime_for(Path::new("a.tiff")), "image/tiff");
        assert_eq!(mime_for(Path::new("a.pdf")), "application/pdf");
        assert_eq!(mime_for(Path::new("a.mp4")), "video/mp4");
        assert_eq!(mime_for(Path::new("a.webm")), "video/webm");
        assert_eq!(mime_for(Path::new("a.mp3")), "audio/mpeg");
        assert_eq!(mime_for(Path::new("a.wav")), "audio/wav");
        assert_eq!(mime_for(Path::new("a.md")), "text/plain; charset=utf-8");
        assert_eq!(
            mime_for(Path::new("a.markdown")),
            "text/plain; charset=utf-8"
        );
        assert_eq!(mime_for(Path::new("a.txt")), "text/plain; charset=utf-8");
        assert_eq!(mime_for(Path::new("a.bin")), "application/octet-stream");
        let dotted: tauri::http::Uri = "asset://localhost/bad.id/a.png".parse().expect("uri");
        assert!(parse_asset_uri(&dotted).is_err());
    }
}
