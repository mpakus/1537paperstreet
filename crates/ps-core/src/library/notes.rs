use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{invalid, *};
use crate::{Result, docio, fsops};

pub(super) const MAX_BYTES: usize = 2 * 1024 * 1024;
const TYPES: &[&str] = &[
    "note",
    "spec",
    "decision",
    "paper",
    "literature-review",
    "research-question",
    "investigation",
    "benchmark",
    "experiment",
];

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Metadata {
    title: String,
    #[serde(rename = "type")]
    note_type: String,
    status: String,
    tags: Vec<String>,
    source: String,
}

pub(super) fn safe_path(root: &Path, path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    if !crate::projects::is_markdown_path(relative) || crate::encrypted::is_age_path(relative) {
        return Err(invalid(
            "Choose a plaintext Markdown file. Protected notes are excluded from Library.",
        ));
    }
    let resolved = fsops::resolve(root, relative)?;
    let mut prefix = PathBuf::new();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            prefix.push(part);
            let candidate = fsops::resolve(root, &prefix)?;
            if fs::symlink_metadata(&candidate).is_ok_and(|meta| meta.file_type().is_symlink()) {
                return Err(invalid("Library does not follow symbolic links."));
            }
        }
    }
    Ok(resolved)
}

pub(super) fn read(root: &Path, path: &str) -> Result<(String, String)> {
    let absolute = safe_path(root, path)?;
    let file = fs::File::open(&absolute)
        .map_err(|e| crate::Error::io("read the library source", &absolute, e))?;
    if !file
        .metadata()
        .map_err(|e| crate::Error::io("inspect the library source", &absolute, e))?
        .is_file()
    {
        return Err(invalid("The library source is not a regular file."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| crate::Error::io("read the library source", &absolute, e))?;
    if bytes.len() > MAX_BYTES || bytes.contains(&0) || bytes.starts_with(b"age-encryption.org/") {
        return Err(invalid(
            "Library reads plaintext UTF-8 Markdown up to 2 MB per file.",
        ));
    }
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let text =
        String::from_utf8(bytes).map_err(|_| invalid("This library source is not UTF-8 text."))?;
    Ok((text.trim_start_matches('\u{feff}').to_owned(), hash))
}

/// Creates a typed note using existing no-overwrite creation and conflict-aware saving.
pub fn create_note(root: &Path, input: &NoteInput) -> Result<()> {
    safe_path(root, &input.path)?;
    if input.title.trim().is_empty()
        || input.title.len() > 300
        || input.title.contains(['\n', '\r'])
        || !TYPES.contains(&input.note_type.as_str())
    {
        return Err(invalid(
            "Enter a title of 1–300 characters and choose a note type.",
        ));
    }
    if input.status.len() > 100 || input.tags.len() > 1000 || input.source.len() > 2000 {
        return Err(invalid("Shorten the status, topics or source reference."));
    }
    let metadata = Metadata {
        title: input.title.trim().into(),
        note_type: input.note_type.clone(),
        status: input.status.trim().into(),
        tags: input
            .tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        source: input.source.trim().into(),
    };
    let yaml = serde_saphyr::to_string(&metadata)
        .map_err(|_| invalid("Could not prepare note properties."))?;
    let headings = match input.note_type.as_str() {
        "spec" => &[
            "Problem",
            "Requirements",
            "Acceptance criteria",
            "References",
        ][..],
        "decision" => &["Context", "Decision", "Consequences", "References"],
        "paper" => &[
            "Research question",
            "Methods",
            "Findings",
            "Limitations",
            "References",
        ],
        "literature-review" => &["Question", "Sources", "Synthesis", "Gaps"],
        "research-question" => &["Question", "Motivation", "Evidence", "Next steps"],
        "experiment" | "benchmark" => &[
            "Hypothesis",
            "Environment",
            "Procedure",
            "Results",
            "Interpretation",
        ],
        "investigation" => &[
            "Question",
            "Approaches",
            "Findings",
            "Decision",
            "References",
        ],
        _ => &["Notes", "References"],
    };
    let mut text = format!("---\n{yaml}---\n\n# {}\n", metadata.title);
    for heading in headings {
        text.push_str(&format!("\n## {heading}\n\n"));
    }
    fsops::create_file(root, Path::new(&input.path))?;
    // Use the known empty-file baseline: never adopt a concurrent writer's bytes.
    docio::write_doc(
        root,
        Path::new(&input.path),
        &text,
        blake3::hash(b"").to_hex().as_ref(),
        docio::RestoreTraits {
            eol: docio::LineEnding::Lf,
            bom: false,
            trailing_newline: true,
        },
    )?;
    Ok(())
}

fn metadata(text: &str) -> Result<Metadata> {
    let mut lines = text.split_inclusive('\n');
    if lines.next().is_none_or(|line| line.trim_end() != "---") {
        return Ok(Metadata::default());
    }
    let mut yaml = String::new();
    for line in lines {
        if matches!(line.trim_end(), "---" | "...") {
            // Metadata is untrusted, including unknown fields skipped by Serde.
            // Bound parser work as well as the source bytes checked below.
            let options = serde_saphyr::options! {
                alias_limits: serde_saphyr::alias_limits! {
                    max_total_replayed_events: 8192,
                    max_replay_stack_depth: 16,
                    max_alias_expansions_per_anchor: 64,
                },
                budget: serde_saphyr::budget! {
                    flow_nesting_limit: 32,
                    max_depth: 32,
                    max_events: 8192,
                    max_nodes: 4096,
                    max_aliases: 64,
                    max_anchors: 64,
                    max_recorded_anchor_events: 8192,
                    max_recorded_anchor_bytes: 256 * 1024,
                    max_total_scalar_bytes: 256 * 1024,
                    max_merge_keys: 64,
                    max_documents: 1,
                    max_inclusion_depth: 0,
                },
            };
            return serde_saphyr::from_str_with_options(&yaml, options).map_err(|_| {
                invalid("Front matter could not be read. Use text fields and a list of tags.")
            });
        }
        if yaml.len() + line.len() > 64 * 1024 {
            break;
        }
        yaml.push_str(line);
    }
    Err(invalid("Front matter is unclosed or exceeds 64 KB."))
}

/// Scans registered roots on demand with bounded I/O and explicit partial results.
pub fn scan(
    projects: &[LibraryProject],
    query: &LibraryQuery,
    data: &LibraryData,
    sections: impl Fn(&str) -> Vec<NoteSection>,
) -> Result<LibrarySnapshot> {
    let mut result = LibrarySnapshot {
        notes: vec![],
        sets: data.sets.clone(),
        revision: data.revision.clone(),
        notices: vec![],
        note_types: TYPES.iter().map(|s| (*s).into()).collect(),
    };
    let mut entries = 0usize;
    let mut bytes = 0usize;
    let mut documents = 0usize;
    let mut capped = false;
    for project in projects
        .iter()
        .filter(|p| query.project_ids.is_empty() || query.project_ids.contains(&p.id))
    {
        let mut pending = vec![(PathBuf::new(), 0usize)];
        while let Some((folder, depth)) = pending.pop() {
            if entries >= 100_000 || documents >= 5000 || bytes >= 64 * 1024 * 1024 {
                capped = true;
                break;
            }
            let directory = match fsops::resolve(&project.root, &folder).and_then(|path| {
                fs::read_dir(&path)
                    .map_err(|e| crate::Error::io("read the library folder", &path, e))
            }) {
                Ok(value) => value,
                Err(error) => {
                    notice(&mut result, format!("{}: {error}", project.name));
                    continue;
                }
            };
            for entry in directory {
                entries += 1;
                if entries >= 100_000 || documents >= 5000 || bytes >= 64 * 1024 * 1024 {
                    capped = true;
                    break;
                }
                let entry = match entry {
                    Ok(value) => value,
                    Err(_) => {
                        notice(&mut result, "A folder entry could not be read.".into());
                        continue;
                    }
                };
                if entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
                    continue;
                }
                let relative = folder.join(entry.file_name());
                let path = relative.to_string_lossy().replace('\\', "/");
                let meta = match fsops::resolve(&project.root, &relative).and_then(|absolute| {
                    fs::symlink_metadata(&absolute)
                        .map_err(|e| crate::Error::io("inspect a library entry", &absolute, e))
                }) {
                    Ok(meta) => meta,
                    Err(error) => {
                        notice(&mut result, format!("{path}: {error}"));
                        continue;
                    }
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    if ![".git", "node_modules", "target", ".1537paperstreet"]
                        .contains(&entry.file_name().to_string_lossy().as_ref())
                    {
                        if depth < 64 {
                            pending.push((relative, depth + 1));
                        } else {
                            capped = true;
                        }
                    }
                    continue;
                }
                if !meta.is_file() || !crate::graph::is_graph_document(&relative) {
                    continue;
                }
                documents += 1;
                match read(&project.root, &path) {
                    Ok((text, hash)) => {
                        bytes += text.len();
                        let info = match metadata(&text) {
                            Ok(value) => value,
                            Err(error) => {
                                notice(&mut result, format!("{path}: {error}"));
                                Metadata::default()
                            }
                        };
                        let title = if info.title.is_empty() {
                            relative
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned()
                        } else {
                            info.title
                        };
                        let note_type = if info.note_type.is_empty() {
                            "note".into()
                        } else {
                            info.note_type
                        };
                        if !matches_filter(&note_type, &query.note_type)
                            || !matches_filter(&info.status, &query.status)
                            || (!query.tag.is_empty()
                                && !info
                                    .tags
                                    .iter()
                                    .any(|tag| tag.eq_ignore_ascii_case(query.tag.trim())))
                            || !format!("{title} {path} {}", project.name)
                                .to_lowercase()
                                .contains(&query.search.trim().to_lowercase())
                        {
                            continue;
                        }
                        let baseline = data
                            .reviews
                            .iter()
                            .find(|source| source.project_id == project.id && source.path == path);
                        let freshness = baseline.map_or("unreviewed", |source| {
                            if source.hash == hash {
                                "reviewed"
                            } else {
                                "changed"
                            }
                        });
                        if !matches_filter(freshness, &query.freshness) {
                            continue;
                        }
                        result.notes.push(LibraryNote {
                            project_id: project.id.clone(),
                            project_name: project.name.clone(),
                            path,
                            title,
                            note_type,
                            status: info.status,
                            tags: info.tags,
                            source: info.source,
                            hash,
                            freshness: freshness.into(),
                            sections: sections(&text),
                        });
                    }
                    Err(error) => notice(&mut result, format!("{path}: {error}")),
                }
            }
        }
    }
    if capped {
        if result.notices.len() == 40 {
            result.notices.pop();
        }
        notice(&mut result, "Partial results: Library scans up to 5,000 notes, 100,000 entries, 64 MB and 64 folder levels. Narrow the project scope.".into());
    }
    result.notes.sort_by_cached_key(|note| {
        (
            note.title.to_lowercase(),
            note.project_id.clone(),
            note.path.clone(),
        )
    });
    Ok(result)
}

fn matches_filter(value: &str, filter: &str) -> bool {
    filter.trim().is_empty() || value.eq_ignore_ascii_case(filter.trim())
}
fn notice(result: &mut LibrarySnapshot, message: String) {
    if result.notices.len() < 40 {
        result.notices.push(message);
    }
}
