use crate::updater::{self, UpdateInfo, UpdateStatus};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub struct UpdateState {
    pub status: Mutex<UpdateStatus>,
    pub available: Mutex<Option<UpdateInfo>>,
    pub update_in_progress: Mutex<bool>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
            available: Mutex::new(None),
            update_in_progress: Mutex::new(false),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct StatusEvent {
    pub status: UpdateStatus,
}

fn emit_status(app: &AppHandle, status: UpdateStatus) {
    let _ = app.emit("update-status", StatusEvent {
        status: status.clone(),
    });
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) -> Option<UpdateInfo> {
    *state.status.lock().unwrap() = UpdateStatus::Checking;
    emit_status(&app, UpdateStatus::Checking);

    let last_updated = config_state
        .config
        .lock()
        .unwrap()
        .last_updated_version
        .clone();

    let result = updater::check_for_update(last_updated.as_deref());

    if let Some(ref info) = result {
        *state.available.lock().unwrap() = Some(info.clone());
        let status = UpdateStatus::Available {
            version: info.version.clone(),
        };
        *state.status.lock().unwrap() = status.clone();
        emit_status(&app, status);
    } else {
        *state.status.lock().unwrap() = UpdateStatus::Idle;
        emit_status(&app, UpdateStatus::Idle);
    }

    result
}

#[tauri::command]
pub fn apply_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) -> Result<(), String> {
    let info = state
        .available
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update available")?;

    *state.update_in_progress.lock().unwrap() = true;

    {
        let mut config = config_state.config.lock().unwrap();
        config.last_updated_version = Some(info.version.clone());
        let _ = crate::config::save_config(&config);
    }

    let url = info.download_url.clone();
    let app_handle = app.clone();

    std::thread::spawn(move || {
        emit_status(&app_handle, UpdateStatus::Downloading);

        match updater::download_and_install(&url) {
            Ok(()) => {
                emit_status(&app_handle, UpdateStatus::Ready);
            }
            Err(e) => {
                emit_status(&app_handle, UpdateStatus::Error(e));
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn restart_after_update() -> Result<(), String> {
    updater::relaunch()?;
    std::process::exit(0);
}

#[tauri::command]
pub fn get_update_status(state: tauri::State<UpdateState>) -> UpdateStatus {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_current_version() -> String {
    updater::current_version().to_string()
}

#[tauri::command]
pub fn dismiss_update(
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) {
    if let Some(info) = state.available.lock().unwrap().take() {
        let mut config = config_state.config.lock().unwrap();
        config.last_updated_version = Some(info.version);
        let _ = crate::config::save_config(&config);
    }
    *state.status.lock().unwrap() = UpdateStatus::Idle;
}
