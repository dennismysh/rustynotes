//! Saving settings panel — save mode and auto-save delay.

use leptos::prelude::*;
use rustynotes_common::{AppConfig, SaveMode};

use crate::components::settings::shared::{SettingRow, SettingSelect, SettingSlider};
use crate::tauri_ipc;

#[component]
pub fn SavingSettings() -> impl IntoView {
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

    let save_mode = Signal::derive(move || {
        config
            .get()
            .map(|c| c.save_mode.to_string())
            .unwrap_or_else(|| "manual".into())
    });

    let auto_save_delay = Signal::derive(move || {
        config
            .get()
            .map(|c| (c.auto_save_delay_ms as f64) / 1000.0)
            .unwrap_or(1.0)
    });

    let is_after_delay = Signal::derive(move || {
        config
            .get()
            .map(|c| c.save_mode == SaveMode::AfterDelay)
            .unwrap_or(false)
    });

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Saving"</h2>
            <p class="settings-category-subtitle">"When and how files are saved"</p>

            <SettingRow label="Save Mode" description="When to save your changes">
                <SettingSelect
                    value=save_mode
                    options=vec![
                        ("manual".into(), "Manual (Cmd+S)".into()),
                        ("after_delay".into(), "After Delay".into()),
                        ("on_focus_loss".into(), "On Focus Loss".into()),
                    ]
                    on_change=move |v| {
                        let mode = match v.as_str() {
                            "after_delay" => SaveMode::AfterDelay,
                            "on_focus_loss" => SaveMode::OnFocusLoss,
                            _ => SaveMode::Manual,
                        };
                        update(Box::new(move |c| { c.save_mode = mode; }));
                    }
                />
            </SettingRow>

            <Show when=move || is_after_delay.get()>
                <SettingRow label="Auto-save Delay" description="Seconds between edits and auto-save">
                    <SettingSlider
                        value=auto_save_delay
                        min=0.2
                        max=10.0
                        step=0.1
                        unit="s".to_string()
                        on_change=move |v| {
                            let ms = (v * 1000.0) as u64;
                            update(Box::new(move |c| { c.auto_save_delay_ms = ms; }));
                        }
                    />
                </SettingRow>
            </Show>
        </div>
    }
}
