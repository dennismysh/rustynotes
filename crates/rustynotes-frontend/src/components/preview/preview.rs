use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use crate::tauri_ipc;

#[component]
pub fn Preview() -> impl IntoView {
    let state = use_app_state();

    Effect::new(move |_| {
        let content = state.active_file_content.get();
        let rendered_html = state.rendered_html;
        if content.is_empty() {
            rendered_html.set(String::new());
        } else {
            spawn_local(async move {
                match tauri_ipc::parse_markdown(&content).await {
                    Ok(html) => rendered_html.set(html),
                    Err(e) => {
                        web_sys::console::error_1(&format!("parse_markdown: {e}").into());
                    }
                }
            });
        }
    });

    view! {
        <div class="preview-container" inner_html=move || state.rendered_html.get() />
    }
}
