//! Lazy, one-level project tree loading.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Config as NucleoConfig, Matcher, Utf32String};

use crate::projects::is_markdown_path;
use crate::{Error, Result, fsops};

/// The filesystem kind represented by a tree node.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TreeNodeKind {
    /// A directory that can be expanded lazily.
    Directory,
    /// A regular file.
    File,
    /// A symbolic link, which the tree reader does not follow.
    Symlink,
    /// Another filesystem object such as a socket or named pipe.
    Other,
}

/// One immediate child of a project directory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    /// NFC-normalized display name.
    pub name: String,
    /// NFC-normalized path relative to the canonical project root.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// Filesystem kind without following symbolic links.
    pub kind: TreeNodeKind,
}

struct SortableNode {
    node: TreeNode,
    folded_name: String,
}

/// Reads exactly one directory level inside a project.
///
/// Directories sort before other nodes, followed by a case-insensitive natural
/// name order. Names beginning with `.` are omitted unless `show_hidden` is true.
pub fn read_dir(project_root: &Path, rel_path: &Path, show_hidden: bool) -> Result<Vec<TreeNode>> {
    let canonical_root = project_root
        .canonicalize()
        .map_err(|source| Error::io("open the project directory", project_root, source))?;
    let directory = fsops::resolve(project_root, rel_path)?
        .canonicalize()
        .map_err(|source| Error::io("open the tree folder", rel_path, source))?;
    if !directory.is_dir() {
        return Err(Error::UnsafePath {
            path: rel_path.to_path_buf(),
            reason: "the tree path is not a folder",
        });
    }
    let directory_rel =
        directory
            .strip_prefix(&canonical_root)
            .map_err(|_| Error::PathOutsideProject {
                path: directory.clone(),
            })?;

    let entries = fs::read_dir(&directory)
        .map_err(|source| Error::io("read the tree folder", &directory, source))?;
    let mut nodes = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::io("read a tree entry", &directory, source))?;
        let raw_name = entry.file_name();
        let name = raw_name
            .to_str()
            .ok_or_else(|| Error::UnsafePath {
                path: entry.path(),
                reason: "file names must be valid Unicode",
            })?
            .nfc()
            .collect::<String>();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|source| Error::io("inspect a tree entry", entry.path(), source))?;
        let kind = kind_from_file_type(file_type);
        nodes.push(SortableNode {
            folded_name: name.to_lowercase(),
            node: TreeNode {
                rel_path: directory_rel.join(&name),
                name,
                kind,
            },
        });
    }

    nodes.sort_unstable_by(|left, right| {
        directory_rank(left.node.kind)
            .cmp(&directory_rank(right.node.kind))
            .then_with(|| natural_cmp(&left.folded_name, &right.folded_name))
            .then_with(|| left.node.name.cmp(&right.node.name))
    });
    Ok(nodes.into_iter().map(|node| node.node).collect())
}

/// Builds a tree node for an existing absolute path inside a project.
///
/// Symbolic links are inspected without being followed. The last path component
/// is NFC-normalized so the node matches [`read_dir`].
pub fn node_at(project_root: &Path, absolute: &Path) -> Result<TreeNode> {
    let canonical_root = project_root
        .canonicalize()
        .map_err(|source| Error::io("open the project directory", project_root, source))?;
    let metadata = fs::symlink_metadata(absolute)
        .map_err(|source| Error::io("inspect the path", absolute, source))?;
    let file_name = absolute.file_name().ok_or_else(|| Error::UnsafePath {
        path: absolute.to_path_buf(),
        reason: "the path does not have a file name",
    })?;
    let parent = absolute.parent().ok_or_else(|| Error::UnsafePath {
        path: absolute.to_path_buf(),
        reason: "the path does not have a parent folder",
    })?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|source| Error::io("open the path's parent directory", parent, source))?;
    let canonical = canonical_parent.join(file_name);
    let rel = canonical
        .strip_prefix(&canonical_root)
        .map_err(|_| Error::PathOutsideProject {
            path: canonical.clone(),
        })?;
    if rel.as_os_str().is_empty() {
        return Err(Error::UnsafePath {
            path: absolute.to_path_buf(),
            reason: "the project root is not a tree item",
        });
    }
    let name = file_name
        .to_str()
        .ok_or_else(|| Error::UnsafePath {
            path: absolute.to_path_buf(),
            reason: "file names must be valid Unicode",
        })?
        .nfc()
        .collect::<String>();
    Ok(TreeNode {
        rel_path: rel.with_file_name(&name),
        name,
        kind: kind_from_file_type(metadata.file_type()),
    })
}

const SEARCH_WALK_LIMIT: usize = 10_000;

/// How many Markdown files a project walk found, and whether the cap applied.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownCount {
    /// Files collected before the walk limit.
    pub files: u32,
    /// True when the walk stopped at the ten-thousand-file cap.
    pub capped: bool,
}

