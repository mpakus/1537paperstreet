use std::collections::HashMap;
use std::fs::{self, File, FileTimes, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{RenderOptions, render_with_options};

const MEMORY_DOCUMENTS: usize = 16;
const MEMORY_BYTES: usize = 64 * 1024 * 1024;
const DISK_BYTES: u64 = 200 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// An error produced while reading or updating the disposable render cache.
#[derive(Debug, thiserror::Error)]
#[error("Couldn't {action} the render cache at '{}'.", path.display())]
pub struct CacheError {
    action: &'static str,
    path: PathBuf,
    #[source]
    source: io::Error,
}

impl CacheError {
    fn new(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self {
            action,
            path: path.into(),
            source,
        }
    }
}

/// A result returned by render-cache operations.
pub type CacheResult<T> = std::result::Result<T, CacheError>;

struct MemoryEntry {
    html: String,
    last_used: u64,
}

/// Two-level LRU cache for rendered HTML.
pub struct RenderCache {
    directory: PathBuf,
    memory: HashMap<String, MemoryEntry>,
    memory_bytes: usize,
    clock: u64,
}

impl RenderCache {
    /// Opens a cache rooted at `directory` and prunes stale disk entries.
    pub fn new(directory: impl Into<PathBuf>) -> CacheResult<Self> {
        let directory = directory.into();
        fs::create_dir_all(&directory)
            .map_err(|source| CacheError::new("create", &directory, source))?;
        prune_disk(&directory)?;
        Ok(Self {
            directory,
            memory: HashMap::new(),
            memory_bytes: 0,
            clock: 0,
        })
    }

    /// Returns cached HTML or renders, stores, and returns a cache miss.
    pub fn render(&mut self, markdown: &str, options: RenderOptions) -> CacheResult<String> {
        let key = cache_key(markdown, options);
        if let Some(html) = self.memory_get(&key) {
            return Ok(html);
        }

        let path = self.directory.join(format!("{key}.html"));
        match fs::read_to_string(&path) {
            Ok(html) => {
                touch(&path)?;
                self.memory_insert(key, html.clone());
                return Ok(html);
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) if source.kind() == io::ErrorKind::InvalidData => {
                fs::remove_file(&path).map_err(|source| {
                    CacheError::new("remove an invalid entry from", &path, source)
                })?;
            }
            Err(source) => return Err(CacheError::new("read", path, source)),
        }

        let html = render_with_options(markdown, options);
        atomic_write(&path, html.as_bytes())?;
        prune_disk(&self.directory)?;
        self.memory_insert(key, html.clone());
        Ok(html)
    }

    fn memory_get(&mut self, key: &str) -> Option<String> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.memory.get_mut(key)?;
        entry.last_used = self.clock;
        Some(entry.html.clone())
    }

    fn memory_insert(&mut self, key: String, html: String) {
        self.memory_insert_with_limits(key, html, MEMORY_DOCUMENTS, MEMORY_BYTES);
    }

    fn memory_insert_with_limits(
        &mut self,
        key: String,
        html: String,
        document_limit: usize,
        byte_limit: usize,
    ) {
        let html_bytes = html.len();
        if html_bytes > byte_limit {
            return;
        }

        self.clock = self.clock.wrapping_add(1);
        if let Some(previous) = self.memory.insert(
            key,
            MemoryEntry {
                last_used: self.clock,
                html,
            },
        ) {
            self.memory_bytes -= previous.html.len();
        }
        self.memory_bytes += html_bytes;

        while self.memory.len() > document_limit || self.memory_bytes > byte_limit {
            let Some(oldest) = self
                .memory
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(removed) = self.memory.remove(&oldest) {
                self.memory_bytes -= removed.html.len();
            }
        }
    }
}

fn cache_key(markdown: &str, options: RenderOptions) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"1537paperstreet-render-v3-continuous-markers\0");
    hasher.update(&[u8::from(options.allow_raw_html)]);
    hasher.update(markdown.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn touch(path: &Path) -> CacheResult<()> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|source| CacheError::new("open", path, source))?;
    file.set_times(FileTimes::new().set_modified(SystemTime::now()))
        .map_err(|source| CacheError::new("update", path, source))
}

fn prune_disk(directory: &Path) -> CacheResult<()> {
    let entries =
        fs::read_dir(directory).map_err(|source| CacheError::new("read", directory, source))?;
    let mut files = Vec::new();
    let mut bytes = 0_u64;

    for entry in entries {
        let entry = entry.map_err(|source| CacheError::new("read", directory, source))?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "html") {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|source| CacheError::new("inspect", &path, source))?;
        if !metadata.is_file() {
            continue;
        }
        bytes = bytes.saturating_add(metadata.len());
        files.push((
            metadata.modified().unwrap_or(UNIX_EPOCH),
            path,
            metadata.len(),
        ));
    }

    files.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    for (_, path, length) in files {
        if bytes <= DISK_BYTES {
            break;
        }
        fs::remove_file(&path)
            .map_err(|source| CacheError::new("remove an old entry from", &path, source))?;
        bytes = bytes.saturating_sub(length);
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> CacheResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let (temporary_path, mut file) = temporary_file(parent)?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temporary_path, path));
    if let Err(source) = result {
        drop(file);
        let _ = fs::remove_file(&temporary_path);
        return Err(CacheError::new("write", path, source));
    }
    Ok(())
}

fn temporary_file(directory: &Path) -> CacheResult<(PathBuf, File)> {
    loop {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            ".1537paperstreet.cache-{}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(CacheError::new(
                    "create a temporary file in",
                    directory,
                    source,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RenderCache;

    #[test]
    fn memory_lru_also_respects_its_byte_limit() {
        let mut cache = RenderCache {
            directory: std::path::PathBuf::new(),
            memory: std::collections::HashMap::new(),
            memory_bytes: 0,
            clock: 0,
        };

        cache.memory_insert_with_limits("first".to_owned(), "123456".to_owned(), 16, 10);
        cache.memory_insert_with_limits("second".to_owned(), "abcdef".to_owned(), 16, 10);

        assert_eq!(cache.memory.len(), 1);
        assert_eq!(cache.memory_bytes, 6);
        assert!(cache.memory.contains_key("second"));
    }
}
