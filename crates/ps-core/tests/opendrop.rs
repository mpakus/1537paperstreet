use std::fs;

use ps_core::projects::{ProjectStore, open_dropped_paths};

#[test]
fn dropping_a_folder_registers_it_as_a_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");
    fs::write(notes.join("readme.md"), b"# Hello\n").expect("document");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let opened = open_dropped_paths(&mut store, std::slice::from_ref(&notes)).expect("open folder");

    assert_eq!(opened.project.name, "notes");
    assert_eq!(
        opened
            .project
            .path
            .canonicalize()
            .expect("canonical project"),
        notes.canonicalize().expect("canonical notes")
    );
    assert!(opened.open_rel_path.is_none());
    assert!(opened.project.last_opened_at.is_some());
    assert_eq!(store.list().len(), 1);
}

#[test]
fn dropping_the_same_folder_twice_reuses_the_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let first = open_dropped_paths(&mut store, std::slice::from_ref(&notes)).expect("first drop");
    let second = open_dropped_paths(&mut store, &[notes]).expect("second drop");

    assert_eq!(first.project.id, second.project.id);
    assert_eq!(store.list().len(), 1);
}

#[test]
fn dropping_a_markdown_file_opens_it_inside_the_parent_folder() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");
    let nested = notes.join("chapters");
    fs::create_dir(&nested).expect("nested folder");
    let document = nested.join("01.md");
    fs::write(&document, b"# One\n").expect("document");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let opened = open_dropped_paths(&mut store, &[document]).expect("open file");

    assert_eq!(opened.project.name, "chapters");
    assert_eq!(
        opened.open_rel_path.as_deref().map(|path| path.to_str()),
        Some(Some("01.md"))
    );
    assert_eq!(opened.project.last_file.as_deref(), Some("01.md"));
}

#[test]
fn dropping_a_markdown_file_reuses_a_containing_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");
    let nested = notes.join("chapters");
    fs::create_dir(&nested).expect("nested folder");
    let document = nested.join("01.md");
    fs::write(&document, b"# One\n").expect("document");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let folder = open_dropped_paths(&mut store, &[notes]).expect("open folder");
    let file = open_dropped_paths(&mut store, &[document]).expect("open file");

    assert_eq!(folder.project.id, file.project.id);
    assert_eq!(store.list().len(), 1);
    assert_eq!(
        file.open_rel_path.as_deref().map(|path| path.to_str()),
        Some(Some("chapters/01.md"))
    );
}

#[test]
fn dropping_a_pdf_opens_it_inside_the_parent_folder() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");
    let document = notes.join("paper.pdf");
    fs::write(&document, b"%PDF-1.4\n").expect("pdf");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let opened = open_dropped_paths(&mut store, &[document]).expect("open pdf");

    assert_eq!(opened.project.name, "notes");
    assert_eq!(
        opened.open_rel_path.as_deref().map(|path| path.to_str()),
        Some(Some("paper.pdf"))
    );
    assert_eq!(opened.project.last_file.as_deref(), Some("paper.pdf"));
}

#[test]
fn launch_arguments_keep_files_and_file_urls() {
    use ps_core::projects::paths_from_launch_args;

    let paths = paths_from_launch_args(&[
        "file://localhost/Users/ada/chapter.md",
        "file://files.example/secret.md",
        "--flag",
    ]);
    assert_eq!(
        paths,
        vec![std::path::PathBuf::from("/Users/ada/chapter.md")]
    );
}

#[test]
fn dropping_a_non_markdown_file_is_rejected() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let notes = temp.path().join("notes");
    fs::create_dir(&notes).expect("notes folder");
    let image = notes.join("cover.png");
    fs::write(&image, b"png").expect("image");

    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let error = open_dropped_paths(&mut store, std::slice::from_ref(&image)).expect_err("rejected");

    assert!(error.to_string().contains("Markdown"));
    assert!(error.to_string().contains(&image.display().to_string()));
    assert!(store.list().is_empty());
}

#[test]
fn dropping_nothing_explains_how_to_open_a_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let mut store = ProjectStore::open(temp.path().join("projects.json")).expect("store");
    let error = open_dropped_paths(&mut store, &[]).expect_err("empty drop");
    assert!(
        error
            .to_string()
            .contains("Markdown file, a PDF, or a folder")
    );
}
