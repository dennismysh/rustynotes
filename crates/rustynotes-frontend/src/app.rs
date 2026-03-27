use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use rustynotes_common::{EditorMode, NavMode};

use crate::components::editor::{SourceEditor, SplitPane, WysiwygEditor};
use crate::components::navigation::{Breadcrumb, MillerColumns, Sidebar};
use crate::components::onboarding::WelcomeEmptyState;
use crate::components::preview::preview::Preview;
use crate::components::settings::SettingsWindow;
use crate::components::toolbar::Toolbar;
use crate::save;
use crate::state::{provide_app_state, use_app_state};
use crate::state::SaveStatus;

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

    let nav_view = move || match state.nav_mode.get() {
        NavMode::Sidebar => view! { <Sidebar /> }.into_any(),
        NavMode::Miller => view! { <MillerColumns /> }.into_any(),
        NavMode::Breadcrumb => view! { <Breadcrumb /> }.into_any(),
    };

    let editor_view = move || match state.editor_mode.get() {
        EditorMode::Source => view! { <SourceEditor /> }.into_any(),
        EditorMode::Wysiwyg => view! { <WysiwygEditor /> }.into_any(),
        EditorMode::Split => view! { <SplitPane /> }.into_any(),
        EditorMode::Preview => view! { <Preview /> }.into_any(),
    };

    let has_folder = move || state.current_folder.get().is_some();

    view! {
        <div class="app-container">
            <Toolbar />
            <Show
                when=has_folder
                fallback=|| view! { <WelcomeEmptyState /> }
            >
                {nav_view}
                <div class="main-content">
                    {editor_view}
                </div>
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
