//! Tauri IPC bindings for the WASM frontend.
//!
//! Uses `js_sys::Reflect` to traverse `window.__TAURI__` and `JsFuture` to
//! await promises. No `async fn` in `wasm_bindgen` extern blocks — everything
//! is done through reflection on the JS runtime objects.

use rustynotes_common::{AppConfig, FileNode, SearchResult};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Get a JS property from an object, returning a human-readable error.
fn reflect_get(target: &JsValue, key: &str) -> Result<JsValue, String> {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .map_err(|e| format!("Reflect::get('{key}'): {e:?}"))
}

/// Call `window.__TAURI__.core.invoke(cmd, args)` and await the returned
/// `Promise`.
async fn tauri_invoke(cmd: &str, args: &impl Serialize) -> Result<JsValue, String> {
    let window = web_sys::window().ok_or("no global `window`")?;
    let tauri = reflect_get(&window, "__TAURI__")?;
    let core = reflect_get(&tauri, "core")?;
    let invoke_fn = js_sys::Function::from(reflect_get(&core, "invoke")?);
    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let promise = invoke_fn
        .call2(&core, &JsValue::from_str(cmd), &args_js)
        .map_err(|e| format!("invoke('{cmd}'): {e:?}"))?;
    JsFuture::from(js_sys::Promise::from(promise))
        .await
        .map_err(|e| format!("invoke('{cmd}') rejected: {e:?}"))
}

/// Shorthand for `tauri_invoke` with no arguments.
async fn tauri_invoke_no_args(cmd: &str) -> Result<JsValue, String> {
    #[derive(Serialize)]
    struct Empty {}
    tauri_invoke(cmd, &Empty {}).await
}

// ---------------------------------------------------------------------------
// File system commands
// ---------------------------------------------------------------------------

pub async fn read_file(path: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    let val = tauri_invoke("read_file", &Args { path }).await?;
    val.as_string()
        .ok_or_else(|| "read_file: expected string result".to_string())
}

pub async fn write_file(path: &str, content: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        content: &'a str,
    }
    tauri_invoke("write_file", &Args { path, content }).await?;
    Ok(())
}

pub async fn list_directory(path: &str) -> Result<Vec<FileNode>, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    let val = tauri_invoke("list_directory", &Args { path }).await?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("list_directory deser: {e}"))
}

pub async fn resolve_wikilink(root: &str, name: &str) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        root: &'a str,
        name: &'a str,
    }
    let val = tauri_invoke("resolve_wikilink", &Args { root, name }).await?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}

pub async fn search_files(root: &str, query: &str) -> Result<Vec<SearchResult>, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        root: &'a str,
        query: &'a str,
    }
    let val = tauri_invoke("search_files", &Args { root, query }).await?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("search_files deser: {e}"))
}

pub async fn watch_folder(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    tauri_invoke("watch_folder", &Args { path }).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

pub async fn get_config() -> Result<AppConfig, String> {
    let val = tauri_invoke_no_args("get_config").await?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("get_config deser: {e}"))
}

pub async fn save_config_cmd(config: AppConfig) -> Result<(), String> {
    // The Tauri command parameter is named `config_data` (snake_case),
    // which maps to `configData` on the JS/IPC side.
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        config_data: AppConfig,
    }
    tauri_invoke("save_config_cmd", &Args { config_data: config }).await?;
    Ok(())
}

pub async fn open_settings() -> Result<(), String> {
    tauri_invoke_no_args("open_settings").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Export command
// ---------------------------------------------------------------------------

pub async fn export_file(
    markdown: &str,
    output_path: &str,
    format: &str,
    include_theme: bool,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args<'a> {
        markdown: &'a str,
        output_path: &'a str,
        format: &'a str,
        include_theme: bool,
    }
    tauri_invoke(
        "export_file",
        &Args {
            markdown,
            output_path,
            format,
            include_theme,
        },
    )
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Markdown command
// ---------------------------------------------------------------------------

pub async fn parse_markdown(content: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        content: &'a str,
    }
    let val = tauri_invoke("parse_markdown", &Args { content }).await?;
    val.as_string()
        .ok_or_else(|| "parse_markdown: expected string result".to_string())
}

