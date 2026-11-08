//! `asset://` handler scoped to registered project roots.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use tauri::http::{StatusCode, header};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;

struct ServedFile {
    status: StatusCode,
    bytes: Vec<u8>,
    mime: &'static str,
    content_range: Option<String>,
    disposition: Option<String>,
}

enum AssetBody {
    File(ServedFile),
    Unsatisfiable { total: u64 },
    Forbidden,
}

#[derive(Debug)]
enum ReadAsset {
    Unsatisfiable { total: u64 },
    Failed,
}

enum ByteRange {
    Ignored,
    Unsatisfiable,
    Span { start: u64, end_inclusive: u64 },
}

/// Loads bytes for a registered-project asset URL.
pub(crate) fn respond_asset(
    ctx: UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let uri = request.uri().clone();
    let state = ctx.app_handle().state::<AppState>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = match load_asset(&state, &uri, range.as_deref()) {
            AssetBody::File(file) => file_response(file),
            AssetBody::Unsatisfiable { total } => unsatisfiable(total),
            AssetBody::Forbidden => forbidden(),
        };
        responder.respond(response);
    });
}

fn load_asset(state: &AppState, uri: &tauri::http::Uri, range_header: Option<&str>) -> AssetBody {
    let Ok((project_id, rel_path)) = parse_asset_uri(uri) else {
        return AssetBody::Forbidden;
    };
    let Ok(absolute) = state.resolve_asset(&project_id, &rel_path) else {
        return AssetBody::Forbidden;
    };
    match read_ranged(&absolute, range_header) {
        Ok(file) => AssetBody::File(file),
        Err(ReadAsset::Unsatisfiable { total }) => AssetBody::Unsatisfiable { total },
        Err(ReadAsset::Failed) => AssetBody::Forbidden,
    }
}

fn read_ranged(path: &Path, range_header: Option<&str>) -> Result<ServedFile, ReadAsset> {
    let mut file = fs::File::open(path).map_err(|_| ReadAsset::Failed)?;
    let total = file.metadata().map_err(|_| ReadAsset::Failed)?.len();
    let mime = mime_for(path);
    let disposition = content_disposition(path);
    let header = range_header.filter(|value| !value.is_empty());
    let Some(header) = header else {
        let bytes = read_all(&mut file)?;
        return Ok(served(StatusCode::OK, bytes, mime, None, disposition));
    };
    match parse_byte_range(header, total) {
        ByteRange::Ignored => {
            let bytes = read_all(&mut file)?;
            Ok(served(StatusCode::OK, bytes, mime, None, disposition))
        }
        ByteRange::Unsatisfiable => Err(ReadAsset::Unsatisfiable { total }),
        ByteRange::Span {
            start,
            end_inclusive,
        } => {
            let len = usize::try_from(end_inclusive - start + 1).map_err(|_| ReadAsset::Failed)?;
            file.seek(SeekFrom::Start(start))
                .map_err(|_| ReadAsset::Failed)?;
            let mut bytes = vec![0; len];
            file.read_exact(&mut bytes).map_err(|_| ReadAsset::Failed)?;
            Ok(served(
                StatusCode::PARTIAL_CONTENT,
                bytes,
                mime,
                Some(format!("bytes {start}-{end_inclusive}/{total}")),
                disposition,
            ))
        }
    }
}

fn read_all(file: &mut fs::File) -> Result<Vec<u8>, ReadAsset> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ReadAsset::Failed)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| ReadAsset::Failed)?;
    Ok(bytes)
}

fn served(
    status: StatusCode,
    bytes: Vec<u8>,
    mime: &'static str,
    content_range: Option<String>,
    disposition: Option<String>,
) -> ServedFile {
    ServedFile {
        status,
        bytes,
        mime,
        content_range,
        disposition,
    }
}

