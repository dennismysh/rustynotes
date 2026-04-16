pub mod overflow_menu;
pub mod slim_titlebar;

use leptos::prelude::*;

use crate::components::editor::WysiwygEditor;
use crate::save;
use crate::state::use_app_state;
use crate::tauri_ipc;

use slim_titlebar::SlimTitleBar;

fn read_path_param() -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let trimmed = search.trim_start_matches('?');
    for pair in trimmed.split('&') {
        if let Some(rest) = pair.strip_prefix("path=") {
            return js_sys::decode_uri_component(rest)
                .ok()
                .and_then(|v| v.as_string());
        }
    }
    None
}

#[component]
pub fn SingleFileView() -> impl IntoView {
    let state = use_app_state();
    // Initialize save handlers (keyboard shortcuts, auto-save timer, focus-loss)
    save::init_save_handlers(&state);

    // Load config on mount, apply theme, load the file, then show window
    {
        let state = state.clone();
        Effect::new(move |_| {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match tauri_ipc::get_config().await {
                    Ok(config) => {
                        // Apply theme before showing window to prevent flash
                        let theme = crate::theme::resolve_theme(&config.theme.active);
                        crate::theme::apply_theme(&theme, Some(&config.theme.overrides));
                        state.app_config.set(Some(config));
                        // Load the file specified in the query param
                        if let Some(path) = read_path_param() {
                            save::load_file(&state, path);
                        }
                        tauri_ipc::show_current_window();
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("get_config: {e}").into());
                        // Still load file and show window even if config fails
                        if let Some(path) = read_path_param() {
                            save::load_file(&state, path);
                        }
                        tauri_ipc::show_current_window();
                    }
                }
            });
        });
    }

    // Listen for config changes from other windows
    {
        let state = state.clone();
        tauri_ipc::listen_config_changed(move |config| {
            let theme = crate::theme::resolve_theme(&config.theme.active);
            crate::theme::apply_theme(&theme, Some(&config.theme.overrides));
            state.app_config.set(Some(config));
        });
    }

    view! {
        <div class="single-file-shell">
            <SlimTitleBar />
            <div class="single-file-content">
                <WysiwygEditor />
            </div>
        </div>
    }
}
