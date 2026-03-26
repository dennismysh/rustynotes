use leptos::prelude::*;
use rustynotes_common::FileNode;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

use crate::state::use_app_state;
use crate::tauri_ipc;

// ---------------------------------------------------------------------------
// Breadcrumb component
// ---------------------------------------------------------------------------

#[component]
pub fn Breadcrumb() -> impl IntoView {
    let state = use_app_state();

    let dropdown_items: RwSignal<Vec<FileNode>> = RwSignal::new(Vec::new());
    let dropdown_index: RwSignal<Option<i32>> = RwSignal::new(None);

    // Derive path segments from active_file_path relative to current_folder
    let path_segments = Memo::new(move |_| {
        let folder = state.current_folder.get();
        let file_path = state.active_file_path.get();
        match (folder, file_path) {
            (Some(folder), Some(file_path)) => {
                let relative = if file_path.starts_with(&folder) {
                    file_path[folder.len()..].trim_start_matches('/').to_string()
                } else {
                    file_path.clone()
                };
                relative
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        }
    });

    let close_dropdown = move || {
        dropdown_index.set(None);
        dropdown_items.set(Vec::new());
    };

    // Close dropdown on Escape
    let close_dropdown_for_esc = close_dropdown.clone();
    let handle_global_keydown = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
        move |ev: web_sys::Event| {
            if let Ok(kev) = ev.dyn_into::<KeyboardEvent>() {
                if kev.key() == "Escape" && dropdown_index.get_untracked().is_some() {
                    kev.prevent_default();
                    close_dropdown_for_esc();
                }
            }
        },
    );
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ = document.add_event_listener_with_callback(
            "keydown",
            handle_global_keydown.as_ref().unchecked_ref(),
        );
    }
    // Leak the closure since this is a component-lifetime listener.
    // In practice the navigation component lives for the duration of the app.
    handle_global_keydown.forget();

    let handle_segment_click = move |segment_index: usize| {
        let folder = state.current_folder.get_untracked();
        let Some(folder) = folder else { return };

        let segments = path_segments.get_untracked();
        let parent_path = if segment_index == 0 {
            folder.clone()
        } else {
            format!(
                "{}/{}",
                folder,
                segments[..segment_index].join("/")
            )
        };

        let seg_idx = segment_index as i32;
        leptos::task::spawn_local(async move {
            match tauri_ipc::list_directory(&parent_path).await {
                Ok(entries) => {
                    dropdown_items.set(entries);
                    dropdown_index.set(Some(seg_idx));
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to list directory for breadcrumb: {e}").into(),
                    );
                }
            }
        });
    };

    let handle_root_click = move |_| {
        let folder = state.current_folder.get_untracked();
        let Some(folder) = folder else { return };

        leptos::task::spawn_local(async move {
            match tauri_ipc::list_directory(&folder).await {
                Ok(entries) => {
                    dropdown_items.set(entries);
                    dropdown_index.set(Some(-1));
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to list root directory: {e}").into(),
                    );
                }
            }
        });
    };

    let close_dropdown_for_item = close_dropdown.clone();
    let handle_dropdown_item_click = move |entry: FileNode| {
        close_dropdown_for_item();

        if entry.is_dir {
            let path = entry.path.clone();
            let cur_idx = dropdown_index.get_untracked();
            leptos::task::spawn_local(async move {
                match tauri_ipc::list_directory(&path).await {
                    Ok(entries) => {
                        dropdown_items.set(entries);
                        dropdown_index.set(cur_idx);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to list directory: {e}").into(),
                        );
                    }
                }
            });
        } else {
            let path = entry.path.clone();
            state.active_file_path.set(Some(path.clone()));
            leptos::task::spawn_local(async move {
                match tauri_ipc::read_file(&path).await {
                    Ok(content) => {
                        state.active_file_content.set(content);
                        state.is_dirty.set(false);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to read file: {e}").into(),
                        );
                    }
                }
            });
        }
    };

    let handle_dropdown_keydown = move |ev: KeyboardEvent, entry: FileNode| {
        let key = ev.key();
        if key == "Enter" || key == " " {
            ev.prevent_default();
            handle_dropdown_item_click(entry);
            return;
        }
        if key == "ArrowDown" {
            ev.prevent_default();
            if let Some(target) = ev.current_target() {
                let el: web_sys::HtmlElement = target.unchecked_into();
                if let Some(next) = el.next_element_sibling() {
                    let _ = next.unchecked_into::<web_sys::HtmlElement>().focus();
                }
            }
        }
        if key == "ArrowUp" {
            ev.prevent_default();
            if let Some(target) = ev.current_target() {
                let el: web_sys::HtmlElement = target.unchecked_into();
                if let Some(prev) = el.previous_element_sibling() {
                    let _ = prev.unchecked_into::<web_sys::HtmlElement>().focus();
                }
            }
        }
    };

    let close_dropdown_for_overlay = close_dropdown.clone();

    view! {
        <nav class="breadcrumb-bar" aria-label="File path">
            <Show when=move || state.current_folder.get().is_some()>
                {
                    let root_name = move || {
                        state.current_folder.get().map(|f| {
                            f.split('/').last().unwrap_or(&f).to_string()
                        }).unwrap_or_default()
                    };
                    let root_label = move || {
                        format!("Root folder: {}", root_name())
                    };
                    view! {
                        <button
                            class="breadcrumb-root"
                            on:click=handle_root_click
                            aria-label=root_label
                        >
                            {root_name}
                        </button>

                        <For
                            each=move || {
                                path_segments.get().into_iter().enumerate().collect::<Vec<_>>()
                            }
                            key=|(i, s)| format!("{}-{}", i, s)
                            children=move |(index, segment)| {
                                let segment_display = segment.clone();
                                let is_last = move || index == path_segments.get().len() - 1;
                                view! {
                                    <span class="breadcrumb-separator" aria-hidden="true">"/"</span>
                                    <button
                                        class="breadcrumb-segment"
                                        class:active=is_last
                                        on:click=move |_| handle_segment_click(index)
                                        aria-current=move || if is_last() { Some("page") } else { None }
                                    >
                                        {segment_display}
                                    </button>
                                }
                            }
                        />
                    }
                }
            </Show>

            <Show when=move || state.current_folder.get().is_none()>
                <span style="color: var(--text-muted); font-size: 13px;">
                    "No folder open"
                </span>
            </Show>

            <Show when=move || dropdown_index.get().is_some()>
                <div
                    class="breadcrumb-dropdown-overlay"
                    on:click=move |_| close_dropdown_for_overlay()
                />
                <div class="breadcrumb-dropdown" role="listbox" aria-label="Directory contents">
                    <For
                        each=move || dropdown_items.get()
                        key=|item| item.path.clone()
                        children=move |entry| {
                            let entry_for_click = entry.clone();
                            let entry_for_key = entry.clone();
                            let entry_name = entry.name.clone();
                            let entry_is_dir = entry.is_dir;

                            let aria_label = format!(
                                "{}: {}",
                                if entry_is_dir { "Folder" } else { "File" },
                                entry_name
                            );

                            view! {
                                <div
                                    class="breadcrumb-dropdown-item"
                                    on:click=move |_| {
                                        handle_dropdown_item_click(entry_for_click.clone());
                                    }
                                    on:keydown=move |ev| {
                                        handle_dropdown_keydown(ev, entry_for_key.clone());
                                    }
                                    tabindex=0
                                    role="option"
                                    aria-label=aria_label
                                >
                                    <span class="icon" aria-hidden="true">
                                        {if entry_is_dir { "\u{25B8}" } else { "\u{2013}" }}
                                    </span>
                                    <span>{entry_name}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </nav>
    }
}
