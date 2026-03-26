//! Preview settings panel — rendering toggle switches.

use leptos::prelude::*;
use rustynotes_common::AppConfig;

use crate::components::settings::shared::{SettingRow, SettingToggle};
use crate::tauri_ipc;

#[component]
pub fn PreviewSettings() -> impl IntoView {
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

    let render_math = Signal::derive(move || {
        config.get().map(|c| c.rendering.render_math).unwrap_or(true)
    });
    let render_diagrams = Signal::derive(move || {
        config.get().map(|c| c.rendering.render_diagrams).unwrap_or(true)
    });
    let render_frontmatter = Signal::derive(move || {
        config.get().map(|c| c.rendering.render_frontmatter).unwrap_or(true)
    });
    let show_line_numbers = Signal::derive(move || {
        config.get().map(|c| c.rendering.show_line_numbers).unwrap_or(true)
    });
    let render_wikilinks = Signal::derive(move || {
        config.get().map(|c| c.rendering.render_wikilinks).unwrap_or(true)
    });

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Preview"</h2>
            <p class="settings-category-subtitle">"Markdown rendering options"</p>

            <SettingRow label="Math Equations" description="Render LaTeX math with KaTeX">
                <SettingToggle
                    checked=render_math
                    on_change=move |_| update(Box::new(|c| { c.rendering.render_math = !c.rendering.render_math; }))
                />
            </SettingRow>

            <SettingRow label="Diagrams" description="Render Mermaid diagrams">
                <SettingToggle
                    checked=render_diagrams
                    on_change=move |_| update(Box::new(|c| { c.rendering.render_diagrams = !c.rendering.render_diagrams; }))
                />
            </SettingRow>

            <SettingRow label="YAML Header" description="Show frontmatter metadata">
                <SettingToggle
                    checked=render_frontmatter
                    on_change=move |_| update(Box::new(|c| { c.rendering.render_frontmatter = !c.rendering.render_frontmatter; }))
                />
            </SettingRow>

            <SettingRow label="Code Line Numbers" description="Show line numbers in code blocks">
                <SettingToggle
                    checked=show_line_numbers
                    on_change=move |_| update(Box::new(|c| { c.rendering.show_line_numbers = !c.rendering.show_line_numbers; }))
                />
            </SettingRow>

            <SettingRow label="Wiki Links" description="Enable [[wikilink]] syntax">
                <SettingToggle
                    checked=render_wikilinks
                    on_change=move |_| update(Box::new(|c| { c.rendering.render_wikilinks = !c.rendering.render_wikilinks; }))
                />
            </SettingRow>
        </div>
    }
}
