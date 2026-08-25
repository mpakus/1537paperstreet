//! Application configuration and validation.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::agents::Agents;
use crate::store::VersionedDocument;
use crate::{Error, Result};

/// Complete application configuration stored in `config.json`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Config {
    /// On-disk schema version.
    pub schema_version: u32,
    /// Theme selection.
    pub appearance: Appearance,
    /// Document typography.
    pub typography: Typography,
    /// Preview behavior.
    pub viewer: Viewer,
    /// Editor behavior.
    pub editor: Editor,
    /// Snapshot-history policy.
    pub history: History,
    /// File-tree and export behavior.
    pub files: Files,
    /// Last saved window geometry.
    pub window: Window,
    /// Update-check behavior.
    pub updates: Updates,
    /// External ACP agents. Missing in older files; default is empty.
    #[serde(default)]
    pub agents: Agents,
}

impl Config {
    /// Validates configuration ranges that affect rendering and layout.
    pub fn validate(&self) -> Result<()> {
        validate_range("font_size", self.typography.font_size, 10, 32)?;
        validate_range("measure_ch", self.typography.measure_ch, 40, 120)?;
        if self.viewer.preview_font_size != 0 {
            validate_range("preview_font_size", self.viewer.preview_font_size, 10, 32)?;
        }
        validate_hex_color("preview_bg", &self.viewer.preview_bg)?;
        validate_hex_color("preview_fg", &self.viewer.preview_fg)?;
        self.agents.validate()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            appearance: Appearance::default(),
            typography: Typography::default(),
            viewer: Viewer::default(),
            editor: Editor::default(),
            history: History::default(),
            files: Files::default(),
            window: Window::default(),
            updates: Updates::default(),
            agents: Agents::default(),
        }
    }
}

impl VersionedDocument for Config {
    const SCHEMA_VERSION: u32 = 1;

    fn validate(&self) -> Result<()> {
        Config::validate(self)
    }
}

/// Theme configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Appearance {
    /// Light theme identifier.
    pub theme: String,
    /// Dark theme identifier.
    pub theme_dark: String,
    /// Whether appearance follows macOS.
    pub follow_system: bool,
    /// User-selected accent color.
    pub accent: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "paper-light".into(),
            theme_dark: "paper-dark".into(),
            follow_system: true,
            accent: "#C1452F".into(),
        }
    }
}

/// Document typography configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Typography {
    /// Body font family.
    pub body_font: String,
    /// Monospace font family.
    pub mono_font: String,
    /// Base font size in CSS pixels.
    pub font_size: u16,
    /// Unitless line height.
    pub line_height: f32,
    /// Reading measure in `ch` units.
    pub measure_ch: u16,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            body_font: "New York".into(),
            mono_font: "JetBrains Mono".into(),
            font_size: 16,
            line_height: 1.65,
            measure_ch: 72,
        }
    }
}

/// Initial document-view mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Rendered Markdown only.
    #[default]
    Preview,
    /// Markdown source only.
    Editor,
    /// Source and rendered Markdown side by side.
    Split,
}

/// Preview behavior.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Viewer {
    /// Mode used when opening a document.
    pub default_mode: ViewMode,
    /// Whether the table of contents is visible.
    pub show_toc: bool,
    /// Whether raw HTML bypasses sanitization.
    pub allow_raw_html: bool,
    /// Whether Mermaid diagrams are enabled.
    pub mermaid_enabled: bool,
    /// Whether mathematical notation is enabled.
    pub math_enabled: bool,
    /// Preview/Split body font; empty uses `typography.body_font`.
    pub preview_font: String,
    /// Preview/Split font size in CSS pixels; `0` uses `typography.font_size`.
    pub preview_font_size: u16,
    /// Preview/Split background; empty uses the theme `--bg` token.
    pub preview_bg: String,
    /// Preview/Split text color; empty uses the theme `--fg` token.
    pub preview_fg: String,
}

impl Default for Viewer {
    fn default() -> Self {
        Self {
            default_mode: ViewMode::Preview,
            show_toc: true,
            allow_raw_html: false,
            mermaid_enabled: true,
            math_enabled: true,
            preview_font: String::new(),
            preview_font_size: 0,
            preview_bg: String::new(),
            preview_fg: String::new(),
        }
    }
}

