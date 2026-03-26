use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn get_bridge() -> js_sys::Object {
    let window = web_sys::window().unwrap();
    js_sys::Reflect::get(&window, &"RustyNotesBridge".into())
        .unwrap()
        .unchecked_into()
}

fn call_bridge(method: &str, args: &[&JsValue]) -> Result<JsValue, JsValue> {
    let bridge = get_bridge();
    let func: js_sys::Function =
        js_sys::Reflect::get(&bridge, &method.into())?.unchecked_into();
    match args.len() {
        0 => func.call0(&bridge),
        1 => func.call1(&bridge, args[0]),
        2 => func.call2(&bridge, args[0], args[1]),
        3 => func.call3(&bridge, args[0], args[1], args[2]),
        _ => {
            let js_args = js_sys::Array::new();
            for arg in args {
                js_args.push(arg);
            }
            func.apply(&bridge, &js_args)
        }
    }
}

/// Mount a CodeMirror editor into the given element.
/// Returns an opaque JS handle used by other CodeMirror functions.
pub fn mount_code_mirror(
    el: &web_sys::HtmlElement,
    content: &str,
    options: &JsValue,
    on_change: &Closure<dyn Fn(String)>,
) -> JsValue {
    let content_val: JsValue = content.into();
    let cb: JsValue = on_change.as_ref().clone();
    call_bridge(
        "mountCodeMirror",
        &[&el.into(), &content_val, options, &cb],
    )
    .unwrap()
}

/// Replace the document content in a CodeMirror instance.
pub fn update_code_mirror(handle: &JsValue, content: &str) {
    let content_val: JsValue = content.into();
    let _ = call_bridge("updateCodeMirror", &[handle, &content_val]);
}

/// Focus the CodeMirror editor.
pub fn focus_code_mirror(handle: &JsValue) {
    let _ = call_bridge("focusCodeMirror", &[handle]);
}

/// Destroy the CodeMirror editor and free resources.
pub fn destroy_code_mirror(handle: &JsValue) {
    let _ = call_bridge("destroyCodeMirror", &[handle]);
}

/// Mount a TipTap rich-text editor into the given element.
/// Returns an opaque JS handle used by other TipTap functions.
pub fn mount_tiptap(
    el: &web_sys::HtmlElement,
    content: &str,
    options: &JsValue,
    on_change: &Closure<dyn Fn(String)>,
) -> JsValue {
    let content_val: JsValue = content.into();
    let cb: JsValue = on_change.as_ref().clone();
    call_bridge("mountTipTap", &[&el.into(), &content_val, options, &cb]).unwrap()
}

/// Replace the document content in a TipTap instance.
pub fn update_tiptap(handle: &JsValue, content: &str) {
    let content_val: JsValue = content.into();
    let _ = call_bridge("updateTipTap", &[handle, &content_val]);
}

/// Get the current markdown string from TipTap.
pub fn get_tiptap_markdown(handle: &JsValue) -> String {
    call_bridge("getTipTapMarkdown", &[handle])
        .unwrap()
        .as_string()
        .unwrap_or_default()
}

/// Focus the TipTap editor.
pub fn focus_tiptap(handle: &JsValue) {
    let _ = call_bridge("focusTipTap", &[handle]);
}

/// Destroy the TipTap editor and free resources.
pub fn destroy_tiptap(handle: &JsValue) {
    let _ = call_bridge("destroyTipTap", &[handle]);
}

/// Render a LaTeX expression into the given element using KaTeX.
/// Lazily loads the KaTeX module on first call.
pub async fn render_katex(
    el: &web_sys::HtmlElement,
    latex: &str,
    display_mode: bool,
) {
    let latex_val: JsValue = latex.into();
    let display_val: JsValue = display_mode.into();
    let result = call_bridge("renderKatex", &[&el.into(), &latex_val, &display_val]).unwrap();
    // The JS function returns a Promise — await it
    if result.has_type::<js_sys::Promise>() {
        let _ = JsFuture::from(js_sys::Promise::from(result)).await;
    }
}

/// Render a Mermaid diagram into the given element.
/// Lazily loads the Mermaid module on first call.
pub async fn render_mermaid(
    el: &web_sys::HtmlElement,
    code: &str,
    theme: &str,
) {
    let code_val: JsValue = code.into();
    let theme_val: JsValue = theme.into();
    let result = call_bridge("renderMermaid", &[&el.into(), &code_val, &theme_val]).unwrap();
    // The JS function returns a Promise — await it
    if result.has_type::<js_sys::Promise>() {
        let _ = JsFuture::from(js_sys::Promise::from(result)).await;
    }
}
