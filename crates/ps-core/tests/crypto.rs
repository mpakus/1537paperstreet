use age::secrecy::SecretString;
use ps_core::Error;
use ps_core::crypto::{self, DocumentFormat};

fn password() -> SecretString {
    SecretString::from("test-only long passphrase".to_owned())
}

#[test]
fn authenticated_round_trip_preserves_bytes_and_rejects_damage() {
    for text in ["", "\u{feff}# Привет 🦀\r\n\r\n- [ ] secret\r\n"] {
        let encrypted = crypto::encrypt_markdown(text.as_bytes(), &password()).unwrap();
        assert_eq!(
            crypto::detect_format(&encrypted),
            DocumentFormat::AgePassphrase
        );
        assert_eq!(
            crypto::decrypt_markdown(&encrypted, &password())
                .unwrap()
                .as_slice(),
            text.as_bytes()
        );
        assert!(
            crypto::decrypt_markdown(&encrypted, &SecretString::from("wrong".to_owned())).is_err()
        );
        assert!(crypto::decrypt_markdown(&encrypted[..encrypted.len() - 1], &password()).is_err());
        let mut damaged = encrypted.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(crypto::decrypt_markdown(&damaged, &password()).is_err());
        assert_ne!(
            encrypted,
            crypto::encrypt_markdown(text.as_bytes(), &password()).unwrap()
        );
    }
}

#[test]
fn detection_and_resource_limits_fail_closed() {
    assert_eq!(
        crypto::detect_format(b"# ordinary Markdown"),
        DocumentFormat::PlainMarkdown
    );
    assert_eq!(
        crypto::detect_format(b"age-encryption.org/v1\ninvalid"),
        DocumentFormat::InvalidOrUnknown
    );
    let key = age::x25519::Identity::generate();
    let encrypted = age::encrypt(&key.to_public(), b"text").unwrap();
    assert_eq!(
        crypto::detect_format(&encrypted),
        DocumentFormat::AgeUnsupportedRecipient
    );
    assert!(matches!(
        crypto::decrypt_markdown(&encrypted, &password()),
        Err(Error::UnsupportedRecipient)
    ));
    let recipient = age::scrypt::Recipient::new(password());
    let invalid = age::encrypt(&recipient, &[0xff, 0xfe]).unwrap();
    assert!(matches!(
        crypto::decrypt_markdown(&invalid, &password()),
        Err(Error::InvalidEncryptedUtf8)
    ));
    let huge = vec![b'x'; crypto::MAX_PLAINTEXT_BYTES + 1];
    assert!(matches!(
        crypto::encrypt_markdown(&huge, &password()),
        Err(Error::EncryptedDocumentTooLarge)
    ));
}

#[test]
fn rejects_expensive_scrypt_before_deriving_a_key() {
    let bytes = crypto::encrypt_markdown(b"test", &password()).unwrap();
    let header_end = bytes
        .windows(4)
        .position(|bytes| bytes == b"\n---")
        .unwrap();
    let header = std::str::from_utf8(&bytes[..header_end]).unwrap();
    let mut lines: Vec<String> = header.lines().map(str::to_owned).collect();
    let stanza = lines
        .iter_mut()
        .find(|line| line.starts_with("-> scrypt "))
        .unwrap();
    let exponent = stanza.rfind(' ').unwrap();
    stanza.replace_range(exponent + 1.., "30");
    let mut hostile = lines.join("\n").into_bytes();
    hostile.extend_from_slice(&bytes[header_end..]);
    assert!(matches!(
        crypto::decrypt_markdown(&hostile, &password()),
        Err(Error::ExcessiveCryptoWork)
    ));
}
