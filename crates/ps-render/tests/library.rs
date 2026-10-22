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
    let preview = ps_core::library::preview(
        &projects,
        std::slice::from_ref(&source),
        ps_render::library_sections,
    )
    .unwrap();
    assert!(preview.complete);
    assert!(preview.text.contains("Keep this"));
    assert!(!preview.text.contains("Omit this"));
    std::fs::write(dir.path().join("paper.md"), format!("\n\n{text}")).unwrap();
    let preview = ps_core::library::preview(
        &projects,
        std::slice::from_ref(&source),
        ps_render::library_sections,
    )
    .unwrap();
    assert!(preview.complete);
    assert_eq!(preview.sources[0].status, "changed");
    std::fs::write(dir.path().join("paper.md"), "# Gone\n").unwrap();
    let preview =
        ps_core::library::preview(&projects, &[source], ps_render::library_sections).unwrap();
    assert!(!preview.complete);
    assert!(preview.text.is_empty());
}

#[test]
fn packs_preserve_source_order_and_contain_indented_fences() {
    let dir = tempfile::tempdir().unwrap();
    let projects = [LibraryProject {
        id: "p".into(),
        name: "Research".into(),
        root: dir.path().into(),
    }];
    let mut sources = vec![];
    for (path, text) in [
        ("second.md", "# Second\n  ```rust\nexample\n  ```\n"),
        ("first.md", "# First\n"),
    ] {
        std::fs::write(dir.path().join(path), text).unwrap();
        sources.push(LibrarySource {
            project_id: "p".into(),
            path: path.into(),
            section: None,
            hash: blake3::hash(text.as_bytes()).to_hex().to_string(),
        });
    }
    let preview =
        ps_core::library::preview(&projects, &sources, ps_render::library_sections).unwrap();
    assert!(preview.complete);
    assert!(preview.text.find("second.md").unwrap() < preview.text.find("first.md").unwrap());
    assert!(preview.text.contains("````markdown\n# Second"));
}

#[test]
#[ignore = "manual local latency measurement"]
fn measure_library_scan_and_pack() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!(
        "---\ntitle: Benchmark\ntype: paper\ntags: [agents]\n---\n# Study\n\n## Findings\n{}\n",
        "Evidence and limitations. ".repeat(100)
    );
    for i in 0..1000 {
        std::fs::write(dir.path().join(format!("note-{i}.md")), &text).unwrap();
    }
    let projects = vec![LibraryProject {
        id: "p".into(),
        name: "Fixture".into(),
        root: dir.path().into(),
    }];
    for pass in 0..5 {
        let start = std::time::Instant::now();
        let snapshot = ps_core::library::scan(
            &projects,
            &Default::default(),
            &Default::default(),
            ps_render::library_sections,
        )
        .unwrap();
        let scan = start.elapsed();
        let sources = snapshot
            .notes
            .iter()
            .take(20)
            .map(|note| LibrarySource {
                project_id: note.project_id.clone(),
                path: note.path.clone(),
                section: Some("findings".into()),
                hash: note.hash.clone(),
            })
            .collect::<Vec<_>>();
        let start = std::time::Instant::now();
        let preview =
            ps_core::library::preview(&projects, &sources, ps_render::library_sections).unwrap();
        assert!(preview.complete);
        println!(
            "pass={pass} notes={} scan_ms={:.2} pack_ms={:.2} output_bytes={}",
            snapshot.notes.len(),
            scan.as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 1000.0,
            preview.text.len()
        );
    }
}
