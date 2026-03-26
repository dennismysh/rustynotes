mod bridge;
mod state;
mod tauri_ipc;
mod theme;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        state::provide_app_state();
        view! { <p>"RustyNotes — state + theme loaded"</p> }
    });
}
