use std::fs;
use std::path::Path;
use std::time::Duration;

use ps_core::Error;
use ps_core::docio::{
    self, DocumentEncoding, LineEnding, RestoreTraits, SOURCE_ONLY_BYTES, WrittenDocument,
};

struct RoundTripCase {
    bytes: &'static [u8],
    text: &'static str,
    eol: LineEnding,
    bom: bool,
    trailing_newline: bool,
}

/// Eight combinations from T-160: LF/CRLF × BOM/none × trailing newline/none.
const EIGHT_COMBINATIONS: &[RoundTripCase] = &[
    RoundTripCase {
        bytes: b"hello\n",
        text: "hello\n",
        eol: LineEnding::Lf,
        bom: false,
        trailing_newline: true,
    },
    RoundTripCase {
        bytes: b"hello\r\n",
        text: "hello\n",
        eol: LineEnding::CrLf,
        bom: false,
        trailing_newline: true,
    },
    RoundTripCase {
        bytes: b"\xEF\xBB\xBFhello\n",
        text: "hello\n",
        eol: LineEnding::Lf,
        bom: true,
        trailing_newline: true,
    },
    RoundTripCase {
        bytes: b"\xEF\xBB\xBFhello\r\n",
        text: "hello\n",
        eol: LineEnding::CrLf,
        bom: true,
        trailing_newline: true,
    },
    RoundTripCase {
        bytes: b"hello",
        text: "hello",
        eol: LineEnding::Lf,
        bom: false,
        trailing_newline: false,
    },
    RoundTripCase {
        bytes: b"hello\r\nworld",
        text: "hello\nworld",
        eol: LineEnding::CrLf,
        bom: false,
        trailing_newline: false,
    },
    RoundTripCase {
        bytes: b"\xEF\xBB\xBFhello",
        text: "hello",
        eol: LineEnding::Lf,
        bom: true,
        trailing_newline: false,
    },
    RoundTripCase {
        bytes: b"\xEF\xBB\xBFhello\r\nworld",
        text: "hello\nworld",
        eol: LineEnding::CrLf,
        bom: true,
        trailing_newline: false,
    },
];

fn project_file(name: &str, bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    let path = root.join(name);
    fs::write(&path, bytes).expect("document");
    (temp, root)
}

fn write_without_edits(
    root: &Path,
    rel: &str,
    loaded: &ps_core::docio::LoadedDocument,
) -> WrittenDocument {
    docio::write_doc(
        root,
        Path::new(rel),
        &loaded.source.text,
        &loaded.hash,
        RestoreTraits::from_source(&loaded.source),
    )
    .expect("write")
}

#[test]
fn reads_eight_line_ending_bom_and_trailing_newline_combinations() {
    assert_eq!(EIGHT_COMBINATIONS.len(), 8);

    for case in EIGHT_COMBINATIONS {
        let (_temp, root) = project_file("note.md", case.bytes);
        let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
        assert_eq!(loaded.source.text, case.text, "bytes={:?}", case.bytes);
        assert_eq!(loaded.source.eol, case.eol);
        assert_eq!(loaded.source.bom, case.bom);
        assert_eq!(loaded.source.trailing_newline, case.trailing_newline);
        assert_eq!(loaded.source.encoding, DocumentEncoding::Utf8);
        assert!(loaded.source.writable);
        assert!(loaded.source.readonly_reason.is_none());
        assert!(!loaded.source_only);
    }
}

#[test]
fn image_files_preview_without_exposing_bytes_and_cannot_be_saved() {
    let png = b"\x89PNG\r\n\x1a\n";
    let (_temp, root) = project_file("cover.png", png);
    let loaded = docio::read_doc(&root, Path::new("cover.png")).expect("read");
    assert_eq!(loaded.source.text, "");
    assert_eq!(loaded.source.encoding, DocumentEncoding::Binary);
    assert!(!loaded.source.writable);
    assert!(loaded.source.readonly_reason.is_none());
    assert!(!loaded.source_only);
    let err = docio::write_doc(
        &root,
        Path::new("cover.png"),
        "",
        &loaded.hash,
        RestoreTraits::from_source(&loaded.source),
    )
    .expect_err("image save");
    assert!(matches!(err, Error::ImageNotEditable));
    assert_eq!(fs::read(root.join("cover.png")).expect("png bytes"), png);

    let (_temp, root) = project_file("icon.svg", b"<svg><script>alert(1)</script></svg>\n");
    let loaded = docio::read_doc(&root, Path::new("icon.svg")).expect("svg");
    assert_eq!(loaded.source.text, "");
    assert!(!loaded.source.writable);
    assert!(!loaded.source_only);
}

