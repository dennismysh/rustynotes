use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::bridge;
use crate::state::use_app_state;

/// WYSIWYG editor component that mounts a TipTap instance via the JS bridge.
///
/// Lifecycle mirrors SourceEditor but uses the TipTap bridge functions.
#[component]
pub fn WysiwygEditor() -> impl IntoView {
    let state = use_app_state();
    let content_signal = state.active_file_content;
    let dirty_signal = state.is_dirty;
    let rendered_html_signal = state.rendered_html;
    let suppress_dirty = state.suppress_dirty;

    let container = NodeRef::<leptos::html::Div>::new();

    // JsValue is Send+Sync, so we can use regular StoredValue
    let handle: StoredValue<Option<JsValue>> = StoredValue::new(None);
    // Closure<dyn Fn(String)> is not Send+Sync, use LocalStorage
    let closure_store: StoredValue<Option<Closure<dyn Fn(String)>>, LocalStorage> =
        StoredValue::new_local(None);

    // Mount TipTap when the container element is available
    Effect::new(move |_| {
        if let Some(el) = container.get() {
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Update rendered HTML for preview sync via backend IPC
                let content_for_render = new_content;
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(html) = crate::tauri_ipc::parse_markdown(&content_for_render).await {
                        rendered_html_signal.set(html);
                    }
                });
            }) as Box<dyn Fn(String)>);

            let options = js_sys::Object::new();

            let el_html: &web_sys::HtmlElement = el.unchecked_ref();
            let h = bridge::mount_tiptap(
                el_html,
                &content_signal.get_untracked(),
                &options.into(),
                &cb,
            );

            if !h.is_null() {
                handle.set_value(Some(h));
            }
            closure_store.set_value(Some(cb));
        }
    });

    // Sync content changes from external sources (e.g., file load) into TipTap
    Effect::new(move |_| {
        let content = content_signal.get();
        handle.with_value(|h| {
            if let Some(h) = h {
                bridge::update_tiptap(h, &content);
            }
        });
    });

    on_cleanup(move || {
        handle.with_value(|h| {
            if let Some(h) = h {
                bridge::destroy_tiptap(h);
            }
        });
        handle.set_value(None);
        // Drop the closure to free the prevent-GC reference
        closure_store.set_value(None);
    });

    view! { <div node_ref=container class="editor-container wysiwyg-editor markdown-content" /> }
}
