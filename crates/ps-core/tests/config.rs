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
    assert!(config.files.show_hidden);
    assert_eq!(config.window.width, 1180);
    assert_eq!(config.window.toc_w, 224);
    assert_eq!(config.window.diagram_w, 896);
    assert_eq!(config.window.diagram_h, 576);
    assert_eq!(config.window.diagram_zoom, 1.0);
    assert_eq!(config.window.diagram_x, None);
    assert_eq!(config.window.diagram_y, None);
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
fn missing_show_hidden_defaults_on_without_a_schema_bump() {
    let files: ps_core::config::Files = serde_json::from_value(serde_json::json!({
        "export_ignore": [".git"],
        "confirm_delete": true
    }))
    .expect("files");
    assert!(files.show_hidden);
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
fn missing_diagram_chrome_defaults_without_a_schema_bump() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "toc_w": 224,
        "editor_w": 480,
        "show_in_dock": true
    }))
    .expect("window");
    assert_eq!(window.diagram_w, 896);
    assert_eq!(window.diagram_h, 576);
    assert_eq!(window.diagram_zoom, 1.0);
    assert_eq!(window.diagram_x, None);
    assert_eq!(window.diagram_y, None);
}

#[test]
fn diagram_position_round_trips_without_a_schema_bump() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "diagram_x": 120.4,
        "diagram_y": -24
    }))
    .expect("window");
    assert_eq!(window.diagram_x, Some(120));
    assert_eq!(window.diagram_y, Some(-24));
}

#[test]
fn diagram_zoom_clamps_and_accepts_integers() {
    let high: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "diagram_zoom": 100
    }))
    .expect("window");
    assert_eq!(high.diagram_zoom, 32.0);

    let low: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "diagram_zoom": 0.1
    }))
    .expect("window");
    assert_eq!(low.diagram_zoom, 0.25);

    let stepped: Window = serde_json::from_value(serde_json::json!({
        "width": 1180,
        "height": 780,
        "sidebar_w": 220,
        "tree_w": 260,
        "diagram_w": 640.4,
        "diagram_h": 400.6,
        "diagram_zoom": 2
    }))
    .expect("window");
    assert_eq!(stepped.diagram_w, 640);
    assert_eq!(stepped.diagram_h, 401);
    assert_eq!(stepped.diagram_zoom, 2.0);
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
