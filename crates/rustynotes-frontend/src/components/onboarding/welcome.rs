use leptos::prelude::*;

use crate::state::use_app_state;
use crate::tauri_ipc;
use crate::state::AppState;

use super::{create_is_first_run, mark_welcomed};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the last path segment as a folder name.
fn folder_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Extract filename and parent directory from a file path.
fn file_name_and_parent(path: &str) -> (String, String) {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    let parent = if let Some(pos) = path.rfind('/') {
        path[..pos].to_string()
    } else {
        String::new()
    };
    (name, parent)
}

/// Detect macOS via `navigator.platform`.
fn is_mac() -> bool {
    js_sys::eval("navigator.platform.includes('Mac')")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// RecentSection sub-component
// ---------------------------------------------------------------------------

/// Renders the "Recent" section with folders and files sub-lists.
/// Extracted as a separate component so each list's closures get their own
/// scope, avoiding `FnOnce` issues from non-Copy state captures in `<Show>`.
#[component]
fn RecentSection(
    state: AppState,
    is_first_run: ReadSignal<bool>,
    set_first_run: WriteSignal<bool>,
    recent_folders: Memo<Vec<String>>,
    recent_files: Memo<Vec<String>>,
) -> impl IntoView {
    let state_for_folders = state.clone();

    view! {
        <div class="recent-folders">
            <h2 class="recent-folders-heading">"Recent"</h2>

            <Show when=move || !recent_folders.get().is_empty()>
                <p class="recent-subheading">"Folders"</p>
                <ul class="recent-folders-list">
                    {
                        let state_for_list = state_for_folders.clone();
                        view! {
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
                                    let state_for_recent = state_for_list.clone();
                                    let handle_recent = move |_| {
                                        let folder = folder_for_click.clone();
                                        let state = state_for_recent.clone();
                                        leptos::task::spawn_local(async move {
                                            if is_first_run.get_untracked() {
                                                mark_welcomed(set_first_run);
                                            }
                                            crate::save::open_folder(&state, folder).await;
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
                        }
                    }
                </ul>
            </Show>

            <Show when=move || !recent_files.get().is_empty()>
                <p class="recent-subheading">"Files"</p>
                <ul class="recent-folders-list">
                    <For
                        each=move || {
                            recent_files.get().into_iter().take(5).collect::<Vec<_>>()
                        }
                        key=|file| file.clone()
                        children=move |file| {
                            let file_for_click = file.clone();
                            let file_for_title = file.clone();
                            let (display_name, parent_path) = file_name_and_parent(&file);
                            let handle_file_click = move |_| {
                                let path = file_for_click.clone();
                                leptos::task::spawn_local(async move {
                                    if let Err(e) =
                                        tauri_ipc::open_file_in_new_window(&path).await
                                    {
                                        web_sys::console::error_1(
                                            &format!(
                                                "open_file_in_new_window failed: {e}"
                                            )
                                            .into(),
                                        );
                                    }
                                });
                            };
                            view! {
                                <li>
                                    <button
                                        class="recent-folder-item"
                                        on:click=handle_file_click
                                        title=file_for_title.clone()
                                    >
                                        <span class="recent-folder-icon" aria-hidden="true">
                                            "\u{2013}"
                                        </span>
                                        <span class="recent-folder-name">
                                            {display_name}
                                        </span>
                                        <span class="recent-folder-path">
                                            {parent_path}
                                        </span>
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
            </Show>
        </div>
    }
}

// ---------------------------------------------------------------------------
// WelcomeEmptyState component
// ---------------------------------------------------------------------------

/// Shown when no folder is open. Displays a welcome message, recent folders,
/// recent files, and keyboard shortcut hints.
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

    // Derived: recent files from app config.
    let recent_files = Memo::new(move |_| {
        state
            .app_config
            .get()
            .map(|c| c.recent_files)
            .unwrap_or_default()
    });

    // ---- handlers ----

    let state_for_open = state.clone();
    let open_folder = move |_| {
        let set_first_run = set_first_run;
        let state = state_for_open.clone();
        leptos::task::spawn_local(async move {
            if is_first_run.get_untracked() {
                mark_welcomed(set_first_run);
            }
            match tauri_ipc::open_folder_dialog().await {
                Ok(Some(folder)) => {
                    crate::save::open_folder(&state, folder).await;
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

    let open_file = move |_| {
        let set_first_run = set_first_run;
        leptos::task::spawn_local(async move {
            if is_first_run.get_untracked() {
                mark_welcomed(set_first_run);
            }
            if let Err(e) = tauri_ipc::open_file_dialog().await {
                web_sys::console::error_1(
                    &format!("open_file_dialog failed: {e}").into(),
                );
            }
        });
    };

    // Modifier string needs to be owned for the closures below.
    let mod_k = format!("{modifier}K");
    let mod_e = format!("{modifier}E");
    let mod_123 = format!("{modifier}1/2/3");
    let mod_comma = format!("{modifier},");

    let state_for_recent = state.clone();

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

            <div class="empty-state-actions">
                <button class="empty-state-cta" on:click=open_folder>
                    "Open Folder"
                </button>
                <button class="empty-state-cta secondary" on:click=open_file>
                    "Open File"
                </button>
            </div>

            <Show when=move || !recent_folders.get().is_empty() || !recent_files.get().is_empty()>
                <RecentSection
                    state=state_for_recent.clone()
                    is_first_run=is_first_run
                    set_first_run=set_first_run
                    recent_folders=recent_folders
                    recent_files=recent_files
                />
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
