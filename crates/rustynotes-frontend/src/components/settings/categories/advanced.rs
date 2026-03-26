//! Advanced settings panel — developer options and resets.

use leptos::prelude::*;

/// Reset onboarding state by clearing it from localStorage.
/// (Matches the Solid.js `resetOnboarding` helper.)
fn reset_onboarding() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("rustynotes-onboarding");
        }
    }
}

#[component]
pub fn AdvancedSettings() -> impl IntoView {
    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Advanced"</h2>
            <p class="settings-category-subtitle">"Developer options and resets"</p>

            <div class="setting-row">
                <div class="setting-info">
                    <div class="setting-label">"Onboarding Tips"</div>
                    <div class="setting-description">
                        "Show the welcome tips and feature highlights again"
                    </div>
                </div>
                <button class="settings-reset-btn" on:click=move |_| reset_onboarding()>
                    "Reset Tips"
                </button>
            </div>
        </div>
    }
}
