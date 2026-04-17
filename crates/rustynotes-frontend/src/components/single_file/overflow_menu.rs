use leptos::prelude::*;
use wasm_bindgen::JsCast;
use rustynotes_common::EditorMode;

use crate::state::use_app_state;
use crate::tauri_ipc;

// ---------------------------------------------------------------------------
// Helpers (mirrors toolbar.rs)
// ---------------------------------------------------------------------------

fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn stem_from_filename(name: &str) -> &str {
    match name.rfind('.') {
        Some(pos) => &name[..pos],
        None => name,
    }
}

// ---------------------------------------------------------------------------
// OverflowMenu component
// ---------------------------------------------------------------------------

#[component]
pub fn OverflowMenu() -> impl IntoView {
    let open = RwSignal::new(false);

    let state = use_app_state();
    let editor_mode = state.editor_mode;
    let active_file_path = state.active_file_path;
    let active_file_content = state.active_file_content;

    let toggle_menu = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.update(|v| *v = !*v);
    };

    // Close menu when clicking outside.
    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        let handler = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
            open.set(false);
        });
        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback_and_bool(
                "click",
                handler.as_ref().unchecked_ref(),
                false,
            );
        }
        handler.forget();
    });

    // ---- item handlers ----

    let handle_source_mode = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.set(false);
        editor_mode.set(EditorMode::Source);
    };

    let handle_open_in_folder = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.set(false);
        let path = active_file_path.get_untracked();
        if let Some(p) = path {
            leptos::task::spawn_local(async move {
                let _ = tauri_ipc::open_folder_in_window(&p).await;
            });
        }
    };

    let handle_settings = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.set(false);
        leptos::task::spawn_local(async move {
            if let Err(e) = tauri_ipc::open_settings().await {
                web_sys::console::error_1(&format!("open_settings failed: {e}").into());
            }
        });
    };

    let handle_export = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.set(false);
        leptos::task::spawn_local(async move {
            let file_path_val = active_file_path.get_untracked();
            let Some(ref path) = file_path_val else { return };
            let content = active_file_content.get_untracked();

            let file_name = filename_from_path(path);
            let stem = stem_from_filename(file_name);
            let default_name = format!("{stem}.html");

            let save_path = match tauri_ipc::save_file_dialog(&default_name).await {
                Ok(Some(p)) => p,
                Ok(None) => return,
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("save_file_dialog failed: {e}").into(),
                    );
                    return;
                }
            };

            if let Err(e) = tauri_ipc::export_file(&content, &save_path, "html", true).await {
                web_sys::console::error_1(&format!("export_file failed: {e}").into());
            }
        });
    };

    view! {
        <div class="overflow-menu-wrapper">
            <button
                class="slim-titlebar-overflow"
                aria-label="More"
                title="More"
                on:click=toggle_menu
            >
                {"\u{22EF}"}
            </button>
            <Show when=move || open.get()>
                <div class="overflow-menu" on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()>
                    <button class="overflow-item" on:click=handle_source_mode>
                        "Switch to Source mode"
                    </button>
                    <button class="overflow-item" on:click=handle_open_in_folder>
                        "Open in folder window"
                    </button>
                    <div class="overflow-sep" />
                    <button class="overflow-item" on:click=handle_settings>
                        "Settings\u{2026}"
                    </button>
                    <div class="overflow-sep" />
                    <button class="overflow-item" on:click=handle_export>
                        "Export HTML\u{2026}"
                    </button>
                </div>
            </Show>
        </div>
    }
}