fn parse_byte_range(header: &str, total: u64) -> ByteRange {
    let header = header.trim();
    let Some(spec) = header.strip_prefix("bytes=") else {
        return ByteRange::Ignored;
    };
    let spec = spec.split(',').next().unwrap_or(spec).trim();
    let Some((start_text, end_text)) = spec.split_once('-') else {
        return ByteRange::Ignored;
    };
    if total == 0 {
        return ByteRange::Unsatisfiable;
    }
    if start_text.is_empty() {
        let Ok(suffix) = end_text.parse::<u64>() else {
            return ByteRange::Ignored;
        };
        if suffix == 0 {
            return ByteRange::Unsatisfiable;
        }
        let len = suffix.min(total);
        return ByteRange::Span {
            start: total - len,
            end_inclusive: total - 1,
        };
    }
    let Ok(start) = start_text.parse::<u64>() else {
        return ByteRange::Ignored;
    };
    if start >= total {
        return ByteRange::Unsatisfiable;
    }
    let end_inclusive = if end_text.is_empty() {
        total - 1
    } else {
        match end_text.parse::<u64>() {
            Ok(end) => end.min(total - 1),
            Err(_) => return ByteRange::Ignored,
        }
    };
    if end_inclusive < start {
        return ByteRange::Unsatisfiable;
    }
    ByteRange::Span {
        start,
        end_inclusive,
    }
}

fn content_disposition(path: &Path) -> Option<String> {
    if mime_for(path) != "application/pdf" {
        return None;
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.is_empty() || name.contains(['"', '\\', '\n', '\r']) {
        return Some(String::from("inline"));
    }
    Some(format!("inline; filename=\"{name}\""))
}

fn file_response(file: ServedFile) -> tauri::http::Response<Vec<u8>> {
    let mut builder = tauri::http::Response::builder()
        .status(file.status)
        .header(header::CONTENT_TYPE, file.mime)
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, file.bytes.len().to_string());
    if let Some(range) = &file.content_range {
        builder = builder.header(header::CONTENT_RANGE, range);
    }
    if let Some(disposition) = &file.disposition {
        builder = builder.header(header::CONTENT_DISPOSITION, disposition);
    }
    builder.body(file.bytes).unwrap_or_else(|_| forbidden())
}

fn unsatisfiable(total: u64) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_RANGE, format!("bytes */{total}"))
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Vec::new())
        .unwrap_or_else(|_| forbidden())
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

    #[test]
    fn parse_byte_range_covers_open_closed_and_suffix_forms() {
        assert!(matches!(
            parse_byte_range("bytes=0-3", 10),
            ByteRange::Span {
                start: 0,
                end_inclusive: 3
            }
        ));
        assert!(matches!(
            parse_byte_range("bytes=4-", 10),
            ByteRange::Span {
                start: 4,
                end_inclusive: 9
            }
        ));
        assert!(matches!(
            parse_byte_range("bytes=-2", 10),
            ByteRange::Span {
                start: 8,
                end_inclusive: 9
            }
        ));
        assert!(matches!(
            parse_byte_range("bytes=10-20", 10),
            ByteRange::Unsatisfiable
        ));
        assert!(matches!(
            parse_byte_range("items=0-1", 10),
            ByteRange::Ignored
        ));
    }

    #[test]
    fn read_ranged_returns_a_slice_and_marks_pdfs_inline() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("note.pdf");
        fs::write(&path, b"0123456789").expect("pdf bytes");
        let full = read_ranged(&path, None).expect("full read");
        assert_eq!(full.status, StatusCode::OK);
        assert_eq!(full.bytes, b"0123456789");
        assert!(full.disposition.expect("disposition").contains("inline"));

        let part = read_ranged(&path, Some("bytes=2-5")).expect("range");
        assert_eq!(part.status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.bytes, b"2345");
        assert_eq!(part.content_range.as_deref(), Some("bytes 2-5/10"));
        assert!(matches!(
            read_ranged(&path, Some("bytes=10-")),
            Err(ReadAsset::Unsatisfiable { total: 10 })
        ));
    }
}
