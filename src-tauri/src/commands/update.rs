use crate::updater::{self, UpdateInfo, UpdateStatus};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

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

/// Shared update check logic used by both the background thread and IPC command.
/// Returns Ok(Some(info)) if an update is available, Ok(None) if up to date,
/// or Err(msg) if the check failed (error is also emitted as UpdateStatus::Error).
pub fn perform_check(
    app: &AppHandle,
    state: &UpdateState,
) -> Result<Option<UpdateInfo>, String> {
    *state.status.lock().unwrap() = UpdateStatus::Checking;
    emit_status(app, UpdateStatus::Checking);

    match updater::check_for_update() {
        Ok(Some(info)) => {
            *state.available.lock().unwrap() = Some(info.clone());
            let status = UpdateStatus::Available {
                version: info.version.clone(),
            };
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
            Ok(Some(info))
        }
        Ok(None) => {
            *state.status.lock().unwrap() = UpdateStatus::Idle;
            emit_status(app, UpdateStatus::Idle);
            Ok(None)
        }
        Err(e) => {
            let msg = e.to_string();
            let status = UpdateStatus::Error(msg.clone());
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
            Err(msg)
        }
    }
}

/// Shared download+install logic used by both the background thread and IPC command.
pub fn perform_install(
    app: &AppHandle,
    state: &UpdateState,
    info: &UpdateInfo,
) {
    *state.update_in_progress.lock().unwrap() = true;
    emit_status(app, UpdateStatus::Downloading);

    match updater::download_and_install(&info.download_url, &info.version) {
        Ok(()) => {
            let status = UpdateStatus::Ready;
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
        }
        Err(e) => {
            *state.update_in_progress.lock().unwrap() = false;
            let status = UpdateStatus::Error(e.to_string());
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
        }
    }
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
) -> Result<Option<UpdateInfo>, String> {
    perform_check(&app, &state)
}

#[tauri::command]
pub fn apply_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
) -> Result<(), String> {
    let info = state
        .available
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update available")?;

    let app_handle = app.clone();
    let version = info.version.clone();
    let url = info.download_url.clone();

    std::thread::spawn(move || {
        let state_ref = app_handle.state::<UpdateState>();
        let info = UpdateInfo {
            version,
            download_url: url,
        };
        perform_install(&app_handle, &state_ref, &info);
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
        config.dismissed_version = Some(info.version);
        let _ = crate::config::save_config(&config);
    }
    *state.status.lock().unwrap() = UpdateStatus::Idle;
}
