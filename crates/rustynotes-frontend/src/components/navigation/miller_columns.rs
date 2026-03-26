use leptos::prelude::*;
use rustynotes_common::FileNode;
use web_sys::KeyboardEvent;

use wasm_bindgen::prelude::*;

use crate::state::use_app_state;
use crate::tauri_ipc;

/// Schedule a callback on the next animation frame.
fn request_animation_frame(f: impl FnOnce() + 'static) {
    let closure = Closure::once_into_js(f);
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(closure.unchecked_ref());
    }
}

// ---------------------------------------------------------------------------
// Filtering helper
// ---------------------------------------------------------------------------

/// Keep only `.md` files and directories that contain at least one `.md` file.
fn filter_md_entries(entries: &[FileNode]) -> Vec<FileNode> {
    entries
        .iter()
        .filter_map(|entry| {
            if entry.is_dir {
                let filtered = entry
                    .children
                    .as_ref()
                    .map(|c| filter_md_entries(c))
                    .unwrap_or_default();
                if filtered.is_empty() {
                    None
                } else {
                    Some(FileNode {
                        name: entry.name.clone(),
                        path: entry.path.clone(),
                        is_dir: true,
                        children: Some(filtered),
                    })
                }
            } else if entry.name.ends_with(".md") {
                Some(entry.clone())
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// MillerColumns component
// ---------------------------------------------------------------------------

#[component]
pub fn MillerColumns() -> impl IntoView {
    let state = use_app_state();

    let columns: RwSignal<Vec<Vec<FileNode>>> = RwSignal::new(Vec::new());
    let selected_paths: RwSignal<Vec<Option<String>>> = RwSignal::new(Vec::new());

    let md_tree = Memo::new(move |_| filter_md_entries(&state.file_tree.get()));

    // Re-initialize columns when the file tree changes
    Effect::new(move || {
        let tree = md_tree.get();
        if !tree.is_empty() {
            columns.set(vec![tree]);
            selected_paths.set(vec![None]);
        } else {
            columns.set(Vec::new());
            selected_paths.set(Vec::new());
        }
    });

    let handle_click = move |entry: FileNode, col_index: usize| {
        if entry.is_dir {
            // Keep columns 0..=col_index, add children as next column
            columns.update(|cols| {
                cols.truncate(col_index + 1);
                if let Some(ref children) = entry.children {
                    if !children.is_empty() {
                        cols.push(children.clone());
                    }
                }
            });
            selected_paths.update(|sp| {
                sp.truncate(col_index + 1);
                sp[col_index] = Some(entry.path.clone());
                if entry
                    .children
                    .as_ref()
                    .map(|c| !c.is_empty())
                    .unwrap_or(false)
                {
                    sp.push(None);
                }
            });
        } else {
            // Select this file, trim columns after
            selected_paths.update(|sp| {
                sp.truncate(col_index + 1);
                sp[col_index] = Some(entry.path.clone());
            });
            columns.update(|cols| {
                cols.truncate(col_index + 1);
            });

            // Open the file
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

    let handle_item_keydown =
        move |ev: KeyboardEvent, entry: FileNode, col_index: usize| {
            let key = ev.key();

            if key == "Enter" || key == " " {
                ev.prevent_default();
                handle_click(entry, col_index);
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
            if key == "ArrowRight" && entry.is_dir {
                ev.prevent_default();
                handle_click(entry, col_index);
                // Focus first item in next column after render
                let next_col = col_index + 1;
                // Use request_animation_frame to wait for the DOM update
                request_animation_frame(move || {
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        if let Ok(cols) = document.query_selector_all(".miller-column") {
                            if let Some(next_col_el) = cols.item(next_col as u32) {
                                if let Ok(Some(first_item)) = next_col_el
                                    .unchecked_into::<web_sys::Element>()
                                    .query_selector("[tabindex='0']")
                                {
                                    let _ = first_item
                                        .unchecked_into::<web_sys::HtmlElement>()
                                        .focus();
                                }
                            }
                        }
                    }
                });
            }
            if key == "ArrowLeft" && col_index > 0 {
                ev.prevent_default();
                let prev_col = col_index - 1;
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(cols) = document.query_selector_all(".miller-column") {
                        if let Some(prev_col_el) = cols.item(prev_col as u32) {
                            let prev_el: web_sys::Element = prev_col_el.unchecked_into();
                            let active_item = prev_el
                                .query_selector(".miller-item.active")
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    prev_el.query_selector("[tabindex='0']").ok().flatten()
                                });
                            if let Some(item) = active_item {
                                let _ =
                                    item.unchecked_into::<web_sys::HtmlElement>().focus();
                            }
                        }
                    }
                }
            }
        };

    view! {
        <div class="sidebar">
            <Show
                when=move || state.current_folder.get().is_some()
                fallback=|| view! {
                    <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
                        "No folder open"
                    </div>
                }
            >
                <div class="miller-columns" role="group" aria-label="Miller column file browser">
                    <For
                        each=move || {
                            columns.get().into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|(i, _)| *i
                        children=move |(col_index, column)| {
                            let col_label = format!("Column {}", col_index + 1);
                            view! {
                                <div class="miller-column" role="listbox" aria-label=col_label>
                                    <For
                                        each=move || column.clone()
                                        key=|item| item.path.clone()
                                        children=move |entry| {
                                            let entry_for_click = entry.clone();
                                            let entry_for_key = entry.clone();
                                            let entry_path = entry.path.clone();
                                            let entry_name = entry.name.clone();
                                            let entry_is_dir = entry.is_dir;

                                            let entry_path_for_active = entry_path.clone();
                                            let is_active = Memo::new(move |_| {
                                                let sp = selected_paths.get();
                                                let selected_match = sp
                                                    .get(col_index)
                                                    .and_then(|s| s.as_deref())
                                                    == Some(entry_path_for_active.as_str());
                                                let active_match = state
                                                    .active_file_path
                                                    .get()
                                                    .as_deref()
                                                    == Some(entry_path_for_active.as_str());
                                                selected_match || active_match
                                            });

                                            let aria_label = format!(
                                                "{}: {}",
                                                if entry_is_dir { "Folder" } else { "File" },
                                                entry_name
                                            );

                                            view! {
                                                <div
                                                    class="miller-item"
                                                    class:active=move || is_active.get()
                                                    on:click=move |_| {
                                                        handle_click(entry_for_click.clone(), col_index);
                                                    }
                                                    on:keydown=move |ev| {
                                                        handle_item_keydown(
                                                            ev,
                                                            entry_for_key.clone(),
                                                            col_index,
                                                        );
                                                    }
                                                    tabindex=0
                                                    role="option"
                                                    aria-selected=move || is_active.get().to_string()
                                                    aria-label=aria_label
                                                >
                                                    <span>{entry_name}</span>
                                                    <Show when=move || entry_is_dir>
                                                        <span class="chevron" aria-hidden="true">
                                                            "\u{25B8}"
                                                        </span>
                                                    </Show>
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}
