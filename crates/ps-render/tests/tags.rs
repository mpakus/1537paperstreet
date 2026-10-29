use ps_render::inline_tags;

#[test]
fn recognizes_bear_syntax_in_prose_and_headings() {
    let tags = inline_tags(
        "# Heading #work\n#project alpha# #work/design #work/design review# #2026/06# #café #日本語 #🚀 #snake_case #hyphen-tag #work\n",
    );
    assert_eq!(
        tags,
        [
            "work",
            "project alpha",
            "work/design",
            "work/design review",
            "2026/06",
            "café",
            "日本語",
            "🚀",
            "snake_case",
            "hyphen-tag"
        ]
    );
}

#[test]
fn excludes_markdown_syntax_escaped_tags_code_links_math_and_metadata() {
    let text = r#"---
title: '#metadata'
---
# Heading
\#escaped `#inline` [#label](https://example.test/#anchor) ![#alt](a.png)
```md
#fenced
```
    #indented

<div>#html</div>

$#math$ https://example.test/#url email@example.test#email word#suffix
#real, #also. (#parent/child) **#bold**
"#;
    assert_eq!(inline_tags(text), ["real", "also", "parent/child", "bold"]);
}

#[test]
fn rejects_invalid_hierarchy_and_keeps_extraction_bounded() {
    assert_eq!(
        inline_tags("#a//b #/bad #end/ #valid\n#not closed\n"),
        ["valid", "not"]
    );
    let source = (0..300).map(|n| format!("#t{n} ")).collect::<String>();
    assert_eq!(inline_tags(&source).len(), 257); // sentinel makes truncation visible
    assert!(inline_tags(&format!("#{}", "a".repeat(257))).is_empty());
}

#[test]
fn cloud_spans_folders_and_preserves_saved_bytes() {
    use ps_core::{
        library::LibraryProject,
        tags::{TagIndex, TagQuery},
    };
    let dir = tempfile::tempdir().unwrap();
    let mut projects = Vec::new();
    for (id, text) in [
        ("a", "---\ntags: [research]\n---\n#work/design #shared"),
        ("b", "#work/design #shared #project beta#"),
    ] {
        let root = dir.path().join(id);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("note.md"), text).unwrap();
        std::fs::write(root.join("secret.md.age"), "#private").unwrap();
        projects.push(LibraryProject {
            id: id.into(),
            name: id.into(),
            root,
        });
    }
    let mut index = TagIndex::default();
    let result = index
        .query(&projects, &TagQuery::default(), true, inline_tags)
        .unwrap();
    assert_eq!(result.notes.len(), 2);
    let shared = result.tags.iter().find(|tag| tag.key == "shared").unwrap();
    assert_eq!((shared.documents, shared.projects), (2, 2));
    let q = TagQuery {
        tags: vec!["work".into(), "research".into()],
        ..Default::default()
    };
    let result = index.query(&projects, &q, false, inline_tags).unwrap();
    assert_eq!(result.notes.len(), 1);
    assert_eq!(result.notes[0].project_id, "a");
    assert!(!result.tags.iter().any(|t| t.key == "project beta"));
    assert_eq!(
        std::fs::read_to_string(projects[0].root.join("note.md")).unwrap(),
        "---\ntags: [research]\n---\n#work/design #shared"
    );
    std::fs::write(projects[0].root.join("note.md"), "#changed").unwrap();
    assert_eq!(
        index
            .query(&projects, &q, true, inline_tags)
            .unwrap()
            .notes
            .len(),
        0
    );
    let result = index
        .query(&projects[1..], &TagQuery::default(), false, inline_tags)
        .unwrap();
    assert_eq!(result.notes.len(), 1);
}

#[cfg(unix)]
#[test]
fn cloud_excludes_symlinks_and_reports_unavailable_roots() {
    use ps_core::{
        library::LibraryProject,
        tags::{TagIndex, TagQuery},
    };
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::fs::write(external.path().join("secret.md"), "#secret").unwrap();
    std::os::unix::fs::symlink(external.path(), dir.path().join("linked")).unwrap();
    std::os::unix::fs::symlink(
        external.path().join("secret.md"),
        dir.path().join("linked.md"),
    )
    .unwrap();
    let projects = [
        LibraryProject {
            id: "p".into(),
            name: "Local".into(),
            root: dir.path().into(),
        },
        LibraryProject {
            id: "missing".into(),
            name: "Missing".into(),
            root: dir.path().join("absent"),
        },
    ];
    let result = TagIndex::default()
        .query(&projects, &TagQuery::default(), true, inline_tags)
        .unwrap();
    assert!(result.tags.is_empty());
    assert!(!result.notices.is_empty());
}

