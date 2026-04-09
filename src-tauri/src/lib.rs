mod commands;
mod config;
mod export;
mod fs_ops;
mod markdown_parser;
mod watcher;
mod updater;
mod binary_watcher;

use std::sync::Mutex;
use tauri::Manager;

struct WatcherState {
    _watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

#[tauri::command]
fn watch_folder(
    path: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<WatcherState>,
) -> Result<(), String> {
    let watcher = watcher::start_watcher(app_handle, std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    *state._watcher.lock().unwrap() = Some(watcher);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::VISIBLE
                        | tauri_plugin_window_state::StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .manage(WatcherState {
            _watcher: Mutex::new(None),
        })
        .manage(commands::config::ConfigState {
            config: Mutex::new(config::load_config()),
        })
        .manage(commands::update::UpdateState::new())
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::fs::resolve_wikilink,
            commands::fs::search_files,
            commands::config::get_config,
            commands::config::save_config_cmd,
            commands::config::open_settings,
            commands::export::export_file,
            commands::markdown::parse_markdown,
            watch_folder,
            commands::update::check_for_update,
            commands::update::apply_update,
            commands::update::restart_after_update,
            commands::update::get_update_status,
            commands::update::get_current_version,
            commands::update::dismiss_update,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background update check on startup + periodic (every 6 hours)
            std::thread::spawn(move || {
                loop {
                    let update_state = app_handle.state::<commands::update::UpdateState>();
                    let config_state = app_handle.state::<commands::config::ConfigState>();

                    if let Ok(Some(info)) = commands::update::perform_check(&app_handle, &update_state) {
                        let dismissed = config_state
                            .config
                            .lock()
                            .unwrap()
                            .dismissed_version
                            .clone();

                        // Skip if user dismissed this version
                        let is_dismissed = dismissed.as_deref() == Some(info.version.as_str());

                        // Clear stale dismissed_version if a newer version appeared
                        if !is_dismissed && dismissed.is_some() {
                            let mut config = config_state.config.lock().unwrap();
                            config.dismissed_version = None;
                            let _ = crate::config::save_config(&config);
                        }

                        let auto_update = config_state.config.lock().unwrap().auto_update;

                        if auto_update && !is_dismissed {
                            commands::update::perform_install(&app_handle, &update_state, &info);
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                }
            });

            // Binary self-watch
            if let Some(mut watcher) = binary_watcher::BinaryWatcher::start() {
                let app_handle2 = app.handle().clone();
                std::thread::spawn(move || {
                    loop {
                        if watcher.poll() {
                            let update_state = app_handle2.state::<commands::update::UpdateState>();
                            let in_progress = *update_state.update_in_progress.lock().unwrap();
                            if !in_progress {
                                let _ = updater::relaunch();
                                std::process::exit(0);
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(250));
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