// ---------------------------------------------------------------------------
// Dialog functions (plugin namespace: __TAURI__.dialog)
// ---------------------------------------------------------------------------

/// Call a function on the `__TAURI__.dialog` plugin namespace and await its
/// `Promise`.
async fn dialog_call(method: &str, options: &JsValue) -> Result<JsValue, String> {
    let window = web_sys::window().ok_or("no global `window`")?;
    let tauri = reflect_get(&window, "__TAURI__")?;
    let dialog = reflect_get(&tauri, "dialog")?;
    let func = js_sys::Function::from(reflect_get(&dialog, method)?);
    let promise = func
        .call1(&dialog, options)
        .map_err(|e| format!("dialog.{method}: {e:?}"))?;
    JsFuture::from(js_sys::Promise::from(promise))
        .await
        .map_err(|e| format!("dialog.{method} rejected: {e:?}"))
}

pub async fn open_folder_dialog() -> Result<Option<String>, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"directory".into(), &JsValue::TRUE)
        .map_err(|e| format!("set directory: {e:?}"))?;
    js_sys::Reflect::set(&opts, &"multiple".into(), &JsValue::FALSE)
        .map_err(|e| format!("set multiple: {e:?}"))?;

    let val = dialog_call("open", &opts.into()).await?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}

pub async fn save_file_dialog(default_name: &str) -> Result<Option<String>, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"defaultPath".into(), &JsValue::from_str(default_name))
        .map_err(|e| format!("set defaultPath: {e:?}"))?;

    let val = dialog_call("save", &opts.into()).await?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

/// Show the current webview window via `__TAURI__.window.getCurrentWindow().show()`.
pub fn show_current_window() {
    let run = || -> Result<(), String> {
        let window = web_sys::window().ok_or("no global `window`")?;
        let tauri = reflect_get(&window, "__TAURI__")?;
        let window_ns = reflect_get(&tauri, "window")?;
        let get_current =
            js_sys::Function::from(reflect_get(&window_ns, "getCurrentWindow")?);
        let current_win = get_current
            .call0(&window_ns)
            .map_err(|e| format!("getCurrentWindow: {e:?}"))?;
        let show_fn = js_sys::Function::from(reflect_get(&current_win, "show")?);
        show_fn
            .call0(&current_win)
            .map_err(|e| format!("show: {e:?}"))?;
        Ok(())
    };
    if let Err(e) = run() {
        web_sys::console::error_1(&format!("show_current_window: {e}").into());
    }
}

/// Helper: call a method on the current Tauri window.
fn call_current_window(method: &str) {
    let run = || -> Result<(), String> {
        let window = web_sys::window().ok_or("no global `window`")?;
        let tauri = reflect_get(&window, "__TAURI__")?;
        let window_ns = reflect_get(&tauri, "window")?;
        let get_current =
            js_sys::Function::from(reflect_get(&window_ns, "getCurrentWindow")?);
        let current_win = get_current
            .call0(&window_ns)
            .map_err(|e| format!("getCurrentWindow: {e:?}"))?;
        let func = js_sys::Function::from(reflect_get(&current_win, method)?);
        func.call0(&current_win)
            .map_err(|e| format!("{method}: {e:?}"))?;
        Ok(())
    };
    if let Err(e) = run() {
        web_sys::console::error_1(&format!("call_current_window({method}): {e}").into());
    }
}

pub fn close_current_window() {
    call_current_window("close");
}

pub fn minimize_current_window() {
    call_current_window("minimize");
}

pub fn toggle_maximize_current_window() {
    call_current_window("toggleMaximize");
}

pub fn start_dragging() {
    call_current_window("startDragging");
}

