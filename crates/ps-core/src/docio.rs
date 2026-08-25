//! Document reading and writing: encoding, BOM, line endings, and atomic saves.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Error, Result, fsops};

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// Files larger than this open as source-only and skip Markdown rendering.
pub const SOURCE_ONLY_BYTES: u64 = 8 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Line ending used by a document on disk.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    /// Unix `\n`.
    Lf,
    /// Windows `\r\n`.
    CrLf,
}

/// Text encoding detected while reading a document.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DocumentEncoding {
    /// Valid UTF-8, with or without a BOM.
    Utf8,
    /// Bytes that are not valid UTF-8.
    Binary,
}

/// On-disk traits restored when encoding a buffer back to bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RestoreTraits {
    /// Line ending to write.
    pub eol: LineEnding,
    /// Whether to prepend a UTF-8 BOM.
    pub bom: bool,
    /// Whether the file should end with a newline.
    pub trailing_newline: bool,
}

impl RestoreTraits {
    /// Copies BOM, EOL, and trailing-newline flags from a loaded source.
    pub fn from_source(source: &DocumentSource) -> Self {
        Self {
            eol: source.eol,
            bom: source.bom,
            trailing_newline: source.trailing_newline,
        }
    }
}

/// Result of [`write_doc`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WrittenDocument {
    /// Lowercase hexadecimal BLAKE3 hash of the on-disk bytes.
    pub hash: String,
    /// Size of the on-disk file in bytes.
    pub size: u64,
    /// True when the encoded buffer already matched the file, so nothing was written.
    pub skipped: bool,
}

/// A heading shown in the document table of contents.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    /// Heading depth from 1 through 6.
    pub level: u8,
    /// Plain-text heading label.
    pub title: String,
    /// Unique HTML identifier assigned to the heading.
    pub id: String,
}

/// Source text and on-disk traits returned by `doc_source`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSource {
    /// Decoded text with LF newlines. Empty when the file is not valid UTF-8.
    pub text: String,
    /// Dominant line ending in the file.
    pub eol: LineEnding,
    /// Whether the file began with a UTF-8 BOM.
    pub bom: bool,
    /// Whether the file ended with a newline.
    pub trailing_newline: bool,
    /// Detected encoding.
    pub encoding: DocumentEncoding,
    /// Whether the document can be edited and saved.
    pub writable: bool,
    /// Why the document is read-only, when it is.
    pub readonly_reason: Option<String>,
}

/// Opening metadata returned by `doc_open`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMeta {
    /// Registered project that owns the file.
    pub project_id: String,
    /// NFC-normalized path relative to the project root.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// First heading title, or the file stem when the document has none.
    pub title: String,
    /// Lowercase hexadecimal BLAKE3 hash of the on-disk bytes.
    pub hash: String,
    /// Size of the on-disk file in bytes.
    pub size: u64,
    /// Whether the document can be edited and saved.
    pub writable: bool,
    /// Why the document is read-only or source-only, when it is.
    pub readonly_reason: Option<String>,
    /// Whether Markdown rendering was skipped.
    pub source_only: bool,
    /// Number of HTML chunks produced for the viewer.
    pub chunk_count: u32,
    /// Headings in source order.
    pub toc: Vec<TocEntry>,
}

/// Synchronous `doc_open` payload: metadata plus the first HTML chunk.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocOpenResult {
    /// Document identity, hash, and table of contents.
    pub meta: DocumentMeta,
    /// First viewer chunk, when the document was rendered.
    pub first_chunk: Option<String>,
}

/// Remaining HTML sent on `doc://chunk` after `doc_open` returns.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocChunkEvent {
    /// Registered project that owns the file.
    pub project_id: String,
    /// NFC-normalized path relative to the project root.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// Zero-based chunk index. Index `0` is returned by `doc_open`.
    pub index: u32,
    /// One `<section class="chunk">` fragment.
    pub html: String,
}

/// Terminal event on `doc://done` after every chunk has been sent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocDoneEvent {
    /// Registered project that owns the file.
    pub project_id: String,
    /// NFC-normalized path relative to the project root.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// Number of HTML chunks produced for the viewer.
    pub chunk_count: u32,
}

/// A document loaded from a project folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedDocument {
    /// Source text and on-disk traits.
    pub source: DocumentSource,
    /// Lowercase hexadecimal BLAKE3 hash of the on-disk bytes.
    pub hash: String,
    /// Size of the on-disk file in bytes.
    pub size: u64,
    /// Whether Markdown rendering should be skipped.
    pub source_only: bool,
}

