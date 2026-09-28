//! Debounced project filesystem observation.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TrySendError};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

use crate::{Error, Result, fsops};

const DEBOUNCE: Duration = Duration::from_millis(150);
const RAW_EVENT_QUEUE: usize = 1024;

/// A coalesced project tree update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WatchUpdate {
    /// Relative paths affected during one debounce window.
    PathsChanged {
        /// Sorted, deduplicated project-relative paths.
        #[ts(type = "Array<string>")]
        paths: Vec<PathBuf>,
    },
    /// Expanded directories to read again after events may have been lost.
    RescanExpanded {
        /// Only the currently expanded project-relative directories.
        #[ts(type = "Array<string>")]
        paths: Vec<PathBuf>,
    },
}

/// Coalesced filesystem update emitted to the UI.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsChangedEvent {
    /// Project that produced the update.
    pub project_id: String,
    /// Debounced paths or an expanded-directory rescan.
    pub update: WatchUpdate,
}

/// A recursive project watcher that emits debounced updates.
pub struct ProjectWatcher {
    root: PathBuf,
    expanded: Arc<RwLock<Vec<PathBuf>>>,
    updates: Receiver<WatchUpdate>,
    watcher: Option<RecommendedWatcher>,
    worker: Option<JoinHandle<()>>,
}

impl ProjectWatcher {
    /// Starts watching one project with the platform-recommended backend.
    pub fn start(project_root: &Path) -> Result<Self> {
        let root = project_root
            .canonicalize()
            .map_err(|source| Error::io("open the watched project", project_root, source))?;
        if !root.is_dir() {
            return Err(Error::UnsafePath {
                path: project_root.to_path_buf(),
                reason: "the watched project path is not a folder",
            });
        }

        let (raw_sender, raw_receiver) = mpsc::sync_channel(RAW_EVENT_QUEUE);
        let queue_overflowed = Arc::new(AtomicBool::new(false));
        let callback_overflow = Arc::clone(&queue_overflowed);
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if let Err(TrySendError::Full(_)) = raw_sender.try_send(event) {
                callback_overflow.store(true, Ordering::Release);
            }
        })
        .map_err(|source| Error::Notify {
            action: "start filesystem observation",
            path: root.clone(),
            source,
        })?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|source| Error::Notify {
                action: "watch the project directory",
                path: root.clone(),
                source,
            })?;

        let expanded = Arc::new(RwLock::new(Vec::new()));
        let worker_expanded = Arc::clone(&expanded);
        let worker_root = root.clone();
        let (update_sender, updates) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("paperstreet-fs-watch".to_owned())
            .spawn(move || {
                watch_loop(
                    &worker_root,
                    raw_receiver,
                    update_sender,
                    queue_overflowed,
                    worker_expanded,
                );
            })
            .map_err(|source| Error::io("start the filesystem watcher worker", &root, source))?;

        Ok(Self {
            root,
            expanded,
            updates,
            watcher: Some(watcher),
            worker: Some(worker),
        })
    }

    /// Replaces the set of expanded directories used for overflow recovery.
    pub fn set_expanded(&self, paths: &[PathBuf]) -> Result<()> {
        let mut validated = BTreeSet::new();
        for rel_path in paths {
            let directory = fsops::resolve(&self.root, rel_path)?
                .canonicalize()
                .map_err(|source| Error::io("open an expanded tree folder", rel_path, source))?;
            if !directory.is_dir() {
                return Err(Error::UnsafePath {
                    path: rel_path.clone(),
                    reason: "an expanded tree path is not a folder",
                });
            }
            let relative = directory
                .strip_prefix(&self.root)
                .map_err(|_| Error::PathOutsideProject {
                    path: directory.clone(),
                })?
                .to_path_buf();
            validated.insert(relative);
        }
        let mut expanded = match self.expanded.write() {
            Ok(expanded) => expanded,
            Err(poisoned) => poisoned.into_inner(),
        };
        *expanded = validated.into_iter().collect();
        Ok(())
    }

    /// Waits up to `timeout` for the next coalesced update.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<WatchUpdate> {
        self.updates.recv_timeout(timeout).ok()
    }

    /// Returns the next coalesced update without waiting.
    pub fn try_recv(&self) -> Option<WatchUpdate> {
        self.updates.try_recv().ok()
    }
}