#[test]
fn parents_count_each_document_once_and_filters_do_not_match_prefix_siblings() {
    use ps_core::{
        library::LibraryProject,
        tags::{TagIndex, TagQuery},
    };
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("one.md"),
        "#Work #work/design #work/code #work/design\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("two.md"), "#workshop #elsewhere\n").unwrap();
    let projects = [LibraryProject {
        id: "p".into(),
        name: "Folder".into(),
        root: dir.path().into(),
    }];
    let mut index = TagIndex::default();
    let query = TagQuery {
        tags: vec!["WORK".into()],
        ..Default::default()
    };
    let result = index.query(&projects, &query, true, inline_tags).unwrap();
    assert_eq!(result.notes.len(), 1);
    assert_eq!(
        result
            .tags
            .iter()
            .find(|t| t.key == "work")
            .unwrap()
            .documents,
        1
    );
    let missing = TagQuery {
        project_id: "removed".into(),
        ..Default::default()
    };
    assert!(
        index
            .query(&projects, &missing, false, inline_tags)
            .unwrap()
            .notes
            .is_empty()
    );
    let filename = TagQuery {
        search: "ONE.MD".into(),
        ..Default::default()
    };
    assert_eq!(
        index
            .query(&projects, &filename, false, inline_tags)
            .unwrap()
            .notes
            .len(),
        1
    );
}

#[test]
fn tag_pages_and_per_document_truncation_are_explicit() {
    use ps_core::{
        library::LibraryProject,
        tags::{TagIndex, TagQuery},
    };
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("many.md"),
        (0..300).map(|n| format!("#t{n} ")).collect::<String>(),
    )
    .unwrap();
    let projects = [LibraryProject {
        id: "p".into(),
        name: "Folder".into(),
        root: dir.path().into(),
    }];
    let mut index = TagIndex::default();
    let result = index
        .query(&projects, &Default::default(), true, inline_tags)
        .unwrap();
    assert_eq!(result.total_tags, 256);
    assert_eq!(result.tags.len(), 80);
    assert!(result.notices.iter().any(|n| n.contains("Partial results")));
    let result = index
        .query(
            &projects,
            &TagQuery {
                tag_offset: u32::MAX,
                ..Default::default()
            },
            false,
            inline_tags,
        )
        .unwrap();
    assert_eq!(result.tag_offset, 240);
    assert_eq!(result.tags.len(), 16);
    assert!(!result.tags[0].label.is_empty());
}

#[test]
fn oversized_closed_multiword_tag_does_not_create_a_prefix_topic() {
    assert!(inline_tags(&format!("#{} {}#", "a".repeat(200), "b".repeat(100))).is_empty());
}

#[test]
fn closed_numeric_and_punctuation_tags_keep_their_complete_names() {
    assert_eq!(
        inline_tags("#!# #rock & roll# #2021# #2022 #work/2026\n"),
        ["!", "rock & roll", "2021", "work/2026"]
    );
}

#[test]
fn folder_scope_rebuilds_after_the_global_scan_limit() {
    use ps_core::{
        library::LibraryProject,
        tags::{TagIndex, TagQuery},
    };
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a");
    let second = dir.path().join("b");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    for n in 0..5000 {
        std::fs::write(first.join(format!("{n}.md")), "#first").unwrap();
    }
    std::fs::write(second.join("only.md"), "#second").unwrap();
    let projects = [
        LibraryProject {
            id: "a".into(),
            name: "A".into(),
            root: first,
        },
        LibraryProject {
            id: "b".into(),
            name: "B".into(),
            root: second,
        },
    ];
    let mut index = TagIndex::default();
    assert!(
        !index
            .query(&projects, &Default::default(), true, inline_tags)
            .unwrap()
            .notices
            .is_empty()
    );
    let result = index
        .query(
            &projects,
            &TagQuery {
                project_id: "b".into(),
                ..Default::default()
            },
            false,
            inline_tags,
        )
        .unwrap();
    assert_eq!(result.notes.len(), 1);
    assert_eq!(result.notes[0].project_id, "b");
    assert!(result.notices.is_empty());
}

#[test]
fn emitted_cloud_labels_round_trip_for_punctuation_numbers_and_unicode() {
    use ps_core::{library::LibraryProject, tags::TagIndex};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("tags.md"),
        "#!# #rock & roll# #2021# #café #🚀 #nested/tag",
    )
    .unwrap();
    let roots = [LibraryProject {
        id: "p".into(),
        name: "P".into(),
        root: dir.path().into(),
    }];
    let result = TagIndex::default()
        .query(&roots, &Default::default(), true, inline_tags)
        .unwrap();
    for tag in result.tags {
        assert_eq!(inline_tags(&tag.label)[0].to_lowercase(), tag.key);
    }
}

#[test]
fn closed_tags_preserve_names_across_markdown_emphasis_boundaries() {
    assert_eq!(
        inline_tags("#foo_bar_# #nested/a_b_# #a*b*#"),
        ["foo_bar_", "nested/a_b_", "a*b*"]
    );
    assert_eq!(inline_tags("#foo_bar_"), ["foo_bar_"]);
}
