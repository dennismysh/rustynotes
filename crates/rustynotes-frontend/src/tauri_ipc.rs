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

// ---------------------------------------------------------------------------
// Event listeners (app-lifetime — closures are `.forget()`-ed)
// ---------------------------------------------------------------------------

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
