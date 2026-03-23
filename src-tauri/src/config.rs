use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default = "default_editor_mode")]
    pub editor_mode: String,
    #[serde(default = "default_nav_mode")]
    pub nav_mode: String,
    #[serde(default)]
    pub rendering: RenderingToggles,
    #[serde(default)]
    pub recent_folders: Vec<String>,
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
    pub colors: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub typography: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub spacing: std::collections::HashMap<String, String>,
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

fn default_true() -> bool { true }
fn default_editor_mode() -> String { "source".to_string() }
fn default_nav_mode() -> String { "sidebar".to_string() }
fn default_active_theme() -> String { "auto".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            editor_mode: default_editor_mode(),
            nav_mode: default_nav_mode(),
            rendering: RenderingToggles::default(),
            recent_folders: Vec::new(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self { active: default_active_theme(), overrides: ThemeOverrides::default() }
    }
}

impl Default for RenderingToggles {
    fn default() -> Self {
        Self { render_math: true, render_diagrams: true, render_frontmatter: true, show_line_numbers: true, render_wikilinks: true }
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("rustynotes")
}

fn config_path() -> PathBuf { config_dir().join("config.json") }

pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serializes() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.editor_mode, "source");
        assert_eq!(parsed.theme.active, "auto");
        assert!(parsed.rendering.render_math);
    }

    #[test]
    fn test_config_with_overrides() {
        let json = r##"{"theme":{"active":"dark","overrides":{"colors":{"accent":"#ff0000"}}}}"##;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.theme.active, "dark");
        assert_eq!(config.theme.overrides.colors.get("accent").unwrap(), "#ff0000");
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let json = r#"{"editor_mode":"wysiwyg"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.editor_mode, "wysiwyg");
        assert!(config.rendering.render_math);
        assert_eq!(config.theme.active, "auto");
    }
}
