//! Project images: intrinsic sizes in Markdown, and a preview page for image files.

use std::fs;
use std::io::Read;
use std::path::{Component, Path};

use ps_core::fsops;
use ps_core::projects::is_image_path;

/// One HTML chunk that shows a project image through `asset://`.
///
/// The file bytes stay on disk. SVG is loaded as an image, so scripts in the
/// file do not run. A path that is not an image, or that leaves the project,
/// becomes a short message instead of a tag.
///
/// ```
/// use std::path::Path;
/// use ps_render::render_image;
///
/// let html = render_image("project-1", Path::new("notes/cover image.png"));
/// assert!(html.contains("src=\"asset://localhost/project-1/notes/cover%20image.png\""));
/// assert!(html.contains("alt=\"cover image.png\""));
/// assert!(html.contains("class=\"image-file\""));
/// ```
#[must_use]
pub fn render_image(project_scope: &str, rel_path: &Path) -> String {
    let Some(src) = image_src(project_scope, rel_path) else {
        return String::from(
            "<section class=\"chunk\"><p class=\"image-file\">This image cannot be shown.</p></section>\n",
        );
    };
    let alt = escape_text(
        rel_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Image"),
    );
    format!(
        "<section class=\"chunk\"><p class=\"image-file\"><img src=\"{src}\" alt=\"{alt}\"></p></section>\n"
    )
}

fn image_src(project_scope: &str, rel_path: &Path) -> Option<String> {
    if !is_image_path(rel_path)
        || project_scope.is_empty()
        || !project_scope
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return None;
    }

    let mut encoded = String::new();
    let mut parts = 0_usize;
    for component in rel_path.components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_str()?;
                if name.is_empty() || name == "." || name == ".." {
                    return None;
                }
                if parts > 0 {
                    encoded.push('/');
                }
                encode_segment(&mut encoded, name);
                parts += 1;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if parts == 0 {
        return None;
    }
    Some(format!("asset://localhost/{project_scope}/{encoded}"))
}

fn encode_segment(output: &mut String, value: &str) {
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                output.push(char::from(byte));
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Adds intrinsic `width`/`height` (and lazy loading) to project `asset://` images.
pub(crate) fn reserve_sizes(html: &str, project_root: &Path, project_scope: &str) -> String {
    let mut output = String::with_capacity(html.len().saturating_add(64));
    let mut rest = html;
    while let Some(offset) = rest.find("<img") {
        output.push_str(&rest[..offset]);
        let after = &rest[offset..];
        let Some(end) = after.find('>') else {
            output.push_str(after);
            return output;
        };
        let tag = &after[..=end];
        output.push_str(&with_reserved_size(tag, project_root, project_scope));
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    output
}

fn with_reserved_size(tag: &str, project_root: &Path, project_scope: &str) -> String {
    if tag.contains(" width=") && tag.contains(" height=") {
        return tag.to_owned();
    }
    let Some(src) = attr(tag, "src") else {
        return tag.to_owned();
    };
    let Some(relative) = asset_relative_path(&src, project_scope) else {
        return tag.to_owned();
    };
    let Ok(absolute) = fsops::resolve(project_root, Path::new(&relative)) else {
        return tag.to_owned();
    };
    let Ok(bytes) = read_prefix(&absolute, 8192) else {
        return tag.to_owned();
    };
    let Some((width, height)) = dimensions(&bytes) else {
        return tag.to_owned();
    };

    let mut attrs = String::new();
    if !tag.contains(" width=") {
        attrs.push_str(&format!(" width=\"{width}\""));
    }
    if !tag.contains(" height=") {
        attrs.push_str(&format!(" height=\"{height}\""));
    }
    if !tag.contains(" loading=") {
        attrs.push_str(" loading=\"lazy\"");
    }
    if !tag.contains(" decoding=") {
        attrs.push_str(" decoding=\"async\"");
    }

    insert_before_close(tag, &attrs)
}

fn insert_before_close(tag: &str, attrs: &str) -> String {
    if let Some(stripped) = tag.strip_suffix("/>") {
        format!("{stripped}{attrs} />")
    } else if let Some(stripped) = tag.strip_suffix('>') {
        format!("{stripped}{attrs}>")
    } else {
        tag.to_owned()
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let rest = tag.get(start..)?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn asset_relative_path(src: &str, project_scope: &str) -> Option<String> {
    let prefix = format!("asset://localhost/{project_scope}/");
    let rest = src.strip_prefix(&prefix)?;
    let path = rest.split(['?', '#']).next().unwrap_or("");
    if path.is_empty() || path.contains("..") {
        return None;
    }
    percent_decode(path)
}

fn percent_decode(input: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '%' if index + 2 < chars.len() => {
                let hex: String = chars[index + 1..index + 3].iter().collect();
                bytes.push(u8::from_str_radix(&hex, 16).ok()?);
                index += 3;
            }
            character => {
                let mut buffer = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                index += 1;
            }
        }
    }
    String::from_utf8(bytes).ok()
}

fn read_prefix(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = vec![0_u8; max];
    let read = file.read(&mut bytes)?;
    bytes.truncate(read);
    Ok(bytes)
}

fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    png_size(bytes)
        .or_else(|| gif_size(bytes))
        .or_else(|| webp_size(bytes))
        .or_else(|| jpeg_size(bytes))
}

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || !bytes.starts_with(SIGNATURE) {
        return None;
    }
    Some((u32_be(bytes, 16)?, u32_be(bytes, 20)?))
}

fn gif_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 10 || !(bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return None;
    }
    Some((u16_le(bytes, 6)? as u32, u16_le(bytes, 8)? as u32))
}

fn webp_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 30 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WEBP" {
        return None;
    }
    match &bytes[12..16] {
        b"VP8X" if bytes.len() >= 30 => Some((u24_le(bytes, 24)? + 1, u24_le(bytes, 27)? + 1)),
        b"VP8 " if bytes.len() >= 30 => {
            Some((u16_le(bytes, 26)? as u32, u16_le(bytes, 28)? as u32))
        }
        b"VP8L" if bytes.len() >= 25 => {
            let bits = u32_le(bytes, 21)?;
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        _ => None,
    }
}

fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut index = 2;
    while index + 8 < bytes.len() {
        if bytes[index] != 0xFF {
            return None;
        }
        let marker = bytes[index + 1];
        index += 2;
        if marker == 0xD8 || marker == 0xD9 {
            continue;
        }
        let length = u16_be(bytes, index)? as usize;
        if matches!(marker, 0xC0..=0xC2) && index + 6 < bytes.len() {
            return Some((
                u16_be(bytes, index + 5)? as u32,
                u16_be(bytes, index + 3)? as u32,
            ));
        }
        index = index.checked_add(length)?;
    }
    None
}

fn u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u16_be(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn u24_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 3)?;
    Some(u32::from(slice[0]) | (u32::from(slice[1]) << 8) | (u32::from(slice[2]) << 16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_header_dimensions() {
        let mut bytes = vec![0_u8; 24];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[16..20].copy_from_slice(&800_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&600_u32.to_be_bytes());
        assert_eq!(png_size(&bytes), Some((800, 600)));
    }
}
