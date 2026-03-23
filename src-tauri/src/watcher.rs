use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub paths: Vec<String>,
    pub kind: String,
}

pub fn start_watcher(
    app_handle: tauri::AppHandle,
    path: &Path,
) -> Result<RecommendedWatcher, notify::Error> {
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    let handle = app_handle.clone();
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if let Ok(event) = event {
                let paths: Vec<String> = event
                    .paths
                    .iter()
                    .filter(|p| {
                        p.extension()
                            .map(|ext| ext == "md")
                            .unwrap_or(false)
                    })
                    .map(|p| p.display().to_string())
                    .collect();

                if !paths.is_empty() {
                    let kind = format!("{:?}", event.kind);
                    let _ = handle.emit(
                        "file-changed",
                        FileChangeEvent { paths, kind },
                    );
                }
            }
        }
    });

    Ok(watcher)
}
