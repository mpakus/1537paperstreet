//! Measure bounded tag indexing against the existing Library scan on synthetic notes.
use ps_core::{
    library::{self, LibraryProject},
    tags::{TagIndex, TagQuery},
};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let mut projects = Vec::new();
    for (id, name) in [
        ("research", "Research"),
        ("product", "Product"),
        ("journal", "Journal"),
        ("engineering", "Engineering"),
    ] {
        let root = temporary.path().join(id);
        std::fs::create_dir(&root)?;
        for n in 0..250 {
            let text = format!(
                "---\ntitle: '{name} note {n}'\ntags: [research]\n---\n# {name}\n#project alpha# #work/design #topic{} #rust\n{}\n",
                n % 36,
                "Research and decisions. ".repeat(100)
            );
            std::fs::write(root.join(format!("note-{n}.md")), text)?;
        }
        projects.push(LibraryProject {
            id: id.into(),
            name: name.into(),
            root,
        });
    }
    let mut index = TagIndex::default();
    let mut baseline = Vec::new();
    let mut build = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let scan = library::scan(
            &projects,
            &Default::default(),
            &Default::default(),
            |_| vec![],
        )?;
        assert_eq!(scan.notes.len(), 1000);
        baseline.push(start.elapsed().as_secs_f64() * 1000.0);
        let start = Instant::now();
        let snapshot = index.query(&projects, &Default::default(), true, ps_render::inline_tags)?;
        assert_eq!(snapshot.notes.len(), 1000);
        build.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let query = TagQuery {
        tags: vec!["work".into(), "rust".into()],
        ..Default::default()
    };
    let start = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(index.query(&projects, &query, false, ps_render::inline_tags)?);
    }
    baseline.sort_by(f64::total_cmp);
    build.sort_by(f64::total_cmp);
    eprintln!(
        "1000 notes / 4 folders: baseline scan median {:.2}ms; tag rebuild median {:.2}ms; cached AND query mean {:.2}ms",
        baseline[2],
        build[2],
        start.elapsed().as_secs_f64() * 10.0
    );
    let all = index.query(
        &projects,
        &Default::default(),
        false,
        ps_render::inline_tags,
    )?;
    let focused = index.query(
        &projects,
        &TagQuery {
            tags: vec!["topic0".into()],
            ..Default::default()
        },
        false,
        ps_render::inline_tags,
    )?;
    let folder = index.query(
        &projects,
        &TagQuery {
            tags: vec!["topic0".into()],
            project_id: "research".into(),
            ..Default::default()
        },
        false,
        ps_render::inline_tags,
    )?;
    println!(
        "{}",
        serde_json::to_string(
            &serde_json::json!({"all": all, "focused": focused, "folder": folder})
        )?
    );
    Ok(())
}
