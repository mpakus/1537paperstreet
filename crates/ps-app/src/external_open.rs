//! Files the operating system asked the app to open.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

/// Frontend wake-up. The paths themselves stay in [`AppState`] until taken.
pub(crate) const EXTERNAL_OPEN_EVENT: &str = "open://external";

/// Accepts process arguments from a cold start or a second instance.
pub(crate) fn accept_args(app: &AppHandle, args: &[String]) {
    accept_paths(app, ps_core::projects::paths_from_launch_args(args));
}

/// Shows the window and queues folders, Markdown files, and PDFs.
pub(crate) fn accept_paths(app: &AppHandle, paths: Vec<PathBuf>) {
    crate::tray::show_main_window(app);
    if app.state::<AppState>().enqueue_external_paths(paths) {
        let _ = app.emit(EXTERNAL_OPEN_EVENT, ());
    }
}
