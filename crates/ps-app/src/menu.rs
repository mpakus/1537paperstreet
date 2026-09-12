//! Native application menu and PLAN § 11.3 keyboard shortcuts.

use ps_core::config::Config;
use tauri::menu::{
    HELP_SUBMENU_ID, Menu, MenuItem, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID,
};
use tauri::{App, AppHandle, Emitter, Manager};

use crate::state::AppState;

/// Frontend event emitted when a custom menu command is chosen.
pub(crate) const MENU_ACTION_EVENT: &str = "menu://action";

/// One custom menu command with an optional accelerator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MenuCommand {
    /// Stable identifier sent to the UI.
    pub id: &'static str,
    /// macOS menu item title.
    pub title: &'static str,
    /// Accelerator parsed by `muda`, using `CmdOrCtrl` for ⌘.
    pub accelerator: Option<&'static str>,
}

const FILE_NEW: MenuCommand = MenuCommand {
    id: "file-new",
    title: "New File",
    accelerator: Some("CmdOrCtrl+N"),
};
const FILE_NEW_FOLDER: MenuCommand = MenuCommand {
    id: "file-new-folder",
    title: "New Folder",
    accelerator: Some("CmdOrCtrl+Shift+N"),
};
const FILE_OPEN_FILE: MenuCommand = MenuCommand {
    id: "file-open-file",
    title: "Open File…",
    accelerator: Some("CmdOrCtrl+O"),
};
const FILE_OPEN_FOLDER: MenuCommand = MenuCommand {
    id: "file-open-folder",
    title: "Open Folder…",
    accelerator: None,
};
const FILE_SAVE: MenuCommand = MenuCommand {
    id: "file-save",
    title: "Save",
    accelerator: Some("CmdOrCtrl+S"),
};
const FILE_EXPORT: MenuCommand = MenuCommand {
    id: "file-export",
    title: "Export PDF…",
    accelerator: Some("CmdOrCtrl+Alt+E"),
};
const FILE_SETTINGS: MenuCommand = MenuCommand {
    id: "file-settings",
    title: "Settings…",
    accelerator: None,
};
const FILE_TRASH: MenuCommand = MenuCommand {
    id: "file-trash",
    title: "Move to Trash",
    accelerator: Some("CmdOrCtrl+Backspace"),
};
const FILE_CLOSE_TAB: MenuCommand = MenuCommand {
    id: "file-close-tab",
    title: "Close Tab",
    accelerator: Some("CmdOrCtrl+W"),
};
const FILE_CLOSE_WINDOW: MenuCommand = MenuCommand {
    id: "file-close-window",
    title: "Close Window",
    accelerator: Some("CmdOrCtrl+Shift+W"),
};

