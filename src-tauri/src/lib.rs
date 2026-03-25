mod commands;
mod config;
mod export;
mod fs_ops;
mod markdown_parser;
mod watcher;

use std::sync::Mutex;

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
        .manage(WatcherState {
            _watcher: Mutex::new(None),
        })
        .manage(commands::config::ConfigState {
            config: Mutex::new(config::load_config()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::fs::resolve_wikilink,
            commands::fs::search_files,
            commands::markdown::parse_markdown,
            commands::config::get_config,
            commands::config::save_config_cmd,
            commands::config::open_settings,
            commands::export::export_file,
            watch_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
