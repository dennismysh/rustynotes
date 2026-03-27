use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// File tree types
// ---------------------------------------------------------------------------

/// A node in the file tree. Uses `String` paths (not `PathBuf`) for wasm32
/// compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

/// A search hit returned by full-text / filename search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub context: String,
}

// ---------------------------------------------------------------------------
// Editor / navigation mode enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EditorMode {
    Source,
    Wysiwyg,
    Split,
    Preview,
}

impl Default for EditorMode {
    fn default() -> Self {
        Self::Wysiwyg
    }
}

impl std::fmt::Display for EditorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source => write!(f, "source"),
            Self::Wysiwyg => write!(f, "wysiwyg"),
            Self::Split => write!(f, "split"),
            Self::Preview => write!(f, "preview"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NavMode {
    Sidebar,
    Miller,
    Breadcrumb,
}

impl Default for NavMode {
    fn default() -> Self {
        Self::Sidebar
    }
}

impl std::fmt::Display for NavMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sidebar => write!(f, "sidebar"),
            Self::Miller => write!(f, "miller"),
            Self::Breadcrumb => write!(f, "breadcrumb"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SaveMode {
    Manual,
    AfterDelay,
    OnFocusLoss,
}

impl Default for SaveMode {
    fn default() -> Self {
        Self::Manual
    }
}

impl std::fmt::Display for SaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "manual"),
            Self::AfterDelay => write!(f, "after_delay"),
            Self::OnFocusLoss => write!(f, "on_focus_loss"),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default = "default_editor_mode")]
    pub editor_mode: String,
    #[serde(default = "default_nav_mode")]
    pub nav_mode: String,
    #[serde(default)]
    pub editor_font: String,
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    #[serde(default)]
    pub rendering: RenderingToggles,
    #[serde(default)]
    pub recent_folders: Vec<String>,
    #[serde(default)]
    pub save_mode: SaveMode,
    #[serde(default = "default_auto_save_delay_ms")]
    pub auto_save_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_active_theme")]
    pub active: String,
    #[serde(default)]
    pub overrides: ThemeOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeOverrides {
    #[serde(default)]
    pub colors: HashMap<String, String>,
    #[serde(default)]
    pub typography: HashMap<String, String>,
    #[serde(default)]
    pub spacing: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingToggles {
    #[serde(default = "default_true")]
    pub render_math: bool,
    #[serde(default = "default_true")]
    pub render_diagrams: bool,
    #[serde(default = "default_true")]
    pub render_frontmatter: bool,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub render_wikilinks: bool,
}

// ---------------------------------------------------------------------------
// Theme data (for loading theme JSON files)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeData {
    pub name: String,
    #[serde(default)]
    pub colors: HashMap<String, String>,
    #[serde(default)]
    pub typography: HashMap<String, String>,
    #[serde(default)]
    pub spacing: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Serde default helpers
// ---------------------------------------------------------------------------

pub fn default_true() -> bool {
    true
}
pub fn default_editor_mode() -> String {
    "wysiwyg".to_string()
}
pub fn default_nav_mode() -> String {
    "sidebar".to_string()
}
pub fn default_active_theme() -> String {
    "auto".to_string()
}
pub fn default_line_height() -> f64 {
    1.6
}
pub fn default_auto_save_delay_ms() -> u64 {
    1000
}

// ---------------------------------------------------------------------------
// Default impls
// ---------------------------------------------------------------------------

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            editor_mode: default_editor_mode(),
            nav_mode: default_nav_mode(),
            editor_font: String::default(),
            line_height: default_line_height(),
            rendering: RenderingToggles::default(),
            recent_folders: Vec::new(),
            save_mode: SaveMode::default(),
            auto_save_delay_ms: default_auto_save_delay_ms(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            active: default_active_theme(),
            overrides: ThemeOverrides::default(),
        }
    }
}

impl Default for RenderingToggles {
    fn default() -> Self {
        Self {
            render_math: true,
            render_diagrams: true,
            render_frontmatter: true,
            show_line_numbers: true,
            render_wikilinks: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.editor_mode, "wysiwyg");
        assert_eq!(parsed.theme.active, "auto");
        assert!(parsed.rendering.render_math);
        assert_eq!(parsed.line_height, 1.6);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let json = r#"{"editor_mode":"source"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.editor_mode, "source");
        assert!(config.rendering.render_math);
        assert_eq!(config.theme.active, "auto");
        assert_eq!(config.line_height, 1.6);
    }

