mod app;
mod bridge;
mod components;
mod state;
mod tauri_ipc;
mod theme;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
