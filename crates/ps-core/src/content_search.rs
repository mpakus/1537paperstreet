//! Full-text search of project files.
//!
//! A query is a case-insensitive regular expression. When it does not compile,
//! the same characters are searched as plain text. A query without regular
//! expression syntax also fuzzy-matches file paths, the way a name search
//! tolerates a typo.

use std::fs;
use std::path::{Path, PathBuf};

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Config as NucleoConfig, Matcher, Utf32String};
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::tree::{TreeNodeKind, read_dir};
use crate::{Error, Result, fsops};

/// Files visited before the walk stops.
const WALK_LIMIT: usize = 10_000;
/// Bytes read from one file. Larger files can still match by path.
const MAX_FILE_BYTES: u64 = 1_048_576;
/// Characters kept around a matching line.
const SNIPPET_CHARS: usize = 160;

const SKIP_DIRECTORIES: &[&str] = &[".git", "node_modules", "target"];

/// One file that matched the query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchHit {
    /// Project-relative path of the matching file.
    #[ts(type = "string")]
    pub rel_path: PathBuf,
    /// 1-based line of the first match. Zero when only the path matched.
    pub line: u32,
    /// A short excerpt of the matching line. Empty for a path-only match.
    pub snippet: String,
}

/// Files matching a text query inside one folder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TextSearch {
    /// Matching files, content hits first.
    pub hits: Vec<TextSearchHit>,
    /// True when the query was not a valid regular expression.
    pub literal: bool,
    /// True when the walk stopped at the file cap.
    pub capped: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum Rank {
    Content,
    Path,
    Fuzzy,
}

struct Candidate {
    rel_path: PathBuf,
    line: u32,
    snippet: String,
    rank: Rank,
    score: u32,
}

/// Searches file contents and paths under `scope` (empty means the project root).
///
/// Symbolic links are not followed. `.git`, `node_modules`, and `target` are
/// skipped. Hidden files follow `show_hidden`. At most `limit` hits are returned.
pub fn search_documents(
    project_root: &Path,
    scope: &Path,
    query: &str,
    show_hidden: bool,
    limit: usize,
) -> Result<TextSearch> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(TextSearch {
            hits: Vec::new(),
            literal: false,
            capped: false,
        });
    }

    let (pattern, literal) = compile_query(query)?;
    let fuzzy = !literal && !looks_like_regex(query);
    let mut matcher = Matcher::new(NucleoConfig::DEFAULT.match_paths());
    let nucleo = if fuzzy {
        Some(Pattern::parse(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
        ))
    } else {
        None
    };

    let mut files = Vec::new();
    let mut capped = false;
    collect_files(project_root, scope, show_hidden, &mut files, &mut capped)?;

    let mut hits = Vec::new();
    let mut unmatched = Vec::new();
    for rel_path in files {
        let display = rel_path.to_string_lossy().replace('\\', "/");
        let path_matched = pattern.is_match(&display);
        let content = read_searchable_text(project_root, &rel_path)?;
        if let Some((line, snippet)) = content
            .as_deref()
            .and_then(|text| first_line(text, &pattern))
        {
            hits.push(Candidate {
                rel_path,
                line,
                snippet,
                rank: Rank::Content,
                score: 0,
            });
        } else if path_matched {
            hits.push(Candidate {
                rel_path,
                line: 0,
                snippet: String::new(),
                rank: Rank::Path,
                score: 0,
            });
        } else {
            unmatched.push(rel_path);
        }
    }

    if let Some(nucleo_pattern) = nucleo {
        for rel_path in unmatched {
            let haystack = Utf32String::from(rel_path.to_string_lossy().as_ref());
            if let Some(score) = nucleo_pattern.score(haystack.slice(..), &mut matcher) {
                hits.push(Candidate {
                    rel_path,
                    line: 0,
                    snippet: String::new(),
                    rank: Rank::Fuzzy,
                    score,
                });
            }
        }
    }

    hits.sort_by(|left, right| {
        left.rank
            .cmp(&right.rank)
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| left.rel_path.cmp(&right.rel_path))
    });
    hits.truncate(limit);

    Ok(TextSearch {
        hits: hits
            .into_iter()
            .map(|hit| TextSearchHit {
                rel_path: hit.rel_path,
                line: hit.line,
                snippet: hit.snippet,
            })
            .collect(),
        literal,
        capped,
    })
}

