use age::secrecy::SecretString;
use ps_core::{
    Error,
    docio::{self, RestoreTraits},
    encrypted::EncryptedDocuments,
};
use std::{fs, path::Path};

fn password() -> SecretString {
    SecretString::from("test-only long passphrase".to_owned())
}

#[test]
fn lifecycle_noop_conflict_password_change_and_export() {
    let root = tempfile::tempdir().unwrap();
    let path = Path::new("secret.md.age");
    let documents = EncryptedDocuments::default();
    documents
        .create(
            root.path(),
            path,
            "\u{feff}# Привет 🦀\r\nmixed\n",
            password(),
        )
        .unwrap();
    let original = fs::read(root.path().join(path)).unwrap();
    assert!(original.starts_with(b"age-encryption.org/v1\n"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    let loaded = documents.read(root.path(), path).unwrap();
    let traits = RestoreTraits::from_source(&loaded.source);
    let written = documents
        .save(
            root.path(),
            path,
            &loaded.source.text,
            &loaded.hash,
            traits,
            None,
        )
        .unwrap();
    assert!(written.skipped);
    assert_eq!(fs::read(root.path().join(path)).unwrap(), original);
    assert!(docio::write_doc(root.path(), path, "oops", &loaded.hash, traits).is_err());
    documents.lock_all().unwrap();
    assert!(matches!(
        documents.read(root.path(), path),
        Err(Error::DocumentLocked)
    ));
    assert!(
        documents
            .unlock(root.path(), path, SecretString::from("wrong".to_owned()))
            .is_err()
    );
    documents.unlock(root.path(), path, password()).unwrap();
    let other_editor = EncryptedDocuments::default();
    other_editor.unlock(root.path(), path, password()).unwrap();
    let replacement = SecretString::from("replacement test passphrase".to_owned());
    let saved = documents
        .save(
            root.path(),
            path,
            "# changed\n",
            &loaded.hash,
            traits,
            Some(replacement.clone()),
        )
        .unwrap();
    assert_ne!(saved.hash, loaded.hash);
    assert!(matches!(
        other_editor.save(
            root.path(),
            path,
            "another editor",
            &loaded.hash,
            traits,
            None
        ),
        Err(Error::DocumentConflict { .. })
    ));
    assert!(matches!(
        documents.save(root.path(), path, "stale", &loaded.hash, traits, None),
        Err(Error::DocumentConflict { .. })
    ));
    documents.lock_all().unwrap();
    assert!(documents.unlock(root.path(), path, password()).is_err());
    documents.unlock(root.path(), path, replacement).unwrap();
    documents
        .export(root.path(), path, Path::new("export.md"), &saved.hash)
        .unwrap();
    assert_eq!(
        fs::read(root.path().join("export.md")).unwrap(),
        "\u{feff}# changed\r\n".as_bytes()
    );
    assert!(
        documents
            .export(root.path(), path, Path::new("export.md"), &saved.hash)
            .is_err()
    );
    assert!(
        documents
            .create(root.path(), path, "overwrite", password())
            .is_err()
    );
}

#[test]
fn rejects_invalid_paths_and_encrypted_garbage() {
    let root = tempfile::tempdir().unwrap();
    let documents = EncryptedDocuments::default();
    // Removing or relocating an unavailable registration is still possible
    // when it has no decrypted session to protect.
    documents
        .ensure_locked(&root.path().join("unavailable"), Path::new(""))
        .unwrap();
    for path in ["../outside.md.age", "plain.md", "not-markdown.age"] {
        assert!(
            documents
                .create(root.path(), Path::new(path), "private", password())
                .is_err()
        );
    }
    fs::write(root.path().join("broken.md.age"), b"not ciphertext").unwrap();
    assert!(
        documents
            .status(root.path(), Path::new("broken.md.age"))
            .is_err()
    );
    let oversized = fs::File::create(root.path().join("oversized.md.age")).unwrap();
    oversized
        .set_len((ps_core::crypto::MAX_CIPHERTEXT_BYTES + 1) as u64)
        .unwrap();
    assert!(matches!(
        documents.stat(root.path(), Path::new("oversized.md.age")),
        Err(Error::EncryptedDocumentTooLarge)
    ));
}

#[test]
fn save_can_retry_after_another_handle_releases_its_exclusive_lock() {
    let root = tempfile::tempdir().unwrap();
    let rel = Path::new("locked.md.age");
    let documents = EncryptedDocuments::default();
    documents
        .create(root.path(), rel, "Saved text", password())
        .unwrap();
    let loaded = documents.read(root.path(), rel).unwrap();
    let traits = RestoreTraits::from_source(&loaded.source);
    let ciphertext = fs::read(root.path().join(rel)).unwrap();
    let other_writer = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join(rel))
        .unwrap();
    other_writer.try_lock().unwrap();
    assert!(
        documents
            .save(root.path(), rel, "Edited text", &loaded.hash, traits, None)
            .is_err()
    );
    drop(other_writer);
    assert_eq!(fs::read(root.path().join(rel)).unwrap(), ciphertext);

    let unchanged = documents
        .save(
            root.path(),
            rel,
            &loaded.source.text,
            &loaded.hash,
            traits,
            None,
        )
        .unwrap();
    assert!(unchanged.skipped);
    assert_eq!(fs::read(root.path().join(rel)).unwrap(), ciphertext);
    documents
        .save(root.path(), rel, "Edited text", &loaded.hash, traits, None)
        .unwrap();
    documents.lock_all().unwrap();
    documents.unlock(root.path(), rel, password()).unwrap();
    assert_eq!(
        documents.read(root.path(), rel).unwrap().source.text,
        "Edited text"
    );
}

#[test]
fn conversion_and_discard_recovery_keep_exact_text_without_overwriting_original() {
    let root = tempfile::tempdir().unwrap();
    let source = Path::new("source.md");
    let destination = Path::new("protected.md.age");
    let original = "\u{feff}# Private\r\nMixed\n";
    fs::write(root.path().join(source), original).unwrap();
    let loaded = docio::read_doc(root.path(), source).unwrap();
    let traits = RestoreTraits::from_source(&loaded.source);
    let documents = EncryptedDocuments::default();
    documents
        .encrypt_existing(
            root.path(),
            source,
            destination,
            &loaded.source.text,
            &loaded.hash,
            traits,
            password(),
        )
        .unwrap();
    assert_eq!(
        fs::read(root.path().join(source)).unwrap(),
        original.as_bytes()
    );
    let cipher = fs::read(root.path().join(destination)).unwrap();
    assert_eq!(
        ps_core::crypto::decrypt_markdown(&cipher, &password())
            .unwrap()
            .as_slice(),
        original.as_bytes()
    );
    documents
        .snapshot_before_discard(root.path(), destination, "unsaved secret", traits)
        .unwrap();
    assert_eq!(fs::read(root.path().join(destination)).unwrap(), cipher);
    let recovery = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.to_string_lossy().contains("pre_discard"))
        .unwrap();
    let recovered =
        ps_core::crypto::decrypt_markdown(&fs::read(recovery).unwrap(), &password()).unwrap();
    assert!(
        std::str::from_utf8(&recovered)
            .unwrap()
            .contains("unsaved secret")
    );
    assert!(documents.ensure_locked(root.path(), Path::new("")).is_err());
    documents.lock_all().unwrap();
    documents.ensure_locked(root.path(), Path::new("")).unwrap();
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_and_uses_private_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("real")).unwrap();
    symlink(root.path().join("real"), root.path().join("link")).unwrap();
    let documents = EncryptedDocuments::default();
    assert!(
        documents
            .create(
                root.path(),
                Path::new("link/note.md.age"),
                "private",
                password()
            )
            .is_err()
    );
    documents
        .create(root.path(), Path::new("note.md.age"), "private", password())
        .unwrap();
    assert_eq!(
        fs::metadata(root.path().join("note.md.age"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let path = Path::new("note.md.age");
    let original = fs::read(root.path().join(path)).unwrap();
    let loaded = documents.read(root.path(), path).unwrap();
    fs::set_permissions(root.path().join(path), fs::Permissions::from_mode(0o400)).unwrap();
    assert!(
        documents
            .save(
                root.path(),
                path,
                "must not overwrite readonly text",
                &loaded.hash,
                RestoreTraits::from_source(&loaded.source),
                None
            )
            .is_err()
    );
    assert_eq!(fs::read(root.path().join(path)).unwrap(), original);
}

#[test]
fn recognizes_original_markdown_extensions_with_an_age_suffix() {
    for name in [
        "Note.md.age",
        "Folder/Original.MARKDOWN.age",
        "a.mdown.age",
        "a.mdwn.AGE",
    ] {
        let path = Path::new(name);
        assert!(ps_core::encrypted::is_encrypted_markdown(path), "{name}");
        assert!(ps_core::projects::is_markdown_path(path));
        assert_eq!(ps_core::projects::source_language(path), None);
    }
    for name in ["a.age", "a.txt.age", "a.md.age.age"] {
        assert!(!ps_core::encrypted::is_encrypted_markdown(Path::new(name)));
    }
}

#[test]
fn original_deletion_requires_the_verified_conversion_and_preserves_exact_source_in_ciphertext() {
    let root = tempfile::tempdir().unwrap();
    let source = Path::new("Original.MARKDOWN");
    let destination = Path::new("Original.MARKDOWN.age");
    let original = "\u{feff}# Saved original\r\nMixed\n";
    fs::write(root.path().join(source), original).unwrap();
    let loaded = docio::read_doc(root.path(), source).unwrap();
    let documents = EncryptedDocuments::default();
    documents
        .encrypt_existing(
            root.path(),
            source,
            destination,
            "Edited converted text",
            &loaded.hash,
            RestoreTraits::from_source(&loaded.source),
            password(),
        )
        .unwrap();
    let ciphertext = fs::read(root.path().join(destination)).unwrap();
    assert!(root.path().join(source).exists());
    fs::write(root.path().join(source), "External source edit").unwrap();
    assert!(matches!(
        documents.delete_original(root.path(), destination),
        Err(Error::DocumentConflict { .. })
    ));
    assert_eq!(
        fs::read_to_string(root.path().join(source)).unwrap(),
        "External source edit"
    );
    fs::write(root.path().join(source), original).unwrap();
    fs::write(root.path().join(destination), "Damaged encrypted copy").unwrap();
    assert!(documents.delete_original(root.path(), destination).is_err());
    assert!(root.path().join(source).exists());
    fs::write(root.path().join(destination), &ciphertext).unwrap();
    documents.delete_original(root.path(), destination).unwrap();
    assert!(!root.path().join(source).exists());
    assert_eq!(fs::read(root.path().join(destination)).unwrap(), ciphertext);
    let recovery = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.to_string_lossy().contains("pre_encrypt_delete"))
        .unwrap();
    let recovered =
        ps_core::crypto::decrypt_markdown(&fs::read(recovery).unwrap(), &password()).unwrap();
    assert_eq!(recovered.as_slice(), original.as_bytes());
    assert!(documents.delete_original(root.path(), destination).is_err());
}