impl Drop for ProjectWatcher {
    fn drop(&mut self) {
        drop(self.watcher.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn watch_loop(
    root: &Path,
    raw_events: Receiver<notify::Result<Event>>,
    updates: mpsc::Sender<WatchUpdate>,
    queue_overflowed: Arc<AtomicBool>,
    expanded: Arc<RwLock<Vec<PathBuf>>>,
) {
    while let Ok(first) = raw_events.recv() {
        let deadline = Instant::now() + DEBOUNCE;
        let mut changed = BTreeSet::new();
        let mut overflow = queue_overflowed.swap(false, Ordering::AcqRel);
        collect_event(root, first, &mut changed, &mut overflow);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match raw_events.recv_timeout(remaining) {
                Ok(event) => collect_event(root, event, &mut changed, &mut overflow),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        overflow |= queue_overflowed.swap(false, Ordering::AcqRel);
        let expanded_paths = match expanded.read() {
            Ok(expanded) => expanded.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        if let Some(update) = finish_update(changed, overflow, expanded_paths)
            && updates.send(update).is_err()
        {
            return;
        }
    }
}

fn collect_event(
    root: &Path,
    event: notify::Result<Event>,
    changed: &mut BTreeSet<PathBuf>,
    overflow: &mut bool,
) {
    let Ok(event) = event else {
        *overflow = true;
        return;
    };
    if event.need_rescan() {
        *overflow = true;
        return;
    }
    // inotify reports opens and reads; treating them as changes makes every
    // document load trigger another reload.
    if matches!(event.kind, notify::EventKind::Access(_)) {
        return;
    }
    for path in event.paths {
        if let Ok(relative) = path.strip_prefix(root)
            && let Some(relative) = normalize_relative(relative)
        {
            changed.insert(relative);
        }
    }
}

fn normalize_relative(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        normalized.push(name.to_str()?.nfc().collect::<String>());
    }
    Some(normalized)
}

fn finish_update(
    changed: BTreeSet<PathBuf>,
    overflow: bool,
    expanded: Vec<PathBuf>,
) -> Option<WatchUpdate> {
    if overflow {
        Some(WatchUpdate::RescanExpanded { paths: expanded })
    } else if changed.is_empty() {
        None
    } else {
        Some(WatchUpdate::PathsChanged {
            paths: changed.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_discards_paths_and_rescans_only_expanded_nodes() {
        let changed = [PathBuf::from("closed/file.md")].into_iter().collect();
        let expanded = vec![PathBuf::new(), PathBuf::from("open")];

        let update = finish_update(changed, true, expanded.clone()).expect("rescan update");

        assert_eq!(update, WatchUpdate::RescanExpanded { paths: expanded });
    }

    #[test]
    fn ordinary_events_are_sorted_and_deduplicated() {
        let changed = [
            PathBuf::from("b.md"),
            PathBuf::from("a.md"),
            PathBuf::from("b.md"),
        ]
        .into_iter()
        .collect();

        let update = finish_update(changed, false, Vec::new()).expect("paths update");

        assert_eq!(
            update,
            WatchUpdate::PathsChanged {
                paths: vec![PathBuf::from("a.md"), PathBuf::from("b.md")]
            }
        );
    }

    #[test]
    fn quiet_windows_emit_nothing() {
        assert!(finish_update(BTreeSet::new(), false, Vec::new()).is_none());
    }

    #[test]
    fn collect_event_marks_backend_errors_and_rescans_as_overflow() {
        let root = Path::new("/tmp/project");
        let mut changed = BTreeSet::new();
        let mut overflow = false;
        collect_event(
            root,
            Err(notify::Error::generic("lost events")),
            &mut changed,
            &mut overflow,
        );
        assert!(overflow);
        assert!(changed.is_empty());

        overflow = false;
        let mut event = notify::Event::new(notify::EventKind::Any);
        event.attrs.set_flag(notify::event::Flag::Rescan);
        collect_event(root, Ok(event), &mut changed, &mut overflow);
        assert!(overflow);

        overflow = false;
        let event =
            notify::Event::new(notify::EventKind::Any).add_path(root.join("notes/./../guide.md"));
        collect_event(root, Ok(event), &mut changed, &mut overflow);
        assert!(!overflow);
        assert!(changed.contains(Path::new("notes/guide.md")));
    }

    #[test]
    fn collect_event_ignores_reads_and_opens() {
        use notify::event::{AccessKind, AccessMode};

        let root = Path::new("/tmp/project");
        let mut changed = BTreeSet::new();
        let mut overflow = false;
        for kind in [
            AccessKind::Open(AccessMode::Any),
            AccessKind::Read,
            AccessKind::Close(AccessMode::Read),
        ] {
            let event =
                notify::Event::new(notify::EventKind::Access(kind)).add_path(root.join("note.md"));
            collect_event(root, Ok(event), &mut changed, &mut overflow);
        }
        assert!(!overflow);
        assert!(changed.is_empty());
    }

    #[test]
    fn normalize_relative_skips_dot_components() {
        assert_eq!(
            normalize_relative(Path::new("a/./b/../c.md")),
            Some(PathBuf::from("a/b/c.md"))
        );
    }
}
