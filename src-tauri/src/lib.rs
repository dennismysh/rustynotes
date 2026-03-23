mod commands;
mod fs_ops;
mod markdown_parser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::markdown::parse_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