fn collect_files(
    project_root: &Path,
    rel_path: &Path,
    show_hidden: bool,
    files: &mut Vec<PathBuf>,
    capped: &mut bool,
) -> Result<()> {
    if files.len() >= WALK_LIMIT {
        *capped = true;
        return Ok(());
    }
    for node in read_dir(project_root, rel_path, show_hidden)? {
        if files.len() >= WALK_LIMIT {
            *capped = true;
            break;
        }
        match node.kind {
            TreeNodeKind::Directory => {
                if SKIP_DIRECTORIES.contains(&node.name.as_str()) {
                    continue;
                }
                collect_files(project_root, &node.rel_path, show_hidden, files, capped)?;
            }
            TreeNodeKind::File | TreeNodeKind::Symlink => files.push(node.rel_path),
            TreeNodeKind::Other => {}
        }
    }
    Ok(())
}

fn compile_query(query: &str) -> Result<(regex::Regex, bool)> {
    if let Some(pattern) = build_regex(query) {
        return Ok((pattern, false));
    }
    let escaped = regex::escape(query);
    build_regex(&escaped)
        .map(|pattern| (pattern, true))
        .ok_or_else(|| Error::UnsafePath {
            path: PathBuf::from(query),
            reason: "that search is too large to run",
        })
}

fn build_regex(query: &str) -> Option<regex::Regex> {
    RegexBuilder::new(query)
        .case_insensitive(true)
        .size_limit(1 << 20)
        .dfa_size_limit(1 << 20)
        .build()
        .ok()
}

fn looks_like_regex(query: &str) -> bool {
    query.chars().any(|character| {
        matches!(
            character,
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\'
        )
    })
}

fn read_searchable_text(project_root: &Path, rel_path: &Path) -> Result<Option<String>> {
    let root = project_root
        .canonicalize()
        .map_err(|source| Error::io("open the project directory", project_root, source))?;
    let unfollowed = root.join(rel_path);
    let metadata = match fs::symlink_metadata(&unfollowed) {
        Ok(metadata) => metadata,
        Err(source) => {
            return Err(Error::io("search a file", &unfollowed, source));
        }
    };
    // `symlink_metadata` does not follow the link, so a link is not a file.
    if !metadata.file_type().is_file() || metadata.len() > MAX_FILE_BYTES {
        return Ok(None);
    }
    let absolute = match fsops::resolve(project_root, rel_path) {
        Ok(path) => path,
        Err(Error::PathOutsideProject { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let bytes =
        fs::read(&absolute).map_err(|source| Error::io("search a file", &absolute, source))?;
    if bytes.contains(&0) {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}

fn first_line(text: &str, pattern: &regex::Regex) -> Option<(u32, String)> {
    for (index, raw) in text.split('\n').enumerate() {
        let line = raw.trim_end_matches('\r');
        let Some(found) = pattern.find(line) else {
            continue;
        };
        let line_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        return Some((line_no, snippet_around(line, found.start())));
    }
    None
}

fn snippet_around(line: &str, byte_start: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= SNIPPET_CHARS {
        return line.trim().to_owned();
    }
    let char_start = line
        .get(..byte_start)
        .map(|prefix| prefix.chars().count())
        .unwrap_or(0);
    let window = SNIPPET_CHARS / 2;
    let from = char_start.saturating_sub(window);
    let to = (char_start + window).min(chars.len());
    let mut excerpt: String = chars[from..to].iter().collect();
    excerpt = excerpt.trim().to_owned();
    if from > 0 {
        excerpt.insert(0, '…');
    }
    if to < chars.len() {
        excerpt.push('…');
    }
    excerpt
}
