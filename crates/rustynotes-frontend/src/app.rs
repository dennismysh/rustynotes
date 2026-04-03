use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use rustynotes_common::{AppConfig, EditorMode, NavMode};

use crate::components::editor::{SourceEditor, SplitPane, WysiwygEditor};
use crate::components::navigation::{Breadcrumb, MillerColumns, Sidebar};
use crate::components::onboarding::WelcomeEmptyState;
use crate::components::preview::preview::Preview;
use crate::components::settings::SettingsWindow;
use crate::components::titlebar::TitleBar;
use crate::components::toolbar::Toolbar;
use crate::save;
use crate::state::{provide_app_state, use_app_state, AppState};
use crate::state::SaveStatus;
use crate::tauri_ipc;

/// Parse editor_mode and nav_mode strings from config and update state signals.
fn sync_modes_from_config(state: &AppState, config: &AppConfig) {
    if let Ok(mode) = serde_json::from_str::<EditorMode>(&format!("\"{}\"", config.editor_mode)) {
        state.editor_mode.set(mode);
    }
    if let Ok(mode) = serde_json::from_str::<NavMode>(&format!("\"{}\"", config.nav_mode)) {
        state.nav_mode.set(mode);
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_app_state();

    // leptos_router 0.7 uses history-based routing via BrowserUrl.
    // This works in Tauri 2 because it serves from a custom scheme (tauri://localhost)
    // which supports the History API. No hash-based router is needed.
    view! {
        <Router>
            <main>
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("") view=MainView />
                    <Route path=path!("/settings") view=SettingsView />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn MainView() -> impl IntoView {
    let state = use_app_state();
    // Initialize save handlers (keyboard shortcuts, auto-save timer, focus-loss)
    save::init_save_handlers(&state);

    // Load config on mount, apply theme, then show window
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
                        sync_modes_from_config(&state, &config);
                        // Auto-open last folder if available
                        let last_folder = config.recent_folders.first().cloned();
                        state.app_config.set(Some(config));
                        if let Some(folder) = last_folder {
                            save::open_folder(&state, folder).await;
                        }
                        tauri_ipc::show_current_window();
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("get_config: {e}").into());
                        // Still show window even if config fails
                        tauri_ipc::show_current_window();
                    }
                }
            });
        });
    }

    // Listen for config changes from settings window
    {
        let state = state.clone();
        tauri_ipc::listen_config_changed(move |config| {
            let theme = crate::theme::resolve_theme(&config.theme.active);
            crate::theme::apply_theme(&theme, Some(&config.theme.overrides));
            sync_modes_from_config(&state, &config);
            state.app_config.set(Some(config));
        });
    }

    let has_folder = move || state.current_folder.get().is_some();
    let nav_mode = state.nav_mode;
    let editor_mode = state.editor_mode;

    view! {
        <div class="app-shell">
            <Toolbar />
            <div class="app-body" style:display=move || if has_folder() { "flex" } else { "none" }>
                {move || match nav_mode.get() {
                    NavMode::Sidebar => view! { <Sidebar /> }.into_any(),
                    NavMode::Miller => view! { <MillerColumns /> }.into_any(),
                    NavMode::Breadcrumb => view! { <Breadcrumb /> }.into_any(),
                }}
                <div class="content-area">
                    {move || match editor_mode.get() {
                        EditorMode::Source => view! { <SourceEditor /> }.into_any(),
                        EditorMode::Wysiwyg => view! { <WysiwygEditor /> }.into_any(),
                        EditorMode::Split => view! { <SplitPane /> }.into_any(),
                        EditorMode::Preview => view! { <Preview /> }.into_any(),
                    }}
                </div>
            </div>
            <Show when=move || !has_folder()>
                <WelcomeEmptyState />
            </Show>
            // Save-before-switch prompt
            <Show when=move || state.pending_file_switch.get().is_some()>
                <div class="modal-overlay">
                    <div class="modal-dialog">
                        <p>"You have unsaved changes"</p>
                        <div class="modal-actions">
                            <button
                                class="modal-btn primary"
                                on:click={
                                    let state = state.clone();
                                    move |_| {
                                        let pending = state.pending_file_switch.get_untracked();
                                        state.pending_file_switch.set(None);
                                        if let Some(path) = pending {
                                            let state = state.clone();
                                            leptos::task::spawn_local(async move {
                                                save::perform_save(&state).await;
                                                save::load_file(&state, path);
                                            });
                                        }
                                    }
                                }
                            >
                                "Save"
                            </button>
                            <button
                                class="modal-btn"
                                on:click={
                                    let state = state.clone();
                                    move |_| {
                                        let pending = state.pending_file_switch.get_untracked();
                                        state.pending_file_switch.set(None);
                                        state.is_dirty.set(false);
                                        state.save_status.set(SaveStatus::Idle);
                                        if let Some(path) = pending {
                                            save::load_file(&state, path);
                                        }
                                    }
                                }
                            >
                                "Discard"
                            </button>
                            <button
                                class="modal-btn"
                                on:click={
                                    let state = state.clone();
                                    move |_| {
                                        state.pending_file_switch.set(None);
                                    }
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn SettingsView() -> impl IntoView {
    view! {
        <SettingsWindow />
    }
}
