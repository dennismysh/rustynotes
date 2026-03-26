use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::preview::preview::Preview;
use crate::state::provide_app_state;

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
    view! {
        <div class="app-container">
            <div class="main-content">
                <Preview />
            </div>
        </div>
    }
}

#[component]
fn SettingsView() -> impl IntoView {
    view! {
        <div class="settings-container">
            <p>"Settings — coming soon"</p>
        </div>
    }
}
