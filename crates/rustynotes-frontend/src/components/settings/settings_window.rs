//! SettingsWindow — container that renders the sidebar + the active category
//! panel. Loaded at the `/settings` route in a separate Tauri webview.

use leptos::prelude::*;

use crate::components::settings::categories::{
    AdvancedSettings, AppearanceSettings, EditorSettings, PreviewSettings,
};
use crate::components::settings::settings_sidebar::{SettingsCategory, SettingsSidebar};
use crate::tauri_ipc;
use crate::theme::{apply_theme, resolve_theme};

fn categories() -> Vec<SettingsCategory> {
    vec![
        SettingsCategory { id: "appearance", label: "Appearance", icon: "\u{1F3A8}" },
        SettingsCategory { id: "editor",     label: "Editor",     icon: "\u{270F}\u{FE0F}" },
        SettingsCategory { id: "preview",    label: "Preview",    icon: "\u{1F441}" },
        SettingsCategory { id: "advanced",   label: "Advanced",   icon: "\u{1F50C}" },
    ]
}

#[component]
pub fn SettingsWindow() -> impl IntoView {
    let active_category = RwSignal::new("appearance".to_string());

    // Load config on mount so the settings window matches the main window theme.
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tauri_ipc::get_config().await {
                Ok(config) => {
                    let theme = resolve_theme(&config.theme.active);
                    apply_theme(&theme, Some(&config.theme.overrides));
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("get_config: {e}").into());
                }
            }
        });
    });

    let active_id_signal = Signal::derive(move || active_category.get());

    view! {
        <div class="settings-window">
            <SettingsSidebar
                categories=categories()
                active_id=active_id_signal
                on_select=move |id: String| active_category.set(id)
            />
            <main class="settings-detail">
                {move || match active_category.get().as_str() {
                    "editor"   => view! { <EditorSettings /> }.into_any(),
                    "preview"  => view! { <PreviewSettings /> }.into_any(),
                    "advanced" => view! { <AdvancedSettings /> }.into_any(),
                    _          => view! { <AppearanceSettings /> }.into_any(),
                }}
            </main>
        </div>
    }
}
