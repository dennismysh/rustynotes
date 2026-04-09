//! Update settings panel — auto-update toggle, check for updates, version display.

use leptos::prelude::*;
use rustynotes_common::AppConfig;

use crate::components::settings::shared::SettingRow;
use crate::tauri_ipc;

#[component]
pub fn UpdateSettings() -> impl IntoView {
    let config = RwSignal::new(Option::<AppConfig>::None);
    let current_version = RwSignal::new(String::new());
    let check_result = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(c) = tauri_ipc::get_config().await {
                config.set(Some(c));
            }
            if let Ok(v) = tauri_ipc::get_current_version().await {
                current_version.set(v);
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

    let auto_update = Signal::derive(move || {
        config.get().map(|c| c.auto_update).unwrap_or(true)
    });

    let available_version = RwSignal::new(Option::<String>::None);
    // "available" | "downloading" | "ready" | "error" | ""
    let update_phase = RwSignal::new(String::new());

    // Listen for backend update status events
    tauri_ipc::listen_update_status(move |json| {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(status) = parsed.get("status") {
                if let Some(s) = status.as_str() {
                    let phase = s.to_lowercase();
                    update_phase.set(phase.clone());
                    match phase.as_str() {
                        "downloading" => check_result.set(Some("Downloading...".to_string())),
                        "ready" => check_result.set(Some("Update ready — restart to apply.".to_string())),
                        _ => {}
                    }
                } else if let Some(obj) = status.as_object() {
                    if let Some(msg) = obj.get("Error").and_then(|v| v.as_str()) {
                        update_phase.set("error".to_string());
                        check_result.set(Some(format!("Error: {msg}")));
                    }
                }
            }
        }
    });

    let handle_check = move |_| {
        check_result.set(Some("Checking...".to_string()));
        available_version.set(None);
        update_phase.set(String::new());
        leptos::task::spawn_local(async move {
            match tauri_ipc::check_for_update_cmd().await {
                Ok(Some(version)) => {
                    check_result.set(Some(format!("v{version} available!")));
                    available_version.set(Some(version));
                    update_phase.set("available".to_string());
                }
                Ok(None) => {
                    check_result.set(Some("You're up to date.".to_string()));
                }
                Err(e) => {
                    check_result.set(Some(format!("Error: {e}")));
                }
            }
        });
    };

    let handle_update = move |_| {
        available_version.set(None);
        leptos::task::spawn_local(async move {
            if let Err(e) = tauri_ipc::apply_update_cmd().await {
                check_result.set(Some(format!("Error: {e}")));
            }
        });
    };

    let handle_restart = move |_| {
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::restart_after_update_cmd().await;
        });
    };

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Updates"</h2>
            <p class="settings-category-subtitle">"Keep RustyNotes up to date"</p>

            <SettingRow label="Current version" description="">
                <span class="setting-value">{move || current_version.get()}</span>
            </SettingRow>

            <SettingRow label="Auto-update" description="Download and install updates silently, prompt only to restart">
                <input
                    type="checkbox"
                    prop:checked=auto_update
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        update(Box::new(move |c| c.auto_update = checked));
                    }
                />
            </SettingRow>

            <SettingRow label="Check for updates" description="">
                <button
                    class="setting-btn"
                    on:click=handle_check
                >
                    "Check now"
                </button>
            </SettingRow>

            <Show when=move || check_result.get().is_some()>
                <div class="setting-check-result">
                    {move || check_result.get().unwrap_or_default()}
                    <Show when=move || update_phase.get() == "available">
                        <button
                            class="setting-btn"
                            style="margin-left: 8px;"
                            on:click=handle_update
                        >
                            "Update"
                        </button>
                    </Show>
                    <Show when=move || update_phase.get() == "ready">
                        <button
                            class="setting-btn"
                            style="margin-left: 8px;"
                            on:click=handle_restart
                        >
                            "Restart"
                        </button>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
