use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::commands::config::ConfigState;

/// Build the full application menu from the current config state.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let config_state = app.state::<ConfigState>();
    let config = config_state.config.lock().unwrap();
    let recent_files = config.recent_files.clone();
    let recent_folders = config.recent_folders.clone();
    drop(config);

    // ── App menu (macOS only — shows as the application name) ─────────────
    let app_menu = SubmenuBuilder::new(app, "RustyNotes")
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    // ── Open Recent submenu ───────────────────────────────────────────────
    let mut recent_builder = SubmenuBuilder::new(app, "Open Recent");

    if recent_files.is_empty() && recent_folders.is_empty() {
        // Show a single disabled placeholder so the submenu isn't empty
        recent_builder = recent_builder.item(
            &MenuItemBuilder::with_id("recent.empty", "No Recent Items")
                .enabled(false)
                .build(app)?,
        );
    } else {
        if !recent_files.is_empty() {
            recent_builder = recent_builder.item(
                &MenuItemBuilder::with_id("recent.header.files", "Recent Files")
                    .enabled(false)
                    .build(app)?,
            );
            for (i, path) in recent_files.iter().take(10).enumerate() {
                let label = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                let id = format!("recent.file:{i}");
                recent_builder = recent_builder.item(
                    &MenuItemBuilder::with_id(id, label)
                        .build(app)?,
                );
            }
        }

        if !recent_files.is_empty() && !recent_folders.is_empty() {
            recent_builder = recent_builder.separator();
        }

        if !recent_folders.is_empty() {
            recent_builder = recent_builder.item(
                &MenuItemBuilder::with_id("recent.header.folders", "Recent Folders")
                    .enabled(false)
                    .build(app)?,
            );
            for (i, folder) in recent_folders.iter().take(10).enumerate() {
                let label = std::path::Path::new(folder)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| folder.clone());
                let id = format!("recent.folder:{i}");
                recent_builder = recent_builder.item(
                    &MenuItemBuilder::with_id(id, label)
                        .build(app)?,
                );
            }
        }

        recent_builder = recent_builder
            .separator()
            .item(
                &MenuItemBuilder::with_id("recent.clear", "Clear Recent")
                    .build(app)?,
            );
    }

    let open_recent = recent_builder.build()?;

    // ── File menu ─────────────────────────────────────────────────────────
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(
            &MenuItemBuilder::with_id("file.new", "New File")
                .accelerator("CmdOrCtrl+N")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("file.open-file", "Open File…")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("file.open-folder", "Open Folder…")
                .accelerator("CmdOrCtrl+Shift+O")
                .build(app)?,
        )
        .separator()
        .item(&open_recent)
        .separator()
        .item(
            &MenuItemBuilder::with_id("file.save", "Save")
                .accelerator("CmdOrCtrl+S")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("file.export", "Export HTML…")
                .build(app)?,
        )
        .build()?;

    // ── Edit menu ─────────────────────────────────────────────────────────
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // ── Window menu ───────────────────────────────────────────────────────
    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    // ── Assemble top-level menu ───────────────────────────────────────────
    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&window_menu)
        .build()
}

/// Dispatch a menu event by its string ID.
pub fn handle_menu_event(app: &AppHandle, id: &str) {
    if let Some(rest) = id.strip_prefix("recent.file:") {
        if let Ok(idx) = rest.parse::<usize>() {
            let config_state = app.state::<ConfigState>();
            let path = config_state
                .config
                .lock()
                .unwrap()
                .recent_files
                .get(idx)
                .cloned();
            if let Some(path) = path {
                let fw = app.state::<crate::commands::window_mgmt::FileWindows>();
                let cs = app.state::<ConfigState>();
                let _ = crate::commands::window_mgmt::open_file_in_new_window_inner(
                    app, path, &fw, &cs,
                );
            }
        }
        return;
    }

    if let Some(rest) = id.strip_prefix("recent.folder:") {
        if let Ok(idx) = rest.parse::<usize>() {
            let config_state = app.state::<ConfigState>();
            let folder = config_state
                .config
                .lock()
                .unwrap()
                .recent_folders
                .get(idx)
                .cloned();
            if let Some(folder) = folder {
                let _ = app.emit(
                    "open-folder-with-file",
                    serde_json::json!({ "folder": folder, "file": "" }),
                );
            }
        }
        return;
    }

    match id {
        "file.new" => {
            let _ = app.emit("menu:new-file", ());
        }
        "file.open-file" => {
            let _ = crate::commands::window_mgmt::open_file_dialog(app.clone());
        }
        "file.open-folder" => {
            let _ = app.emit("menu:open-folder", ());
        }
        "file.save" => {
            let _ = app.emit("menu:save", ());
        }
        "file.export" => {
            let _ = app.emit("menu:export", ());
        }
        "recent.clear" => {
            let config_state = app.state::<ConfigState>();
            {
                let mut config = config_state.config.lock().unwrap();
                config.recent_files.clear();
                config.recent_folders.clear();
                let _ = crate::config::save_config(&config);
                let _ = app.emit("config-changed", config.clone());
            }
            // Rebuild menu so Open Recent becomes empty
            if let Ok(new_menu) = build_menu(app) {
                let _ = app.set_menu(new_menu);
            }
        }
        _ => {}
    }
}
