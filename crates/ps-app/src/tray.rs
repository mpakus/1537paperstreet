//! Menu-bar status item that keeps the app alive after the window is closed.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, WindowEvent};

use crate::menu::MENU_ACTION_EVENT;
use crate::state::AppState;

const TRAY_ICON_PNG: &[u8] = include_bytes!("../../../icon-system.png");

/// Installs the macOS menu-bar icon and its Show / About / Quit menu.
pub(crate) fn install(app: &App) -> tauri::Result<()> {
    let handle = app.handle();
    let show = MenuItem::with_id(handle, "tray-show", "Show", true, None::<&str>)?;
    let about = MenuItem::with_id(
        handle,
        "tray-about",
        "About 1537paperstreet",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(handle, "tray-quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        handle,
        &[
            &show,
            &about,
            &PredefinedMenuItem::separator(handle)?,
            &quit,
        ],
    )?;
    let icon = Image::from_bytes(TRAY_ICON_PNG)?;

    let mut builder = TrayIconBuilder::with_id("status")
        .icon(icon)
        .tooltip("1537paperstreet")
        .menu(&menu)
        .show_menu_on_left_click(false);
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-show" => show_main_window(app),
            "tray-about" => {
                show_main_window(app);
                let _ = app.emit(MENU_ACTION_EVENT, "app-about");
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Brings the main window back when the dock, tray, or a second instance asks.
pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    apply_dock_visibility(app);
}

/// Hides or shows the Dock icon to match the window and `window.show_in_dock`.
pub(crate) fn apply_dock_visibility(app: &AppHandle) {
    let show_in_dock = app.state::<AppState>().config_get().window.show_in_dock;
    let window_visible = app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(true);
    let visible = dock_icon_visible(window_visible, show_in_dock);
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(visible);
    #[cfg(not(target_os = "macos"))]
    let _ = (app, visible);
}

/// Dock stays when the window is on screen, or when the user asked to keep it.
pub(crate) fn dock_icon_visible(window_visible: bool, show_in_dock: bool) -> bool {
    window_visible || show_in_dock
}

/// Hides the reader instead of quitting when the red traffic light is clicked.
pub(crate) fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
        apply_dock_visibility(window.app_handle());
    }
}

#[cfg(test)]
mod tests {
    use super::{TRAY_ICON_PNG, dock_icon_visible};

    #[test]
    fn menu_bar_icon_is_png() {
        assert!(TRAY_ICON_PNG.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(TRAY_ICON_PNG.len() > 1_000);
    }

    #[test]
    fn dock_icon_follows_window_unless_the_user_keeps_it() {
        assert!(dock_icon_visible(true, true));
        assert!(dock_icon_visible(true, false));
        assert!(dock_icon_visible(false, true));
        assert!(!dock_icon_visible(false, false));
    }

    #[test]
    fn branding_pngs_live_at_the_repo_root() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for name in ["icon.png", "icon-system.png", "logo.png"] {
            assert!(
                root.join(name).is_file(),
                "{name} must exist at the repository root"
            );
        }
    }
}
