use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use rustynotes_common::{EditorMode, NavMode};

use crate::components::editor::{SourceEditor, SplitPane, WysiwygEditor};
use crate::components::navigation::{Breadcrumb, MillerColumns, Sidebar};
use crate::components::preview::preview::Preview;
use crate::components::settings::SettingsWindow;
use crate::components::toolbar::Toolbar;
use crate::state::{provide_app_state, use_app_state};

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

    view! {
        <div class="app-container">
            <Toolbar />
            {nav_view}
            <div class="main-content">
                {editor_view}
            </div>
        </div>
    }
}

#[component]
fn SettingsView() -> impl IntoView {
    view! {
        <SettingsWindow />
    }
}