#[test]
fn binary_files_are_read_only_with_empty_text() {
    let (_temp, root) = project_file("note.md", &[0xff, 0xfe, 0x00]);
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    assert_eq!(loaded.source.text, "");
    assert_eq!(loaded.source.encoding, DocumentEncoding::Binary);
    assert!(!loaded.source.writable);
    assert!(loaded.source_only);
    assert!(loaded.source.readonly_reason.is_some());
}

#[test]
fn files_over_eight_megabytes_open_as_source_only() {
    let mut bytes = vec![b'a'; SOURCE_ONLY_BYTES as usize + 1];
    bytes.push(b'\n');
    let (_temp, root) = project_file("huge.md", &bytes);
    let loaded = docio::read_doc(&root, Path::new("huge.md")).expect("read");
    assert_eq!(loaded.source.encoding, DocumentEncoding::Utf8);
    assert!(!loaded.source.writable);
    assert!(loaded.source_only);
    assert_eq!(loaded.size, SOURCE_ONLY_BYTES + 2);
}

#[test]
fn rejects_paths_outside_the_project() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    assert!(docio::read_doc(&root, Path::new("..")).is_err());
}

#[test]
fn rejects_a_directory_and_a_missing_file() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    fs::create_dir(root.join("chapters")).expect("folder");
    let err = docio::read_doc(&root, Path::new("chapters")).expect_err("directory");
    assert!(err.to_string().contains("folders cannot be opened"));
    assert!(docio::read_doc(&root, Path::new("missing.md")).is_err());
}

#[test]
fn lone_carriage_return_is_treated_as_lf() {
    let (_temp, root) = project_file("note.md", b"hello\rworld");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    assert_eq!(loaded.source.eol, LineEnding::Lf);
    assert_eq!(loaded.source.text, "hello\rworld");
    assert!(!loaded.source.trailing_newline);
}

#[cfg(unix)]
#[test]
fn unwritable_utf8_files_are_marked_read_only() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp, root) = project_file("note.md", b"hello\n");
    let path = root.join("note.md");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o444);
    fs::set_permissions(&path, permissions).expect("chmod");

    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    assert!(!loaded.source.writable);
    assert!(!loaded.source_only);
    assert!(loaded.source.readonly_reason.is_some());
}

#[cfg(unix)]
#[test]
fn unreadable_files_surface_as_io_errors() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp, root) = project_file("note.md", b"hello\n");
    let path = root.join("note.md");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("lock");
    let result = docio::read_doc(&root, Path::new("note.md"));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("unlock");
    assert!(result.is_err());
}

#[test]
fn write_doc_round_trips_eight_combinations_byte_for_byte() {
    for case in EIGHT_COMBINATIONS {
        let (_temp, root) = project_file("note.md", case.bytes);
        let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
        let written = write_without_edits(&root, "note.md", &loaded);
        assert_eq!(fs::read(root.join("note.md")).expect("disk"), case.bytes);
        assert_eq!(written.hash, loaded.hash);
        assert_eq!(written.size, case.bytes.len() as u64);
    }
}

#[test]
fn write_doc_restores_bom_eol_and_trailing_newline_on_disk() {
    for case in EIGHT_COMBINATIONS {
        let (_temp, root) = project_file("note.md", b"seed\n");
        let seed = docio::read_doc(&root, Path::new("note.md")).expect("seed");
        let written = docio::write_doc(
            &root,
            Path::new("note.md"),
            case.text,
            &seed.hash,
            RestoreTraits {
                eol: case.eol,
                bom: case.bom,
                trailing_newline: case.trailing_newline,
            },
        )
        .expect("write");
        assert!(!written.skipped, "bytes={:?}", case.bytes);
        assert_eq!(fs::read(root.join("note.md")).expect("disk"), case.bytes);
        assert_eq!(written.size, case.bytes.len() as u64);

        let loaded = docio::read_doc(&root, Path::new("note.md")).expect("re-read");
        assert_eq!(written.hash, loaded.hash);
        assert_eq!(loaded.source.text, case.text);
        assert_eq!(loaded.source.eol, case.eol);
        assert_eq!(loaded.source.bom, case.bom);
        assert_eq!(loaded.source.trailing_newline, case.trailing_newline);
    }
}

