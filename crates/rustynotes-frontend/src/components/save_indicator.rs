use leptos::prelude::*;
use crate::state::{use_app_state, SaveStatus};

fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[component]
pub fn SaveIndicator() -> impl IntoView {
    let state = use_app_state();
    let active_file_path = state.active_file_path;
    let is_dirty = state.is_dirty;
    let save_status = state.save_status;

    let active_filename = Memo::new(move |_| {
        active_file_path
            .get()
            .as_deref()
            .map(filename_from_path)
            .map(String::from)
    });

    view! {
        <div class="toolbar-filename">
            {move || {
                let status = save_status.get();
                let dirty = is_dirty.get();
                match status {
                    SaveStatus::Saving => {
                        view! { <span class="save-indicator saving" aria-label="Saving">{"\u{21BB}"}</span> }.into_any()
                    }
                    SaveStatus::Saved => {
                        view! { <span class="save-indicator saved" aria-label="Saved">{"\u{2713}"}</span> }.into_any()
                    }
                    SaveStatus::Error(ref msg) => {
                        let title = msg.clone();
                        view! { <span class="save-indicator error" title=title aria-label="Save error">{"\u{26A0}"}</span> }.into_any()
                    }
                    SaveStatus::Idle if dirty => {
                        view! { <span class="dirty-indicator" aria-label="Unsaved changes" /> }.into_any()
                    }
                    _ => {
                        view! { <span /> }.into_any()
                    }
                }
            }}
            <span
                class="toolbar-filename-text"
                title=move || active_file_path.get().unwrap_or_default()
            >
                {move || {
                    let name = active_filename.get().unwrap_or_default();
                    let path = active_file_path.get();
                    if path.is_none() && name.is_empty() {
                        "Untitled".to_string()
                    } else {
                        name
                    }
                }}
            </span>
        </div>
    }
}
