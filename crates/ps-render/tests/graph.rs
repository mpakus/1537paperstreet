use std::{fs, path::Path};

use ps_core::graph::GraphQuery;
use ps_render::project_graph;

#[test]
fn graph_tracks_real_links_and_reuses_persistent_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("research")).unwrap();
    fs::write(root.path().join("index.md"), "[[research/paper|Paper]] [again](research/paper.md#result) [[future]] ![image](image.md) `[[code]]`\n\n```md\n[[fenced]]\n```\n[web](https://example.com) [escape](../outside.md) [secret](secret.md.age)").unwrap();
    fs::write(root.path().join("research/paper.md"), "[[index]]").unwrap();
    fs::write(root.path().join("orphan.markdown"), "body-never-in-cache").unwrap();
    fs::write(
        root.path().join("secret.md.age"),
        "[[index]] encrypted-never-in-cache",
    )
    .unwrap();
    let query = GraphQuery::default();
    let first = project_graph(cache.path(), root.path(), &query).unwrap();
    assert_eq!(first.total_nodes, 3);
    assert_eq!(first.total_edges, 2);
    assert_eq!(first.parsed_files, 3);
    assert_eq!(first.cached_files, 0);
    assert!(
        first
            .nodes
            .iter()
            .all(|node| node.x.is_finite() && node.y.is_finite())
    );
    let warm = project_graph(cache.path(), root.path(), &query).unwrap();
    assert_eq!(warm.parsed_files, 0);
    assert_eq!(warm.cached_files, 3);
    assert_eq!(first.nodes, warm.nodes);
    fs::write(root.path().join("future.md"), "new").unwrap();
    let added = project_graph(cache.path(), root.path(), &query).unwrap();
    assert_eq!(
        added.total_edges, 3,
        "cached unresolved links must resolve after creation"
    );
    fs::write(root.path().join("research/paper.md"), "changed").unwrap();
    fs::remove_file(root.path().join("future.md")).unwrap();
    let changed = project_graph(cache.path(), root.path(), &query).unwrap();
    assert_eq!(changed.total_edges, 1);
    assert_eq!(changed.parsed_files, 1);
    for entry in fs::read_dir(cache.path()).unwrap() {
        let bytes = fs::read(entry.unwrap().path()).unwrap();
        for secret in [
            "body-never-in-cache",
            "encrypted-never-in-cache",
            "secret.md.age",
        ] {
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|part| part == secret.as_bytes())
            );
        }
    }
}

#[test]
fn graph_queries_isolate_projects_and_rebuild_only_cache() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let source = "[[b]] [[c]]";
    for (path, text) in [
        ("a.md", source),
        ("b.md", "[[c]]"),
        ("c.md", ""),
        ("alone.md", ""),
    ] {
        fs::write(root.path().join(path), text).unwrap();
    }
    let mut query = GraphQuery {
        focus: Some("b.md".into()),
        ..GraphQuery::default()
    };
    let local = project_graph(cache.path(), root.path(), &query).unwrap();
    assert_eq!(local.nodes.len(), 3);
    query.focus = None;
    query.search = "alone".into();
    assert_eq!(
        project_graph(cache.path(), root.path(), &query)
            .unwrap()
            .nodes
            .len(),
        1
    );
    query.hide_isolated = true;
    assert!(
        project_graph(cache.path(), root.path(), &query)
            .unwrap()
            .nodes
            .is_empty()
    );
    assert!(
        project_graph(cache.path(), other.path(), &GraphQuery::default())
            .unwrap()
            .nodes
            .is_empty()
    );
    query = GraphQuery {
        rebuild: true,
        ..GraphQuery::default()
    };
    assert_eq!(
        project_graph(cache.path(), root.path(), &query)
            .unwrap()
            .parsed_files,
        4
    );
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        source
    );
    assert!(project_graph(cache.path(), Path::new("/not/a/project"), &query).is_err());
}

