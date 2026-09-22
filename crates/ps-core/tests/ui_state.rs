use std::path::PathBuf;

use ps_core::store::{JsonStore, VersionedDocument};
use ps_core::ui_state::{
    DiskKind, OpenSession, RestoredSession, SessionProbe, SessionTab, UiState,
};

#[test]
fn expanded_directories_round_trip_and_reject_escapes() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let path = temp.path().join("ui-state.json");
    let mut store = JsonStore::<UiState>::open(&path).expect("open");
    let mut state = store.value().clone();
    state
        .set_expanded(
            "proj".into(),
            vec![PathBuf::from("chapters"), PathBuf::from("chapters/drafts")],
        )
        .expect("set");
    store.replace(state);
    store.flush().expect("flush");

    let reopened = JsonStore::<UiState>::open(&path).expect("reopen");
    assert_eq!(
        reopened.value().expanded_for("proj"),
        vec!["chapters".to_owned(), "chapters/drafts".to_owned()]
    );
    assert!(reopened.value().expanded_for("missing").is_empty());

    let mut state = UiState::default();
    assert!(
        state
            .set_expanded("proj".into(), vec![PathBuf::from("../secret")])
            .is_err()
    );
    assert!(
        state
            .set_expanded("proj".into(), vec![PathBuf::from("/tmp/notes")])
            .is_err()
    );

    state
        .set_expanded("proj".into(), vec![PathBuf::from("inbox")])
        .expect("inbox");
    state.remove_project("proj");
    assert!(state.expanded_for("proj").is_empty());

    let mut cleared = UiState::default();
    cleared
        .set_expanded(
            "proj".into(),
            vec![PathBuf::from(""), PathBuf::from("inbox")],
        )
        .expect("skip empty");
    assert_eq!(cleared.expanded_for("proj"), vec!["inbox".to_owned()]);
    cleared
        .set_expanded("proj".into(), Vec::new())
        .expect("clear");
    assert!(cleared.expanded_for("proj").is_empty());
    assert!(UiState::migrate(serde_json::json!({}), 0).is_err());

    let invalid = UiState {
        schema_version: 0,
        expanded: Default::default(),
        session: None,
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn saved_session_round_trips_without_a_schema_bump() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let path = temp.path().join("ui-state.json");
    std::fs::write(
        &path,
        r#"{"schema_version":1,"expanded":{"proj":["inbox"]}}"#,
    )
    .expect("old file");
    let mut store = JsonStore::<UiState>::open(&path).expect("old file opens");
    assert!(store.value().session.is_none());

    let mut state = store.value().clone();
    state
        .set_session(OpenSession {
            project_id: "proj".into(),
            tabs: vec![
                SessionTab {
                    rel_path: PathBuf::from("readme.md"),
                    preview: false,
                    view_mode: "preview".into(),
                },
                SessionTab {
                    rel_path: PathBuf::from("notes.md"),
                    preview: true,
                    view_mode: "editor".into(),
                },
            ],
            active_rel_path: Some(PathBuf::from("notes.md")),
            workspace_tabs: vec!["dashboard".into()],
            page: "assistant".into(),
            view_mode: "editor".into(),
        })
        .expect("save session");
    assert!(
        state
            .set_session(OpenSession {
                project_id: "proj".into(),
                tabs: vec![SessionTab {
                    rel_path: PathBuf::from("../secret"),
                    preview: false,
                    view_mode: String::new(),
                }],
                active_rel_path: None,
                workspace_tabs: Vec::new(),
                page: "document".into(),
                view_mode: String::new(),
            })
            .is_err()
    );
    store.replace(state);
    store.flush().expect("flush");

    let reopened = JsonStore::<UiState>::open(&path).expect("reopen");
    let session = reopened.value().session.clone().expect("session");
    assert_eq!(session.project_id, "proj");
    assert_eq!(session.page, "assistant");
    assert_eq!(session.view_mode, "editor");
    assert_eq!(
        session.workspace_tabs,
        vec!["dashboard".to_owned(), "assistant".to_owned()]
    );
    assert_eq!(
        session.tabs,
        vec![
            SessionTab {
                rel_path: PathBuf::from("readme.md"),
                preview: false,
                view_mode: "preview".into(),
            },
            SessionTab {
                rel_path: PathBuf::from("notes.md"),
                preview: true,
                view_mode: "editor".into(),
            },
        ]
    );
    reopened.value().clone().remove_project("proj");
    let mut cleared = reopened.value().clone();
    cleared.remove_project("proj");
    assert!(cleared.session.is_none());
    assert!(cleared.expanded_for("proj").is_empty());
}