const EDIT_BOLD: MenuCommand = MenuCommand {
    id: "edit-bold",
    title: "Bold",
    accelerator: Some("CmdOrCtrl+B"),
};
const EDIT_ITALIC: MenuCommand = MenuCommand {
    id: "edit-italic",
    title: "Italic",
    accelerator: Some("CmdOrCtrl+I"),
};
const EDIT_LINK: MenuCommand = MenuCommand {
    id: "edit-link",
    title: "Link",
    accelerator: Some("CmdOrCtrl+K"),
};
const EDIT_CODE: MenuCommand = MenuCommand {
    id: "edit-inline-code",
    title: "Inline Code",
    accelerator: Some("CmdOrCtrl+Shift+K"),
};
const EDIT_HEADING_1: MenuCommand = MenuCommand {
    id: "edit-heading-1",
    title: "Heading 1",
    accelerator: Some("CmdOrCtrl+Shift+1"),
};
const EDIT_HEADING_2: MenuCommand = MenuCommand {
    id: "edit-heading-2",
    title: "Heading 2",
    accelerator: Some("CmdOrCtrl+Shift+2"),
};
const EDIT_HEADING_3: MenuCommand = MenuCommand {
    id: "edit-heading-3",
    title: "Heading 3",
    accelerator: Some("CmdOrCtrl+Shift+3"),
};
const EDIT_HEADING_4: MenuCommand = MenuCommand {
    id: "edit-heading-4",
    title: "Heading 4",
    accelerator: Some("CmdOrCtrl+Shift+4"),
};
const EDIT_HEADING_5: MenuCommand = MenuCommand {
    id: "edit-heading-5",
    title: "Heading 5",
    accelerator: Some("CmdOrCtrl+Shift+5"),
};
const EDIT_HEADING_6: MenuCommand = MenuCommand {
    id: "edit-heading-6",
    title: "Heading 6",
    accelerator: Some("CmdOrCtrl+Shift+6"),
};
const EDIT_LIST: MenuCommand = MenuCommand {
    id: "edit-list",
    title: "List",
    accelerator: Some("CmdOrCtrl+Shift+L"),
};
const EDIT_QUOTE: MenuCommand = MenuCommand {
    id: "edit-quote",
    title: "Quote",
    accelerator: Some("CmdOrCtrl+Shift+."),
};
const EDIT_FIND: MenuCommand = MenuCommand {
    id: "edit-find",
    title: "Find",
    accelerator: Some("CmdOrCtrl+F"),
};
const EDIT_FIND_REPLACE: MenuCommand = MenuCommand {
    id: "edit-find-replace",
    title: "Find and Replace",
    accelerator: Some("CmdOrCtrl+Alt+F"),
};

const VIEW_TOGGLE_EDITOR: MenuCommand = MenuCommand {
    id: "view-toggle-editor",
    title: "Toggle Editor / Preview",
    accelerator: Some("CmdOrCtrl+E"),
};
const VIEW_TOGGLE_SPLIT: MenuCommand = MenuCommand {
    id: "view-toggle-split",
    title: "Toggle Split",
    accelerator: Some("CmdOrCtrl+Shift+E"),
};
const VIEW_FONT_LARGER: MenuCommand = MenuCommand {
    id: "view-font-larger",
    title: "Larger Font",
    accelerator: Some("CmdOrCtrl+="),
};
const VIEW_FONT_SMALLER: MenuCommand = MenuCommand {
    id: "view-font-smaller",
    title: "Smaller Font",
    accelerator: Some("CmdOrCtrl+-"),
};
const VIEW_FONT_RESET: MenuCommand = MenuCommand {
    id: "view-font-reset",
    title: "Reset Font Size",
    accelerator: Some("CmdOrCtrl+0"),
};
const VIEW_TOGGLE_PROJECTS: MenuCommand = MenuCommand {
    id: "view-toggle-projects",
    title: "Hide Projects Panel",
    accelerator: Some("CmdOrCtrl+1"),
};
const VIEW_TOGGLE_TREE: MenuCommand = MenuCommand {
    id: "view-toggle-tree",
    title: "Hide File Tree",
    accelerator: Some("CmdOrCtrl+2"),
};
const VIEW_TOGGLE_THEME: MenuCommand = MenuCommand {
    id: "view-toggle-theme",
    title: "Toggle Light / Dark Theme",
    accelerator: Some("CmdOrCtrl+Alt+T"),
};
const VIEW_ASSISTANT: MenuCommand = MenuCommand {
    id: "view-assistant",
    title: "Assistant",
    accelerator: Some("CmdOrCtrl+Alt+A"),
};
const VIEW_DASHBOARD: MenuCommand = MenuCommand {
    id: "view-dashboard",
    title: "Dashboard",
    accelerator: None,
};