#[test]
fn damaged_cache_is_reported_and_explicit_rebuild_preserves_source() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "original\r\n").unwrap();
    project_graph(cache.path(), root.path(), &GraphQuery::default()).unwrap();
    let db = fs::read_dir(cache.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(db, "damaged").unwrap();
    assert!(project_graph(cache.path(), root.path(), &GraphQuery::default()).is_err());
    let graph = project_graph(
        cache.path(),
        root.path(),
        &GraphQuery {
            rebuild: true,
            ..GraphQuery::default()
        },
    )
    .unwrap();
    assert_eq!(graph.parsed_files, 1);
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"original\r\n");
}

#[test]
fn standard_links_references_percent_encoding_and_wiki_precedence() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("sub")).unwrap();
    for path in [
        "space note.md",
        "100%20.md",
        "choice.md",
        "sub/choice.md",
        "ref.MD",
    ] {
        fs::write(root.path().join(path), "").unwrap();
    }
    fs::write(root.path().join("sub/start.md"), "[space](../space%20note.md#heading)\n[[100%20]] [[choice]]\n[ref][r]\n\n[r]: ../ref.MD\n\n<a href=\"../choice.md\">html</a>\n").unwrap();
    let graph = project_graph(cache.path(), root.path(), &GraphQuery::default()).unwrap();
    let targets: Vec<_> = graph
        .edges
        .iter()
        .map(|edge| graph.nodes[edge.target as usize].path.as_str())
        .collect();
    assert_eq!(
        targets,
        vec!["100%20.md", "ref.MD", "space note.md", "sub/choice.md"]
    );
}

#[test]
fn graph_is_bounded_and_reports_unsupported_files() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for i in 0..601 {
        fs::write(root.path().join(format!("note-{i:04}.md")), "").unwrap();
    }
    fs::write(
        root.path().join("oversized.md"),
        vec![b'x'; 2 * 1024 * 1024 + 1],
    )
    .unwrap();
    fs::write(root.path().join("invalid.md"), [0xff, 0xff]).unwrap();
    let graph = project_graph(cache.path(), root.path(), &GraphQuery::default()).unwrap();
    assert_eq!(graph.total_nodes, 601);
    assert_eq!(graph.nodes.len(), 600);
    assert!(graph.notices.iter().any(|notice| notice.contains("600")));
    assert!(
        graph
            .notices
            .iter()
            .any(|notice| notice.contains("Skipped 2"))
    );
}

#[test]
fn a_large_neighborhood_always_keeps_its_focus_note_visible() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for i in 0..601 {
        fs::write(root.path().join(format!("note-{i:04}.md")), "[[zz-hub]]").unwrap();
    }
    fs::write(root.path().join("zz-hub.md"), "").unwrap();
    let graph = project_graph(
        cache.path(),
        root.path(),
        &GraphQuery {
            focus: Some("zz-hub.md".into()),
            ..GraphQuery::default()
        },
    )
    .unwrap();
    assert_eq!(graph.nodes.len(), 600);
    assert!(graph.nodes.iter().any(|node| node.path == "zz-hub.md"));
    assert_eq!(graph.edges.len(), 599);
}

#[test]
#[ignore = "manual release-mode performance measurement"]
fn graph_cold_and_warm_timings() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for i in 0..300 {
        fs::write(
            root.path().join(format!("note-{i}.md")),
            format!(
                "# Note {i}\n\n[[note-{}]] [[note-{}]]\n{}",
                (i + 1) % 300,
                (i + 7) % 300,
                "Research notes. ".repeat(3000)
            ),
        )
        .unwrap();
    }
    for label in ["cold", "warm"] {
        let start = std::time::Instant::now();
        let graph = project_graph(cache.path(), root.path(), &GraphQuery::default()).unwrap();
        eprintln!(
            "graph {label}: {:?}; {} parsed, {} reused, {} edges",
            start.elapsed(),
            graph.parsed_files,
            graph.cached_files,
            graph.total_edges
        );
    }
}

#[cfg(unix)]
#[test]
fn graph_never_follows_symlinks_outside_project() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.md"), "private").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
    fs::write(root.path().join("index.md"), "[escape](linked/outside.md)").unwrap();
    let graph = project_graph(cache.path(), root.path(), &GraphQuery::default()).unwrap();
    assert_eq!(graph.total_nodes, 1);
    assert!(graph.edges.is_empty());
}