/// Counts Markdown files under a project without following symbolic links.
pub fn count_markdown(project_root: &Path, show_hidden: bool) -> Result<MarkdownCount> {
    let mut files = Vec::new();
    collect_markdown(
        project_root,
        Path::new(""),
        show_hidden,
        &mut files,
        SEARCH_WALK_LIMIT,
    )?;
    Ok(MarkdownCount {
        files: u32::try_from(files.len()).unwrap_or(u32::MAX),
        capped: files.len() >= SEARCH_WALK_LIMIT,
    })
}

/// Fuzzy-searches Markdown files under a project by relative path.
///
/// An empty query returns files in tree order, capped at `limit`. Hidden files
/// follow the same rule as [`read_dir`]. Symbolic links are not followed.
pub fn search_markdown(
    project_root: &Path,
    query: &str,
    show_hidden: bool,
    limit: usize,
) -> Result<Vec<TreeNode>> {
    let mut files = Vec::new();
    collect_markdown(
        project_root,
        Path::new(""),
        show_hidden,
        &mut files,
        SEARCH_WALK_LIMIT,
    )?;
    let limit = limit.min(files.len());
    let trimmed = query.trim();
    if trimmed.is_empty() {
        files.truncate(limit);
        return Ok(files);
    }

    let pattern = Pattern::parse(trimmed, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(NucleoConfig::DEFAULT.match_paths());
    let mut matches: Vec<_> = files
        .into_iter()
        .filter_map(|node| {
            let haystack = Utf32String::from(node.rel_path.to_string_lossy().as_ref());
            pattern
                .score(haystack.slice(..), &mut matcher)
                .map(|score| (score, node))
        })
        .collect();
    matches.sort_unstable_by(|(left, left_node), (right, right_node)| {
        right
            .cmp(left)
            .then_with(|| left_node.rel_path.cmp(&right_node.rel_path))
    });
    Ok(matches
        .into_iter()
        .take(limit)
        .map(|(_, node)| node)
        .collect())
}

fn collect_markdown(
    project_root: &Path,
    rel_path: &Path,
    show_hidden: bool,
    files: &mut Vec<TreeNode>,
    walk_limit: usize,
) -> Result<()> {
    if files.len() >= walk_limit {
        return Ok(());
    }
    for node in read_dir(project_root, rel_path, show_hidden)? {
        if files.len() >= walk_limit {
            break;
        }
        match node.kind {
            TreeNodeKind::Directory => {
                collect_markdown(project_root, &node.rel_path, show_hidden, files, walk_limit)?;
            }
            TreeNodeKind::File if is_markdown_path(&node.rel_path) => files.push(node),
            _ => {}
        }
    }
    Ok(())
}

fn kind_from_file_type(file_type: fs::FileType) -> TreeNodeKind {
    if file_type.is_dir() {
        TreeNodeKind::Directory
    } else if file_type.is_file() {
        TreeNodeKind::File
    } else if file_type.is_symlink() {
        TreeNodeKind::Symlink
    } else {
        TreeNodeKind::Other
    }
}

fn directory_rank(kind: TreeNodeKind) -> u8 {
    u8::from(kind != TreeNodeKind::Directory)
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let left_significant = trim_leading_zeroes(&left[left_index..left_end]);
            let right_significant = trim_leading_zeroes(&right[right_index..right_end]);
            let numeric = left_significant
                .len()
                .cmp(&right_significant.len())
                .then_with(|| left_significant.cmp(right_significant))
                .then_with(|| (left_end - left_index).cmp(&(right_end - right_index)));
            if numeric != Ordering::Equal {
                return numeric;
            }
            left_index = left_end;
            right_index = right_end;
        } else {
            let ordering = left[left_index].cmp(&right[right_index]);
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index += 1;
            right_index += 1;
        }
    }
    left.len().cmp(&right.len())
}

fn digit_run_end(value: &[u8], start: usize) -> usize {
    value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map_or(value.len(), |offset| start + offset)
}

fn trim_leading_zeroes(value: &[u8]) -> &[u8] {
    let first_nonzero = value
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(value.len().saturating_sub(1));
    &value[first_nonzero..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_markdown_stops_at_the_walk_limit() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let root = temp.path().join("project");
        fs::create_dir(&root).expect("project directory");
        for index in 0..8 {
            fs::write(root.join(format!("note-{index}.md")), b"").expect("markdown file");
        }

        let mut files = Vec::new();
        collect_markdown(&root, Path::new(""), false, &mut files, 6).expect("walk");
        assert_eq!(files.len(), 6);

        let mut nested_files = Vec::new();
        fs::create_dir(root.join("more")).expect("nested folder");
        for index in 0..4 {
            fs::write(root.join("more").join(format!("extra-{index}.md")), b"")
                .expect("nested markdown");
        }
        collect_markdown(&root, Path::new(""), false, &mut nested_files, 6).expect("nested walk");
        assert_eq!(nested_files.len(), 6);
    }
}
