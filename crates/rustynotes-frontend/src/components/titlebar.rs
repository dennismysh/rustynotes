use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::tauri_ipc;

/// Custom title bar with macOS-style traffic light buttons.
/// Used when `decorations: false` in Tauri config.
#[component]
pub fn TitleBar() -> impl IntoView {
    let handle_close = move |_: web_sys::MouseEvent| {
        tauri_ipc::close_current_window();
    };

    let handle_minimize = move |_: web_sys::MouseEvent| {
        tauri_ipc::minimize_current_window();
    };

    let handle_maximize = move |_: web_sys::MouseEvent| {
        tauri_ipc::toggle_maximize_current_window();
    };

    let handle_drag = move |ev: web_sys::MouseEvent| {
        // Only start drag on left-click, not on button clicks
        if ev.button() == 0 {
            tauri_ipc::start_dragging();
        }
    };

    let handle_dblclick = move |_: web_sys::MouseEvent| {
        tauri_ipc::toggle_maximize_current_window();
    };

    view! {
        <div
            class="custom-titlebar"
            on:mousedown=handle_drag
            on:dblclick=handle_dblclick
        >
            <div class="titlebar-buttons">
                <button
                    class="titlebar-btn close"
                    on:click=handle_close
                    aria-label="Close"
                />
                <button
                    class="titlebar-btn minimize"
                    on:click=handle_minimize
                    aria-label="Minimize"
                />
                <button
                    class="titlebar-btn maximize"
                    on:click=handle_maximize
                    aria-label="Maximize"
                />
            </div>
        </div>
    }
}
