//! Reusable form primitives for the settings UI.
//!
//! Ports of the Solid.js shared components: SettingRow, SettingToggle,
//! SettingSlider, SettingSelect, and SettingColorPicker.

use leptos::prelude::*;

// ---------------------------------------------------------------------------
// SettingRow
// ---------------------------------------------------------------------------

/// A labelled row with an optional description and a control slot.
#[component]
pub fn SettingRow(
    #[prop(into)] label: String,
    #[prop(into, optional)] description: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="setting-row">
            <div class="setting-info">
                <div class="setting-label">{label}</div>
                {description.map(|desc| view! {
                    <div class="setting-description">{desc}</div>
                })}
            </div>
            <div class="setting-control">{children()}</div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// SettingToggle
// ---------------------------------------------------------------------------

/// A toggle switch (on/off) rendered as an accessible button with
/// `role="switch"`.
#[component]
pub fn SettingToggle(
    #[prop(into)] checked: Signal<bool>,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <button
            class="toggle-switch"
            class:on=move || checked.get()
            on:click=move |_| on_change(!checked.get())
            role="switch"
            aria-checked=move || checked.get().to_string()
        >
            <span class="toggle-knob" />
        </button>
    }
}

// ---------------------------------------------------------------------------
// SettingSlider
// ---------------------------------------------------------------------------

/// A range slider with a displayed value and optional unit suffix.
#[component]
pub fn SettingSlider(
    #[prop(into)] value: Signal<f64>,
    #[prop(into)] min: f64,
    #[prop(into)] max: f64,
    #[prop(default = 1.0)] step: f64,
    #[prop(into, optional)] unit: Option<String>,
    on_change: impl Fn(f64) + 'static,
) -> impl IntoView {
    let unit_str = unit.unwrap_or_default();
    view! {
        <div class="setting-slider">
            <input
                type="range"
                min=min.to_string()
                max=max.to_string()
                step=step.to_string()
                prop:value=move || value.get().to_string()
                on:input=move |ev| {
                    let v: f64 = event_target_value(&ev).parse().unwrap_or(min);
                    on_change(v);
                }
            />
            <span class="setting-slider-value">
                {move || format!("{}{}", value.get(), unit_str)}
            </span>
        </div>
    }
}

// ---------------------------------------------------------------------------
// SettingSelect
// ---------------------------------------------------------------------------

/// A `<select>` dropdown built from a list of (value, label) pairs.
#[component]
pub fn SettingSelect(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] options: Vec<(String, String)>,
    on_change: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <select
            class="setting-select"
            prop:value=move || value.get()
            on:change=move |ev| {
                on_change(event_target_value(&ev));
            }
        >
            {options.into_iter().map(|(val, label)| {
                let val_clone = val.clone();
                view! {
                    <option value=val selected=move || value.get() == val_clone>
                        {label}
                    </option>
                }
            }).collect::<Vec<_>>()}
        </select>
    }
}

// ---------------------------------------------------------------------------
// SettingColorPicker
// ---------------------------------------------------------------------------

/// A native `<input type="color">` picker.
#[component]
pub fn SettingColorPicker(
    #[prop(into)] value: Signal<String>,
    on_change: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <input
            type="color"
            class="setting-color"
            prop:value=move || value.get()
            on:input=move |ev| {
                on_change(event_target_value(&ev));
            }
        />
    }
}
