pub use rustynotes_common::{AppConfig, RenderingToggles, ThemeConfig, ThemeOverrides};

use std::fs;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("rustynotes")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

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
        assert_eq!(parsed.editor_mode, "wysiwyg");
        assert_eq!(parsed.theme.active, "auto");
        assert!(parsed.rendering.render_math);
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
    fn test_partial_config_uses_defaults() {
        let json = r#"{"editor_mode":"wysiwyg"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.editor_mode, "wysiwyg");
        assert!(config.rendering.render_math);
        assert_eq!(config.theme.active, "auto");
    }
}