const GO_SWITCH_PROJECT: MenuCommand = MenuCommand {
    id: "go-switch-project",
    title: "Switch Project",
    accelerator: Some("CmdOrCtrl+Shift+P"),
};
const GO_OPEN_FILE: MenuCommand = MenuCommand {
    id: "go-open-file",
    title: "Open File",
    accelerator: Some("CmdOrCtrl+P"),
};
const GO_REVEAL: MenuCommand = MenuCommand {
    id: "go-reveal",
    title: "Reveal in Finder",
    accelerator: Some("CmdOrCtrl+Shift+R"),
};
const GO_EXTERNAL_EDITOR: MenuCommand = MenuCommand {
    id: "go-external-editor",
    title: "Open in External Editor",
    accelerator: Some("CmdOrCtrl+Shift+O"),
};

const APP_SETTINGS: MenuCommand = MenuCommand {
    id: "app-settings",
    title: "Settings…",
    accelerator: Some("CmdOrCtrl+,"),
};
const APP_ABOUT: MenuCommand = MenuCommand {
    id: "app-about",
    title: "About 1537paperstreet",
    accelerator: None,
};
const APP_CHECK_UPDATES: MenuCommand = MenuCommand {
    id: "app-check-updates",
    title: "Check for Updates…",
    accelerator: None,
};
const FILE_CHECK_UPDATES: MenuCommand = MenuCommand {
    id: "file-check-updates",
    title: "Check for Updates…",
    accelerator: None,
};

#[cfg(test)]
const HEADINGS: [MenuCommand; 6] = [
    EDIT_HEADING_1,
    EDIT_HEADING_2,
    EDIT_HEADING_3,
    EDIT_HEADING_4,
    EDIT_HEADING_5,
    EDIT_HEADING_6,
];

/// Every custom command that carries a PLAN § 11.3 shortcut.
pub(crate) fn plan_commands() -> &'static [MenuCommand] {
    &[
        FILE_NEW,
        FILE_NEW_FOLDER,
        FILE_SAVE,
        FILE_EXPORT,
        FILE_TRASH,
        FILE_CLOSE_TAB,
        FILE_CLOSE_WINDOW,
        EDIT_BOLD,
        EDIT_ITALIC,
        EDIT_LINK,
        EDIT_CODE,
        EDIT_HEADING_1,
        EDIT_HEADING_2,
        EDIT_HEADING_3,
        EDIT_HEADING_4,
        EDIT_HEADING_5,
        EDIT_HEADING_6,
        EDIT_LIST,
        EDIT_QUOTE,
        EDIT_FIND,
        EDIT_FIND_REPLACE,
        VIEW_TOGGLE_EDITOR,
        VIEW_TOGGLE_SPLIT,
        VIEW_FONT_LARGER,
        VIEW_FONT_SMALLER,
        VIEW_FONT_RESET,
        VIEW_TOGGLE_PROJECTS,
        VIEW_TOGGLE_TREE,
        VIEW_TOGGLE_THEME,
        VIEW_ASSISTANT,
        VIEW_DASHBOARD,
        GO_SWITCH_PROJECT,
        GO_OPEN_FILE,
        GO_REVEAL,
        GO_EXTERNAL_EDITOR,
        APP_SETTINGS,
        FILE_SETTINGS,
    ]
}