#[cfg(unix)]
#[test]
fn write_doc_preserves_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    for mode in [0o644_u32, 0o600] {
        let (_temp, root) = project_file("note.md", b"seed\n");
        let path = root.join("note.md");
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");
        let seed = docio::read_doc(&root, Path::new("note.md")).expect("seed");
        docio::write_doc(
            &root,
            Path::new("note.md"),
            "hello\n",
            &seed.hash,
            RestoreTraits {
                eol: LineEnding::Lf,
                bom: false,
                trailing_newline: true,
            },
        )
        .expect("write");
        let kept = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(kept, mode);
        assert_eq!(fs::read(&path).expect("disk"), b"hello\n");
    }
}

#[test]
fn write_doc_skips_when_encoded_bytes_match_disk() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    let path = root.join("note.md");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    let before = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");

    let written = write_without_edits(&root, "note.md", &loaded);
    assert!(written.skipped);
    let after = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(after, before);
    assert_eq!(fs::read(&path).expect("disk"), b"hello\n");
}

#[test]
fn write_doc_skip_does_not_notify_the_watcher() {
    use ps_core::watch::ProjectWatcher;

    let (_temp, root) = project_file("note.md", b"hello\n");
    let watcher = ProjectWatcher::start(&root).expect("watcher");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    let written = write_without_edits(&root, "note.md", &loaded);
    assert!(written.skipped);
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        watcher.try_recv().is_none(),
        "identical save must not emit a filesystem event"
    );
}

#[test]
fn write_doc_returns_conflict_and_does_not_overwrite_external_bytes() {
    let (_temp, root) = project_file("note.md", b"original\n");
    let path = root.join("note.md");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    fs::write(&path, b"from another process\n").expect("external edit");

    let error = docio::write_doc(
        &root,
        Path::new("note.md"),
        &loaded.source.text,
        &loaded.hash,
        RestoreTraits::from_source(&loaded.source),
    )
    .expect_err("conflict");
    assert!(matches!(error, Error::DocumentConflict { .. }), "{error:?}");
    assert!(error.to_string().contains("changed on disk"));
    assert_eq!(fs::read(&path).expect("disk"), b"from another process\n");
}

#[test]
fn write_doc_appends_a_trailing_newline_when_the_buffer_has_none() {
    let (_temp, root) = project_file("note.md", b"seed\n");
    let seed = docio::read_doc(&root, Path::new("note.md")).expect("seed");
    let written = docio::write_doc(
        &root,
        Path::new("note.md"),
        "hello",
        &seed.hash,
        RestoreTraits {
            eol: LineEnding::Lf,
            bom: false,
            trailing_newline: true,
        },
    )
    .expect("write");
    assert!(!written.skipped);
    assert_eq!(fs::read(root.join("note.md")).expect("disk"), b"hello\n");
}

#[test]
fn write_doc_rejects_a_directory_and_a_missing_file() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    fs::create_dir(root.join("chapters")).expect("folder");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    let traits = RestoreTraits::from_source(&loaded.source);
    let err = docio::write_doc(
        &root,
        Path::new("chapters"),
        &loaded.source.text,
        &loaded.hash,
        traits,
    )
    .expect_err("directory");
    assert!(err.to_string().contains("folders cannot be opened"));
    assert!(
        docio::write_doc(
            &root,
            Path::new("missing.md"),
            &loaded.source.text,
            &loaded.hash,
            traits,
        )
        .is_err()
    );
}

#[test]
fn empty_file_defaults_to_lf_without_trailing_newline() {
    let (_temp, root) = project_file("note.md", b"");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    assert_eq!(loaded.source.text, "");
    assert_eq!(loaded.source.eol, LineEnding::Lf);
    assert!(!loaded.source.trailing_newline);
    assert!(!loaded.source.bom);
}

#[test]
fn stat_matches_the_loaded_document_hash_and_size() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    let stat = docio::stat_doc(&root, Path::new("note.md")).expect("stat");
    assert_eq!(stat.hash, loaded.hash);
    assert_eq!(stat.size, loaded.size);
}

#[test]
fn stat_rejects_a_directory_and_a_path_outside_the_project() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    fs::create_dir(root.join("chapters")).expect("folder");
    assert!(docio::stat_doc(&root, Path::new("chapters")).is_err());
    assert!(docio::stat_doc(&root, Path::new("../secret.md")).is_err());
}

#[test]
fn write_doc_rejects_paths_outside_the_project() {
    let (_temp, root) = project_file("note.md", b"hello\n");
    let loaded = docio::read_doc(&root, Path::new("note.md")).expect("read");
    assert!(
        docio::write_doc(
            &root,
            Path::new("../escape.md"),
            &loaded.source.text,
            &loaded.hash,
            RestoreTraits::from_source(&loaded.source),
        )
        .is_err()
    );
}
