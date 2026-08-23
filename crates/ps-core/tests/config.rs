use ps_core::Error;
use ps_core::config::{Config, ViewMode, Window};
use ps_core::store::VersionedDocument;

#[test]
fn defaults_match_the_product_plan() {
    let config = Config::default();

    assert_eq!(config.schema_version, 1);
    assert_eq!(config.appearance.theme, "paper-light");
    assert_eq!(config.appearance.theme_dark, "paper-dark");
    assert!(config.appearance.follow_system);
    assert_eq!(config.typography.font_size, 16);
    assert_eq!(config.typography.measure_ch, 72);
    assert_eq!(config.viewer.default_mode, ViewMode::Preview);
    assert_eq!(config.editor.autosave_ms, 800);
    assert_eq!(config.editor.assets_dir, "assets");
    assert_eq!(config.history.interval_min, 5);
    assert_eq!(config.history.global_cap_mb, 2048);
    assert_eq!(config.files.export_ignore.len(), 4);
    assert_eq!(config.window.width, 1180);
    assert_eq!(config.window.toc_w, 224);
    assert!(config.window.show_in_dock);
    assert!(config.viewer.preview_font.is_empty());
    assert_eq!(config.viewer.preview_font_size, 0);
    assert!(config.viewer.preview_bg.is_empty());
    assert!(config.viewer.preview_fg.is_empty());
    assert!(config.updates.check_on_launch);
    assert!(config.agents.servers.is_empty());
    assert_eq!(
        config.agents.permission,
        ps_core::agents::AgentPermission::Allowance
    );
    config.validate().expect("valid defaults");
}

#[test]
fn missing_toc_width_defaults_without_a_schema_bump() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260
    }))
    .expect("window");
    assert_eq!(window.toc_w, 224);
    assert_eq!(window.editor_w, 480);
    assert!(window.show_in_dock);
}

#[test]
fn missing_editor_width_defaults_without_a_schema_bump() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "toc_w": 224
    }))
    .expect("window");
    assert_eq!(window.editor_w, 480);
}

#[test]
fn fractional_panel_widths_round_to_u32() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "toc_w": 224,
        "editor_w": 128.1653125
    }))
    .expect("window");
    assert_eq!(window.editor_w, 128);
}

#[test]
fn missing_preview_chrome_defaults_without_a_schema_bump() {
    let viewer: ps_core::config::Viewer = serde_json::from_value(serde_json::json!({
        "default_mode": "preview",
        "show_toc": true,
        "allow_raw_html": false,
        "mermaid_enabled": true,
        "math_enabled": true
    }))
    .expect("viewer");
    assert!(viewer.preview_font.is_empty());
    assert_eq!(viewer.preview_font_size, 0);
    assert!(viewer.preview_bg.is_empty());
    assert!(viewer.preview_fg.is_empty());
}

#[test]
fn migrate_has_no_path_from_older_schemas() {
    assert!(Config::migrate(serde_json::json!({}), 0).is_err());
}

#[test]
fn rejects_typography_values_outside_the_supported_ranges() {
    let mut config = Config::default();
    config.typography.font_size = 9;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: "font_size",
            ..
        })
    ));

    config.typography.font_size = 16;
    config.typography.measure_ch = 121;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: "measure_ch",
            ..
        })
    ));
}

#[test]
fn preview_overrides_validate_size_and_hex_colors() {
    let mut config = Config::default();
    config.viewer.preview_font_size = 9;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfig {
            field: "preview_font_size",
            ..
        })
    ));

    config.viewer.preview_font_size = 18;
    config.viewer.preview_bg = "#112233".into();
    config.viewer.preview_fg = "#abcdef".into();
    config.validate().expect("custom preview chrome");

    config.viewer.preview_bg = "red".into();
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfigFormat {
            field: "preview_bg",
            ..
        })
    ));
}