    #[test]
    fn test_config_with_overrides() {
        let json = r##"{"theme":{"active":"dark","overrides":{"colors":{"accent":"#ff0000"}}}}"##;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.theme.active, "dark");
        assert_eq!(
            config.theme.overrides.colors.get("accent").unwrap(),
            "#ff0000"
        );
    }

    #[test]
    fn test_rendering_toggles_roundtrip() {
        let toggles = RenderingToggles {
            render_math: false,
            render_diagrams: true,
            render_frontmatter: false,
            show_line_numbers: true,
            render_wikilinks: false,
        };
        let json = serde_json::to_string(&toggles).unwrap();
        let parsed: RenderingToggles = serde_json::from_str(&json).unwrap();
        assert!(!parsed.render_math);
        assert!(parsed.render_diagrams);
        assert!(!parsed.render_frontmatter);
        assert!(parsed.show_line_numbers);
        assert!(!parsed.render_wikilinks);
    }

    #[test]
    fn test_file_node_roundtrip() {
        let node = FileNode {
            name: "test.md".to_string(),
            path: "/some/path/test.md".to_string(),
            is_dir: false,
            children: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        let parsed: FileNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, parsed);
    }

    #[test]
    fn test_file_node_with_children() {
        let node = FileNode {
            name: "folder".to_string(),
            path: "/some/folder".to_string(),
            is_dir: true,
            children: Some(vec![FileNode {
                name: "child.md".to_string(),
                path: "/some/folder/child.md".to_string(),
                is_dir: false,
                children: None,
            }]),
        };
        let json = serde_json::to_string(&node).unwrap();
        let parsed: FileNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, parsed);
        assert_eq!(parsed.children.unwrap().len(), 1);
    }

    #[test]
    fn test_search_result_roundtrip() {
        let result = SearchResult {
            path: "/path/to/file.md".to_string(),
            name: "file.md".to_string(),
            context: "some matching text".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, parsed);
    }

    #[test]
    fn test_editor_mode_serde() {
        let mode = EditorMode::Source;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"source\"");
        let parsed: EditorMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, EditorMode::Source);
    }

    #[test]
    fn test_editor_mode_default() {
        assert_eq!(EditorMode::default(), EditorMode::Wysiwyg);
    }

    #[test]
    fn test_nav_mode_serde() {
        let mode = NavMode::Miller;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"miller\"");
        let parsed: NavMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, NavMode::Miller);
    }

    #[test]
    fn test_nav_mode_default() {
        assert_eq!(NavMode::default(), NavMode::Sidebar);
    }

    #[test]
    fn test_theme_data_roundtrip() {
        let theme = ThemeData {
            name: "dark".to_string(),
            colors: [("bg".to_string(), "#000".to_string())]
                .into_iter()
                .collect(),
            typography: HashMap::new(),
            spacing: HashMap::new(),
        };
        let json = serde_json::to_string(&theme).unwrap();
        let parsed: ThemeData = serde_json::from_str(&json).unwrap();
        assert_eq!(theme, parsed);
    }

    #[test]
    fn test_save_mode_serde() {
        let mode = SaveMode::AfterDelay;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"after_delay\"");
        let parsed: SaveMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SaveMode::AfterDelay);
    }

    #[test]
    fn test_save_mode_default() {
        assert_eq!(SaveMode::default(), SaveMode::Manual);
    }

    #[test]
    fn test_config_save_mode_defaults() {
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.save_mode, SaveMode::Manual);
        assert_eq!(config.auto_save_delay_ms, 1000);
    }

    #[test]
    fn test_config_with_save_mode() {
        let json = r#"{"save_mode":"after_delay","auto_save_delay_ms":2000}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.save_mode, SaveMode::AfterDelay);
        assert_eq!(config.auto_save_delay_ms, 2000);
    }

    #[test]
    fn test_empty_json_gives_defaults() {
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.editor_mode, "wysiwyg");
        assert_eq!(config.nav_mode, "sidebar");
        assert_eq!(config.theme.active, "auto");
        assert!(config.rendering.render_math);
        assert!(config.rendering.render_diagrams);
        assert!(config.rendering.render_frontmatter);
        assert!(config.rendering.show_line_numbers);
        assert!(config.rendering.render_wikilinks);
        assert_eq!(config.line_height, 1.6);
        assert!(config.editor_font.is_empty());
        assert!(config.recent_folders.is_empty());
    }
}
