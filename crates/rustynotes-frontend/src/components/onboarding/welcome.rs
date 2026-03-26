use leptos::prelude::*;

use crate::state::use_app_state;
use crate::tauri_ipc;

use super::{create_is_first_run, mark_welcomed};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the last path segment as a folder name.
fn folder_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Detect macOS via `navigator.platform`.
fn is_mac() -> bool {
    js_sys::eval("navigator.platform.includes('Mac')")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// WelcomeEmptyState component
// ---------------------------------------------------------------------------

/// Shown when no folder is open. Displays a welcome message, recent folders,
/// and keyboard shortcut hints.
#[component]
pub fn WelcomeEmptyState() -> impl IntoView {
    let state = use_app_state();
    let (is_first_run, set_first_run) = create_is_first_run();

    let modifier = if is_mac() { "\u{2318}" } else { "Ctrl+" };

    // Derived: recent folders from app config (max 5).
    let recent_folders = Memo::new(move |_| {
        state
            .app_config
            .get()
            .map(|c| c.recent_folders)
            .unwrap_or_default()
    });

    // ---- handlers ----

    let current_folder = state.current_folder;
    let file_tree = state.file_tree;

    let open_folder = move |_| {
        let set_first_run = set_first_run;
        leptos::task::spawn_local(async move {
            if is_first_run.get_untracked() {
                mark_welcomed(set_first_run);
            }
            match tauri_ipc::open_folder_dialog().await {
                Ok(Some(folder)) => {
                    current_folder.set(Some(folder.clone()));
                    match tauri_ipc::list_directory(&folder).await {
                        Ok(tree) => file_tree.set(tree),
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("list_directory failed: {e}").into(),
                            );
                        }
                    }
                    if let Err(e) = tauri_ipc::watch_folder(&folder).await {
                        web_sys::console::error_1(
                            &format!("watch_folder failed: {e}").into(),
                        );
                    }
                }
                Ok(None) => { /* user cancelled */ }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("open_folder_dialog failed: {e}").into(),
                    );
                }
            }
        });
    };

    // Modifier string needs to be owned for the closures below.
    let mod_k = format!("{modifier}K");
    let mod_e = format!("{modifier}E");
    let mod_123 = format!("{modifier}1/2/3");
    let mod_comma = format!("{modifier},");

    view! {
        <div class="empty-state">
            <Show
                when=move || is_first_run.get()
                fallback=|| view! { <h1 class="empty-state-title">"RustyNotes"</h1> }
            >
                <h1 class="empty-state-title">"Welcome to RustyNotes"</h1>
                <p class="empty-state-welcome">
                    "A local-first markdown editor. Your files stay on your machine."
                </p>
            </Show>

            <p class="hint">
                "WYSIWYG editing, LaTeX math, Mermaid diagrams, and syntax-highlighted code."
            </p>

            <button class="empty-state-cta" on:click=open_folder>
                "Open Folder"
            </button>

            <Show when=move || !recent_folders.get().is_empty()>
                <div class="recent-folders">
                    <h2 class="recent-folders-heading">"Recent"</h2>
                    <ul class="recent-folders-list">
                        <For
                            each=move || {
                                recent_folders.get().into_iter().take(5).collect::<Vec<_>>()
                            }
                            key=|folder| folder.clone()
                            children=move |folder| {
                                let folder_for_click = folder.clone();
                                let folder_for_title = folder.clone();
                                let folder_for_path = folder.clone();
                                let folder_for_name = folder.clone();
                                let set_first_run = set_first_run;
                                let handle_recent = move |_| {
                                    let folder = folder_for_click.clone();
                                    leptos::task::spawn_local(async move {
                                        if is_first_run.get_untracked() {
                                            mark_welcomed(set_first_run);
                                        }
                                        current_folder.set(Some(folder.clone()));
                                        match tauri_ipc::list_directory(&folder).await {
                                            Ok(tree) => file_tree.set(tree),
                                            Err(e) => {
                                                web_sys::console::error_1(
                                                    &format!("list_directory failed: {e}").into(),
                                                );
                                            }
                                        }
                                        if let Err(e) = tauri_ipc::watch_folder(&folder).await {
                                            web_sys::console::error_1(
                                                &format!("watch_folder failed: {e}").into(),
                                            );
                                        }
                                    });
                                };
                                let display_name = folder_name(&folder_for_name).to_string();
                                view! {
                                    <li>
                                        <button
                                            class="recent-folder-item"
                                            on:click=handle_recent
                                            title=folder_for_title.clone()
                                        >
                                            <span class="recent-folder-icon" aria-hidden="true">
                                                "\u{2013}"
                                            </span>
                                            <span class="recent-folder-name">
                                                {display_name}
                                            </span>
                                            <span class="recent-folder-path">
                                                {folder_for_path}
                                            </span>
                                        </button>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </div>
            </Show>

            <div class="empty-state-shortcuts">
                <div class="shortcut-row">
                    <kbd>{mod_k.clone()}</kbd>
                    <span>"Search files"</span>
                </div>
                <div class="shortcut-row">
                    <kbd>{mod_e.clone()}</kbd>
                    <span>"Cycle editor mode"</span>
                </div>
                <div class="shortcut-row">
                    <kbd>{mod_123.clone()}</kbd>
                    <span>"Switch navigation"</span>
                </div>
                <Show when=move || is_first_run.get()>
                    <div class="shortcut-row">
                        <kbd>{mod_comma.clone()}</kbd>
                        <span>"Open settings"</span>
                    </div>
                </Show>
            </div>
        </div>
    }
}
