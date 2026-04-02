//! File save logic — perform_save, keyboard shortcuts, auto-save timer,
//! focus-loss handler, and file-switch guard.

use leptos::prelude::*;
use rustynotes_common::{EditorMode, SaveMode};
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

use crate::state::{AppState, SaveStatus};
use crate::tauri_ipc;

// ---------------------------------------------------------------------------
// Core save function
// ---------------------------------------------------------------------------

/// Save the current editor content to disk. If no file path is set (new file),
/// opens a save dialog first.
pub async fn perform_save(state: &AppState) {
    let content = state.active_file_content.get_untracked();
    let path = state.active_file_path.get_untracked();

    let path = match path {
        Some(p) => p,
        None => {
            // New file — open save dialog
            match tauri_ipc::save_file_dialog("Untitled.md").await {
                Ok(Some(p)) => {
                    state.active_file_path.set(Some(p.clone()));
                    p
                }
                Ok(None) => return, // user cancelled
                Err(e) => {
                    state.save_status.set(SaveStatus::Error(e));
                    return;
                }
            }
        }
    };

    state.save_status.set(SaveStatus::Saving);

    // Record timestamp so the file watcher ignores our own write
    let now = js_sys::Date::now();
    state.last_save_timestamp.set(Some(now));

    match tauri_ipc::write_file(&path, &content).await {
        Ok(()) => {
            state.is_dirty.set(false);
            state.save_status.set(SaveStatus::Saved);
        }
        Err(e) => {
            state.save_status.set(SaveStatus::Error(e));
        }
    }
}

// ---------------------------------------------------------------------------
// Save handlers (keyboard, auto-save, focus loss)
// ---------------------------------------------------------------------------

/// Initialize all save-related event handlers. Call once at app mount.
pub fn init_save_handlers(state: &AppState) {
    init_keyboard_shortcuts(state);
    init_auto_save(state);
    init_focus_loss_save(state);
}

fn is_mac() -> bool {
    js_sys::eval("navigator.platform.includes('Mac')")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn init_keyboard_shortcuts(state: &AppState) {
    let state = state.clone();
    let mac = is_mac();

    let handler = Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let Ok(ke) = ev.dyn_into::<KeyboardEvent>() else { return };
        let modifier = if mac { ke.meta_key() } else { ke.ctrl_key() };
        if !modifier {
            return;
        }

        match ke.key().as_str() {
            "s" => {
                ke.prevent_default();
                let state = state.clone();
                leptos::task::spawn_local(async move {
                    perform_save(&state).await;
                });
            }
            "n" => {
                ke.prevent_default();
                state.active_file_path.set(None);
                state.active_file_content.set(String::new());
                state.is_dirty.set(false);
                state.save_status.set(SaveStatus::Idle);
                state.rendered_html.set(String::new());
            }
            "1" => {
                ke.prevent_default();
                state.editor_mode.set(EditorMode::Source);
            }
            "2" => {
                ke.prevent_default();
                state.editor_mode.set(EditorMode::Wysiwyg);
            }
            "3" => {
                ke.prevent_default();
                state.editor_mode.set(EditorMode::Split);
            }
            "4" => {
                ke.prevent_default();
                state.editor_mode.set(EditorMode::Preview);
            }
            _ => {}
        }
    });

    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback(
            "keydown",
            handler.as_ref().unchecked_ref(),
        );
    }
    handler.forget();
}

fn init_auto_save(state: &AppState) {
    let state = state.clone();

    // Reactive effect: when config changes, (re)start or stop the auto-save timer.
    Effect::new(move |prev_handle: Option<Option<gloo_timers::callback::Interval>>| {
        // Drop previous interval if any
        drop(prev_handle.flatten());

        let config = state.app_config.get();
        let Some(config) = config else {
            return None;
        };

        if config.save_mode != SaveMode::AfterDelay {
            return None;
        }

        let delay = config.auto_save_delay_ms.max(200); // floor at 200ms
        let state = state.clone();

        let interval = gloo_timers::callback::Interval::new(delay as u32, move || {
            if state.is_dirty.get_untracked() {
                let state = state.clone();
                leptos::task::spawn_local(async move {
                    perform_save(&state).await;
                });
            }
        });

        Some(interval)
    });
}

fn init_focus_loss_save(state: &AppState) {
    let state = state.clone();

    let handler = Closure::<dyn Fn()>::new(move || {
        let config = state.app_config.get_untracked();
        let is_focus_mode = config
            .as_ref()
            .map(|c| c.save_mode == SaveMode::OnFocusLoss)
            .unwrap_or(false);

        if !is_focus_mode || !state.is_dirty.get_untracked() {
            return;
        }

        // Check that the page is actually hidden (not just blurred)
        let hidden = web_sys::window()
            .and_then(|w| w.document())
            .map(|d| d.hidden())
            .unwrap_or(false);

        if hidden {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                perform_save(&state).await;
            });
        }
    });

    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ = document.add_event_listener_with_callback(
            "visibilitychange",
            handler.as_ref().unchecked_ref(),
        );
    }
    handler.forget();
}

// ---------------------------------------------------------------------------
// File-switch guard
// ---------------------------------------------------------------------------

/// Call before switching to a new file. Handles save-before-switch logic
/// based on the current save mode.
///
/// - Not dirty: loads `pending_path` immediately.
/// - Auto-save modes: saves silently, then loads.
/// - Manual mode: sets `pending_file_switch` signal to show the prompt UI.
pub fn guard_file_switch(state: &AppState, pending_path: String) {
    if !state.is_dirty.get_untracked() {
        load_file(state, pending_path);
        return;
    }

    let config = state.app_config.get_untracked();
    let save_mode = config
        .as_ref()
        .map(|c| c.save_mode.clone())
        .unwrap_or_default();

    match save_mode {
        SaveMode::AfterDelay | SaveMode::OnFocusLoss => {
            // Auto-save silently, then switch
            let state = state.clone();
            leptos::task::spawn_local(async move {
                perform_save(&state).await;
                load_file(&state, pending_path);
            });
        }
        SaveMode::Manual => {
            // Show the save-before-switch prompt
            state.pending_file_switch.set(Some(pending_path));
        }
    }
}

/// Load a file by path into the editor state.
pub fn load_file(state: &AppState, path: String) {
    state.active_file_path.set(Some(path.clone()));
    // Suppress dirty flag while the editor processes the new content.
    // TipTap normalizes markdown on parse, which fires onChange even though
    // the user didn't edit anything.
    state.suppress_dirty.set(true);
    let state = state.clone();
    leptos::task::spawn_local(async move {
        match tauri_ipc::read_file(&path).await {
            Ok(content) => {
                state.active_file_content.set(content);
                state.is_dirty.set(false);
                state.save_status.set(SaveStatus::Idle);
                // Reset scroll position to top for the new file
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(el) = doc.query_selector(".content-area").ok().flatten() {
                        el.set_scroll_top(0);
                    }
                    if let Some(el) = doc.query_selector(".editor-container").ok().flatten() {
                        el.set_scroll_top(0);
                    }
                    if let Some(el) = doc.query_selector(".preview-container").ok().flatten() {
                        el.set_scroll_top(0);
                    }
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to read file: {e}").into());
            }
        }
        // Clear suppression after a short delay to let the editor's onChange settle.
        let state2 = state.clone();
        gloo_timers::callback::Timeout::new(100, move || {
            state2.suppress_dirty.set(false);
        })
        .forget();
    });
}