/// Reads a project document through [`fsops::resolve`].
///
/// ```
/// use std::fs;
/// use std::path::Path;
/// use ps_core::docio::read_doc;
///
/// let dir = tempfile::tempdir().unwrap();
/// fs::write(dir.path().join("note.md"), b"hello\n").unwrap();
/// let doc = read_doc(dir.path(), Path::new("note.md")).unwrap();
/// assert_eq!(doc.source.text, "hello\n");
/// assert!(doc.source.trailing_newline);
/// assert!(!doc.source_only);
/// ```
pub fn read_doc(project_root: &Path, rel_path: &Path) -> Result<LoadedDocument> {
    let path = fsops::resolve(project_root, rel_path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|source| Error::io("open the document", &path, source))?;
    if metadata.is_dir() {
        return Err(Error::UnsafePath {
            path: rel_path.to_path_buf(),
            reason: "folders cannot be opened as documents",
        });
    }

    let bytes = fs::read(&path).map_err(|source| Error::io("read the document", &path, source))?;
    let writable_on_disk = is_writable(&path);
    let bom = bytes.starts_with(UTF8_BOM);
    let payload = if bom { &bytes[3..] } else { bytes.as_slice() };
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let too_large = size > SOURCE_ONLY_BYTES;
    let eol = detect_eol(payload);
    let trailing_newline = payload.ends_with(b"\n");
    let (encoding, text, reason) = match std::str::from_utf8(payload) {
        Ok(text) => {
            let reason = if too_large {
                Some(String::from(
                    "This file is larger than 8 MB, so it opens as source only.",
                ))
            } else if !writable_on_disk {
                Some(String::from(
                    "This file cannot be saved because it is not writable.",
                ))
            } else {
                None
            };
            (DocumentEncoding::Utf8, decode_text(text, eol), reason)
        }
        Err(_) => (
            DocumentEncoding::Binary,
            String::new(),
            Some(String::from(
                "This file is not valid UTF-8, so it can only be viewed as source.",
            )),
        ),
    };

    Ok(LoadedDocument {
        source: DocumentSource {
            eol,
            bom,
            trailing_newline,
            encoding,
            writable: writable_on_disk && reason.is_none(),
            readonly_reason: reason,
            text,
        },
        hash: blake3::hash(&bytes).to_hex().to_string(),
        size,
        source_only: encoding == DocumentEncoding::Binary || too_large,
    })
}

/// On-disk identity of a document, without decoding its text.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentStat {
    /// Lowercase hexadecimal BLAKE3 hash of the on-disk bytes.
    pub hash: String,
    /// Size of the on-disk file in bytes.
    pub size: u64,
}

/// Hashes a project document through [`fsops::resolve`] without decoding text.
///
/// ```
/// use std::fs;
/// use std::path::Path;
/// use ps_core::docio::{read_doc, stat_doc};
///
/// let dir = tempfile::tempdir().unwrap();
/// fs::write(dir.path().join("note.md"), b"hello\n").unwrap();
/// let loaded = read_doc(dir.path(), Path::new("note.md")).unwrap();
/// let stat = stat_doc(dir.path(), Path::new("note.md")).unwrap();
/// assert_eq!(stat.hash, loaded.hash);
/// assert_eq!(stat.size, loaded.size);
/// ```
pub fn stat_doc(project_root: &Path, rel_path: &Path) -> Result<DocumentStat> {
    let path = fsops::resolve(project_root, rel_path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|source| Error::io("open the document", &path, source))?;
    if metadata.is_dir() {
        return Err(Error::UnsafePath {
            path: rel_path.to_path_buf(),
            reason: "folders cannot be opened as documents",
        });
    }

    let bytes = fs::read(&path).map_err(|source| Error::io("read the document", &path, source))?;
    Ok(DocumentStat {
        hash: blake3::hash(&bytes).to_hex().to_string(),
        size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    })
}

