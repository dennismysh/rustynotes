//! Appearance settings panel — theme, colors, and typography.

use leptos::prelude::*;
use rustynotes_common::AppConfig;

use crate::components::settings::shared::{
    SettingColorPicker, SettingRow, SettingSelect, SettingSlider,
};
use crate::tauri_ipc;
use crate::theme::{apply_theme, resolve_theme};

#[component]
pub fn AppearanceSettings() -> impl IntoView {
    let config = RwSignal::new(Option::<AppConfig>::None);

    // Load config on mount.
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

    // Helper: clone config, apply updater, save, and live-preview theme.
    let update = move |updater: Box<dyn FnOnce(&mut AppConfig)>| {
        if let Some(mut c) = config.get_untracked() {
            updater(&mut c);
            config.set(Some(c.clone()));
            let theme = resolve_theme(&c.theme.active);
            apply_theme(&theme, Some(&c.theme.overrides));
            leptos::task::spawn_local(async move {
                if let Err(e) = tauri_ipc::save_config_cmd(c).await {
                    web_sys::console::error_1(&format!("save_config: {e}").into());
                }
            });
        }
    };

    // Derived signals for each control.
    let theme_active = Signal::derive(move || {
        config.get().map(|c| c.theme.active.clone()).unwrap_or_else(|| "auto".into())
    });
    let accent_color = Signal::derive(move || {
        config
            .get()
            .and_then(|c| c.theme.overrides.colors.get("accent").cloned())
            .unwrap_or_else(|| "#89b4fa".into())
    });
    let font_size = Signal::derive(move || {
        config
            .get()
            .and_then(|c| {
                c.theme
                    .overrides
                    .typography
                    .get("body-size")
                    .and_then(|s| s.trim_end_matches("px").parse::<f64>().ok())
            })
            .unwrap_or(15.0)
    });
    let editor_font = Signal::derive(move || {
        config.get().map(|c| c.editor_font.clone()).unwrap_or_default()
    });
    let line_height = Signal::derive(move || {
        config.get().map(|c| c.line_height.to_string()).unwrap_or_else(|| "1.6".into())
    });

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Appearance"</h2>
            <p class="settings-category-subtitle">"Theme, colors, and typography"</p>

            <SettingRow label="Theme" description="Follow system or choose manually">
                <SettingSelect
                    value=theme_active
                    options=vec![
                        ("auto".into(), "Auto (System)".into()),
                        ("light".into(), "Light".into()),
                        ("dark".into(), "Dark".into()),
                    ]
                    on_change=move |v| update(Box::new(move |c| { c.theme.active = v; }))
                />
            </SettingRow>

            <SettingRow label="Accent Color" description="Used for links, selections, and highlights">
                <SettingColorPicker
                    value=accent_color
                    on_change=move |v| update(Box::new(move |c| {
                        c.theme.overrides.colors.insert("accent".into(), v);
                    }))
                />
            </SettingRow>

            <SettingRow label="Font Size" description="Base size for editor content">
                <SettingSlider
                    value=font_size
                    min=12.0
                    max=24.0
                    step=1.0
                    unit="px"
                    on_change=move |v| update(Box::new(move |c| {
                        c.theme.overrides.typography.insert(
                            "body-size".into(),
                            format!("{v}px"),
                        );
                    }))
                />
            </SettingRow>

            <SettingRow label="Editor Font" description="Font family for source editing (blank = system monospace)">
                <input
                    type="text"
                    class="setting-text-input"
                    prop:value=move || editor_font.get()
                    placeholder="System Default"
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        update(Box::new(move |c| { c.editor_font = v; }));
                    }
                />
            </SettingRow>

            <SettingRow label="Line Height" description="Spacing between lines in the editor">
                <SettingSelect
                    value=line_height
                    options=vec![
                        ("1.2".into(), "1.2 (Compact)".into()),
                        ("1.4".into(), "1.4 (Normal)".into()),
                        ("1.6".into(), "1.6 (Comfortable)".into()),
                        ("1.8".into(), "1.8 (Relaxed)".into()),
                    ]
                    on_change=move |v| {
                        let parsed: f64 = v.parse().unwrap_or(1.6);
                        update(Box::new(move |c| { c.line_height = parsed; }));
                    }
                />
            </SettingRow>
        </div>
    }
}
