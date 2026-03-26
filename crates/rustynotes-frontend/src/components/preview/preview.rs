use leptos::prelude::*;

use crate::state::use_app_state;
use super::markdown::render_markdown;

#[component]
pub fn Preview() -> impl IntoView {
    let state = use_app_state();

    let html = Memo::new(move |_| {
        let content = state.active_file_content.get();
        if content.is_empty() {
            String::new()
        } else {
            render_markdown(&content)
        }
    });

    Effect::new(move || {
        state.rendered_html.set(html.get());
    });

    view! {
        <div class="preview-container" inner_html=move || html.get() />
    }
}
