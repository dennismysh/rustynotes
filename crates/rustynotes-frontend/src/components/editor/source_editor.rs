use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::bridge;
use crate::state::use_app_state;

/// Source editor component that mounts a CodeMirror instance via the JS bridge.
///
/// Lifecycle:
/// 1. Creates a `NodeRef<Div>` for the mount point
/// 2. When the element is available, creates an onChange closure and mounts CodeMirror
/// 3. Stores both the editor handle and closure to prevent GC
/// 4. On cleanup, destroys the editor and drops the closure
#[component]
pub fn SourceEditor() -> impl IntoView {
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

    // Mount CodeMirror when the container element is available
    Effect::new(move |_| {
        if let Some(el) = container.get() {
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                // Skip if content matches what we already have (programmatic update).
                if new_content == content_signal.get_untracked() {
                    return;
                }
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Also update rendered HTML for preview sync
                let html = crate::components::preview::markdown::render_markdown(&new_content);
                rendered_html_signal.set(html);
            }) as Box<dyn Fn(String)>);

            let options = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&options, &"theme".into(), &"dark".into());
            let _ = js_sys::Reflect::set(&options, &"lineNumbers".into(), &true.into());

            let el_html: &web_sys::HtmlElement = el.unchecked_ref();
            let h = bridge::mount_code_mirror(
                el_html,
                &content_signal.get_untracked(),
                &options.into(),
                &cb,
            );

            handle.set_value(Some(h));
            closure_store.set_value(Some(cb));
        }
    });

    // Sync content changes from external sources (e.g., file load) into CodeMirror
    Effect::new(move |_| {
        let content = content_signal.get();
        handle.with_value(|h| {
            if let Some(h) = h {
                bridge::update_code_mirror(h, &content);
            }
        });
    });

    on_cleanup(move || {
        handle.with_value(|h| {
            if let Some(h) = h {
                bridge::destroy_code_mirror(h);
            }
        });
        handle.set_value(None);
        // Drop the closure to free the prevent-GC reference
        closure_store.set_value(None);
    });

    view! { <div node_ref=container class="editor-container source-editor" /> }
}
