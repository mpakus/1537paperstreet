use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use ps_core::content_search;

#[test]
fn finds_a_word_and_a_regular_expression_in_file_text() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path();
    fs::write(root.join("chapter.md"), "title\nthe introduction\n").expect("chapter");

    let word = content_search::search_documents(root, Path::new(""), "introduction", false, 20)
        .expect("word search");
    assert!(!word.literal);
    assert_eq!(word.hits.len(), 1);
    assert_eq!(word.hits[0].rel_path, Path::new("chapter.md"));
    assert_eq!(word.hits[0].line, 2);
    assert!(word.hits[0].snippet.contains("introduction"));

    let pattern =
        content_search::search_documents(root, Path::new(""), "int.*", false, 20).expect("regex");
    assert!(!pattern.literal);
    assert_eq!(pattern.hits[0].line, 2);
}

#[test]
fn folder_scope_ignores_sibling_files() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path();
    fs::create_dir_all(root.join("a")).expect("folder a");
    fs::create_dir_all(root.join("b")).expect("folder b");
    fs::write(root.join("a/one.md"), "alpha").expect("one");
    fs::write(root.join("b/two.md"), "alpha").expect("two");

    let found = content_search::search_documents(root, Path::new("a"), "alpha", false, 20)
        .expect("scoped search");
    assert_eq!(found.hits.len(), 1);
    assert_eq!(found.hits[0].rel_path, Path::new("a/one.md"));
}

#[test]
fn invalid_regular_expression_searches_the_exact_text() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path();
    fs::write(root.join("notes.md"), "see (group here").expect("notes");

    let found = content_search::search_documents(root, Path::new(""), "(group", false, 20)
        .expect("literal");
    assert!(found.literal);
    assert_eq!(found.hits.len(), 1);
    assert_eq!(found.hits[0].line, 1);
}

#[test]
fn fuzzy_path_match_tolerates_a_typo() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path();
    fs::write(root.join("introduction.md"), "nothing relevant").expect("file");

    let found = content_search::search_documents(root, Path::new(""), "introdction", false, 20)
        .expect("fuzzy");
    assert!(!found.literal);
    assert_eq!(found.hits.len(), 1);
    assert_eq!(found.hits[0].rel_path, Path::new("introduction.md"));
    assert_eq!(found.hits[0].line, 0);
}

#[test]
fn skips_binary_files_git_metadata_and_hidden_files() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path();
    fs::write(root.join("photo.bin"), b"hello\0world").expect("binary");
    fs::create_dir_all(root.join("node_modules/pkg")).expect("modules");
    fs::write(root.join("node_modules/pkg/index.js"), "secret-token").expect("module");
    fs::write(root.join(".draft.md"), "needle").expect("hidden");
    fs::write(root.join("visible.md"), "plain").expect("visible");

    let binary =
        content_search::search_documents(root, Path::new(""), "world", false, 20).expect("binary");
    assert!(binary.hits.is_empty());

    let git = content_search::search_documents(root, Path::new(""), "secret-token", false, 20)
        .expect("git");
    assert!(git.hits.is_empty());

    let hidden =
        content_search::search_documents(root, Path::new(""), "needle", false, 20).expect("hidden");
    assert!(hidden.hits.is_empty());

    let shown =
        content_search::search_documents(root, Path::new(""), "needle", true, 20).expect("shown");
    assert_eq!(shown.hits.len(), 1);
}

#[test]
fn does_not_read_a_symbolic_link_or_leave_the_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    let outside = temp.path().join("outside.md");
    fs::create_dir(&root).expect("project");
    fs::write(&outside, "unique-outside-word").expect("outside");
    symlink(&outside, root.join("link.md")).expect("symlink");

    let found =
        content_search::search_documents(&root, Path::new(""), "unique-outside-word", false, 20)
            .expect("symlink search");
    assert!(found.hits.is_empty());

    let escaped =
        content_search::search_documents(&root, Path::new("../outside.md"), "unique", false, 20);
    assert!(escaped.is_err());
}
