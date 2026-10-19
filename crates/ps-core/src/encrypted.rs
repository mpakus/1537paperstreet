//! Encrypted document sessions and ciphertext-only atomic file replacement.
//!
//! Sessions hold secrets only in memory. A single mutex serializes scrypt work
//! and document writes. Files are checked again after encryption, immediately
//! before replacement. Existing destinations are never removed first.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use age::secrecy::SecretString;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeroize::Zeroizing;

use crate::crypto::{self, DocumentFormat};
use crate::docio::{self, DocumentStat, LoadedDocument, RestoreTraits, WrittenDocument};
use crate::{Error, Result, fsops};

/// Lifecycle state sent to the UI, never containing a password.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DocumentSecurity {
    /// Ordinary document.
    #[default]
    Plain,
    /// Encrypted file requiring authentication.
    Locked,
    /// Authenticated encrypted file with an in-memory session.
    Unlocked,
}

struct Session {
    passphrase: SecretString,
    plaintext: Zeroizing<Vec<u8>>,
    hash: String,
    original: Option<ConversionSource>,
}

struct ConversionSource {
    rel: PathBuf,
    hash: String,
    encrypted_hash: String,
}

/// In-memory sessions. Drop or [`Self::lock_all`] wipes owned secret buffers.
#[derive(Default)]
pub struct EncryptedDocuments {
    sessions: Mutex<HashMap<PathBuf, Session>>,
}

/// Any age extension is excluded from the ordinary text writer and search.
pub fn is_age_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("age"))
}

/// A supported Markdown extension followed by `.age`.
pub fn is_encrypted_markdown(path: &Path) -> bool {
    is_age_path(path)
        && path
            .file_stem()
            .and_then(|stem| Path::new(stem).extension())
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                ["md", "markdown", "mdown", "mdwn"]
                    .iter()
                    .any(|supported| extension.eq_ignore_ascii_case(supported))
            })
}

