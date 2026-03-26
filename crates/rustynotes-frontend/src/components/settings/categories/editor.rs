//! Editor settings panel — editing mode and navigation.

use leptos::prelude::*;
use rustynotes_common::AppConfig;

use crate::components::settings::shared::{SettingRow, SettingSelect};
use crate::tauri_ipc;

#[component]
pub fn EditorSettings() -> impl IntoView {
    let config = RwSignal::new(Option::<AppConfig>::None);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tauri_ipc::get_config().await {
                Ok(c) => config.set(Some(c)),
                Err(e) => {
                    web_sys::console::error_1(&format!("get_config: {e}").into());
                }
            }
        });
    });

    let update = move |updater: Box<dyn FnOnce(&mut AppConfig)>| {
        if let Some(mut c) = config.get_untracked() {
            updater(&mut c);
            config.set(Some(c.clone()));
            leptos::task::spawn_local(async move {
                if let Err(e) = tauri_ipc::save_config_cmd(c).await {
                    web_sys::console::error_1(&format!("save_config: {e}").into());
                }
            });
        }
    };

    let editor_mode = Signal::derive(move || {
        config.get().map(|c| c.editor_mode.clone()).unwrap_or_else(|| "wysiwyg".into())
    });
    let nav_mode = Signal::derive(move || {
        config.get().map(|c| c.nav_mode.clone()).unwrap_or_else(|| "sidebar".into())
    });

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Editor"</h2>
            <p class="settings-category-subtitle">"Editing mode and navigation"</p>

            <SettingRow label="Editor Mode" description="How you write and preview content">
                <SettingSelect
                    value=editor_mode
                    options=vec![
                        ("wysiwyg".into(), "Rich Text (WYSIWYG)".into()),
                        ("source".into(), "Markdown Source".into()),
                        ("split".into(), "Split View".into()),
                        ("preview".into(), "Preview Only".into()),
                    ]
                    on_change=move |v| update(Box::new(move |c| { c.editor_mode = v; }))
                />
            </SettingRow>

            <SettingRow label="Navigation" description="How you browse files">
                <SettingSelect
                    value=nav_mode
                    options=vec![
                        ("sidebar".into(), "Sidebar Tree".into()),
                        ("miller".into(), "Miller Columns".into()),
                        ("breadcrumb".into(), "Breadcrumb Path".into()),
                    ]
                    on_change=move |v| update(Box::new(move |c| { c.nav_mode = v; }))
                />
            </SettingRow>
        </div>
    }
}
