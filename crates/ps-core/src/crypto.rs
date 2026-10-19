//! Standard binary age encryption with bounded, authenticated decryption.

use std::io::{Read, Write};

use age::secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeroize::Zeroizing;

use crate::{Error, Result};

/// Maximum decrypted size, matching the editor's existing limit.
pub const MAX_PLAINTEXT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum input, including age headers and stream authentication overhead.
pub const MAX_CIPHERTEXT_BYTES: usize = 9 * 1024 * 1024;
/// At most 1 GiB of scrypt working memory (N = 2^20, r = 8, p = 1).
pub const MAX_SCRYPT_WORK_FACTOR: u8 = 20;

/// Content classification; an extension alone never authenticates a document.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DocumentFormat {
    /// Ordinary UTF-8 text, without an age header.
    PlainMarkdown,
    /// Binary age with a single scrypt recipient.
    AgePassphrase,
    /// Valid age header with an unsupported recipient.
    AgeUnsupportedRecipient,
    /// Invalid, oversized, binary, or armored input.
    InvalidOrUnknown,
}

/// Parses the complete age header. This does not authenticate the payload.
pub fn detect_format(bytes: &[u8]) -> DocumentFormat {
    if bytes.len() > MAX_CIPHERTEXT_BYTES {
        return DocumentFormat::InvalidOrUnknown;
    }
    match age::Decryptor::new(bytes) {
        Ok(decryptor) if decryptor.is_scrypt() => DocumentFormat::AgePassphrase,
        Ok(_) => DocumentFormat::AgeUnsupportedRecipient,
        Err(_)
            if bytes.starts_with(b"age-encryption.org/")
                || bytes.starts_with(b"-----BEGIN AGE") =>
        {
            DocumentFormat::InvalidOrUnknown
        }
        Err(_) if std::str::from_utf8(bytes).is_ok() => DocumentFormat::PlainMarkdown,
        Err(_) => DocumentFormat::InvalidOrUnknown,
    }
}

/// Encrypts UTF-8 bytes without normalizing BOM or line endings.
///
/// The age library chooses the file key, salt and scrypt work factor. No
/// plaintext is written to disk, and `finish` authenticates the final chunk.
pub fn encrypt_markdown(bytes: &[u8], passphrase: &SecretString) -> Result<Vec<u8>> {
    encrypt_to(bytes, passphrase, Vec::new())
}

/// Streams ciphertext directly to a caller-owned writer and finishes authentication.
pub(crate) fn encrypt_to<W: Write>(
    bytes: &[u8],
    passphrase: &SecretString,
    output: W,
) -> Result<W> {
    if passphrase.expose_secret().is_empty() {
        return Err(Error::EmptyPassphrase);
    }
    if bytes.len() > MAX_PLAINTEXT_BYTES {
        return Err(Error::EncryptedDocumentTooLarge);
    }
    std::str::from_utf8(bytes).map_err(|_| Error::InvalidEncryptedUtf8)?;
    let encryptor = age::Encryptor::with_user_passphrase(passphrase.clone());
    let mut writer = encryptor
        .wrap_output(output)
        .map_err(|_| Error::EncryptionFailed)?;
    writer
        .write_all(bytes)
        .map_err(|_| Error::EncryptionFailed)?;
    writer.finish().map_err(|_| Error::EncryptionFailed)
}

/// Returns bytes only after EOF, integrity verification and strict UTF-8 validation.
/// Temporary plaintext is zeroed on both success and failure when dropped.
pub fn decrypt_markdown(encrypted: &[u8], passphrase: &SecretString) -> Result<Zeroizing<Vec<u8>>> {
    if encrypted.len() > MAX_CIPHERTEXT_BYTES {
        return Err(Error::EncryptedDocumentTooLarge);
    }
    let decryptor = age::Decryptor::new(encrypted).map_err(|_| Error::NotAgeFile)?;
    if !decryptor.is_scrypt() {
        return Err(Error::UnsupportedRecipient);
    }
    let mut identity = age::scrypt::Identity::new(passphrase.clone());
    identity.set_max_work_factor(MAX_SCRYPT_WORK_FACTOR);
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|error| match error {
            age::DecryptError::ExcessiveWork { .. } => Error::ExcessiveCryptoWork,
            _ => Error::DecryptionFailed,
        })?;
    let mut plaintext = Zeroizing::new(Vec::new());
    reader
        .take((MAX_PLAINTEXT_BYTES + 1) as u64)
        .read_to_end(&mut plaintext)
        .map_err(|_| Error::DecryptionFailed)?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(Error::EncryptedDocumentTooLarge);
    }
    std::str::from_utf8(&plaintext).map_err(|_| Error::InvalidEncryptedUtf8)?;
    Ok(plaintext)
}
