//! Settings sidebar — category navigation tabs.

use leptos::prelude::*;

/// A settings category definition (id, label, icon).
#[derive(Clone)]
pub struct SettingsCategory {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
}

/// Sidebar navigation for the settings window.
#[component]
pub fn SettingsSidebar(
    categories: Vec<SettingsCategory>,
    #[prop(into)] active_id: Signal<String>,
    on_select: impl Fn(String) + 'static + Clone,
) -> impl IntoView {
    view! {
        <nav class="settings-sidebar">
            <div class="settings-sidebar-header">"Settings"</div>
            {categories.into_iter().map(|cat| {
                let on_select = on_select.clone();
                let id = cat.id.to_string();
                view! {
                    <button
                        class="settings-sidebar-item"
                        class:active=move || active_id.get() == cat.id
                        on:click=move |_| on_select(id.clone())
                    >
                        <span class="settings-sidebar-icon">{cat.icon}</span>
                        <span class="settings-sidebar-label">{cat.label}</span>
                    </button>
                }
            }).collect::<Vec<_>>()}
        </nav>
    }
}
