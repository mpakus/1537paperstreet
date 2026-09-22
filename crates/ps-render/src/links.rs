use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use ps_core::fsops;
use pulldown_cmark::{Event, LinkType, Tag};

use crate::blocks::SpannedEvent;

pub(crate) struct Links<'input, 'context, I> {
    events: I,
    project_root: &'context Path,
    canonical_root: Option<PathBuf>,
    document_dir: PathBuf,
    project_scope: &'context str,
    destinations: HashMap<String, String>,
    marker: std::marker::PhantomData<&'input str>,
}

impl<'input, 'context, I> Links<'input, 'context, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    pub(crate) fn new(
        events: I,
        project_root: &'context Path,
        document_path: &Path,
        project_scope: &'context str,
    ) -> Self {
        Self {
            events,
            project_root,
            canonical_root: fsops::resolve(project_root, Path::new("")).ok(),
            document_dir: document_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_owned(),
            project_scope,
            destinations: HashMap::new(),
            marker: std::marker::PhantomData,
        }
    }

    fn rewrite(
        &mut self,
        destination: pulldown_cmark::CowStr<'input>,
    ) -> pulldown_cmark::CowStr<'input> {
        if let Some(rewritten) = self.destinations.get(destination.as_ref()) {
            return rewritten.clone().into();
        }

        match self.asset_url(&destination) {
            Ok(Some(url)) => {
                self.destinations
                    .insert(destination.to_string(), url.clone());
                url.into()
            }
            Ok(None) => destination,
            Err(()) => {
                self.destinations
                    .insert(destination.to_string(), "#invalid-path".to_owned());
                "#invalid-path".into()
            }
        }
    }

    fn rewrite_wiki(
        &mut self,
        destination: pulldown_cmark::CowStr<'input>,
    ) -> pulldown_cmark::CowStr<'input> {
        if let Some(rewritten) = self.destinations.get(destination.as_ref()) {
            return rewritten.clone().into();
        }

        match self.wiki_url(&destination) {
            Ok(Some(url)) => {
                self.destinations
                    .insert(destination.to_string(), url.clone());
                url.into()
            }
            Ok(None) => {
                self.destinations
                    .insert(destination.to_string(), "#missing-wiki".to_owned());
                "#missing-wiki".into()
            }
            Err(()) => {
                self.destinations
                    .insert(destination.to_string(), "#invalid-path".to_owned());
                "#invalid-path".into()
            }
        }
    }

    fn asset_url(&self, destination: &str) -> Result<Option<String>, ()> {
        if destination.starts_with('#') || has_scheme(destination) {
            return Ok(None);
        }

        let (path, suffix) = split_suffix(destination);
        if path.is_empty() {
            return Ok(None);
        }
        if self.project_scope.is_empty()
            || !self
                .project_scope
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(());
        }

        let canonical_root = self.canonical_root.as_ref().ok_or(())?;
        let relative = collapse_relative(&self.document_dir.join(path))?;
        let resolved = fsops::resolve(self.project_root, &relative).map_err(|_| ())?;
        let relative = resolved.strip_prefix(canonical_root).map_err(|_| ())?;
        let relative = relative
            .iter()
            .map(|part| part.to_str().ok_or(()))
            .collect::<Result<Vec<_>, _>>()?
            .join("/");

        Ok(Some(format!(
            "asset://localhost/{}/{relative}{suffix}",
            self.project_scope
        )))
    }

    fn wiki_url(&self, destination: &str) -> Result<Option<String>, ()> {
        if destination.starts_with('#') || has_scheme(destination) {
            return Ok(Some(destination.to_owned()));
        }

        let (path, suffix) = split_suffix(destination);
        if path.is_empty() {
            return Ok(Some(destination.to_owned()));
        }

        for candidate in wiki_candidates(path) {
            let from_document = self.document_dir.join(&candidate);
            if let Some(url) = self.existing_asset_url(&from_document, suffix)? {
                return Ok(Some(url));
            }
            if let Some(url) = self.existing_asset_url(&candidate, suffix)? {
                return Ok(Some(url));
            }
        }
        Ok(None)
    }

    fn existing_asset_url(&self, relative: &Path, suffix: &str) -> Result<Option<String>, ()> {
        let relative = collapse_relative(relative)?;
        let Ok(resolved) = fsops::resolve(self.project_root, &relative) else {
            return Ok(None);
        };
        if !resolved.is_file() {
            return Ok(None);
        }
        self.asset_url_for_relative(&relative, suffix).map(Some)
    }

    fn asset_url_for_relative(&self, relative: &Path, suffix: &str) -> Result<String, ()> {
        if self.project_scope.is_empty()
            || !self
                .project_scope
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(());
        }

        let canonical_root = self.canonical_root.as_ref().ok_or(())?;
        let resolved = fsops::resolve(self.project_root, relative).map_err(|_| ())?;
        let relative = resolved.strip_prefix(canonical_root).map_err(|_| ())?;
        let relative = relative
            .iter()
            .map(|part| part.to_str().ok_or(()))
            .collect::<Result<Vec<_>, _>>()?
            .join("/");

        Ok(format!(
            "asset://localhost/{}/{relative}{suffix}",
            self.project_scope
        ))
    }
}

fn wiki_candidates(path: &str) -> Vec<PathBuf> {
    let base = PathBuf::from(path);
    if base.extension().is_some() {
        vec![base]
    } else {
        vec![
            base.with_extension("md"),
            base.with_extension("markdown"),
            base.join("index.md"),
        ]
    }
}

/// Collapses `.` and `..` so a link can name a file in a parent folder.
/// A path that would leave the project is rejected.
fn collapse_relative(path: &Path) -> Result<PathBuf, ()> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => parts.push(name.to_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(());
                }
            }
            Component::RootDir | Component::Prefix(_) => return Err(()),
        }
    }
    Ok(parts.iter().collect())
}

impl<'input, I> Iterator for Links<'input, '_, I>
where
    I: Iterator<Item = SpannedEvent<'input>>,
{
    type Item = SpannedEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let (event, source_range) = self.events.next()?;
        let event = match event {
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { has_pothole },
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { has_pothole },
                dest_url: self.rewrite_wiki(dest_url),
                title,
                id,
            }),
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Link {
                link_type,
                dest_url: self.rewrite(dest_url),
                title,
                id,
            }),
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Image {
                link_type,
                dest_url: self.rewrite(dest_url),
                title,
                id,
            }),
            event => event,
        };
        Some((event, source_range))
    }
}

pub(crate) fn has_scheme(destination: &str) -> bool {
    destination
        .char_indices()
        .take_while(|(_, character)| !matches!(character, '/' | '?' | '#'))
        .any(|(_, character)| character == ':')
}

pub(crate) fn split_suffix(destination: &str) -> (&str, &str) {
    destination
        .char_indices()
        .find(|(_, character)| matches!(character, '?' | '#'))
        .map_or((destination, ""), |(index, _)| destination.split_at(index))
}
