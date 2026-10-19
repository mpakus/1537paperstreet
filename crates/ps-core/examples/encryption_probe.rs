//! Synthetic interoperability fixture and latency probe; never use a real note.
use std::{env, fs, path::Path, time::Instant};

use age::secrecy::SecretString;
use ps_core::{crypto, docio::RestoreTraits, encrypted::EncryptedDocuments};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let password = SecretString::from("paperstreet interoperability test only".to_owned());
    let fixture = "\u{feff}# Interoperability 🦀\r\nПривет\n";
    match args.get(1).map(String::as_str) {
        Some("emit") => {
            let directory = Path::new(args.get(2).ok_or("missing temporary directory")?);
            let documents = EncryptedDocuments::default();
            documents.create(directory, Path::new("app.md.age"), fixture, password)?;
            // Deliberately public synthetic input for independent CLI tools.
            fs::write(directory.join("fixture.md"), fixture)?;
        }
        Some("verify") => {
            let input = args.get(2).ok_or("missing ciphertext fixture")?;
            let plaintext = crypto::decrypt_markdown(&fs::read(input)?, &password)?;
            if plaintext.as_slice() != fixture.as_bytes() {
                return Err("byte mismatch".into());
            }
            println!("External ciphertext: authenticated and byte-identical");
        }
        Some("bench") => {
            let directory = tempfile::tempdir()?;
            let documents = EncryptedDocuments::default();
            for bytes in [10 * 1024, 1024 * 1024] {
                let source = "x".repeat(bytes);
                let path = format!("{bytes}.md.age");
                let path = Path::new(&path);
                let start = Instant::now();
                documents.create(directory.path(), path, &source, password.clone())?;
                let create = start.elapsed();
                documents.lock_all()?;
                let start = Instant::now();
                documents.unlock(directory.path(), path, password.clone())?;
                let unlock = start.elapsed();
                let loaded = documents.read(directory.path(), path)?;
                let traits = RestoreTraits::from_source(&loaded.source);
                let start = Instant::now();
                let unchanged =
                    documents.save(directory.path(), path, &source, &loaded.hash, traits, None)?;
                let noop = start.elapsed();
                assert!(unchanged.skipped);
                let start = Instant::now();
                documents.save(
                    directory.path(),
                    path,
                    &(source + " edit"),
                    &loaded.hash,
                    traits,
                    None,
                )?;
                println!(
                    "{bytes} bytes: create {create:?}, unlock {unlock:?}, unchanged {noop:?}, changed {:?}",
                    start.elapsed()
                );
            }
        }
        _ => return Err("usage: encryption_probe emit DIR | verify FILE | bench".into()),
    }
    Ok(())
}
