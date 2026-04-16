use std::path::{Path, PathBuf};

/// Push `path` to the front of `list`, dedup by equality, cap at `cap`
/// entries. Returns true if the list changed.
pub fn push_recent(list: &mut Vec<String>, path: String, cap: usize) -> bool {
    if list.first().map(|s| s == &path).unwrap_or(false) {
        return false;
    }
    list.retain(|p| p != &path);
    list.insert(0, path);
    if list.len() > cap {
        list.truncate(cap);
    }
    true
}

/// Remove entries from `list` whose paths no longer exist on disk.
/// Returns true if the list changed.
pub fn prune_missing(list: &mut Vec<String>) -> bool {
    let before = list.len();
    list.retain(|p| Path::new(p).exists());
    list.len() != before
}

/// Canonicalize (resolve symlinks + make absolute). Falls back to the
/// input if the path doesn't exist or can't be canonicalized.
pub fn canonicalize_or_same(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use crate::commands::config::ConfigState;
use crate::config as config_io;

const RECENT_FILES_CAP: usize = 10;

pub struct FileWindows {
    map: Mutex<HashMap<PathBuf, String>>,
}

impl FileWindows {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Path) -> Option<String> {
        self.map.lock().unwrap().get(path).cloned()
    }

    pub fn insert(&self, path: PathBuf, label: String) {
        self.map.lock().unwrap().insert(path, label);
    }

    pub fn remove_by_label(&self, label: &str) {
        self.map.lock().unwrap().retain(|_, v| v != label);
    }
}

pub fn open_file_in_new_window_inner(
    app: &AppHandle,
    path: String,
    file_windows: &FileWindows,
    config_state: &ConfigState,
) -> Result<(), String> {
    let canonical = canonicalize_or_same(&path);

    if !canonical.exists() {
        return Err(format!("File not found: {}", canonical.display()));
    }
    std::fs::read_to_string(&canonical)
        .map_err(|e| format!("Cannot read file as UTF-8: {e}"))?;

    if let Some(label) = file_windows.get(&canonical) {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_focus();
            return Ok(());
        }
        file_windows.remove_by_label(&label);
    }

    let canonical_lossy = canonical.to_string_lossy();
    let encoded = urlencoding::encode(&canonical_lossy);
    let url = format!("/file?path={encoded}");
    let label = format!("file-{}", Uuid::new_v4().simple());

    let canonical_str = canonical.to_string_lossy().into_owned();
    let filename = canonical
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string());

    WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title(&filename)
        .inner_size(800.0, 650.0)
        .min_inner_size(400.0, 300.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    file_windows.insert(canonical, label);

    let mut config = config_state.config.lock().unwrap();
    if push_recent(&mut config.recent_files, canonical_str, RECENT_FILES_CAP) {
        config_io::save_config(&config).map_err(|e| e.to_string())?;
        let _ = app.emit("config-changed", config.clone());
    }

    Ok(())
}

#[tauri::command]
pub fn open_file_in_new_window(
    app: AppHandle,
    path: String,
    file_windows: tauri::State<FileWindows>,
    config_state: tauri::State<ConfigState>,
) -> Result<(), String> {
    open_file_in_new_window_inner(&app, path, &file_windows, &config_state)
}

#[tauri::command]
pub fn open_folder_in_window(app: AppHandle, path: String) -> Result<(), String> {
    let canonical = canonicalize_or_same(&path);
    let parent = canonical
        .parent()
        .ok_or_else(|| "File has no parent directory".to_string())?
        .to_string_lossy()
        .into_owned();
    let filename = canonical
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
        let _ = app.emit(
            "open-folder-with-file",
            serde_json::json!({ "folder": parent, "file": filename }),
        );
        return Ok(());
    }

    // No main window — create one
    WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("/".into()))
        .title("RustyNotes")
        .inner_size(1100.0, 750.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    // Update recent_folders so MainView auto-opens on mount
    let config_state = app.state::<ConfigState>();
    let mut config = config_state.config.lock().unwrap();
    if !config.recent_folders.iter().any(|f| f == &parent) {
        config.recent_folders.insert(0, parent.clone());
        if config.recent_folders.len() > 10 {
            config.recent_folders.truncate(10);
        }
        let _ = config_io::save_config(&config);
    }

    let _ = app.emit(
        "open-folder-with-file",
        serde_json::json!({ "folder": parent, "file": filename }),
    );

    Ok(())
}

#[tauri::command]
pub fn open_file_dialog(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;

    let app_clone = app.clone();
    app.dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .pick_file(move |file| {
            if let Some(path) = file {
                let path_str = path.to_string();
                let fw = app_clone.state::<FileWindows>();
                let cs = app_clone.state::<ConfigState>();
                let _ = open_file_in_new_window_inner(&app_clone, path_str, &fw, &cs);
            }
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_adds_new_entry_at_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/c".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/c", "/a", "/b"]);
    }

    #[test]
    fn push_recent_moves_existing_entry_to_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
        let changed = push_recent(&mut list, "/b".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/b", "/a", "/c"]);
    }

    #[test]
    fn push_recent_no_op_if_already_first() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/a".to_string(), 10);
        assert!(!changed);
        assert_eq!(list, vec!["/a", "/b"]);
    }

    #[test]
    fn push_recent_caps_length() {
        let mut list: Vec<String> = (0..10).map(|i| format!("/{i}")).collect();
        push_recent(&mut list, "/new".to_string(), 10);
        assert_eq!(list.len(), 10);
        assert_eq!(list[0], "/new");
        assert_eq!(list[9], "/8");
    }

    #[test]
    fn prune_missing_removes_nonexistent() {
        let mut list = vec!["/definitely/not/a/real/path.md".to_string()];
        let changed = prune_missing(&mut list);
        assert!(changed);
        assert!(list.is_empty());
    }

    #[test]
    fn prune_missing_keeps_existing() {
        let tmp = std::env::temp_dir();
        let mut list = vec![tmp.to_string_lossy().into_owned()];
        let changed = prune_missing(&mut list);
        assert!(!changed);
        assert_eq!(list.len(), 1);
    }
}