impl EncryptedDocuments {
    fn sessions(&self) -> Result<MutexGuard<'_, HashMap<PathBuf, Session>>> {
        self.sessions.lock().map_err(|_| Error::DocumentLocked)
    }

    /// Requires sessions to be closed before changing a registered project root.
    /// No disk access is needed, so unavailable projects can still be relocated.
    pub fn ensure_all_locked(&self) -> Result<()> {
        if self.sessions()?.is_empty() {
            Ok(())
        } else {
            Err(Error::UnsafePath {
                path: PathBuf::from("."),
                reason: "lock or close encrypted notes before relocating a project",
            })
        }
    }

    /// Prevents filesystem operations from invalidating an unlocked session.
    pub fn ensure_locked(&self, root: &Path, rel: &Path) -> Result<()> {
        let sessions = self.sessions()?;
        if sessions.is_empty() {
            return Ok(());
        }
        let path = fsops::resolve(root, rel)?;
        if sessions.keys().any(|open| open.starts_with(&path)) {
            return Err(Error::UnsafePath {
                path: rel.into(),
                reason: "lock or close encrypted notes before moving, renaming, removing, or replacing them",
            });
        }
        Ok(())
    }

    /// Converts a current plaintext buffer, preserving original bytes when unchanged.
    /// The source is hash-checked and kept until an explicit [`Self::delete_original`].
    #[allow(clippy::too_many_arguments)]
    pub fn encrypt_existing(
        &self,
        root: &Path,
        rel: &Path,
        destination: &Path,
        text: &str,
        base_hash: &str,
        traits: RestoreTraits,
        passphrase: SecretString,
    ) -> Result<()> {
        if is_age_path(rel) || !crate::projects::is_markdown_path(rel) {
            return Err(Error::NotAgeFile);
        }
        let path = resolve_regular(root, rel)?;
        let bytes = Zeroizing::new(read_ciphertext(&path)?);
        check_hash(rel, &bytes, base_hash)?;
        let source =
            docio::decode_document_bytes(&bytes, true, base_hash.into(), bytes.len() as u64).source;
        let encoded = if source.text == text && RestoreTraits::from_source(&source) == traits {
            bytes
        } else {
            Zeroizing::new(docio::encode_document(text, traits))
        };
        let utf8 = std::str::from_utf8(&encoded).map_err(|_| Error::InvalidEncryptedUtf8)?;
        self.create_with_original(
            root,
            destination,
            utf8,
            passphrase,
            Some((rel.into(), base_hash.into())),
        )
    }

    /// Deletes a conversion's original after preserving an encrypted recovery copy.
    /// Requires explicit user confirmation. Both files must still match conversion;
    /// failed verification or recovery leaves the original untouched.
    pub fn delete_original(&self, root: &Path, rel: &Path) -> Result<()> {
        self.delete_original_with(root, rel, |root, recovery, bytes, passphrase| {
            persist_encrypted(root, recovery, bytes, passphrase, None).map(|_| ())
        })
    }

    fn delete_original_with(
        &self,
        root: &Path,
        rel: &Path,
        snapshot: impl FnOnce(&Path, &Path, &[u8], &SecretString) -> Result<()>,
    ) -> Result<()> {
        let encrypted_path = resolve(root, rel)?;
        let mut sessions = self.sessions()?;
        let session = sessions
            .get_mut(&encrypted_path)
            .ok_or(Error::DocumentLocked)?;
        let original = session.original.as_ref().ok_or_else(|| Error::UnsafePath {
            path: rel.into(),
            reason: "there is no original awaiting deletion for this encrypted copy",
        })?;
        let source_path = resolve_regular(root, &original.rel)?;
        let lock = File::open(&source_path)
            .map_err(|source| Error::io("open the original note", &source_path, source))?;
        lock.try_lock().map_err(|source| {
            Error::io(
                "lock the original note before deletion",
                &source_path,
                source.into(),
            )
        })?;
        let bytes = Zeroizing::new(read_ciphertext(&source_path)?);
        check_hash(&original.rel, &bytes, &original.hash)?;
        check_hash(
            rel,
            &read_ciphertext(&encrypted_path)?,
            &original.encrypted_hash,
        )?;
        let name = original
            .rel
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::NotAgeFile)?;
        let recovery = original.rel.with_file_name(format!(
            ".{name}.pre_encrypt_delete-{}.md.age",
            ulid::Ulid::generate()
        ));
        snapshot(root, &recovery, &bytes, &session.passphrase)?;
        fsops::permanently_delete(root, std::slice::from_ref(&original.rel), |_| {
            // Encryption can take seconds. Re-resolve and recheck both files
            // immediately before deletion, without trusting the earlier reads.
            let current_source = resolve_regular(root, &original.rel)?;
            let current_encrypted = resolve(root, rel)?;
            check_hash(
                &original.rel,
                &read_ciphertext(&current_source)?,
                &original.hash,
            )?;
            check_hash(
                rel,
                &read_ciphertext(&current_encrypted)?,
                &original.encrypted_hash,
            )
        })?;
        session.original = None;
        Ok(())
    }

    /// Creates an encrypted recovery copy before discarding unsaved text.
    pub fn snapshot_before_discard(
        &self,
        root: &Path,
        rel: &Path,
        text: &str,
        traits: RestoreTraits,
    ) -> Result<()> {
        let path = resolve(root, rel)?;
        let sessions = self.sessions()?;
        let session = sessions.get(&path).ok_or(Error::DocumentLocked)?;
        let plaintext = Zeroizing::new(docio::encode_document(text, traits));
        let name = rel
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::NotAgeFile)?;
        let recovery = rel.with_file_name(format!(
            ".{name}.pre_discard-{}.md.age",
            ulid::Ulid::generate()
        ));
        persist_encrypted(root, &recovery, &plaintext, &session.passphrase, None).map(|_| ())
    }

    /// Hashes bounded ciphertext without decoding or exposing its contents.
    pub fn stat(&self, root: &Path, rel: &Path) -> Result<DocumentStat> {
        let bytes = read_ciphertext(&resolve(root, rel)?)?;
        Ok(DocumentStat {
            hash: hash(&bytes),
            size: bytes.len() as u64,
        })
    }

    /// Validates the header before offering to unlock; no decryption is performed.
    pub fn status(&self, root: &Path, rel: &Path) -> Result<DocumentSecurity> {
        let path = resolve(root, rel)?;
        validate_header(&read_ciphertext(&path)?)?;
        Ok(if self.sessions()?.contains_key(&path) {
            DocumentSecurity::Unlocked
        } else {
            DocumentSecurity::Locked
        })
    }

    /// Authenticates a complete document before installing its session.
    pub fn unlock(&self, root: &Path, rel: &Path, passphrase: SecretString) -> Result<()> {
        let path = resolve(root, rel)?;
        let mut sessions = self.sessions()?;
        let bytes = read_ciphertext(&path)?;
        let plaintext = crypto::decrypt_markdown(&bytes, &passphrase)?;
        sessions.insert(
            path,
            Session {
                passphrase,
                plaintext,
                hash: hash(&bytes),
                original: None,
            },
        );
        Ok(())
    }

    /// Forgets one session, even if its file was removed externally.
    pub fn lock(&self, root: &Path, rel: &Path) -> Result<()> {
        let path = fsops::resolve(root, rel)?;
        self.sessions()?.remove(&path);
        Ok(())
    }

    /// Forgets every password and decrypted source (project switch/window close).
    pub fn lock_all(&self) -> Result<()> {
        self.sessions()?.clear();
        Ok(())
    }

    /// Reads a session, re-authenticating externally changed ciphertext before use.
    pub fn read(&self, root: &Path, rel: &Path) -> Result<LoadedDocument> {
        let path = resolve(root, rel)?;
        let mut sessions = self.sessions()?;
        let session = sessions.get_mut(&path).ok_or(Error::DocumentLocked)?;
        let bytes = read_ciphertext(&path)?;
        let current = hash(&bytes);
        if current != session.hash {
            let plaintext = crypto::decrypt_markdown(&bytes, &session.passphrase)?;
            session.plaintext = plaintext;
            session.hash = current;
        }
        Ok(docio::decode_document_bytes(
            &session.plaintext,
            OpenOptions::new().write(true).open(&path).is_ok(),
            session.hash.clone(),
            bytes.len() as u64,
        ))
    }

    /// Creates a new encrypted file, refusing to overwrite any existing entry.
    pub fn create(
        &self,
        root: &Path,
        rel: &Path,
        text: &str,
        passphrase: SecretString,
    ) -> Result<()> {
        self.create_with_original(root, rel, text, passphrase, None)
    }

    fn create_with_original(
        &self,
        root: &Path,
        rel: &Path,
        text: &str,
        passphrase: SecretString,
        original: Option<(PathBuf, String)>,
    ) -> Result<()> {
        let path = resolve(root, rel)?;
        let mut sessions = self.sessions()?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(occupied(rel));
        }
        let written = persist_encrypted(root, rel, text.as_bytes(), &passphrase, None)?;
        sessions.insert(
            path,
            Session {
                passphrase,
                plaintext: Zeroizing::new(text.as_bytes().to_vec()),
                original: original.map(|(rel, hash)| ConversionSource {
                    rel,
                    hash,
                    encrypted_hash: written.hash.clone(),
                }),
                hash: written.hash,
            },
        );
        Ok(())
    }

    /// Saves with a ciphertext hash guard. A new password forces fresh encryption.
    /// A password change first retains the old ciphertext as a private `pre_*` copy.
    #[allow(clippy::too_many_arguments)]
    pub fn save(
        &self,
        root: &Path,
        rel: &Path,
        text: &str,
        base_hash: &str,
        traits: RestoreTraits,
        new_password: Option<SecretString>,
    ) -> Result<WrittenDocument> {
        let path = resolve(root, rel)?;
        let mut sessions = self.sessions()?;
        let session = sessions.get_mut(&path).ok_or(Error::DocumentLocked)?;
        // Cooperating app processes cannot both commit the same base version.
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| Error::io("open the encrypted note for saving", &path, source))?;
        lock_file.try_lock().map_err(|source| {
            Error::io(
                "save while another process is using this note",
                &path,
                source.into(),
            )
        })?;
        let disk = read_ciphertext(&path)?;
        check_hash(rel, &disk, base_hash)?;
        if session.hash != base_hash {
            return Err(Error::DocumentConflict {
                path: rel.into(),
                disk_hash: hash(&disk),
            });
        }
        let loaded = docio::decode_document_bytes(
            &session.plaintext,
            true,
            base_hash.into(),
            disk.len() as u64,
        );
        let unchanged =
            text == loaded.source.text && traits == RestoreTraits::from_source(&loaded.source);
        if unchanged && new_password.is_none() {
            return Ok(WrittenDocument {
                hash: base_hash.into(),
                size: disk.len() as u64,
                skipped: true,
            });
        }
        let plaintext = if unchanged {
            session.plaintext.clone()
        } else {
            Zeroizing::new(docio::encode_document(text, traits))
        };
        let password = new_password.as_ref().unwrap_or(&session.passphrase);
        if new_password.is_some() {
            let filename = rel
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or(Error::NotAgeFile)?;
            let backup = rel.with_file_name(format!(
                ".{filename}.pre_password_change-{base_hash}.md.age"
            ));
            let backup_path = resolve(root, &backup)?;
            if fs::symlink_metadata(&backup_path).is_ok() {
                if read_ciphertext(&backup_path)? != disk {
                    return Err(occupied(&backup));
                }
            } else {
                persist(root, &backup, &disk, None, || Ok(()))?;
            }
        }
        let result = persist_encrypted(root, rel, &plaintext, password, Some(base_hash))?;
        session.plaintext = plaintext;
        session.hash = result.hash.clone();
        if let Some(password) = new_password {
            session.passphrase = password;
        }
        Ok(result)
    }

    /// Explicitly exports the saved version as a new plaintext `.md` file.
    /// Existing files are never replaced. Call only after the UI warning is accepted.
    pub fn export(
        &self,
        root: &Path,
        rel: &Path,
        destination: &Path,
        base_hash: &str,
    ) -> Result<()> {
        if destination
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case("md"))
        {
            return Err(Error::UnsafePath {
                path: destination.into(),
                reason: "decrypted exports must use .md",
            });
        }
        let path = resolve(root, rel)?;
        let sessions = self.sessions()?;
        let session = sessions.get(&path).ok_or(Error::DocumentLocked)?;
        check_hash(rel, &read_ciphertext(&path)?, base_hash)?;
        if session.hash != base_hash {
            return Err(Error::DocumentLocked);
        }
        persist(root, destination, &session.plaintext, None, || Ok(()))
    }
}