/// Installs the native menu and routes custom commands to the UI.
pub(crate) fn install(app: &App) -> tauri::Result<()> {
    let handle = app.handle();
    handle.set_menu(build_menu(handle)?)?;
    handle.on_menu_event(|app, event| {
        on_menu_event(app, event.id().as_ref());
    });
    Ok(())
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let name = &app.package_info().name;
    let settings = item(app, APP_SETTINGS)?;
    let about = item(app, APP_ABOUT)?;
    let app_check_updates = item(app, APP_CHECK_UPDATES)?;
    let app_menu = Submenu::with_items(
        app,
        name,
        true,
        &[
            &about,
            &app_check_updates,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let new_file = item(app, FILE_NEW)?;
    let new_folder = item(app, FILE_NEW_FOLDER)?;
    let open_file = item(app, FILE_OPEN_FILE)?;
    let open_folder = item(app, FILE_OPEN_FOLDER)?;
    let save = item(app, FILE_SAVE)?;
    let export = item(app, FILE_EXPORT)?;
    let file_settings = item(app, FILE_SETTINGS)?;
    let trash = item(app, FILE_TRASH)?;
    let close_tab = item(app, FILE_CLOSE_TAB)?;
    let close_window = item(app, FILE_CLOSE_WINDOW)?;
    let file_about = MenuItem::with_id(
        app,
        "file-about",
        APP_ABOUT.title,
        true,
        APP_ABOUT.accelerator,
    )?;
    let file_check_updates = item(app, FILE_CHECK_UPDATES)?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &open_file,
            &open_folder,
            &PredefinedMenuItem::separator(app)?,
            &new_file,
            &new_folder,
            &PredefinedMenuItem::separator(app)?,
            &save,
            &export,
            &PredefinedMenuItem::separator(app)?,
            &file_settings,
            &PredefinedMenuItem::separator(app)?,
            &trash,
            &PredefinedMenuItem::separator(app)?,
            &close_tab,
            &close_window,
            &PredefinedMenuItem::separator(app)?,
            &file_check_updates,
            &file_about,
        ],
    )?;

    let bold = item(app, EDIT_BOLD)?;
    let italic = item(app, EDIT_ITALIC)?;
    let link = item(app, EDIT_LINK)?;
    let code = item(app, EDIT_CODE)?;
    let heading_1 = item(app, EDIT_HEADING_1)?;
    let heading_2 = item(app, EDIT_HEADING_2)?;
    let heading_3 = item(app, EDIT_HEADING_3)?;
    let heading_4 = item(app, EDIT_HEADING_4)?;
    let heading_5 = item(app, EDIT_HEADING_5)?;
    let heading_6 = item(app, EDIT_HEADING_6)?;
    let headings = Submenu::with_items(
        app,
        "Heading",
        true,
        &[
            &heading_1, &heading_2, &heading_3, &heading_4, &heading_5, &heading_6,
        ],
    )?;
    let list = item(app, EDIT_LIST)?;
    let quote = item(app, EDIT_QUOTE)?;
    let find = item(app, EDIT_FIND)?;
    let find_replace = item(app, EDIT_FIND_REPLACE)?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &bold,
            &italic,
            &link,
            &code,
            &PredefinedMenuItem::separator(app)?,
            &headings,
            &list,
            &quote,
            &PredefinedMenuItem::separator(app)?,
            &find,
            &find_replace,
        ],
    )?;

    let toggle_editor = item(app, VIEW_TOGGLE_EDITOR)?;
    let toggle_split = item(app, VIEW_TOGGLE_SPLIT)?;
    let font_larger = item(app, VIEW_FONT_LARGER)?;
    let font_smaller = item(app, VIEW_FONT_SMALLER)?;
    let font_reset = item(app, VIEW_FONT_RESET)?;
    let toggle_projects = item(app, VIEW_TOGGLE_PROJECTS)?;
    let toggle_tree = item(app, VIEW_TOGGLE_TREE)?;
    let toggle_theme = item(app, VIEW_TOGGLE_THEME)?;
    let assistant = item(app, VIEW_ASSISTANT)?;
    let dashboard = item(app, VIEW_DASHBOARD)?;
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &toggle_editor,
            &toggle_split,
            &PredefinedMenuItem::separator(app)?,
            &font_larger,
            &font_smaller,
            &font_reset,
            &PredefinedMenuItem::separator(app)?,
            &toggle_projects,
            &toggle_tree,
            &PredefinedMenuItem::separator(app)?,
            &toggle_theme,
            &assistant,
            &dashboard,
        ],
    )?;

    let switch_project = item(app, GO_SWITCH_PROJECT)?;
    let open_file = item(app, GO_OPEN_FILE)?;
    let reveal = item(app, GO_REVEAL)?;
    let external = item(app, GO_EXTERNAL_EDITOR)?;
    let go_menu = Submenu::with_items(
        app,
        "Go",
        true,
        &[
            &switch_project,
            &open_file,
            &PredefinedMenuItem::separator(app)?,
            &reveal,
            &external,
        ],
    )?;

    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Help", true, &[])?;

    Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &go_menu,
            &window_menu,
            &help_menu,
        ],
    )
}

