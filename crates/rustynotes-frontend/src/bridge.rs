use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn get_bridge() -> Option<js_sys::Object> {
    let window = web_sys::window()?;
    let val = js_sys::Reflect::get(&window, &"RustyNotesBridge".into()).ok()?;
    if val.is_undefined() || val.is_null() {
        web_sys::console::error_1(&"RustyNotesBridge not found on window".into());
        None
    } else {
        Some(val.unchecked_into())
    }
}

fn call_bridge(method: &str, args: &[&JsValue]) -> Result<JsValue, JsValue> {
    let bridge = get_bridge().ok_or_else(|| JsValue::from_str("RustyNotesBridge not available"))?;
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
    match call_bridge("mountCodeMirror", &[&el.into(), &content_val, options, &cb]) {
        Ok(handle) => handle,
        Err(e) => {
            web_sys::console::error_1(&format!("mountCodeMirror failed: {e:?}").into());
            JsValue::NULL
        }
    }
}

/// Replace the document content in a CodeMirror instance.
pub fn update_code_mirror(handle: &JsValue, content: &str) {
    let content_val: JsValue = content.into();
    let _ = call_bridge("updateCodeMirror", &[handle, &content_val]);
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
    match call_bridge("mountTipTap", &[&el.into(), &content_val, options, &cb]) {
        Ok(handle) => handle,
        Err(e) => {
            web_sys::console::error_1(&format!("mountTipTap failed: {e:?}").into());
            JsValue::NULL
        }
    }
}

/// Replace the document content in a TipTap instance.
pub fn update_tiptap(handle: &JsValue, content: &str) {
    let content_val: JsValue = content.into();
    let _ = call_bridge("updateTipTap", &[handle, &content_val]);
}

/// Destroy the TipTap editor and free resources.
pub fn destroy_tiptap(handle: &JsValue) {
    let _ = call_bridge("destroyTipTap", &[handle]);
}