/// Writes a document through [`fsops::resolve`], restoring BOM, EOL, and a trailing newline.
///
/// The write is skipped when the encoded buffer already matches the file. When the on-disk
/// BLAKE3 hash does not match `base_hash`, the file is left untouched and
/// [`Error::DocumentConflict`] is returned.
///
/// ```
/// use std::fs;
/// use std::path::Path;
/// use ps_core::docio::{read_doc, write_doc, RestoreTraits};
///
/// let dir = tempfile::tempdir().unwrap();
/// fs::write(dir.path().join("note.md"), b"hello\r\n").unwrap();
/// let doc = read_doc(dir.path(), Path::new("note.md")).unwrap();
/// write_doc(
///     dir.path(),
///     Path::new("note.md"),
///     &doc.source.text,
///     &doc.hash,
///     RestoreTraits::from_source(&doc.source),
/// )
/// .unwrap();
/// assert_eq!(fs::read(dir.path().join("note.md")).unwrap(), b"hello\r\n");
/// ```
pub fn write_doc(
    project_root: &Path,
    rel_path: &Path,
    text: &str,
    base_hash: &str,
    traits: RestoreTraits,
) -> Result<WrittenDocument> {
    let path = fsops::resolve(project_root, rel_path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|source| Error::io("open the document", &path, source))?;
    if metadata.is_dir() {
        return Err(Error::UnsafePath {
            path: rel_path.to_path_buf(),
            reason: "folders cannot be opened as documents",
        });
    }

    let disk = fs::read(&path).map_err(|source| Error::io("read the document", &path, source))?;
    let encoded = encode_document(text, traits);
    let size = u64::try_from(encoded.len()).unwrap_or(u64::MAX);
    let hash = blake3::hash(&encoded).to_hex().to_string();

    if encoded == disk {
        return Ok(WrittenDocument {
            hash,
            size,
            skipped: true,
        });
    }

    let disk_hash = blake3::hash(&disk).to_hex().to_string();
    if disk_hash != base_hash {
        return Err(Error::DocumentConflict {
            path: rel_path.to_path_buf(),
            disk_hash,
        });
    }

    atomic_replace(&path, &encoded, metadata.permissions())?;
    Ok(WrittenDocument {
        hash,
        size,
        skipped: false,
    })
}

fn decode_text(text: &str, eol: LineEnding) -> String {
    if eol == LineEnding::CrLf {
        text.replace("\r\n", "\n")
    } else {
        text.to_owned()
    }
}

fn encode_document(text: &str, traits: RestoreTraits) -> Vec<u8> {
    let mut normalized = String::from(text);
    if traits.trailing_newline && !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    let body = match traits.eol {
        LineEnding::Lf => normalized,
        LineEnding::CrLf => normalized.replace('\n', "\r\n"),
    };
    let mut bytes = Vec::with_capacity(body.len() + usize::from(traits.bom) * UTF8_BOM.len());
    if traits.bom {
        bytes.extend_from_slice(UTF8_BOM);
    }
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

fn detect_eol(bytes: &[u8]) -> LineEnding {
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => return LineEnding::Lf,
            b'\r' => {
                return if bytes.get(index + 1) == Some(&b'\n') {
                    LineEnding::CrLf
                } else {
                    LineEnding::Lf
                };
            }
            _ => index += 1,
        }
    }
    LineEnding::Lf
}

fn is_writable(path: &Path) -> bool {
    OpenOptions::new().write(true).open(path).is_ok()
}

fn atomic_replace(path: &Path, bytes: &[u8], permissions: fs::Permissions) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::UnsafePath {
        path: path.to_path_buf(),
        reason: "the file does not have a parent directory",
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::UnsafePath {
            path: path.to_path_buf(),
            reason: "file names must be valid Unicode",
        })?;
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        counter
    ));
    let mut cleanup = TemporaryFile::new(temporary.clone());

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| Error::io("create a temporary document file", &temporary, source))?;
    file.write_all(bytes)
        .map_err(|source| Error::io("write the document", &temporary, source))?;
    fs::set_permissions(&temporary, permissions)
        .map_err(|source| Error::io("preserve the document permissions", &temporary, source))?;
    file.sync_all()
        .map_err(|source| Error::io("sync the document", &temporary, source))?;
    drop(file);

    fs::rename(&temporary, path)
        .map_err(|source| Error::io("replace the document", path, source))?;
    cleanup.disarm();
    Ok(())
}

struct TemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl TemporaryFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod coverage {
    use super::*;
    use std::fs;

    #[test]
    fn armed_temporary_file_is_removed_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scratch.tmp");
        fs::write(&path, b"x").unwrap();
        drop(TemporaryFile::new(path.clone()));
        assert!(!path.exists());
    }

    #[test]
    fn disarmed_temporary_file_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scratch.tmp");
        fs::write(&path, b"x").unwrap();
        let mut tmp = TemporaryFile::new(path.clone());
        tmp.disarm();
        drop(tmp);
        assert_eq!(fs::read(&path).unwrap(), b"x");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_rejects_a_non_unicode_file_name() {
        use std::os::unix::ffi::OsStrExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(std::ffi::OsStr::from_bytes(b"bad\xff.md"));
        let permissions = fs::metadata(dir.path()).unwrap().permissions();
        let error = atomic_replace(&path, b"x", permissions).expect_err("unicode");
        assert!(error.to_string().contains("valid Unicode"));
    }

    #[test]
    fn atomic_replace_rejects_a_path_without_a_parent() {
        let permissions = fs::metadata("/").unwrap().permissions();
        let error = atomic_replace(Path::new("/"), b"x", permissions).expect_err("parent");
        assert!(error.to_string().contains("parent directory"));
    }
}