pub async fn open_folder_in_window(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    tauri_invoke("open_folder_in_window", &Args { path }).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Update commands
// ---------------------------------------------------------------------------

pub async fn check_for_update_cmd() -> Result<Option<String>, String> {
    let val = tauri_invoke_no_args("check_for_update").await?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        let version = js_sys::Reflect::get(&val, &"version".into())
            .ok()
            .and_then(|v| v.as_string());
        Ok(version)
    }
}

pub async fn apply_update_cmd() -> Result<(), String> {
    tauri_invoke_no_args("apply_update").await?;
    Ok(())
}

pub async fn restart_after_update_cmd() -> Result<(), String> {
    tauri_invoke_no_args("restart_after_update").await?;
    Ok(())
}

pub async fn get_current_version() -> Result<String, String> {
    let val = tauri_invoke_no_args("get_current_version").await?;
    val.as_string()
        .ok_or_else(|| "get_current_version: expected string".to_string())
}

pub async fn dismiss_update_cmd() -> Result<(), String> {
    tauri_invoke_no_args("dismiss_update").await?;
    Ok(())
}

pub fn listen_update_status(callback: impl Fn(String) + 'static) {
    listen_event("update-status", move |payload: JsValue| {
        if let Ok(inner) = reflect_get(&payload, "payload") {
            if let Ok(json) = js_sys::JSON::stringify(&inner) {
                if let Some(s) = json.as_string() {
                    callback(s);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Event listeners (app-lifetime — closures are `.forget()`-ed)
// ---------------------------------------------------------------------------

/// Listen to the `open-folder-with-file` Tauri event.
/// Calls the callback with `(folder, file)` strings.
pub fn listen_open_folder_with_file(callback: impl Fn(String, String) + 'static) {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Payload {
        folder: String,
        file: String,
    }
    listen_event("open-folder-with-file", move |payload: JsValue| {
        if let Ok(inner) = reflect_get(&payload, "payload") {
            match serde_wasm_bindgen::from_value::<Payload>(inner) {
                Ok(p) => callback(p.folder, p.file),
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("open-folder-with-file deser: {e}").into(),
                    );
                }
            }
        }
    });
}

/// Listen to a payload-less Tauri menu event (e.g. `menu:save`).
/// The closure is called with no arguments each time the event fires.
pub fn listen_menu_event(event_name: &str, cb: impl Fn() + 'static) {
    listen_event(event_name, move |_payload: JsValue| {
        cb();
    });
}

/// Listen to the `config-changed` Tauri event. Deserialises the payload
/// into `AppConfig` before calling the callback.
pub fn listen_config_changed(callback: impl Fn(AppConfig) + 'static) {
    listen_event("config-changed", move |payload: JsValue| {
        if let Ok(inner) = reflect_get(&payload, "payload") {
            match serde_wasm_bindgen::from_value::<AppConfig>(inner) {
                Ok(config) => callback(config),
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("config-changed deser: {e}").into(),
                    );
                }
            }
        }
    });
}

/// Low-level helper: call `__TAURI__.event.listen(event_name, handler)`.
/// The `Closure` is `.forget()`-ed since these are app-lifetime listeners.
fn listen_event(event_name: &str, handler: impl Fn(JsValue) + 'static) {
    let run = || -> Result<(), String> {
        let window = web_sys::window().ok_or("no global `window`")?;
        let tauri = reflect_get(&window, "__TAURI__")?;
        let event_ns = reflect_get(&tauri, "event")?;
        let listen_fn = js_sys::Function::from(reflect_get(&event_ns, "listen")?);

        let closure = Closure::wrap(Box::new(handler) as Box<dyn Fn(JsValue)>);
        listen_fn
            .call2(
                &event_ns,
                &JsValue::from_str(event_name),
                closure.as_ref(),
            )
            .map_err(|e| format!("event.listen('{event_name}'): {e:?}"))?;

        // App-lifetime listener — intentionally leak the closure.
        closure.forget();
        Ok(())
    };
    if let Err(e) = run() {
        web_sys::console::error_1(&format!("listen_event('{event_name}'): {e}").into());
    }
}
