use crate::config::{self, AppConfig};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub struct ConfigState {
    pub config: Mutex<AppConfig>,
}

#[tauri::command]
pub fn get_config(state: tauri::State<ConfigState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config_cmd(
    app: AppHandle,
    config_data: AppConfig,
    state: tauri::State<ConfigState>,
) -> Result<(), String> {
    config::save_config(&config_data)?;
    *state.config.lock().unwrap() = config_data.clone();
    let _ = app.emit("config-changed", config_data);
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    // If settings window already exists, focus it
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
        return Ok(());
    }

    // Create new settings window
    WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("index.html#/settings".into()))
        .title("Settings")
        .inner_size(700.0, 500.0)
        .min_inner_size(500.0, 350.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}
