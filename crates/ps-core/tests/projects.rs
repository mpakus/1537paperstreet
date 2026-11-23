use std::fs;

use ps_core::projects::{
    ProjectStore, ProjectsDocument, file_drop_folders, open_dropped_paths, open_file_drop,
};
use ps_core::store::VersionedDocument;
use ulid::Ulid;

#[test]
fn project_crud_never_modifies_the_project_directory() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let store_path = temp.path().join("projects.json");
    let project_root = temp.path().join("notes");
    fs::create_dir(&project_root).expect("project directory");
    let document = project_root.join("important.md");
    fs::write(&document, b"Never delete this text").expect("project document");

    let mut store = ProjectStore::open(&store_path).expect("project store");
    let project = store
        .add("Notes", project_root.clone())
        .expect("added project");
    Ulid::from_string(&project.id).expect("ULID project identifier");
    assert_eq!(project.available, None);
    assert!(
        store
            .refresh_availability(&project.id)
            .expect("availability")
    );

    store
        .rename(&project.id, "Writing")
        .expect("renamed project");
    assert_eq!(store.get(&project.id).expect("found").name, "Writing");
    assert!(store.get("missing").is_err());
    store.remove(&project.id).expect("removed project record");
    store.flush().expect("saved project list");

    assert_eq!(
        fs::read(&document).expect("preserved document"),
        b"Never delete this text"
    );
    assert!(
        ProjectStore::open(&store_path)
            .expect("reopened project store")
            .list()
            .is_empty()
    );
}

#[test]
fn availability_is_checked_only_when_requested() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let missing = temp.path().join("not-mounted");
    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("project store");
    let project = store.add("Archive", missing).expect("added project");

    assert_eq!(project.available, None);
    assert!(
        !store
            .refresh_availability(&project.id)
            .expect("availability")
    );
    assert_eq!(store.list()[0].available, Some(false));
}

#[test]
fn relocate_changes_the_recorded_folder_without_touching_files() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let old = temp.path().join("old");
    let next = temp.path().join("next");
    fs::create_dir(&old).expect("old folder");
    fs::create_dir(&next).expect("next folder");
    fs::write(old.join("keep.md"), b"stay").expect("old file");
    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let project = store.add("Notes", old.clone()).expect("added");

    let relocated = store
        .relocate(&project.id, next.clone())
        .expect("relocated");
    assert_eq!(relocated.path, next.canonicalize().expect("canonical"));
    assert_eq!(fs::read(old.join("keep.md")).expect("untouched"), b"stay");
}

#[test]
fn touch_opened_ensure_folder_and_containing_lookups() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes");
    fs::write(notes.join("a.md"), b"hi").expect("file");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let first = store.ensure_folder(notes.clone()).expect("ensure");
    let again = store.ensure_folder(notes.clone()).expect("reuse");
    assert_eq!(first.id, again.id);

    store
        .touch_opened(&first.id, None)
        .expect("touch without file");
    store
        .touch_opened(&first.id, Some("a.md".into()))
        .expect("touch");
    assert_eq!(
        store.get(&first.id).expect("found").last_file.as_deref(),
        Some("a.md")
    );

    let file = notes.join("a.md").canonicalize().expect("file");
    assert_eq!(
        store.find_containing(&file).expect("containing").id,
        first.id
    );
    assert_eq!(store.find_by_path(&notes).expect("by path").id, first.id);

    assert!(store.rename(&first.id, "   ").is_err());
    assert!(
        store
            .add("Rel", std::path::PathBuf::from("relative"))
            .is_err()
    );
    assert!(store.remove("missing").is_err());
    assert!(store.ensure_folder(notes.join("a.md")).is_err());
    assert!(store.relocate(&first.id, notes.join("a.md")).is_err());

    let other = temp.path().join("other");
    fs::create_dir(&other).expect("other");
    let second = store.add("Other", other.clone()).expect("second");
    assert!(store.relocate(&first.id, other.clone()).is_err());
    let same = store
        .relocate(&second.id, other.clone())
        .expect("same folder");
    assert_eq!(same.id, second.id);

    store.close().expect("close");
}

#[test]
fn migrate_and_lookups_cover_missing_and_corrupt_records() {
    let temp = tempfile::tempdir().expect("temporary directory");
    assert!(ProjectsDocument::migrate(serde_json::json!({}), 0).is_err());

    let invalid = ProjectsDocument {
        schema_version: 0,
        ..ProjectsDocument::default()
    };
    assert!(invalid.validate().is_err());

    let store_path = temp.path().join("projects.json");
    fs::write(&store_path, b"not json").expect("corrupt");
    let mut store = ProjectStore::open(&store_path).expect("defaults after corrupt");
    assert!(store.take_warning().is_some());

    assert!(
        store
            .find_containing(&temp.path().join("missing.md"))
            .is_none()
    );
    assert!(store.ensure_folder(temp.path().join("missing")).is_err());

    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes");
    let project = store.add("Notes", notes.clone()).expect("added");
    assert!(
        store
            .relocate(&project.id, temp.path().join("missing"))
            .is_err()
    );
    assert!(store.relocate("missing", notes.clone()).is_err());
    assert!(open_dropped_paths(&mut store, &[temp.path().join("gone")]).is_err());
}

#[cfg(unix)]
#[test]
fn dropping_a_socket_is_rejected() {
    use std::os::unix::net::UnixListener;

    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes");
    let socket = notes.join("sock");
    let _listener = UnixListener::bind(&socket).expect("socket");
    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    assert!(open_dropped_paths(&mut store, &[socket]).is_err());
}

#[test]
fn dropped_files_open_from_existing_and_approved_projects_only() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let registered = temp.path().join("registered");
    let new = temp.path().join("new");
    fs::create_dir(&registered).expect("registered folder");
    fs::create_dir(&new).expect("new folder");
    let paths = [
        registered.join("settings.yaml"),
        new.join("settings.yaml"),
        new.join("app.toml"),
    ];
    for path in &paths {
        fs::write(path, "").expect("configuration");
    }
    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let project = store.add("Registered", registered).expect("project");
    let canonical = new.canonicalize().expect("canonical new folder");
    assert_eq!(
        file_drop_folders(&store, &paths).expect("plan"),
        vec![canonical.clone()]
    );
    let opened = open_file_drop(&mut store, &paths, &[]).expect("decline new folder");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].project.id, project.id);
    assert_eq!(store.list().len(), 1);
    let opened = open_file_drop(&mut store, &paths, &[canonical]).expect("approve new folder");
    assert_eq!(opened.len(), 3);
    assert_eq!(store.list().len(), 2);
    assert_eq!(opened[1].project.id, opened[2].project.id);
    assert_ne!(opened[0].project.id, opened[1].project.id);
}
