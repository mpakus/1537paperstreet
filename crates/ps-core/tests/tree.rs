use std::fs;
use std::path::Path;

use ps_core::tree::{self, TreeNodeKind};

#[test]
fn reads_only_one_level_with_directories_first_and_natural_sort() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir_all(root.join("folder10/nested")).expect("folder ten");
    fs::create_dir(root.join("folder2")).expect("folder two");
    for name in ["file10.md", "file2.md", "file1.md", "file02.md"] {
        fs::write(root.join(name), b"").expect("tree file");
    }
    fs::write(root.join("folder10/nested/deep.md"), b"deep").expect("nested file");

    let nodes = tree::read_dir(&root, Path::new(""), false).expect("tree nodes");
    let names = nodes
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        [
            "folder2",
            "folder10",
            "file1.md",
            "file2.md",
            "file02.md",
            "file10.md"
        ]
    );
    assert!(
        nodes[..2]
            .iter()
            .all(|node| node.kind == TreeNodeKind::Directory)
    );
    assert!(
        nodes[2..]
            .iter()
            .all(|node| node.kind == TreeNodeKind::File)
    );
    assert!(nodes.iter().all(|node| !node.rel_path.ends_with("deep.md")));
}

#[test]
fn natural_sort_compares_equal_digit_runs_then_the_rest_of_the_name() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    for name in ["aa.md", "aa1.md", "file1a.md", "file1b.md"] {
        fs::write(root.join(name), b"").expect("tree file");
    }

    let names = tree::read_dir(&root, Path::new(""), false)
        .expect("tree nodes")
        .into_iter()
        .map(|node| node.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["aa.md", "aa1.md", "file1a.md", "file1b.md"]);
}

#[test]
fn hidden_filter_can_be_disabled_explicitly() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("visible.md"), b"").expect("visible file");
    fs::write(root.join(".hidden.md"), b"").expect("hidden file");
    fs::create_dir(root.join(".docs")).expect("hidden folder");

    let filtered = tree::read_dir(&root, Path::new(""), false).expect("filtered tree");
    let visible = tree::read_dir(&root, Path::new(""), true).expect("complete tree");

    assert_eq!(filtered.len(), 1);
    assert_eq!(visible.len(), 3);
    assert!(visible.iter().any(|node| node.name == ".hidden.md"));
    assert!(visible.iter().any(|node| node.name == ".docs"));
}

#[cfg(unix)]
#[test]
fn identifies_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("target.md"), b"Target").expect("target file");
    symlink(root.join("target.md"), root.join("alias.md")).expect("symlink");

    let nodes = tree::read_dir(&root, Path::new(""), false).expect("tree nodes");
    let alias = nodes
        .iter()
        .find(|node| node.name == "alias.md")
        .expect("alias node");

    assert_eq!(alias.kind, TreeNodeKind::Symlink);
}

#[test]
fn node_at_matches_read_dir_for_the_same_path() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir_all(root.join("inbox")).expect("project directory");
    fs::write(root.join("inbox/note.md"), b"Note").expect("tree file");

    let listed = tree::read_dir(&root, Path::new("inbox"), false).expect("tree nodes");
    let note = listed
        .iter()
        .find(|node| node.name == "note.md")
        .expect("listed note");
    let from_absolute = tree::node_at(&root, &root.join("inbox/note.md")).expect("absolute node");

    assert_eq!(from_absolute, *note);
}

#[test]
fn node_at_rejects_paths_outside_the_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(temp.path().join("outside.md"), b"no").expect("outside file");

    assert!(tree::node_at(&root, &temp.path().join("outside.md")).is_err());
    assert!(tree::node_at(&root, &root).is_err());
    assert!(tree::node_at(&root, Path::new("/")).is_err());
}

#[cfg(unix)]
#[test]
fn node_at_identifies_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("target.md"), b"Target").expect("target file");
    symlink(root.join("target.md"), root.join("alias.md")).expect("symlink");

    let alias = tree::node_at(&root, &root.join("alias.md")).expect("alias node");
    assert_eq!(alias.kind, TreeNodeKind::Symlink);
    assert_eq!(alias.name, "alias.md");
}

#[test]
fn rejects_directories_outside_the_project() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");

    assert!(tree::read_dir(&root, Path::new(".."), false).is_err());
}

#[test]
fn search_markdown_ranks_by_path_and_skips_non_markdown() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir_all(root.join("chapters")).expect("chapters");
    fs::write(root.join("readme.md"), b"").expect("readme");
    fs::write(root.join("chapters/intro.md"), b"").expect("intro");
    fs::write(root.join("cover.png"), b"").expect("image");

    let empty = tree::search_markdown(&root, "", false, 10).expect("all markdown");
    let names = empty
        .iter()
        .map(|node| node.rel_path.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    assert_eq!(names, ["chapters/intro.md", "readme.md"]);

    let ranked = tree::search_markdown(&root, "intro", false, 10).expect("ranked");
    assert_eq!(
        ranked[0].rel_path.to_string_lossy().replace('\\', "/"),
        "chapters/intro.md"
    );
    assert!(
        tree::search_markdown(&root, "intro", false, 10)
            .expect("ranked")
            .iter()
            .all(|node| node.name.ends_with(".md"))
    );

    let tied = tree::search_markdown(&root, "md", false, 10).expect("several matches");
    assert!(tied.len() >= 2);
}

#[test]
fn read_dir_rejects_a_file_path() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("note.md"), b"text").expect("file");

    let error =
        tree::read_dir(&root, Path::new("note.md"), false).expect_err("file is not a folder");
    assert!(error.to_string().contains("not a folder"));
}

#[test]
fn search_respects_hidden_files_and_unmatched_queries() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("visible.md"), b"").expect("visible");
    fs::write(root.join(".secret.md"), b"").expect("hidden");
    fs::write(root.join("notes.markdown"), b"").expect("markdown extension");

    let hidden_off = tree::search_markdown(&root, "", false, 10).expect("filtered");
    assert_eq!(hidden_off.len(), 2);
    assert!(hidden_off.iter().all(|node| node.name != ".secret.md"));

    let hidden_on = tree::search_markdown(&root, "", true, 10).expect("with hidden");
    assert_eq!(hidden_on.len(), 3);
    assert!(
        tree::search_markdown(&root, "zzzz", false, 10)
            .expect("no matches")
            .is_empty()
    );
}

#[cfg(unix)]
#[test]
fn identifies_sockets_as_other_nodes() {
    use std::os::unix::net::UnixListener;

    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    let _listener = UnixListener::bind(root.join("sock")).expect("socket");

    let nodes = tree::read_dir(&root, Path::new(""), false).expect("tree nodes");
    let sock = nodes
        .iter()
        .find(|node| node.name == "sock")
        .expect("socket node");
    assert_eq!(sock.kind, TreeNodeKind::Other);
}

#[cfg(unix)]
#[test]
fn search_does_not_follow_markdown_symlinks() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    fs::write(root.join("real.md"), b"").expect("target");
    symlink(root.join("real.md"), root.join("alias.md")).expect("symlink");

    let files = tree::search_markdown(&root, "", false, 10).expect("search");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "real.md");
}

#[cfg(unix)]
#[test]
fn read_dir_rejects_non_unicode_file_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("project");
    fs::create_dir(&root).expect("project directory");
    let name = OsString::from_vec(vec![0xff, 0xfe, 0xfd]);
    if fs::write(root.join(&name), b"x").is_ok() {
        assert!(tree::read_dir(&root, Path::new(""), false).is_err());
        assert!(tree::node_at(&root, &root.join(&name)).is_err());
    }
}
