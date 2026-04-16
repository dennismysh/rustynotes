use leptos::prelude::*;

use crate::components::save_indicator::SaveIndicator;
use crate::tauri_ipc;
use super::overflow_menu::OverflowMenu;

#[component]
pub fn SlimTitleBar() -> impl IntoView {
    let handle_close = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::close_current_window();
    };
    let handle_minimize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::minimize_current_window();
    };
    let handle_maximize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::toggle_maximize_current_window();
    };
    let handle_drag = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            tauri_ipc::start_dragging();
        }
    };
    let handle_dblclick = move |_: web_sys::MouseEvent| {
        tauri_ipc::toggle_maximize_current_window();
    };

    view! {
        <div class="slim-titlebar" on:mousedown=handle_drag on:dblclick=handle_dblclick>
            <div class="titlebar-buttons">
                <button class="titlebar-btn close" on:click=handle_close aria-label="Close" />
                <button class="titlebar-btn minimize" on:click=handle_minimize aria-label="Minimize" />
                <button class="titlebar-btn maximize" on:click=handle_maximize aria-label="Maximize" />
            </div>
            <SaveIndicator />
            <div class="spacer" />
            <OverflowMenu />
        </div>
    }
}
