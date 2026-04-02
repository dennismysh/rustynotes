use leptos::prelude::*;
use rustynotes_common::FileNode;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::save;
use crate::state::use_app_state;

// ---------------------------------------------------------------------------
// Filtering helpers
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

/// Recursively filter entries whose name contains the query (case-insensitive).
fn filter_by_query(entries: &[FileNode], query: &str) -> Vec<FileNode> {
    let q = query.to_lowercase();
    entries
        .iter()
        .filter_map(|entry| {
            if entry.is_dir {
                let filtered = entry
                    .children
                    .as_ref()
                    .map(|c| filter_by_query(c, &q))
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
            } else if entry.name.to_lowercase().contains(&q) {
                Some(entry.clone())
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// TreeNode (recursive)
// ---------------------------------------------------------------------------

#[component]
fn TreeNode(entry: FileNode, depth: usize) -> AnyView {
    let state = use_app_state();
    let expanded = RwSignal::new(false);

    let entry_path = entry.path.clone();
    let entry_name = entry.name.clone();
    let entry_is_dir = entry.is_dir;
    let children = entry.children.clone();

    // Clone for closures
    let path_for_click = entry_path.clone();
    let path_for_active = entry_path.clone();

    let state_for_click = state.clone();
    let handle_click = move |_| {
        if entry_is_dir {
            expanded.update(|v| *v = !*v);
        } else {
            save::guard_file_switch(&state_for_click, path_for_click.clone());
        }
    };

    let state_for_keydown = state.clone();
    let handle_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        if key == "Enter" || key == " " {
            ev.prevent_default();
            // Simulate click by dispatching the same logic
            if entry_is_dir {
                expanded.update(|v| *v = !*v);
            } else {
                save::guard_file_switch(&state_for_keydown, entry_path.clone());
            }
        }
        if key == "ArrowRight" && entry_is_dir && !expanded.get_untracked() {
            ev.prevent_default();
            expanded.set(true);
        }
        if key == "ArrowLeft" && entry_is_dir && expanded.get_untracked() {
            ev.prevent_default();
            expanded.set(false);
        }
        if key == "ArrowDown" {
            ev.prevent_default();
            if let Some(target) = ev.current_target() {
                let el: web_sys::HtmlElement = target.unchecked_into();
                if let Some(tree_item) = el.closest("[role='treeitem']").ok().flatten() {
                    if let Some(next) = tree_item.next_element_sibling() {
                        if let Ok(Some(focusable)) =
                            next.query_selector("[role='treeitem'] > [tabindex]")
                        {
                            let _ = focusable.unchecked_into::<web_sys::HtmlElement>().focus();
                        }
                    } else if let Some(parent) = el.parent_element() {
                        if let Ok(Some(focusable)) =
                            parent.query_selector(".tree-children [tabindex]")
                        {
                            let _ = focusable.unchecked_into::<web_sys::HtmlElement>().focus();
                        }
                    }
                }
            }
        }
        if key == "ArrowUp" {
            ev.prevent_default();
            if let Some(target) = ev.current_target() {
                let el: web_sys::HtmlElement = target.unchecked_into();
                if let Some(container) = el
                    .closest("[role='tree'], .tree-children")
                    .ok()
                    .flatten()
                {
                    if let Ok(items) = container.query_selector_all("[tabindex='0']") {
                        let el_node: &web_sys::Node = el.as_ref();
                        let mut found_idx = None;
                        for i in 0..items.length() {
                            if let Some(item) = items.item(i) {
                                if item == *el_node {
                                    found_idx = Some(i);
                                    break;
                                }
                            }
                        }
                        if let Some(idx) = found_idx {
                            if idx > 0 {
                                if let Some(prev) = items.item(idx - 1) {
                                    let _ = prev
                                        .unchecked_into::<web_sys::HtmlElement>()
                                        .focus();
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    // Cap indentation at depth 6 to prevent deep trees from pushing names off-screen
    let capped_depth = depth.min(6);
    let padding_left = format!("{}px", 12 + capped_depth * 14);

    let icon = move || {
        if entry_is_dir {
            if expanded.get() {
                "\u{25BE}" // down triangle
            } else {
                "\u{25B8}" // right triangle
            }
        } else {
            "\u{2013}" // en-dash
        }
    };

    let aria_label = format!(
        "{}: {}",
        if entry_is_dir { "Folder" } else { "File" },
        entry_name
    );

    let aria_label_clone = aria_label.clone();

    view! {
        <div
            role="treeitem"
            aria-expanded=move || if entry_is_dir { Some(expanded.get().to_string()) } else { None }
        >
            <div
                class="tree-item"
                class:active=move || state.active_file_path.get().as_deref() == Some(&path_for_active)
                style:padding-left=padding_left
                on:click=handle_click
                on:keydown=handle_keydown
                tabindex=0
                role="button"
                aria-label=aria_label_clone
            >
                <span class="icon" aria-hidden="true">{icon}</span>
                {
                    let title = entry_name.clone();
                    view! { <span class="name" title=title>{entry_name}</span> }
                }
            </div>
            <Show when=move || entry_is_dir && expanded.get()>
                {
                    let children_inner = children.clone();
                    view! {
                        <div class="tree-children" role="group">
                            <For
                                each=move || children_inner.clone().unwrap_or_default()
                                key=|item| item.path.clone()
                                children=move |item| {
                                    view! { <TreeNode entry=item depth=depth+1 /> }
                                }
                            />
                        </div>
                    }
                }
            </Show>
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Sidebar (public component)
// ---------------------------------------------------------------------------

#[component]
pub fn Sidebar() -> impl IntoView {
    let state = use_app_state();

    let filtered_tree = Memo::new(move |_| {
        let tree = filter_md_entries(&state.file_tree.get());
        let q = state.search_query.get();
        if q.is_empty() {
            tree
        } else {
            filter_by_query(&tree, &q)
        }
    });

    view! {
        <div class="sidebar">
            <Show when=move || state.show_search.get()>
                <div class="sidebar-search">
                    <input
                        type="text"
                        placeholder="Filter files..."
                        prop:value=move || state.search_query.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            state.search_query.set(val);
                        }
                        autofocus=true
                    />
                </div>
            </Show>
            <div role="tree" aria-label="File tree">
                <Show
                    when=move || state.current_folder.get().is_some()
                    fallback=|| view! {
                        <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
                            "Open a folder to browse files"
                        </div>
                    }
                >
                    <Show
                        when=move || !filtered_tree.get().is_empty()
                        fallback=move || {
                            let msg = if state.search_query.get().is_empty() {
                                "No .md files in this folder"
                            } else {
                                "No files match your search"
                            };
                            view! {
                                <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
                                    {msg}
                                </div>
                            }
                        }
                    >
                        <For
                            each=move || filtered_tree.get()
                            key=|item| item.path.clone()
                            children=move |item| {
                                view! { <TreeNode entry=item depth=0 /> }
                            }
                        />
                    </Show>
                </Show>
            </div>
        </div>
    }
}