/// Editor behavior.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Editor {
    /// Idle time before autosave, in milliseconds.
    pub autosave_ms: u64,
    /// Whether line numbers are shown.
    pub line_numbers: bool,
    /// Whether long lines wrap.
    pub soft_wrap: bool,
    /// Whether brackets and quotes close automatically.
    pub auto_pairs: bool,
    /// Whether lists continue on Enter.
    pub continue_lists: bool,
    /// Whether system spellchecking is enabled.
    pub spellcheck: bool,
    /// Whether editor and preview scroll together.
    pub sync_scroll: bool,
    /// Project-relative directory for imported assets.
    pub assets_dir: String,
    /// Whether the first H1 may rename a document.
    pub rename_file_from_h1: bool,
    /// Spaces added for one indentation level.
    pub indent_unit: u8,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            autosave_ms: 800,
            line_numbers: false,
            soft_wrap: true,
            auto_pairs: true,
            continue_lists: true,
            spellcheck: true,
            sync_scroll: true,
            assets_dir: "assets".into(),
            rename_file_from_h1: false,
            indent_unit: 2,
        }
    }
}

/// Snapshot-history policy.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct History {
    /// Whether new snapshots are recorded.
    pub enabled: bool,
    /// Minutes between interval snapshots.
    pub interval_min: u32,
    /// Per-project storage cap in megabytes.
    pub project_cap_mb: u32,
    /// Global storage cap in megabytes.
    pub global_cap_mb: u32,
    /// Hours for which every snapshot is kept.
    pub keep_all_hours: u32,
    /// Days for which hourly snapshots are kept.
    pub keep_hourly_days: u32,
    /// Days for which daily snapshots are kept.
    pub keep_daily_days: u32,
}

impl Default for History {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_min: 5,
            project_cap_mb: 200,
            global_cap_mb: 2048,
            keep_all_hours: 24,
            keep_hourly_days: 7,
            keep_daily_days: 90,
        }
    }
}

/// File-tree and export behavior.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Files {
    /// Whether hidden files are visible.
    pub show_hidden: bool,
    /// Ignore patterns applied during export.
    pub export_ignore: Vec<String>,
    /// Whether moving an item to Trash asks for confirmation.
    pub confirm_delete: bool,
}

impl Default for Files {
    fn default() -> Self {
        Self {
            show_hidden: false,
            export_ignore: vec![".git", "node_modules", ".DS_Store", "*.zip"]
                .into_iter()
                .map(String::from)
                .collect(),
            confirm_delete: true,
        }
    }
}

/// Saved window geometry.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Window {
    /// Window width in logical pixels.
    #[serde(deserialize_with = "deserialize_px")]
    pub width: u32,
    /// Window height in logical pixels.
    #[serde(deserialize_with = "deserialize_px")]
    pub height: u32,
    /// Project-sidebar width in logical pixels.
    #[serde(deserialize_with = "deserialize_px")]
    pub sidebar_w: u32,
    /// File-tree width in logical pixels.
    #[serde(deserialize_with = "deserialize_px")]
    pub tree_w: u32,
    /// Table-of-contents width in logical pixels.
    #[serde(default = "default_toc_w", deserialize_with = "deserialize_px")]
    pub toc_w: u32,
    /// Editor column width in Split, in logical pixels.
    #[serde(default = "default_editor_w", deserialize_with = "deserialize_px")]
    pub editor_w: u32,
    /// Whether the Dock icon stays visible after the window is hidden.
    #[serde(default = "default_true")]
    pub show_in_dock: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 1180,
            height: 780,
            sidebar_w: 220,
            tree_w: 260,
            toc_w: 224,
            editor_w: 480,
            show_in_dock: true,
        }
    }
}

/// Update-check behavior.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Updates {
    /// Whether the optional update check runs at launch.
    pub check_on_launch: bool,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            check_on_launch: true,
        }
    }
}

fn default_toc_w() -> u32 {
    224
}

fn default_editor_w() -> u32 {
    480
}

fn deserialize_px<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Px {
        U64(u64),
        I64(i64),
        F64(f64),
    }

    match Px::deserialize(deserializer)? {
        Px::U64(value) => u32::try_from(value).map_err(serde::de::Error::custom),
        Px::I64(value) => u32::try_from(value).map_err(serde::de::Error::custom),
        Px::F64(value) => {
            if !value.is_finite() || value < 0.0 {
                return Err(serde::de::Error::custom(
                    "pixel size must be a finite non-negative number",
                ));
            }
            Ok(value.round().clamp(0.0, f64::from(u32::MAX)) as u32)
        }
    }
}

fn default_true() -> bool {
    true
}

fn validate_hex_color(field: &'static str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let digits = value.strip_prefix('#').unwrap_or("");
    if digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(Error::InvalidConfigFormat {
        field,
        expected: "#RRGGBB color",
    })
}

fn validate_range(field: &'static str, value: u16, min: u16, max: u16) -> Result<()> {
    if (min..=max).contains(&value) {
        return Ok(());
    }
    Err(Error::InvalidConfig {
        field,
        min: i64::from(min),
        max: i64::from(max),
        actual: i64::from(value),
    })
}
