//! Overlay titlebar, traffic-light inset, and Sidebar vibrancy.

use tauri::WebviewWindow;

/// Applies the macOS `Sidebar` vibrancy material behind the webview.
pub(crate) fn apply_sidebar_vibrancy(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    window_vibrancy::apply_vibrancy(
        window,
        window_vibrancy::NSVisualEffectMaterial::Sidebar,
        None,
        None,
    )
    .map_err(|error| error.to_string())?;

    #[cfg(not(target_os = "macos"))]
    let _ = window;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn overlay_window_enables_macos_private_api() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let window = &config["app"]["windows"][0];

        assert_eq!(config["app"]["macOSPrivateApi"].as_bool(), Some(true));
        assert_eq!(window["transparent"].as_bool(), Some(true));
        assert_eq!(window["titleBarStyle"].as_str(), Some("Overlay"));
        assert_eq!(window["hiddenTitle"].as_bool(), Some(true));
        assert_eq!(window["dragDropEnabled"].as_bool(), Some(true));
        assert_eq!(window["trafficLightPosition"]["x"].as_f64(), Some(16.0));
        assert_eq!(window["trafficLightPosition"]["y"].as_f64(), Some(18.0));
    }

    #[test]
    fn tauri_crate_enables_macos_private_api_feature() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            manifest.contains("macos-private-api"),
            "transparent windows require the macos-private-api Cargo feature"
        );
    }

    #[test]
    fn overlay_titlebar_may_start_a_window_drag() {
        let capabilities: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("capabilities");
        let permissions = capabilities["permissions"].as_array().expect("permissions");
        assert!(
            permissions.iter().any(|permission| {
                permission.as_str() == Some("core:window:allow-start-dragging")
            })
        );
    }

    #[test]
    fn bundle_uses_generated_icons_from_icon_png() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let icons = config["bundle"]["icon"].as_array().expect("bundle.icon");
        for name in ["icons/icon.icns", "icons/icon.png"] {
            assert!(
                icons.iter().any(|icon| icon.as_str() == Some(name)),
                "bundle.icon must include {name}"
            );
        }

        let png = include_bytes!("../icons/icon.png");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(
            png.len() > 20_000,
            "icons/icon.png must be generated from the repository icon.png, not the scaffold P"
        );
    }

    #[test]
    fn tauri_crate_enables_tray_icon() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            manifest.contains("tray-icon"),
            "the menu-bar status item requires the tray-icon Cargo feature"
        );
        assert!(manifest.contains("image-png"));
    }

    #[test]
    fn bundle_registers_markdown_and_pdf_without_taking_ownership() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let associations = config["bundle"]["fileAssociations"]
            .as_array()
            .expect("fileAssociations");
        let markdown = associations
            .iter()
            .find(|item| item["name"].as_str() == Some("Markdown"))
            .expect("markdown association");
        let extensions: Vec<&str> = markdown["ext"]
            .as_array()
            .expect("ext")
            .iter()
            .filter_map(|value| value.as_str())
            .collect();
        assert_eq!(extensions, ["md", "markdown", "mdown", "mdwn"]);
        assert_eq!(markdown["role"].as_str(), Some("Editor"));
        assert_eq!(markdown["rank"].as_str(), Some("Alternate"));
        let pdf = associations
            .iter()
            .find(|item| item["name"].as_str() == Some("PDF"))
            .expect("pdf association");
        assert_eq!(pdf["ext"][0].as_str(), Some("pdf"));
        assert_eq!(pdf["role"].as_str(), Some("Viewer"));
        assert_eq!(pdf["rank"].as_str(), Some("Alternate"));
        let csp = config["app"]["security"]["csp"].as_str().expect("csp");
        assert!(csp.contains("object-src 'self' asset: blob:"));
        assert!(csp.contains("frame-src 'self' asset: blob:"));
    }

    #[test]
    fn linux_and_windows_use_a_decorated_window() {
        let linux: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.linux.conf.json")).expect("linux config");
        let windows: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.windows.conf.json"))
                .expect("windows config");
        assert_eq!(linux["bundle"]["targets"][0].as_str(), Some("deb"));
        assert_eq!(windows["bundle"]["targets"][0].as_str(), Some("nsis"));
        for config in [linux, windows] {
            let window = &config["app"]["windows"][0];
            assert_eq!(window["label"].as_str(), Some("main"));
            assert_eq!(window["transparent"].as_bool(), Some(false));
            assert_eq!(window["decorations"].as_bool(), Some(true));
            assert_eq!(window["dragDropEnabled"].as_bool(), Some(true));
        }
    }
}
