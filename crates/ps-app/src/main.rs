//! Desktop application entry point for 1537paperstreet.

#![warn(missing_docs)]

use std::error::Error;

use ps_core::paths::AppPaths;
use tauri::Manager;

mod agent;
mod commands;
mod fs_watch;
mod menu;
mod pdf;
mod protocol;
mod state;
mod tray;
mod updates;
mod window_chrome;

use commands::{
    agent_cancel, agent_make_server, agent_permission_reply, agent_presets, agent_prompt,
    agent_prompt_history, agent_set_model, agent_start, agent_stop, config_get, config_set,
    copy_conflicts, dashboard_get, doc_open, doc_save, doc_source, doc_stat, export_pdf,
    files_search, fs_copy, fs_create_file, fs_create_untitled, fs_import, fs_mkdir, fs_move,
    fs_rename, fs_transfer, fs_trash, mermaid_cache_get, mermaid_cache_put, open_dropped_paths,
    open_external, open_url, projects_add, projects_list, projects_relocate, projects_remove,
    projects_rename, reveal_in_finder, save_user_file, themes_css, themes_list, tree_expanded_get,
    tree_expanded_set, tree_read_dir, updates_check, watch_set_expanded, watch_start, watch_stop,
};
use fs_watch::WatchHub;
use state::AppState;

fn main() -> Result<(), Box<dyn Error>> {
    let state = AppState::open(AppPaths::discover()?)?;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            crate::tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .register_asynchronous_uri_scheme_protocol("asset", |ctx, request, responder| {
            protocol::respond_asset(ctx, request, responder);
        })
        .setup(|app| {
            crate::menu::install(app)?;
            crate::tray::install(app)?;
            app.manage(WatchHub::spawn(app.handle().clone()));
            app.manage(crate::agent::AgentHub::new());
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = crate::window_chrome::apply_sidebar_vibrancy(&window) {
                    app.state::<AppState>()
                        .log_warn(&format!("sidebar vibrancy could not be applied: {error}"));
                }
                let _ = window.show();
                let _ = window.set_focus();
                crate::tray::apply_dock_visibility(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            crate::tray::on_window_event(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            config_get,
            config_set,
            projects_list,
            projects_add,
            projects_rename,
            projects_remove,
            projects_relocate,
            themes_list,
            themes_css,
            mermaid_cache_get,
            mermaid_cache_put,
            save_user_file,
            open_dropped_paths,
            tree_read_dir,
            tree_expanded_get,
            tree_expanded_set,
            fs_mkdir,
            fs_create_file,
            fs_create_untitled,
            fs_rename,
            fs_copy,
            fs_import,
            fs_move,
            fs_transfer,
            fs_trash,
            reveal_in_finder,
            open_external,
            copy_conflicts,
            files_search,
            open_url,
            updates_check,
            watch_start,
            watch_set_expanded,
            watch_stop,
            doc_open,
            doc_save,
            doc_source,
            doc_stat,
            export_pdf,
            agent_presets,
            agent_make_server,
            agent_prompt_history,
            agent_start,
            agent_stop,
            agent_prompt,
            agent_cancel,
            agent_set_model,
            agent_permission_reply,
            dashboard_get,
        ])
        .build(tauri::generate_context!())?
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                crate::tray::show_main_window(app);
            }
        });
    Ok(())
}
