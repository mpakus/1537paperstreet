use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use ps_render::render_project;
use tempfile::tempdir;

#[test]
fn rewrites_only_paths_that_resolve_inside_the_project() {
    let project = tempdir().expect("temporary project");
    let outside = tempdir().expect("outside directory");
    fs::create_dir_all(project.path().join("notes/images")).expect("asset directory");
    fs::write(
        project.path().join("notes/images/cover image.png"),
        b"image",
    )
    .expect("image");
    fs::write(project.path().join("notes/guide.md"), b"# Guide").expect("guide");
    fs::write(project.path().join("notes/manual.pdf"), b"pdf").expect("pdf");
    fs::write(outside.path().join("secret.txt"), b"secret").expect("outside file");
    symlink(outside.path(), project.path().join("notes/escape")).expect("escape symlink");

    let markdown = r#"![Cover](<images/cover image.png>)
[Guide](guide.md#intro)
[Manual](manual.pdf)
[Website](https://example.com/docs)
[Email](mailto:reader@example.com)
[Section](#intro)
[Traversal](../../secret.txt)
[Symlink](escape/secret.txt)
"#;

    let html = render_project(
        markdown,
        project.path(),
        Path::new("notes/readme.md"),
        "project-1",
    );

    assert!(
        html.html
            .contains("src=\"asset://localhost/project-1/notes/images/cover%20image.png\""),
        "{}",
        html.html
    );
    assert!(
        html.html
            .contains("href=\"asset://localhost/project-1/notes/guide.md#intro\"")
    );
    assert!(
        html.html
            .contains("href=\"asset://localhost/project-1/notes/manual.pdf\"")
    );
    assert!(html.html.contains("href=\"https://example.com/docs\""));
    assert!(html.html.contains("href=\"mailto:reader@example.com\""));
    assert!(html.html.contains("href=\"#intro\""));
    assert_eq!(html.html.matches("href=\"#invalid-path\"").count(), 2);
    assert!(
        !html
            .html
            .contains(outside.path().to_string_lossy().as_ref())
    );
}

#[test]
fn opens_parent_and_source_links_inside_the_project() {
    let project = tempdir().expect("temporary project");
    fs::create_dir_all(project.path().join("notes")).expect("notes");
    fs::create_dir_all(project.path().join("src")).expect("src");
    fs::write(project.path().join("readme.md"), b"# Root").expect("readme");
    fs::write(project.path().join("src/main.rs"), b"fn main() {}\n").expect("source");

    let html = render_project(
        "[Root](../readme.md)\n[Code](../src/main.rs)\n[[../src/main.rs]]\n",
        project.path(),
        Path::new("notes/chapter.md"),
        "project-1",
    );

    assert!(
        html.html
            .contains("href=\"asset://localhost/project-1/readme.md\""),
        "{}",
        html.html
    );
    assert!(
        html.html
            .matches("href=\"asset://localhost/project-1/src/main.rs\"")
            .count()
            >= 2,
        "{}",
        html.html
    );
}

#[test]
fn reserves_png_width_and_height_on_project_images() {
    let project = tempdir().expect("temporary project");
    let mut png = vec![0_u8; 24];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[16..20].copy_from_slice(&320_u32.to_be_bytes());
    png[20..24].copy_from_slice(&240_u32.to_be_bytes());
    fs::write(project.path().join("cover.png"), png).expect("png");

    let html = render_project(
        "![Cover](cover.png)",
        project.path(),
        Path::new("readme.md"),
        "project-1",
    );

    assert!(
        html.html
            .contains("src=\"asset://localhost/project-1/cover.png\""),
        "{}",
        html.html
    );
    assert!(html.html.contains("width=\"320\""), "{}", html.html);
    assert!(html.html.contains("height=\"240\""), "{}", html.html);
    assert!(html.html.contains("loading=\"lazy\""), "{}", html.html);
}
