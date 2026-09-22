use ps_render::{render, render_project};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn renders_gfm_task_lists_with_item_class() {
    let markdown = "- [x] Parsed\n- [ ] Open\n";
    let html = render(markdown);
    assert!(html.contains("class=\"task-list-item\""));
    assert!(html.contains("type=\"checkbox\""));
    assert!(!html.contains("disabled"), "{html}");
    assert!(
        html.contains(&format!(
            "data-task-at=\"{}\" checked=\"\"",
            markdown.find("[x]").expect("checked marker")
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            "data-task-at=\"{}\"",
            markdown.find("[ ]").expect("open marker")
        )),
        "{html}"
    );
}

#[test]
fn renders_gfm_alerts() {
    let html = render("> [!WARNING]\n> Disk is full\n");
    assert!(html.contains("markdown-alert-warning"), "{html}");
    assert!(html.contains("Disk is full"));
}

#[test]
fn renders_definition_lists() {
    let html = render("Term\n: Definition\n");
    assert!(html.contains("<dl>"), "{html}");
    assert!(html.contains("<dt>Term</dt>"), "{html}");
    assert!(html.contains("<dd>"), "{html}");
}

#[test]
fn renders_yaml_front_matter_as_a_definition_list() {
    let html = render("---\ntitle: Hello\ntags: rust\n---\n\n# Body\n");
    assert!(html.contains("class=\"front-matter\""), "{html}");
    assert!(html.contains("<dt>title</dt><dd>Hello</dd>"), "{html}");
    assert!(html.contains("<h1"), "{html}");
    assert!(!html.contains("<hr"), "{html}");
}

#[test]
fn renders_wiki_links_as_markdown_hrefs() {
    let html = render("See [[Guide]] and [[Other|label]].\n");
    assert!(
        html.contains("href=\"Guide.md\" class=\"wiki-link\""),
        "{html}"
    );
    assert!(html.contains(">Guide</a>"), "{html}");
    assert!(
        html.contains("href=\"Other.md\" class=\"wiki-link\""),
        "{html}"
    );
    assert!(html.contains(">label</a>"), "{html}");
}

#[test]
fn resolves_wiki_links_inside_a_project() {
    let project = tempdir().expect("temporary project");
    fs::create_dir_all(project.path().join("notes")).expect("notes");
    fs::write(project.path().join("notes/guide.md"), b"# Guide").expect("guide");
    fs::write(project.path().join("readme.md"), b"# Root").expect("readme");

    let html = render_project(
        "[[guide]] [[guide|shown]] [[Missing]] [[readme]]\n",
        project.path(),
        Path::new("notes/index.md"),
        "project-1",
    );

    assert!(
        html.html
            .contains("href=\"asset://localhost/project-1/notes/guide.md\""),
        "{}",
        html.html
    );
    assert!(html.html.contains(">shown</a>"), "{}", html.html);
    assert!(
        html.html
            .contains("href=\"#missing-wiki\" class=\"wiki-link is-missing\""),
        "{}",
        html.html
    );
    assert!(
        html.html
            .contains("href=\"asset://localhost/project-1/readme.md\""),
        "{}",
        html.html
    );
}
