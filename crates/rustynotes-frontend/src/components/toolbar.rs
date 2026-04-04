use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::KeyboardEvent;

use crate::state::{use_app_state, SaveStatus};
use crate::tauri_ipc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the filename from a path (the last `/`-separated segment).
fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Strip the file extension from a filename.
fn stem_from_filename(name: &str) -> &str {
    match name.rfind('.') {
        Some(pos) => &name[..pos],
        None => name,
    }
}

/// Sleep for `ms` milliseconds (WASM-compatible).
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = JsFuture::from(promise).await;
}

// ---------------------------------------------------------------------------
// Toolbar component
// ---------------------------------------------------------------------------

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_app_state();
    let export_status = RwSignal::new(Option::<String>::None);

    // Update banner state
    let update_version = RwSignal::new(Option::<String>::None);
    let update_status = RwSignal::new(String::from("idle"));

    // Listen for update status events
    tauri_ipc::listen_update_status(move |json| {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(status) = parsed.get("status") {
                if let Some(s) = status.as_str() {
                    update_status.set(s.to_lowercase());
                } else if let Some(obj) = status.as_object() {
                    if let Some(v) = obj.get("Available").and_then(|v| v.get("version")).and_then(|v| v.as_str()) {
                        update_version.set(Some(v.to_string()));
                        update_status.set("available".to_string());
                    } else if obj.contains_key("Error") {
                        update_status.set("error".to_string());
                    }
                }
            }
        }
    });

    let handle_update_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::apply_update_cmd().await;
        });
    };

    let handle_restart_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::restart_after_update_cmd().await;
        });
    };

    let handle_dismiss_update = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        update_version.set(None);
        update_status.set("idle".to_string());
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::dismiss_update_cmd().await;
        });
    };

    // Pull out the signals we need — RwSignal is Copy so these are cheap.
    let current_folder = state.current_folder;
    let file_tree = state.file_tree;
    let active_file_path = state.active_file_path;
    let active_file_content = state.active_file_content;
    let is_dirty = state.is_dirty;
    let show_search = state.show_search;
    let save_status = state.save_status;

    // Derived: active filename (just the basename, e.g. "notes.md")
    let active_filename = Memo::new(move |_| {
        active_file_path
            .get()
            .as_deref()
            .map(filename_from_path)
            .map(String::from)
    });

    // ---- handlers ----

    let state_for_folder = state.clone();
    let handle_open_folder = move |_| {
        let state = state_for_folder.clone();
        leptos::task::spawn_local(async move {
            match tauri_ipc::open_folder_dialog().await {
                Ok(Some(folder)) => {
                    crate::save::open_folder(&state, folder).await;
                }
                Ok(None) => { /* user cancelled */ }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("open_folder_dialog failed: {e}").into(),
                    );
                }
            }
        });
    };

    let handle_export = move |_| {
        leptos::task::spawn_local(async move {
            let file_path_val = active_file_path.get_untracked();
            let Some(ref path) = file_path_val else { return };
            let content = active_file_content.get_untracked();

            let file_name = filename_from_path(path);
            let stem = stem_from_filename(file_name);
            let default_name = format!("{stem}.html");

            let save_path = match tauri_ipc::save_file_dialog(&default_name).await {
                Ok(Some(p)) => p,
                Ok(None) => return, // user cancelled
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("save_file_dialog failed: {e}").into(),
                    );
                    export_status.set(Some("Could not export".to_string()));
                    leptos::task::spawn_local(async move {
                        sleep_ms(2000).await;
                        export_status.set(None);
                    });
                    return;
                }
            };

            match tauri_ipc::export_file(&content, &save_path, "html", true).await {
                Ok(()) => {
                    let saved_name = filename_from_path(&save_path);
                    export_status.set(Some(format!("Saved {saved_name}")));
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Export failed: {e}").into());
                    export_status.set(Some("Could not export".to_string()));
                }
            }

            leptos::task::spawn_local(async move {
                sleep_ms(2000).await;
                export_status.set(None);
            });
        });
    };

    let handle_search_toggle = move |_| {
        show_search.update(|v| *v = !*v);
    };

    let handle_settings = move |_| {
        leptos::task::spawn_local(async move {
            if let Err(e) = tauri_ipc::open_settings().await {
                web_sys::console::error_1(&format!("open_settings failed: {e}").into());
            }
        });
    };

    // ---- Cmd+, keyboard shortcut (app-lifetime) ----

    let is_mac = js_sys::eval("navigator.platform.includes('Mac')")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Register the global keydown listener once on mount.
    Effect::new(move |_| {
        let handler = Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
            if let Ok(ke) = ev.dyn_into::<KeyboardEvent>() {
                let modifier = if is_mac { ke.meta_key() } else { ke.ctrl_key() };
                if modifier && ke.key() == "," {
                    ke.prevent_default();
                    leptos::task::spawn_local(async move {
                        if let Err(e) = tauri_ipc::open_settings().await {
                            web_sys::console::error_1(
                                &format!("open_settings failed: {e}").into(),
                            );
                        }
                    });
                }
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "keydown",
                handler.as_ref().unchecked_ref(),
            );
        }

        // Leak intentionally — this is an app-lifetime listener.
        handler.forget();
    });

    // Reset "Saved" status back to Idle after 1.5s
    Effect::new(move |_| {
        if save_status.get() == SaveStatus::Saved {
            let save_status = save_status;
            leptos::task::spawn_local(async move {
                sleep_ms(1500).await;
                // Only reset if still in Saved state
                if save_status.get_untracked() == SaveStatus::Saved {
                    save_status.set(SaveStatus::Idle);
                }
            });
        }
    });

    // ---- search shortcut label ----

    let search_shortcut = if is_mac {
        "\u{2318}K"
    } else {
        "Ctrl+K"
    };

    let handle_close = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::close_current_window();
    };
    let handle_minimize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::minimize_current_window();
    };
    let handle_maximize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::toggle_maximize_current_window();
    };
    let handle_drag = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            tauri_ipc::start_dragging();
        }
    };
    let handle_dblclick = move |_: web_sys::MouseEvent| {
        tauri_ipc::toggle_maximize_current_window();
    };

    view! {
        <div class="toolbar" on:mousedown=handle_drag on:dblclick=handle_dblclick>
            <div class="titlebar-buttons">
                <button class="titlebar-btn close" on:click=handle_close aria-label="Close" />
                <button class="titlebar-btn minimize" on:click=handle_minimize aria-label="Minimize" />
                <button class="titlebar-btn maximize" on:click=handle_maximize aria-label="Maximize" />
            </div>
            <button on:click=handle_open_folder>"Open Folder"</button>
            // Update banner
            <Show when=move || {
                let s = update_status.get();
                s != "idle" && s != "checking"
            }>
                <div class="update-banner">
                    {move || {
                        let s = update_status.get();
                        match s.as_str() {
                            "available" => {
                                let v = update_version.get().unwrap_or_default();
                                view! {
                                    <span class="update-text">{format!("v{v} available")}</span>
                                    <button class="update-btn" on:click=handle_update_click>"Update"</button>
                                    <button class="update-dismiss" on:click=handle_dismiss_update>{"\u{00D7}"}</button>
                                }.into_any()
                            }
                            "downloading" => {
                                view! { <span class="update-text">"Downloading..."</span> }.into_any()
                            }
                            "installing" => {
                                view! { <span class="update-text">"Installing..."</span> }.into_any()
                            }
                            "ready" => {
                                view! {
                                    <span class="update-text">"Update ready"</span>
                                    <button class="update-btn" on:click=handle_restart_click>"Restart"</button>
                                }.into_any()
                            }
                            "error" => {
                                view! { <span class="update-text update-error">"Update failed"</span> }.into_any()
                            }
                            _ => view! { <span /> }.into_any()
                        }
                    }}
                </div>
            </Show>
            <div class="spacer" />
            <Show when=move || active_filename.get().is_some() || active_file_path.get().is_none()>
                <div class="toolbar-filename">
                    {move || {
                        let status = save_status.get();
                        let dirty = is_dirty.get();
                        match status {
                            SaveStatus::Saving => {
                                view! { <span class="save-indicator saving" aria-label="Saving">{"\u{21BB}"}</span> }.into_any()
                            }
                            SaveStatus::Saved => {
                                view! { <span class="save-indicator saved" aria-label="Saved">{"\u{2713}"}</span> }.into_any()
                            }
                            SaveStatus::Error(ref msg) => {
                                let title = msg.clone();
                                view! { <span class="save-indicator error" title=title aria-label="Save error">{"\u{26A0}"}</span> }.into_any()
                            }
                            SaveStatus::Idle if dirty => {
                                view! { <span class="dirty-indicator" aria-label="Unsaved changes" /> }.into_any()
                            }
                            _ => {
                                view! { <span /> }.into_any()
                            }
                        }
                    }}
                    <span
                        class="toolbar-filename-text"
                        title=move || active_file_path.get().unwrap_or_default()
                    >
                        {move || {
                            let name = active_filename.get().unwrap_or_default();
                            let path = active_file_path.get();
                            if path.is_none() && name.is_empty() {
                                "Untitled".to_string()
                            } else {
                                name
                            }
                        }}
                    </span>
                </div>
            </Show>
            <div class="spacer" />
            <Show when=move || export_status.get().is_some()>
                <span class="toolbar-status">
                    {move || export_status.get().unwrap_or_default()}
                </span>
            </Show>
            <button
                class="toolbar-icon-btn"
                class:active=move || show_search.get()
                on:click=handle_search_toggle
                title=format!("Search files ({search_shortcut})")
            >
                "\u{2315}"
            </button>
            <button
                class="toolbar-icon-btn"
                on:click=handle_export
                title="Export as HTML"
            >
                "\u{21E5}"
            </button>
            <button
                class="toolbar-icon-btn"
                on:click=handle_settings
                title="Settings"
            >
                "\u{2699}"
            </button>
        </div>
    }
}
