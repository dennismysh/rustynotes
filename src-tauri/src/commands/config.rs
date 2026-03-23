use crate::config::{self, AppConfig};
use std::sync::Mutex;

pub struct ConfigState {
    pub config: Mutex<AppConfig>,
}

#[tauri::command]
pub fn get_config(state: tauri::State<ConfigState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config_cmd(config_data: AppConfig, state: tauri::State<ConfigState>) -> Result<(), String> {
    config::save_config(&config_data)?;
    *state.config.lock().unwrap() = config_data;
    Ok(())
}