fn occupied(rel: &Path) -> Error {
    Error::UnsafePath {
        path: rel.into(),
        reason: "the destination already exists; choose a different name",
    }
}

fn resolve(root: &Path, rel: &Path) -> Result<PathBuf> {
    if !is_encrypted_markdown(rel) {
        return Err(Error::UnsafePath {
            path: rel.into(),
            reason: "encrypted notes must keep a Markdown extension followed by .age",
        });
    }
    resolve_regular(root, rel)
}

fn resolve_regular(root: &Path, rel: &Path) -> Result<PathBuf> {
    let path = fsops::resolve(root, rel)?;
    let canonical_root = fsops::resolve(root, Path::new(""))?;
    let mut next = canonical_root.clone();
    for component in path
        .strip_prefix(&canonical_root)
        .map_err(|_| Error::PathOutsideProject { path: rel.into() })?
        .components()
    {
        next.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&next)
            && metadata.file_type().is_symlink()
        {
            return Err(Error::UnsafePath {
                path: rel.into(),
                reason: "encrypted operations do not follow symbolic links",
            });
        }
    }
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err(Error::UnsafePath {
            path: rel.into(),
            reason: "the destination is not a regular file",
        });
    }
    Ok(path)
}

fn read_ciphertext(path: &Path) -> Result<Vec<u8>> {
    let file =
        File::open(path).map_err(|source| Error::io("read the encrypted note", path, source))?;
    let mut bytes = Vec::new();
    file.take((crypto::MAX_CIPHERTEXT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| Error::io("read the encrypted note", path, source))?;
    if bytes.len() > crypto::MAX_CIPHERTEXT_BYTES {
        return Err(Error::EncryptedDocumentTooLarge);
    }
    Ok(bytes)
}

fn validate_header(bytes: &[u8]) -> Result<()> {
    match crypto::detect_format(bytes) {
        DocumentFormat::AgePassphrase => Ok(()),
        DocumentFormat::AgeUnsupportedRecipient => Err(Error::UnsupportedRecipient),
        _ => Err(Error::NotAgeFile),
    }
}

fn hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn check_hash(rel: &Path, disk: &[u8], expected: &str) -> Result<()> {
    let disk_hash = hash(disk);
    if disk_hash != expected {
        return Err(Error::DocumentConflict {
            path: rel.into(),
            disk_hash,
        });
    }
    Ok(())
}

fn persist(
    root: &Path,
    rel: &Path,
    bytes: &[u8],
    expected: Option<&str>,
    before_replace: impl FnOnce() -> Result<()>,
) -> Result<()> {
    persist_with(
        root,
        rel,
        expected,
        |temporary| {
            temporary
                .write_all(bytes)
                .map_err(|source| Error::io("write the note", rel, source))
        },
        before_replace,
    )
}

fn persist_encrypted(
    root: &Path,
    rel: &Path,
    plaintext: &[u8],
    password: &SecretString,
    expected: Option<&str>,
) -> Result<WrittenDocument> {
    persist_with(
        root,
        rel,
        expected,
        |temporary| {
            crypto::encrypt_to(plaintext, password, temporary.as_file_mut())?;
            // Authenticate the complete temporary file before committing it. This
            // also checks that the library's chosen work factor fits our policy.
            let bytes = read_ciphertext(temporary.path())?;
            let verified = crypto::decrypt_markdown(&bytes, password)?;
            if verified.as_slice() != plaintext {
                return Err(Error::EncryptionFailed);
            }
            Ok(WrittenDocument {
                hash: hash(&bytes),
                size: bytes.len() as u64,
                skipped: false,
            })
        },
        || Ok(()),
    )
}

fn persist_with<T>(
    root: &Path,
    rel: &Path,
    expected: Option<&str>,
    write: impl FnOnce(&mut tempfile::NamedTempFile) -> Result<T>,
    before_replace: impl FnOnce() -> Result<()>,
) -> Result<T> {
    let path = resolve_regular(root, rel)?;
    let parent = path.parent().ok_or(Error::NotAgeFile)?;
    // A complete orphan after a crash can be opened from the file tree using
    // the same password; partial files fail authentication. Never auto-promote.
    let recovery_name = format!(
        ".{}.recovery-",
        rel.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("note")
    );
    let suffix = if is_age_path(rel) { ".md.age" } else { ".tmp" };
    let mut temporary = tempfile::Builder::new()
        .prefix(&recovery_name)
        .suffix(suffix)
        .tempfile_in(parent)
        .map_err(|source| Error::io("create a private temporary file", parent, source))?;
    let result = write(&mut temporary)?;
    temporary
        .flush()
        .map_err(|source| Error::io("flush the encrypted note", &path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io("sync the encrypted note", &path, source))?;
    before_replace()?;
    let checked = resolve_regular(root, rel)?;
    if checked != path {
        return Err(Error::PathOutsideProject { path: rel.into() });
    }
    if let Some(expected) = expected {
        check_hash(rel, &read_ciphertext(&path)?, expected)?;
        temporary
            .persist(&path)
            .map_err(|error| Error::io("replace the encrypted note", &path, error.error))?;
    } else {
        temporary.persist_noclobber(&path).map_err(|error| {
            Error::io(
                "create the note without overwriting another file",
                &path,
                error.error,
            )
        })?;
    }
    // The replacement is already committed. A directory-sync failure must not
    // report that the old file was retained; crash durability remains OS-specific.
    #[cfg(unix)]
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_aborts_on_recovery_failure_and_changes_during_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let source = Path::new("original.md");
        let destination = Path::new("original.md.age");
        fs::write(root.path().join(source), "Saved original").unwrap();
        let loaded = docio::read_doc(root.path(), source).unwrap();
        let documents = EncryptedDocuments::default();
        documents
            .encrypt_existing(
                root.path(),
                source,
                destination,
                &loaded.source.text,
                &loaded.hash,
                RestoreTraits::from_source(&loaded.source),
                "test-only passphrase".to_owned().into(),
            )
            .unwrap();
        assert!(
            documents
                .delete_original_with(root.path(), destination, |_, _, _, _| Err(
                    Error::EncryptionFailed
                ))
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(root.path().join(source)).unwrap(),
            "Saved original"
        );
        assert!(matches!(
            documents.delete_original_with(root.path(), destination, |_, _, _, _| {
                fs::write(root.path().join(source), "Concurrent edit").unwrap();
                Ok(())
            }),
            Err(Error::DocumentConflict { .. })
        ));
        assert_eq!(
            fs::read_to_string(root.path().join(source)).unwrap(),
            "Concurrent edit"
        );
        fs::write(root.path().join(source), "Saved original").unwrap();
        assert!(matches!(
            documents.delete_original_with(root.path(), destination, |_, _, _, _| {
                fs::write(root.path().join(destination), "Changed encrypted copy").unwrap();
                Ok(())
            }),
            Err(Error::DocumentConflict { .. })
        ));
        assert_eq!(
            fs::read_to_string(root.path().join(source)).unwrap(),
            "Saved original"
        );
    }

    #[test]
    fn failed_commit_and_racing_writer_preserve_existing_file_and_clean_temp() {
        let root = tempfile::tempdir().unwrap();
        let rel = Path::new("test.md.age");
        fs::write(root.path().join(rel), b"old ciphertext").unwrap();
        let base = hash(b"old ciphertext");
        let partial_write: Result<()> = persist_with(
            root.path(),
            rel,
            Some(&base),
            |temporary| {
                temporary.write_all(b"partial ciphertext").unwrap();
                Err(Error::EncryptionFailed)
            },
            || Ok(()),
        );
        assert!(partial_write.is_err());
        assert_eq!(fs::read(root.path().join(rel)).unwrap(), b"old ciphertext");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert!(
            persist(root.path(), rel, b"new ciphertext", Some(&base), || Err(
                Error::EncryptionFailed
            ))
            .is_err()
        );
        assert_eq!(fs::read(root.path().join(rel)).unwrap(), b"old ciphertext");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        let result = persist(root.path(), rel, b"new ciphertext", Some(&base), || {
            fs::write(root.path().join(rel), b"external ciphertext").unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(Error::DocumentConflict { .. })));
        assert_eq!(
            fs::read(root.path().join(rel)).unwrap(),
            b"external ciphertext"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
