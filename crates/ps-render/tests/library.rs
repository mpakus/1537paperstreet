use ps_core::library::{LibraryProject, LibrarySource};

#[test]
fn sections_follow_markdown_headings_ignore_fences_and_include_children() {
    let text = "# Intro\n\n## Choice\nα\n### Detail\nβ\n```md\n## Fake\n```\n## Choice\nOther\n";
    let sections = ps_render::library_sections(text);
    assert_eq!(
        sections.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["intro", "choice", "detail", "choice-1"]
    );
    let choice = &sections[1];
    let selected = &text[choice.start as usize..choice.end as usize];
    assert!(selected.contains("β"));
    assert!(!selected.contains("Other"));
}

#[test]
fn pack_sections_survive_line_insertions_and_fail_closed_when_heading_disappears() {
    let dir = tempfile::tempdir().unwrap();
    let text = "# Intro\n\n## Evidence\nKeep this\n\n## Appendix\nOmit this\n";
    std::fs::write(dir.path().join("paper.md"), text).unwrap();
    let projects = vec![LibraryProject {
        id: "p".into(),
        name: "Research".into(),
        root: dir.path().into(),
    }];
    let source = LibrarySource {
        project_id: "p".into(),
        path: "paper.md".into(),
        section: Some("evidence".into()),
        hash: blake3::hash(text.as_bytes()).to_hex().to_string(),
    };
    let preview =
        ps_core::library::preview(&projects, &[source.clone()], ps_render::library_sections)
            .unwrap();
    assert!(preview.complete);
    assert!(preview.text.contains("Keep this"));
    assert!(!preview.text.contains("Omit this"));
    std::fs::write(dir.path().join("paper.md"), format!("\n\n{text}")).unwrap();
    let preview =
        ps_core::library::preview(&projects, &[source.clone()], ps_render::library_sections)
            .unwrap();
    assert!(preview.complete);
    assert_eq!(preview.sources[0].status, "changed");
    std::fs::write(dir.path().join("paper.md"), "# Gone\n").unwrap();
    let preview =
        ps_core::library::preview(&projects, &[source], ps_render::library_sections).unwrap();
    assert!(!preview.complete);
    assert!(preview.text.is_empty());
}