fn item(app: &AppHandle, command: MenuCommand) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, command.id, command.title, true, command.accelerator)
}

fn on_menu_event(app: &AppHandle, id: &str) {
    if id == FILE_CLOSE_WINDOW.id {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.close();
        }
        return;
    }
    if let Some(size) = next_font_size(
        app.state::<AppState>().config_get().typography.font_size,
        id,
    ) {
        let state = app.state::<AppState>().inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut config = state.config_get();
            config.typography.font_size = size;
            let _ = state.config_set(config);
        });
    }

    if emits_menu_action(id) {
        let _ = app.emit(MENU_ACTION_EVENT, id);
    }
}

fn emits_menu_action(id: &str) -> bool {
    matches!(
        id,
        "file-open-file"
            | "file-open-folder"
            | "app-about"
            | "file-about"
            | "file-settings"
            | "app-check-updates"
            | "file-check-updates"
    ) || plan_commands().iter().any(|command| command.id == id)
}

fn next_font_size(current: u16, id: &str) -> Option<u16> {
    match id {
        "view-font-larger" => Some(current.saturating_add(1).min(32)),
        "view-font-smaller" => Some(current.saturating_sub(1).max(10)),
        "view-font-reset" => Some(Config::default().typography.font_size),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        APP_SETTINGS, EDIT_BOLD, EDIT_CODE, EDIT_FIND, EDIT_FIND_REPLACE, EDIT_ITALIC, EDIT_LINK,
        EDIT_LIST, EDIT_QUOTE, FILE_CLOSE_TAB, FILE_CLOSE_WINDOW, FILE_EXPORT, FILE_NEW,
        FILE_NEW_FOLDER, FILE_SAVE, FILE_TRASH, GO_EXTERNAL_EDITOR, GO_OPEN_FILE, GO_REVEAL,
        GO_SWITCH_PROJECT, HEADINGS, VIEW_ASSISTANT, VIEW_FONT_LARGER, VIEW_FONT_RESET,
        VIEW_FONT_SMALLER, VIEW_TOGGLE_EDITOR, VIEW_TOGGLE_PROJECTS, VIEW_TOGGLE_SPLIT,
        VIEW_TOGGLE_THEME, VIEW_TOGGLE_TREE, emits_menu_action, next_font_size, plan_commands,
    };
    use ps_core::config::Config;

    #[test]
    fn registers_every_plan_section_11_3_shortcut() {
        let accelerators = plan_commands()
            .iter()
            .filter_map(|command| command.accelerator)
            .collect::<HashSet<_>>();

        for expected in [
            FILE_NEW.accelerator,
            FILE_NEW_FOLDER.accelerator,
            FILE_SAVE.accelerator,
            FILE_EXPORT.accelerator,
            FILE_TRASH.accelerator,
            FILE_CLOSE_TAB.accelerator,
            FILE_CLOSE_WINDOW.accelerator,
            EDIT_BOLD.accelerator,
            EDIT_ITALIC.accelerator,
            EDIT_LINK.accelerator,
            EDIT_CODE.accelerator,
            EDIT_LIST.accelerator,
            EDIT_QUOTE.accelerator,
            EDIT_FIND.accelerator,
            EDIT_FIND_REPLACE.accelerator,
            VIEW_TOGGLE_EDITOR.accelerator,
            VIEW_TOGGLE_SPLIT.accelerator,
            VIEW_FONT_LARGER.accelerator,
            VIEW_FONT_SMALLER.accelerator,
            VIEW_FONT_RESET.accelerator,
            VIEW_TOGGLE_PROJECTS.accelerator,
            VIEW_TOGGLE_TREE.accelerator,
            VIEW_TOGGLE_THEME.accelerator,
            VIEW_ASSISTANT.accelerator,
            GO_SWITCH_PROJECT.accelerator,
            GO_OPEN_FILE.accelerator,
            GO_REVEAL.accelerator,
            GO_EXTERNAL_EDITOR.accelerator,
            APP_SETTINGS.accelerator,
        ]
        .into_iter()
        .flatten()
        .chain(HEADINGS.iter().filter_map(|command| command.accelerator))
        {
            assert!(
                accelerators.contains(expected),
                "PLAN § 11.3 shortcut {expected} is missing from the native menu"
            );
        }

        assert_eq!(accelerators.len(), 35);
    }

    #[test]
    fn menu_command_ids_and_shortcuts_are_unique() {
        let mut ids = HashSet::new();
        let mut accelerators = HashSet::new();
        for command in plan_commands() {
            assert!(ids.insert(command.id), "duplicate menu id {}", command.id);
            if let Some(accelerator) = command.accelerator {
                assert!(
                    accelerators.insert(accelerator),
                    "duplicate accelerator {accelerator}"
                );
                assert_supported_accelerator(accelerator);
            }
        }
    }

    #[test]
    fn font_size_commands_stay_within_config_limits() {
        let default = Config::default().typography.font_size;
        assert_eq!(next_font_size(16, "view-font-larger"), Some(17));
        assert_eq!(next_font_size(16, "view-font-smaller"), Some(15));
        assert_eq!(next_font_size(11, "view-font-reset"), Some(default));
        assert_eq!(next_font_size(32, "view-font-larger"), Some(32));
        assert_eq!(next_font_size(10, "view-font-smaller"), Some(10));
        assert_eq!(next_font_size(16, "file-save"), None);
    }

    #[test]
    fn close_tab_uses_the_standard_mac_shortcut() {
        assert_eq!(FILE_CLOSE_TAB.id, "file-close-tab");
        assert_eq!(FILE_CLOSE_TAB.title, "Close Tab");
        assert_eq!(FILE_CLOSE_TAB.accelerator, Some("CmdOrCtrl+W"));
        assert_eq!(FILE_CLOSE_WINDOW.accelerator, Some("CmdOrCtrl+Shift+W"));
        assert!(emits_menu_action("file-close-tab"));
    }

    #[test]
    fn about_is_a_custom_menu_action() {
        assert!(emits_menu_action("app-about"));
        assert!(emits_menu_action("file-about"));
        assert!(emits_menu_action("app-check-updates"));
        assert!(emits_menu_action("file-check-updates"));
        assert_eq!(super::APP_ABOUT.id, "app-about");
        assert_eq!(super::APP_ABOUT.title, "About 1537paperstreet");
        assert_eq!(super::APP_CHECK_UPDATES.title, "Check for Updates…");
        assert_eq!(super::FILE_CHECK_UPDATES.title, "Check for Updates…");
    }

    #[test]
    fn file_menu_exports_pdf_and_opens_settings() {
        assert_eq!(FILE_EXPORT.title, "Export PDF…");
        assert!(emits_menu_action("file-settings"));
        assert_eq!(super::FILE_SETTINGS.id, "file-settings");
        assert_eq!(super::FILE_SETTINGS.title, "Settings…");
        assert_eq!(APP_SETTINGS.title, "Settings…");
    }

    fn assert_supported_accelerator(spec: &str) {
        let mut parts = spec.split('+').collect::<Vec<_>>();
        let key = parts.pop().expect("accelerator key");
        for modifier in parts {
            assert!(
                matches!(modifier, "CmdOrCtrl" | "Shift" | "Alt"),
                "unsupported modifier {modifier} in {spec}"
            );
        }
        let supported = key == "Backspace"
            || (key.len() == 1
                && (key.as_bytes()[0].is_ascii_alphanumeric()
                    || matches!(key, "," | "." | "-" | "=")));
        assert!(supported, "unsupported key {key} in {spec}");
    }
}