#[test]
fn restore_skips_missing_files_and_folders_once() {
    let mut state = UiState::default();
    state
        .set_expanded(
            "proj".into(),
            vec![PathBuf::from("chapters"), PathBuf::from("gone")],
        )
        .expect("expanded");
    state
        .set_session(OpenSession {
            project_id: "proj".into(),
            tabs: vec![
                SessionTab {
                    rel_path: PathBuf::from("keep.md"),
                    preview: false,
                    view_mode: "preview".into(),
                },
                SessionTab {
                    rel_path: PathBuf::from("gone.md"),
                    preview: false,
                    view_mode: "editor".into(),
                },
                SessionTab {
                    rel_path: PathBuf::from("next.md"),
                    preview: true,
                    view_mode: "split".into(),
                },
            ],
            active_rel_path: Some(PathBuf::from("gone.md")),
            workspace_tabs: vec!["assistant".into()],
            page: "document".into(),
            view_mode: "split".into(),
        })
        .expect("session");

    let listed = SessionProbe {
        project_listed: true,
        project_name: "Notes".into(),
        root_is_dir: true,
    };
    let restored = state.restore_session(&listed, |rel| match rel {
        "keep.md" | "next.md" => DiskKind::File,
        "chapters" => DiskKind::Directory,
        _ => DiskKind::Missing,
    });
    assert_eq!(
        restored.notices,
        vec![
            "Couldn't reopen \"gone.md\". That file is no longer in the project.".to_owned(),
            "Couldn't reopen \"gone\". That folder is no longer in the project.".to_owned(),
        ]
    );
    assert_eq!(restored.project_id.as_deref(), Some("proj"));
    assert_eq!(
        restored.active_rel_path.as_deref(),
        Some(std::path::Path::new("next.md"))
    );
    assert_eq!(restored.tabs.len(), 2);
    assert_eq!(restored.tabs[0].view_mode, "preview");
    assert_eq!(restored.tabs[1].view_mode, "split");
    assert_eq!(state.expanded_for("proj"), vec!["chapters".to_owned()]);

    let again = state.restore_session(&listed, |rel| match rel {
        "keep.md" | "next.md" => DiskKind::File,
        "chapters" => DiskKind::Directory,
        _ => DiskKind::Missing,
    });
    assert!(again.notices.is_empty());

    let missing_folder = state.restore_session(
        &SessionProbe {
            project_listed: true,
            project_name: "Notes".into(),
            root_is_dir: false,
        },
        |_| DiskKind::File,
    );
    assert_eq!(
        missing_folder.notices,
        vec!["Couldn't reopen \"Notes\". That folder is no longer on disk.".to_owned()]
    );
    assert_eq!(missing_folder, {
        let mut empty = RestoredSession::empty();
        empty.notices = missing_folder.notices.clone();
        empty
    });
    assert!(state.session.is_none());

    state
        .set_session(OpenSession {
            project_id: "proj".into(),
            tabs: vec![SessionTab {
                rel_path: PathBuf::from("keep.md"),
                preview: false,
                view_mode: String::new(),
            }],
            active_rel_path: Some(PathBuf::from("keep.md")),
            workspace_tabs: Vec::new(),
            page: "document".into(),
            view_mode: String::new(),
        })
        .expect("session again");
    let unlisted = state.restore_session(
        &SessionProbe {
            project_listed: false,
            project_name: String::new(),
            root_is_dir: false,
        },
        |_| DiskKind::File,
    );
    assert_eq!(
        unlisted.notices,
        vec!["Couldn't reopen the last folder. It is no longer in the project list.".to_owned()]
    );
    assert!(state.session.is_none());
}

#[test]
fn missing_file_starts_with_a_valid_default() {
    let opened = JsonStore::<UiState>::open(
        tempfile::tempdir()
            .expect("temporary directory")
            .path()
            .join("missing.json"),
    )
    .expect("missing file uses defaults");
    assert_eq!(opened.value().schema_version, 1);
    assert!(opened.value().expanded_for("any").is_empty());
}
