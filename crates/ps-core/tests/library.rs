use std::fs;

use ps_core::library::{self, LibraryQuery, NoteInput};

#[test]
fn creates_portable_notes_without_overwriting_and_preserves_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let input = NoteInput {
        path: "paper.md".into(),
        title: "Evidence: café".into(),
        note_type: "paper".into(),
        status: "unread".into(),
        tags: "agents, research".into(),
        source: "https://example.org/paper".into(),
    };
    library::create_note(dir.path(), &input).unwrap();
    let bytes = fs::read(dir.path().join("paper.md")).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("## Findings"));
    assert!(library::create_note(dir.path(), &input).is_err());
    let doc = ps_core::docio::read_doc(dir.path(), "paper.md".as_ref()).unwrap();
    ps_core::docio::write_doc(
        dir.path(),
        "paper.md".as_ref(),
        &doc.source.text,
        &doc.hash,
        ps_core::docio::RestoreTraits {
            eol: doc.source.eol,
            bom: doc.source.bom,
            trailing_newline: doc.source.trailing_newline,
        },
    )
    .unwrap();
    assert_eq!(fs::read(dir.path().join("paper.md")).unwrap(), bytes);
}

#[test]
fn reads_yaml_metadata_and_filters_without_touching_documents() {
    let dir = tempfile::tempdir().unwrap();
    let text = "---\ntitle: 'Quoted: title'\ntype: experiment\nstatus: reviewed\ntags:\n  - rust\n  - research\ncustom: { untouched: true }\n---\n# Experiment\n";
    fs::write(dir.path().join("experiment.md"), text).unwrap();
    fs::write(dir.path().join("private.md.age"), "secret").unwrap();
    let project = library::LibraryProject {
        id: "p".into(),
        name: "Research".into(),
        root: dir.path().into(),
    };
    let query = LibraryQuery {
        tag: "rust".into(),
        ..Default::default()
    };
    let result = library::scan(&[project], &query, &Default::default(), |_| vec![]).unwrap();
    assert_eq!(result.notes.len(), 1);
    assert_eq!(result.notes[0].title, "Quoted: title");
    assert_eq!(result.notes[0].note_type, "experiment");
    assert_eq!(result.notes[0].freshness, "unreviewed");
    assert_eq!(
        fs::read_to_string(dir.path().join("experiment.md")).unwrap(),
        text
    );
}

#[test]
fn refuses_unsafe_or_encrypted_note_creation() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["../escape.md", "secret.md.age", "not-markdown.txt"] {
        let input = NoteInput {
            path: path.into(),
            title: "Note".into(),
            note_type: "spec".into(),
            status: "draft".into(),
            tags: String::new(),
            source: String::new(),
        };
        assert!(library::create_note(dir.path(), &input).is_err());
    }
}

#[test]
fn saved_collections_persist_and_reviews_detect_changes_with_conflict_protection() {
    use ps_core::library::{LibraryMutation, LibrarySource, SavedSet, SavedSetKind};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("spec.md"), "# Spec\nOriginal\n").unwrap();
    let projects = vec![library::LibraryProject {
        id: "p".into(),
        name: "Project".into(),
        root: root.clone(),
    }];
    let path = dir.path().join("library.json");
    let source = LibrarySource {
        project_id: "p".into(),
        path: "spec.md".into(),
        section: None,
        hash: blake3::hash(b"# Spec\nOriginal\n").to_hex().to_string(),
    };
    let set = SavedSet {
        id: String::new(),
        name: "Release".into(),
        kind: SavedSetKind::Collection,
        sources: vec![source.clone()],
    };
    let data = library::mutate(&path, "", LibraryMutation::Save { set }, &projects).unwrap();
    assert_eq!(data.sets.len(), 1);
    assert!(!fs::read_to_string(&path).unwrap().contains("Original"));
    assert!(
        library::mutate(
            &path,
            "",
            LibraryMutation::Review {
                source: source.clone()
            },
            &projects
        )
        .is_err()
    );
    let data = library::mutate(
        &path,
        &data.revision,
        LibraryMutation::Review {
            source: source.clone(),
        },
        &projects,
    )
    .unwrap();
    let view = library::scan(&projects, &Default::default(), &data, |_| vec![]).unwrap();
    assert_eq!(view.notes[0].freshness, "reviewed");
    fs::write(root.join("spec.md"), "# Spec\nModified\n").unwrap();
    let view = library::scan(&projects, &Default::default(), &data, |_| vec![]).unwrap();
    assert_eq!(view.notes[0].freshness, "changed");
    assert!(
        library::mutate(
            &path,
            &data.revision,
            LibraryMutation::Review { source },
            &projects
        )
        .is_err()
    );
    let pack = library::preview(&projects, &data.sets[0].sources, |_| vec![]).unwrap();
    assert!(pack.complete);
    assert_eq!(pack.sources[0].status, "changed");
    assert!(pack.text.contains("Modified"));
    fs::remove_file(root.join("spec.md")).unwrap();
    let pack = library::preview(&projects, &data.sets[0].sources, |_| vec![]).unwrap();
    assert!(!pack.complete);
    assert!(pack.text.is_empty());
    assert_eq!(pack.sources[0].status, "missing");
}

#[test]
fn failed_metadata_save_keeps_previous_data_and_sources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/library.json");
    let mutation = library::LibraryMutation::Save {
        set: library::SavedSet {
            id: String::new(),
            name: "Keep".into(),
            kind: library::SavedSetKind::Pack,
            sources: vec![],
        },
    };
    assert!(library::mutate(&path, "", mutation, &[]).is_err());
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn symlink_sources_and_encrypted_payloads_never_enter_packs() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.md"), "secret").unwrap();
    symlink(outside.path().join("secret.md"), dir.path().join("link.md")).unwrap();
    fs::write(
        dir.path().join("renamed.md"),
        "age-encryption.org/v1\nciphertext",
    )
    .unwrap();
    let projects = vec![library::LibraryProject {
        id: "p".into(),
        name: "p".into(),
        root: dir.path().into(),
    }];
    for path in ["link.md", "renamed.md", "../secret.md", "private.md.age"] {
        let source = library::LibrarySource {
            project_id: "p".into(),
            path: path.into(),
            section: None,
            hash: String::new(),
        };
        let preview = library::preview(&projects, &[source], |_| vec![]).unwrap();
        assert!(!preview.complete);
        assert!(preview.text.is_empty());
    }
    let view = library::scan(
        &projects,
        &Default::default(),
        &Default::default(),
        |_| vec![],
    )
    .unwrap();
    assert!(view.notes.is_empty());
}
