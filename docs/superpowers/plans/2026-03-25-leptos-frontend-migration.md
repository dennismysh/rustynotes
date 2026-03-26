# Leptos Frontend Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Solid.js/TypeScript frontend with a Leptos/Rust WASM frontend, keeping a thin JS bridge for CodeMirror, TipTap, KaTeX, and Mermaid.

**Architecture:** Cargo workspace with three crates: `rustynotes-common` (shared types), `rustynotes-frontend` (Leptos CSR app), and the existing `src-tauri` backend. Trunk builds the WASM bundle. A pre-bundled `bridge.bundle.js` handles all JS library interop, exposed on `window.RustyNotesBridge` and called from Rust via `js_sys::Reflect`. Comrak and syntect run client-side in WASM.

**Tech Stack:** Leptos 0.7 (CSR), Trunk, comrak (no default features + needed extensions), syntect (default-fancy), wasm-bindgen, web-sys, js-sys, gloo-storage, serde, serde-wasm-bindgen, wasm-bindgen-futures

**Spec:** `docs/superpowers/specs/2026-03-25-leptos-frontend-migration-design.md`

---

## File Structure

### New files to create

```
Cargo.toml                                    # workspace root
Trunk.toml                                    # Trunk build config
crates/rustynotes-common/Cargo.toml
crates/rustynotes-common/src/lib.rs           # shared types: AppConfig, FileNode, SearchResult, enums
crates/rustynotes-frontend/Cargo.toml
crates/rustynotes-frontend/src/main.rs        # Leptos mount + router
crates/rustynotes-frontend/src/app.rs         # App shell component
crates/rustynotes-frontend/src/state.rs       # RwSignal<AppState> context
crates/rustynotes-frontend/src/tauri_ipc.rs   # Tauri IPC via js_sys
crates/rustynotes-frontend/src/bridge.rs      # bridge.js calls via js_sys::Reflect on window
crates/rustynotes-frontend/src/theme.rs       # CSS custom property application + embedded theme JSON
crates/rustynotes-frontend/src/components/mod.rs
crates/rustynotes-frontend/src/components/toolbar.rs
crates/rustynotes-frontend/src/components/editor/mod.rs
crates/rustynotes-frontend/src/components/editor/source_editor.rs
crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs
crates/rustynotes-frontend/src/components/editor/split_pane.rs
crates/rustynotes-frontend/src/components/preview/mod.rs
crates/rustynotes-frontend/src/components/preview/preview.rs
crates/rustynotes-frontend/src/components/preview/markdown.rs
crates/rustynotes-frontend/src/components/navigation/mod.rs
crates/rustynotes-frontend/src/components/navigation/sidebar.rs
crates/rustynotes-frontend/src/components/navigation/miller_columns.rs
crates/rustynotes-frontend/src/components/navigation/breadcrumb.rs
crates/rustynotes-frontend/src/components/settings/mod.rs
crates/rustynotes-frontend/src/components/settings/settings_window.rs
crates/rustynotes-frontend/src/components/settings/settings_sidebar.rs
crates/rustynotes-frontend/src/components/settings/shared.rs        # SettingRow, SettingToggle, etc.
crates/rustynotes-frontend/src/components/settings/categories/mod.rs
crates/rustynotes-frontend/src/components/settings/categories/appearance.rs
crates/rustynotes-frontend/src/components/settings/categories/editor.rs
crates/rustynotes-frontend/src/components/settings/categories/preview.rs
crates/rustynotes-frontend/src/components/settings/categories/advanced.rs
crates/rustynotes-frontend/src/components/onboarding/mod.rs
crates/rustynotes-frontend/src/components/onboarding/welcome.rs
crates/rustynotes-frontend/src/components/onboarding/feature_tip.rs
js/bridge-src.js                              # bridge source (unbundled, imports from npm)
js/bundle-vendor.sh                           # esbuild: bundles bridge-src.js -> bridge.bundle.js
static/index.html                             # Trunk entry point
```

### Files to modify

```
src-tauri/Cargo.toml                          # add rustynotes-common dependency, add workspace key
src-tauri/src/config.rs                       # extract types to common crate, keep load/save
src-tauri/src/fs_ops.rs                       # FileEntry -> FileNode with String paths
src-tauri/src/commands/fs.rs                  # SearchResult -> use common crate type
src-tauri/src/commands/mod.rs                 # update CommandError to use common types
src-tauri/src/lib.rs                          # remove parse_markdown from invoke_handler
src-tauri/tauri.conf.json                     # update build commands for Trunk
```

### Files to move (unchanged content)

```
src/styles/base.css          -> styles/base.css
src/styles/settings.css      -> styles/settings.css
src/styles/themes/*.json     -> styles/themes/*.json
```

### Files to delete (after migration complete)

```
src/                          # entire Solid.js frontend
package.json
pnpm-lock.yaml
tsconfig.json
tsconfig.node.json
vite.config.ts
index.html                    # replaced by static/index.html
public/                       # Vite public dir, no longer needed
```

---

## JS Bridge Loading Strategy

The bridge does NOT use `wasm_bindgen(module = ...)` — that requires the JS file to be resolvable relative to the crate root, which doesn't work with Trunk's build pipeline.

Instead:
1. `js/bridge-src.js` is the source file that imports from npm packages
2. `js/bundle-vendor.sh` runs esbuild to bundle everything into a single `js/bridge.bundle.js`
3. `bridge.bundle.js` registers all functions on `window.RustyNotesBridge`
4. `static/index.html` loads `bridge.bundle.js` via `<script type="module">`
5. Rust code calls bridge functions via `js_sys::Reflect::get(&window, "RustyNotesBridge")` — no `wasm_bindgen(module=...)` needed

This is the standard pattern for Trunk + external JS interop.

---

## Task 1: Workspace + Common Crate

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/rustynotes-common/Cargo.toml`
- Create: `crates/rustynotes-common/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/fs_ops.rs`
- Modify: `src-tauri/src/commands/fs.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Move existing Cargo.lock to workspace root**

```bash
cp src-tauri/Cargo.lock Cargo.lock
```

This preserves dependency resolution when the workspace subsumes `src-tauri`.

- [ ] **Step 2: Create workspace root Cargo.toml**

```toml
# Cargo.toml (project root)
[workspace]
members = [
    "crates/rustynotes-common",
    "crates/rustynotes-frontend",
    "src-tauri",
]
resolver = "2"
```

- [ ] **Step 3: Create rustynotes-common crate with shared types**

`crates/rustynotes-common/Cargo.toml`:
```toml
[package]
name = "rustynotes-common"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`crates/rustynotes-common/src/lib.rs` — all shared types: `AppConfig`, `ThemeConfig`, `ThemeOverrides`, `RenderingToggles`, `FileNode` (with `String` path, not `PathBuf`), `SearchResult`, `EditorMode` enum, `NavMode` enum, `ThemeData`. Include `Default` impls and serde defaults. Include unit tests for serialization round-trips.

Key: `FileNode.path` is `String` for wasm32 compatibility. All default functions are `pub` so the backend can use them.

- [ ] **Step 4: Run common crate tests**

Run: `cargo test -p rustynotes-common`
Expected: All serialization round-trip tests pass

- [ ] **Step 5: Update src-tauri/Cargo.toml**

Add dependency:
```toml
rustynotes-common = { path = "../crates/rustynotes-common" }
```

- [ ] **Step 6: Update src-tauri/src/config.rs**

Replace type definitions with re-exports from common crate:
```rust
pub use rustynotes_common::{AppConfig, ThemeConfig, ThemeOverrides, RenderingToggles};
```
Keep `load_config`, `save_config`, `config_dir`, `config_path` functions and all tests.

- [ ] **Step 7: Update src-tauri/src/fs_ops.rs**

Replace `FileEntry` struct with `pub use rustynotes_common::FileNode;`. Update `list_directory` to build `FileNode` with `path: entry.path().display().to_string()` instead of `path: entry.path()`. Update all tests to use `FileNode`.

- [ ] **Step 8: Update src-tauri/src/commands/fs.rs**

Replace local `SearchResult` struct with `use rustynotes_common::SearchResult;`. Update `use crate::fs_ops::FileEntry` to `use crate::fs_ops::FileNode`. Update `list_directory` return type from `Vec<FileEntry>` to `Vec<FileNode>`.

- [ ] **Step 9: Update src-tauri/src/commands/mod.rs**

Update `CommandError` to reference `crate::fs_ops::FsError` (unchanged — the error type stays in the backend).

- [ ] **Step 10: Update src-tauri/src/lib.rs — remove parse_markdown**

Remove `commands::markdown::parse_markdown` from the `invoke_handler` list. Keep the module and `markdown_parser.rs` — used by the HTML exporter.

- [ ] **Step 11: Run all backend tests**

Run: `cargo test -p rustynotes`
Expected: All existing tests pass

- [ ] **Step 12: Commit**

```bash
git add Cargo.toml Cargo.lock crates/ src-tauri/
git commit -m "feat: create workspace with rustynotes-common shared types crate"
```

---

## Task 2: Trunk + Leptos Skeleton

**Files:**
- Create: `Trunk.toml`
- Create: `static/index.html`
- Create: `crates/rustynotes-frontend/Cargo.toml`
- Create: `crates/rustynotes-frontend/src/main.rs`
- Move: `src/styles/` -> `styles/`

- [ ] **Step 1: Create Trunk.toml**

```toml
[build]
target = "static/index.html"
dist = "dist"

[watch]
watch = ["crates/rustynotes-frontend/src", "js", "styles"]
```

- [ ] **Step 2: Create static/index.html**

Note: `data-bin="rustynotes-frontend"` tells Trunk which workspace member to build.

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="theme-color" content="#1e1e2e" />
    <title>RustyNotes</title>
    <link data-trunk rel="css" href="styles/base.css" />
    <link data-trunk rel="css" href="styles/settings.css" />
    <link data-trunk rel="copy-dir" href="styles/themes" />
    <link data-trunk rel="copy-file" href="js/bridge.bundle.js" />
    <link data-trunk rel="rust" data-wasm-opt="z" data-bin="rustynotes-frontend" />
    <script type="module" src="bridge.bundle.js"></script>
</head>
<body></body>
</html>
```

- [ ] **Step 3: Create frontend Cargo.toml**

```toml
[package]
name = "rustynotes-frontend"
version = "0.1.0"
edition = "2021"

[dependencies]
rustynotes-common = { path = "../rustynotes-common" }
leptos = { version = "0.7", features = ["csr"] }
leptos_router = { version = "0.7", features = ["csr"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "HtmlElement", "Document", "Window",
    "CssStyleDeclaration", "MediaQueryList", "Element",
] }
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde-wasm-bindgen = "0.6"
gloo-storage = "0.3"
console_error_panic_hook = "0.1"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 4: Create minimal main.rs**

```rust
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <p>"RustyNotes — Leptos frontend loaded"</p> });
}
```

- [ ] **Step 5: Move CSS files**

```bash
cp -r src/styles styles
```

- [ ] **Step 6: Create placeholder bridge.bundle.js**

```bash
mkdir -p js
echo "// placeholder — will be generated by bundle-vendor.sh" > js/bridge.bundle.js
echo "window.RustyNotesBridge = {};" >> js/bridge.bundle.js
```

- [ ] **Step 7: Install Trunk and build**

Run: `cargo install trunk && trunk build`
Expected: `dist/` directory created with `index.html`, `.wasm` file, and `bridge.bundle.js`

- [ ] **Step 8: Commit**

```bash
git add Trunk.toml static/ crates/rustynotes-frontend/ styles/ js/
git commit -m "feat: add Leptos frontend skeleton with Trunk build"
```

---

## Task 3: Tauri IPC Bindings

**Files:**
- Create: `crates/rustynotes-frontend/src/tauri_ipc.rs`
- Modify: `crates/rustynotes-frontend/src/main.rs`

- [ ] **Step 1: Create tauri_ipc.rs**

Uses `js_sys::Reflect` + `JsFuture` pattern — NOT `async fn` in wasm_bindgen extern blocks (which doesn't work).

```rust
// crates/rustynotes-frontend/src/tauri_ipc.rs
use rustynotes_common::{AppConfig, FileNode, SearchResult};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Call window.__TAURI__.core.invoke(cmd, args) and await the Promise.
async fn tauri_invoke(cmd: &str, args: &impl Serialize) -> Result<JsValue, String> {
    let window = web_sys::window().unwrap();
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("No __TAURI__: {:?}", e))?;
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("No core: {:?}", e))?;
    let invoke_fn = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("No invoke: {:?}", e))?;
    let invoke_fn = js_sys::Function::from(invoke_fn);

    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let promise = invoke_fn
        .call2(&core, &cmd.into(), &args_js)
        .map_err(|e| format!("invoke call failed: {:?}", e))?;
    let promise = js_sys::Promise::from(promise);
    JsFuture::from(promise).await.map_err(|e| format!("invoke rejected: {:?}", e))
}

async fn invoke_cmd<T: serde::de::DeserializeOwned>(
    cmd: &str,
    args: &impl Serialize,
) -> Result<T, String> {
    let result = tauri_invoke(cmd, args).await?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

async fn invoke_cmd_unit(cmd: &str, args: &impl Serialize) -> Result<(), String> {
    tauri_invoke(cmd, args).await?;
    Ok(())
}

// --- Arg structs ---
#[derive(Serialize)] struct PathArg<'a> { path: &'a str }
#[derive(Serialize)] struct WriteArgs<'a> { path: &'a str, content: &'a str }
#[derive(Serialize)] struct WikilinkArgs<'a> { root: &'a str, name: &'a str }
#[derive(Serialize)] struct SearchArgs<'a> { root: &'a str, query: &'a str }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct ExportArgs<'a> { markdown: &'a str, output_path: &'a str, format: &'a str, include_theme: bool }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct ConfigSaveArgs { config_data: AppConfig }
#[derive(Serialize)] struct Empty {}

// --- Public API ---
pub async fn read_file(path: &str) -> Result<String, String> {
    invoke_cmd("read_file", &PathArg { path }).await
}
pub async fn write_file(path: &str, content: &str) -> Result<(), String> {
    invoke_cmd_unit("write_file", &WriteArgs { path, content }).await
}
pub async fn list_directory(path: &str) -> Result<Vec<FileNode>, String> {
    invoke_cmd("list_directory", &PathArg { path }).await
}
pub async fn resolve_wikilink(root: &str, name: &str) -> Result<Option<String>, String> {
    invoke_cmd("resolve_wikilink", &WikilinkArgs { root, name }).await
}
pub async fn search_files(root: &str, query: &str) -> Result<Vec<SearchResult>, String> {
    invoke_cmd("search_files", &SearchArgs { root, query }).await
}
pub async fn watch_folder(path: &str) -> Result<(), String> {
    invoke_cmd_unit("watch_folder", &PathArg { path }).await
}
pub async fn get_config() -> Result<AppConfig, String> {
    invoke_cmd("get_config", &Empty {}).await
}
pub async fn save_config_cmd(config: AppConfig) -> Result<(), String> {
    invoke_cmd_unit("save_config_cmd", &ConfigSaveArgs { config_data: config }).await
}
pub async fn open_settings() -> Result<(), String> {
    invoke_cmd_unit("open_settings", &Empty {}).await
}
pub async fn export_file(markdown: &str, output_path: &str, format: &str, include_theme: bool) -> Result<(), String> {
    invoke_cmd_unit("export_file", &ExportArgs { markdown, output_path, format, include_theme }).await
}

// --- Dialog (plugin namespace, not core.invoke) ---
pub async fn open_folder_dialog() -> Result<Option<String>, String> {
    let window = web_sys::window().unwrap();
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into()).map_err(|e| format!("{:?}", e))?;
    let dialog = js_sys::Reflect::get(&tauri, &"dialog".into()).map_err(|e| format!("{:?}", e))?;
    let open_fn = js_sys::Function::from(
        js_sys::Reflect::get(&dialog, &"open".into()).map_err(|e| format!("{:?}", e))?
    );
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"directory".into(), &true.into());
    let _ = js_sys::Reflect::set(&opts, &"multiple".into(), &false.into());
    let promise = open_fn.call1(&dialog, &opts).map_err(|e| format!("{:?}", e))?;
    let result = JsFuture::from(js_sys::Promise::from(promise)).await
        .map_err(|e| format!("{:?}", e))?;
    if result.is_null() || result.is_undefined() { Ok(None) } else { Ok(result.as_string()) }
}

pub async fn save_file_dialog(default_name: &str) -> Result<Option<String>, String> {
    let window = web_sys::window().unwrap();
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into()).map_err(|e| format!("{:?}", e))?;
    let dialog = js_sys::Reflect::get(&tauri, &"dialog".into()).map_err(|e| format!("{:?}", e))?;
    let save_fn = js_sys::Function::from(
        js_sys::Reflect::get(&dialog, &"save".into()).map_err(|e| format!("{:?}", e))?
    );
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"defaultPath".into(), &default_name.into());
    let promise = save_fn.call1(&dialog, &opts).map_err(|e| format!("{:?}", e))?;
    let result = JsFuture::from(js_sys::Promise::from(promise)).await
        .map_err(|e| format!("{:?}", e))?;
    if result.is_null() || result.is_undefined() { Ok(None) } else { Ok(result.as_string()) }
}

// --- Window ---
pub fn show_current_window() {
    let window = web_sys::window().unwrap();
    if let Ok(tauri) = js_sys::Reflect::get(&window, &"__TAURI__".into()) {
        if let Ok(win_ns) = js_sys::Reflect::get(&tauri, &"window".into()) {
            if let Ok(get_fn) = js_sys::Reflect::get(&win_ns, &"getCurrentWindow".into()) {
                let get_fn = js_sys::Function::from(get_fn);
                if let Ok(current) = get_fn.call0(&win_ns) {
                    if let Ok(show_fn) = js_sys::Reflect::get(&current, &"show".into()) {
                        let _ = js_sys::Function::from(show_fn).call0(&current);
                    }
                }
            }
        }
    }
}

// --- Event listeners ---
pub fn listen_file_changed(callback: impl Fn(String) + 'static) {
    let closure = Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &"payload".into()) {
            if let Some(s) = payload.as_string() {
                callback(s);
            }
        }
    }) as Box<dyn Fn(JsValue)>);

    // Store closure on window FIRST, then register listener
    let window = web_sys::window().unwrap();
    let _ = js_sys::Reflect::set(&window, &"__rn_file_cb".into(), closure.as_ref());
    closure.forget(); // intentional — app-lifetime listener

    let _ = js_sys::eval(
        "window.__TAURI__.event.listen('file-changed', window.__rn_file_cb)"
    );
}

pub fn listen_config_changed(callback: impl Fn(AppConfig) + 'static) {
    let closure = Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &"payload".into()) {
            if let Ok(config) = serde_wasm_bindgen::from_value::<AppConfig>(payload) {
                callback(config);
            }
        }
    }) as Box<dyn Fn(JsValue)>);

    // Store closure on window FIRST, then register listener
    let window = web_sys::window().unwrap();
    let _ = js_sys::Reflect::set(&window, &"__rn_config_cb".into(), closure.as_ref());
    closure.forget();

    let _ = js_sys::eval(
        "window.__TAURI__.event.listen('config-changed', window.__rn_config_cb)"
    );
}
```

- [ ] **Step 2: Add module declaration to main.rs**

```rust
mod tauri_ipc;
```

- [ ] **Step 3: Verify it compiles**

Run: `trunk build`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/
git commit -m "feat: add Tauri IPC bindings using js_sys::Reflect"
```

---

## Task 4: State Management + Theme

**Files:**
- Create: `crates/rustynotes-frontend/src/state.rs`
- Create: `crates/rustynotes-frontend/src/theme.rs`
- Modify: `crates/rustynotes-frontend/src/main.rs`

- [ ] **Step 1: Create state.rs**

```rust
use leptos::prelude::*;
use rustynotes_common::{AppConfig, EditorMode, FileNode, NavMode};

#[derive(Clone)]
pub struct AppState {
    pub current_folder: RwSignal<Option<String>>,
    pub file_tree: RwSignal<Vec<FileNode>>,
    pub active_file_path: RwSignal<Option<String>>,
    pub active_file_content: RwSignal<String>,
    pub editor_mode: RwSignal<EditorMode>,
    pub is_dirty: RwSignal<bool>,
    pub rendered_html: RwSignal<String>,
    pub app_config: RwSignal<Option<AppConfig>>,
    pub nav_mode: RwSignal<NavMode>,
    pub search_query: RwSignal<String>,
    pub show_search: RwSignal<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_folder: RwSignal::new(None),
            file_tree: RwSignal::new(Vec::new()),
            active_file_path: RwSignal::new(None),
            active_file_content: RwSignal::new(String::new()),
            editor_mode: RwSignal::new(EditorMode::Wysiwyg),
            is_dirty: RwSignal::new(false),
            rendered_html: RwSignal::new(String::new()),
            app_config: RwSignal::new(None),
            nav_mode: RwSignal::new(NavMode::Sidebar),
            search_query: RwSignal::new(String::new()),
            show_search: RwSignal::new(false),
        }
    }
}

pub fn provide_app_state() { provide_context(AppState::new()); }
pub fn use_app_state() -> AppState { expect_context::<AppState>() }
```

- [ ] **Step 2: Create theme.rs — with embedded theme JSON**

Theme JSON files are embedded via `include_str!` so they're available in WASM without fetch.

```rust
use rustynotes_common::{ThemeData, ThemeOverrides};
use web_sys::window;
use wasm_bindgen::JsCast;

// Embed theme JSON files at compile time
const LIGHT_THEME_JSON: &str = include_str!("../../../styles/themes/default-light.json");
const DARK_THEME_JSON: &str = include_str!("../../../styles/themes/default-dark.json");

pub fn load_theme(name: &str) -> ThemeData {
    let json = match name {
        "dark" => DARK_THEME_JSON,
        _ => LIGHT_THEME_JSON,
    };
    serde_json::from_str(json).unwrap_or_else(|_| ThemeData {
        name: name.to_string(),
        colors: Default::default(),
        typography: Default::default(),
        spacing: Default::default(),
    })
}

pub fn resolve_theme(active: &str) -> ThemeData {
    if active == "auto" { load_theme(get_system_theme()) } else { load_theme(active) }
}

pub fn apply_theme(theme: &ThemeData, overrides: Option<&ThemeOverrides>) {
    let document = window().unwrap().document().unwrap();
    let root = document.document_element().unwrap();
    let style = root.unchecked_ref::<web_sys::HtmlElement>().style();

    for (key, value) in &theme.colors {
        let _ = style.set_property(&format!("--{}", key), value);
    }

    let typo_map: &[(&str, &str)] = &[
        ("body-font", "--font-body"), ("body-size", "--font-size"),
        ("mono-font", "--font-mono"), ("line-height", "--line-height"),
    ];
    for (key, value) in &theme.typography {
        let css_var = typo_map.iter()
            .find(|(k, _)| *k == key.as_str())
            .map(|(_, v)| v.to_string())
            .unwrap_or_else(|| format!("--{}", key));
        let _ = style.set_property(&css_var, value);
    }

    for (key, value) in &theme.spacing {
        let _ = style.set_property(&format!("--{}", key), value);
    }

    if let Some(ovr) = overrides {
        for (k, v) in &ovr.colors { let _ = style.set_property(&format!("--{}", k), v); }
        for (k, v) in &ovr.typography {
            let css_var = typo_map.iter()
                .find(|(key, _)| *key == k.as_str())
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| format!("--{}", k));
            let _ = style.set_property(&css_var, v);
        }
        for (k, v) in &ovr.spacing { let _ = style.set_property(&format!("--{}", k), v); }
    }
}

pub fn get_system_theme() -> &'static str {
    let window = window().unwrap();
    let mq = window.match_media("(prefers-color-scheme: dark)").unwrap();
    if mq.map(|m| m.matches()).unwrap_or(false) { "dark" } else { "light" }
}
```

- [ ] **Step 3: Wire into main.rs**

```rust
mod state;
mod tauri_ipc;
mod theme;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        state::provide_app_state();
        view! { <p>"RustyNotes — state + theme loaded"</p> }
    });
}
```

- [ ] **Step 4: Verify it compiles** — `trunk build`
- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/
git commit -m "feat: add Leptos state management and theme engine with embedded JSON"
```

---

## Task 5: JS Bridge + Vendor Bundling

**Files:**
- Create: `js/bridge-src.js`
- Create: `js/bundle-vendor.sh`
- Create: `crates/rustynotes-frontend/src/bridge.rs`

- [ ] **Step 1: Create bridge-src.js**

This is the unbundled source. It imports from npm packages and registers everything on `window.RustyNotesBridge`. KaTeX CSS is vendored locally (not CDN).

```javascript
// js/bridge-src.js
import { EditorView, basicSetup } from 'codemirror';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { searchKeymap } from '@codemirror/search';
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Markdown } from '@tiptap/extension-markdown';

let katexModule = null;
let mermaidModule = null;

window.RustyNotesBridge = {
  mountCodeMirror(element, content, options, onChange) {
    const extensions = [
      basicSetup, markdown(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      history(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) onChange(update.state.doc.toString());
      }),
    ];
    if (options.theme === 'dark') extensions.push(oneDark);
    const state = EditorState.create({ doc: content, extensions });
    return { view: new EditorView({ state, parent: element }) };
  },
  updateCodeMirror(handle, content) {
    const cur = handle.view.state.doc.toString();
    if (cur !== content) handle.view.dispatch({ changes: { from: 0, to: cur.length, insert: content } });
  },
  focusCodeMirror(handle) { handle.view.focus(); },
  destroyCodeMirror(handle) { handle.view.destroy(); },

  mountTipTap(element, content, options, onChange) {
    const exts = [StarterKit.configure({ codeBlock: false }), Markdown];
    if (options.taskLists !== false) exts.push(TaskList, TaskItem.configure({ nested: true }));
    const editor = new Editor({
      element, extensions: exts, content,
      onUpdate: ({ editor }) => onChange(editor.storage.markdown.getMarkdown()),
    });
    return { editor };
  },
  updateTipTap(handle, content) {
    if (handle.editor.storage.markdown.getMarkdown() !== content) handle.editor.commands.setContent(content);
  },
  getTipTapMarkdown(handle) { return handle.editor.storage.markdown.getMarkdown(); },
  focusTipTap(handle) { handle.editor.commands.focus(); },
  destroyTipTap(handle) { handle.editor.destroy(); },

  async renderKatex(element, latex, displayMode) {
    if (!katexModule) {
      katexModule = await import('katex');
      // CSS is vendored in the bundle — loaded via link tag in index.html
    }
    element.innerHTML = katexModule.default.renderToString(latex, { throwOnError: false, displayMode });
  },
  async renderMermaid(element, code, theme) {
    if (!mermaidModule) {
      mermaidModule = await import('mermaid');
      mermaidModule.default.initialize({ startOnLoad: false, theme: theme === 'dark' ? 'dark' : 'default', securityLevel: 'loose' });
    }
    const id = `mermaid-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const { svg } = await mermaidModule.default.render(id, code);
    element.innerHTML = svg;
  },
};
```

- [ ] **Step 2: Create bundle-vendor.sh**

Bundles bridge-src.js + all npm deps into a single self-contained `bridge.bundle.js`. Also vendors KaTeX CSS.

```bash
#!/bin/bash
set -e
cd "$(dirname "$0")/.."

npm install --save-dev \
  codemirror @codemirror/state @codemirror/view @codemirror/lang-markdown \
  @codemirror/commands @codemirror/search @codemirror/theme-one-dark \
  @tiptap/core @tiptap/starter-kit @tiptap/extension-task-list \
  @tiptap/extension-task-item @tiptap/extension-markdown \
  katex mermaid esbuild

npx esbuild js/bridge-src.js \
  --bundle --format=esm --outfile=js/bridge.bundle.js

# Vendor KaTeX CSS locally
cp node_modules/katex/dist/katex.min.css styles/katex.min.css

echo "Done: js/bridge.bundle.js + styles/katex.min.css"
```

- [ ] **Step 3: Add KaTeX CSS to index.html**

Add to `<head>` in `static/index.html`:
```html
<link data-trunk rel="css" href="styles/katex.min.css" />
```

- [ ] **Step 4: Create bridge.rs — calls window.RustyNotesBridge via js_sys**

```rust
// crates/rustynotes-frontend/src/bridge.rs
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

fn get_bridge() -> js_sys::Object {
    let window = web_sys::window().unwrap();
    js_sys::Reflect::get(&window, &"RustyNotesBridge".into())
        .unwrap()
        .unchecked_into()
}

fn call_bridge(method: &str, args: &[&JsValue]) -> Result<JsValue, JsValue> {
    let bridge = get_bridge();
    let func = js_sys::Reflect::get(&bridge, &method.into())?;
    let func = js_sys::Function::from(func);
    match args.len() {
        0 => func.call0(&bridge),
        1 => func.call1(&bridge, args[0]),
        2 => func.call2(&bridge, args[0], args[1]),
        3 => func.call3(&bridge, args[0], args[1], args[2]),
        _ => {
            let js_args = js_sys::Array::new();
            for arg in args { js_args.push(arg); }
            func.apply(&bridge, &js_args)
        }
    }
}

pub fn mount_code_mirror(
    el: &web_sys::HtmlElement,
    content: &str,
    options: &JsValue,
    on_change: &Closure<dyn Fn(String)>,
) -> JsValue {
    call_bridge("mountCodeMirror", &[
        &el.into(), &content.into(), options, on_change.as_ref(),
    ]).unwrap()
}

pub fn update_code_mirror(handle: &JsValue, content: &str) {
    let _ = call_bridge("updateCodeMirror", &[handle, &content.into()]);
}

pub fn focus_code_mirror(handle: &JsValue) {
    let _ = call_bridge("focusCodeMirror", &[handle]);
}

pub fn destroy_code_mirror(handle: &JsValue) {
    let _ = call_bridge("destroyCodeMirror", &[handle]);
}

pub fn mount_tiptap(
    el: &web_sys::HtmlElement,
    content: &str,
    options: &JsValue,
    on_change: &Closure<dyn Fn(String)>,
) -> JsValue {
    call_bridge("mountTipTap", &[
        &el.into(), &content.into(), options, on_change.as_ref(),
    ]).unwrap()
}

pub fn update_tiptap(handle: &JsValue, content: &str) {
    let _ = call_bridge("updateTipTap", &[handle, &content.into()]);
}

pub fn get_tiptap_markdown(handle: &JsValue) -> String {
    call_bridge("getTipTapMarkdown", &[handle])
        .unwrap()
        .as_string()
        .unwrap_or_default()
}

pub fn focus_tiptap(handle: &JsValue) {
    let _ = call_bridge("focusTipTap", &[handle]);
}

pub fn destroy_tiptap(handle: &JsValue) {
    let _ = call_bridge("destroyTipTap", &[handle]);
}

pub async fn render_katex(el: &web_sys::HtmlElement, latex: &str, display_mode: bool) {
    let result = call_bridge("renderKatex", &[
        &el.into(), &latex.into(), &display_mode.into(),
    ]);
    if let Ok(promise) = result {
        if promise.has_type::<js_sys::Promise>() {
            let _ = JsFuture::from(js_sys::Promise::from(promise)).await;
        }
    }
}

pub async fn render_mermaid(el: &web_sys::HtmlElement, code: &str, theme: &str) {
    let result = call_bridge("renderMermaid", &[
        &el.into(), &code.into(), &theme.into(),
    ]);
    if let Ok(promise) = result {
        if promise.has_type::<js_sys::Promise>() {
            let _ = JsFuture::from(js_sys::Promise::from(promise)).await;
        }
    }
}
```

- [ ] **Step 5: Add module to main.rs** — `mod bridge;`
- [ ] **Step 6: Verify it compiles** — `trunk build`
- [ ] **Step 7: Commit**

```bash
git add js/ crates/rustynotes-frontend/src/bridge.rs
git commit -m "feat: add JS bridge with window.RustyNotesBridge pattern"
```

---

## Task 6: Markdown Rendering in WASM (comrak + syntect)

**Files:**
- Create: `crates/rustynotes-frontend/src/components/mod.rs`
- Create: `crates/rustynotes-frontend/src/components/preview/mod.rs`
- Create: `crates/rustynotes-frontend/src/components/preview/markdown.rs`
- Modify: `crates/rustynotes-frontend/Cargo.toml`

- [ ] **Step 1: Add comrak and syntect dependencies**

```toml
comrak = "0.51"
syntect = { version = "5", default-features = false, features = ["default-fancy", "html"] }
regex-lite = "0.1"
html-escape = "0.2"
once_cell = "1"
```

Note: comrak 0.51 `Options` has no lifetime parameter. syntect uses `default-fancy` for pure-Rust regex (no Oniguruma). `once_cell::unsync::Lazy` is used instead of `std::sync::LazyLock` — more appropriate for single-threaded WASM.

- [ ] **Step 2: Create markdown.rs**

```rust
use comrak::{markdown_to_html, Options};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use once_cell::unsync::Lazy;

// Thread-local lazy statics — WASM is single-threaded
thread_local! {
    static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
    static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);
}

pub fn render_markdown(input: &str) -> String {
    let html = markdown_to_html(input, &comrak_options());
    highlight_code_blocks(&html)
}

fn comrak_options() -> Options {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.alerts = true;
    options.extension.math_dollars = true;
    options.extension.header_ids = Some(String::new());
    options.extension.front_matter_delimiter = Some("---".to_string());
    options.extension.wikilinks_title_after_pipe = true;
    options.render.r#unsafe = true;
    options
}

fn highlight_code_blocks(html: &str) -> String {
    let re = regex_lite::Regex::new(
        r#"<pre><code class="language-(\w+)">([\s\S]*?)</code></pre>"#
    ).unwrap();

    re.replace_all(html, |caps: &regex_lite::Captures| {
        let lang = &caps[1];
        let code = html_escape::decode_html_entities(&caps[2]);
        if lang == "mermaid" { return caps[0].to_string(); }

        SYNTAX_SET.with(|ss| {
            THEME_SET.with(|ts| {
                let syntax = ss.find_syntax_by_token(lang)
                    .unwrap_or_else(|| ss.find_syntax_plain_text());
                let theme = &ts.themes["base16-ocean.dark"];
                match highlighted_html_for_string(&code, ss, syntax, theme) {
                    Ok(highlighted) => format!(r#"<div class="shiki-wrapper">{}</div>"#, highlighted),
                    Err(_) => caps[0].to_string(),
                }
            })
        })
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_markdown() {
        let html = render_markdown("# Hello\n\nWorld");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn code_block_highlighted() {
        let html = render_markdown("```rust\nfn main() {}\n```");
        assert!(html.contains("shiki-wrapper") || html.contains("<pre"));
    }

    #[test]
    fn mermaid_block_preserved() {
        let html = render_markdown("```mermaid\ngraph LR\nA-->B\n```");
        assert!(html.contains("language-mermaid"));
    }
}
```

- [ ] **Step 3: Create module files**

```rust
// crates/rustynotes-frontend/src/components/mod.rs
pub mod preview;

// crates/rustynotes-frontend/src/components/preview/mod.rs
pub mod markdown;
pub mod preview;
```

- [ ] **Step 4: Add `mod components;` to main.rs**
- [ ] **Step 5: Run tests** — `cargo test -p rustynotes-frontend`
- [ ] **Step 6: Commit**

```bash
git add crates/rustynotes-frontend/
git commit -m "feat: add WASM-local markdown rendering with comrak + syntect"
```

---

## Task 7: App Shell + Router + Preview Component

**Files:**
- Create: `crates/rustynotes-frontend/src/app.rs`
- Create: `crates/rustynotes-frontend/src/components/preview/preview.rs`
- Modify: `crates/rustynotes-frontend/src/main.rs`

- [ ] **Step 1: Create preview.rs** — reactive component that re-renders markdown when content signal changes. Uses `Memo::new` for derived HTML, `Effect::new` to sync to `rendered_html` signal.

- [ ] **Step 2: Create app.rs** — main shell with `leptos_router::Router`, hash-based routing (`#/` for main, `#/settings` for settings). Provides `AppState` context. MainView renders toolbar + navigation + editor/preview area.

- [ ] **Step 3: Update main.rs** to mount `app::App`

- [ ] **Step 4: Verify it compiles** — `trunk build`
- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/
git commit -m "feat: add app shell with router and preview component"
```

---

## Task 8: Tauri Configuration Update

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Update build commands for Trunk**

```json
"beforeDevCommand": "trunk serve --port 1420",
"devUrl": "http://localhost:1420",
"beforeBuildCommand": "trunk build --release",
"frontendDist": "../dist"
```

- [ ] **Step 2: Test with cargo tauri dev** — app window opens, Leptos WASM loads
- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: update Tauri config to use Trunk build"
```

---

## Task 9: Navigation Components

**Files:**
- Create: `crates/rustynotes-frontend/src/components/navigation/mod.rs`
- Create: `crates/rustynotes-frontend/src/components/navigation/sidebar.rs`
- Create: `crates/rustynotes-frontend/src/components/navigation/miller_columns.rs`
- Create: `crates/rustynotes-frontend/src/components/navigation/breadcrumb.rs`

Port from `src/components/navigation/` (514 lines total). Translation: JSX -> `view!` macro, `createSignal` -> `RwSignal::new`, `createEffect` -> `Effect::new`, event handlers -> Leptos event syntax. Call `tauri_ipc::list_directory`, `tauri_ipc::read_file` via `leptos::spawn_local`.

- [ ] **Step 1–4:** Port each component
- [ ] **Step 5:** Wire into app.rs with NavMode switching
- [ ] **Step 6:** Verify — `trunk build`
- [ ] **Step 7: Commit**

---

## Task 10: Toolbar Component

Port from `src/components/Toolbar.tsx` (93 lines). Calls `tauri_ipc::open_folder_dialog`, `tauri_ipc::open_settings`. Handles Cmd+, keyboard shortcut.

- [ ] **Step 1:** Port toolbar.rs
- [ ] **Step 2:** Wire into app.rs
- [ ] **Step 3:** Verify — `trunk build`
- [ ] **Step 4: Commit**

---

## Task 11: Editor Components

**Files:**
- Create: `crates/rustynotes-frontend/src/components/editor/mod.rs`
- Create: `crates/rustynotes-frontend/src/components/editor/source_editor.rs`
- Create: `crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs`
- Create: `crates/rustynotes-frontend/src/components/editor/split_pane.rs`

Uses bridge.rs functions. Lifecycle pattern:
- `NodeRef::new` for mount point
- `Effect::new` to call `bridge::mount_code_mirror` / `bridge::mount_tiptap`
- `StoredValue::new` for the handle AND the onChange `Closure`
- `on_cleanup` drops both handle and closure

```rust
// Lifecycle pattern (source_editor.rs sketch)
#[component]
pub fn SourceEditor() -> impl IntoView {
    let state = use_app_state();
    let container = NodeRef::<Div>::new();
    let handle = StoredValue::new(None::<JsValue>);
    let closure_store = StoredValue::new(None::<Closure<dyn Fn(String)>>);

    Effect::new(move |_| {
        if let Some(el) = container.get() {
            let set_content = state.active_file_content.write_only();
            let set_dirty = state.is_dirty.write_only();
            let cb = Closure::wrap(Box::new(move |content: String| {
                set_content.set(content);
                set_dirty.set(true);
            }) as Box<dyn Fn(String)>);

            let opts = /* build JsValue options */;
            let h = bridge::mount_code_mirror(
                &el.unchecked_into(), &state.active_file_content.get(), &opts, &cb
            );
            handle.set_value(Some(h));
            closure_store.set_value(Some(cb));
        }
    });

    on_cleanup(move || {
        if let Some(h) = handle.get_value() { bridge::destroy_code_mirror(&h); }
        closure_store.set_value(None); // drop closure
    });

    view! { <div node_ref=container class="editor-container" /> }
}
```

- [ ] **Step 1–4:** Create mod.rs, port source_editor, wysiwyg_editor, split_pane
- [ ] **Step 5:** Wire into app.rs with EditorMode switching
- [ ] **Step 6:** Verify — `trunk build`
- [ ] **Step 7: Commit**

---

## Task 12: Settings Components

**Files:**
- Create: `crates/rustynotes-frontend/src/components/settings/mod.rs`
- Create: `crates/rustynotes-frontend/src/components/settings/shared.rs`
- Create: `crates/rustynotes-frontend/src/components/settings/settings_window.rs`
- Create: `crates/rustynotes-frontend/src/components/settings/settings_sidebar.rs`
- Create: `crates/rustynotes-frontend/src/components/settings/categories/*.rs`

Port from `src/components/settings/` (303 lines). **Important:** also port the 5 shared components from `src/components/settings/shared/` (SettingRow, SettingToggle, SettingSlider, SettingSelect, SettingColorPicker) into `shared.rs`. These are reusable primitives used by all category components.

- [ ] **Step 1:** Create shared.rs with SettingRow, SettingToggle, SettingSlider, SettingSelect, SettingColorPicker
- [ ] **Step 2:** Port SettingsWindow and SettingsSidebar
- [ ] **Step 3:** Port AppearanceSettings, EditorSettings, PreviewSettings, AdvancedSettings
- [ ] **Step 4:** Wire SettingsView into app.rs router (`#/settings` route)
- [ ] **Step 5:** Verify — `trunk build`
- [ ] **Step 6: Commit**

---

## Task 13: Onboarding Components

Port from `src/components/onboarding/` (113 lines) + `src/lib/onboarding.ts` (53 lines). Uses `gloo_storage::LocalStorage` for persistence.

- [ ] **Step 1:** Port welcome.rs and feature_tip.rs
- [ ] **Step 2:** Wire into app.rs
- [ ] **Step 3:** Verify — `trunk build`
- [ ] **Step 4: Commit**

---

## Task 14: Integration — Vendor Bundle + Full Testing

- [ ] **Step 1: Run vendor bundling**

```bash
chmod +x js/bundle-vendor.sh && ./js/bundle-vendor.sh
```

Expected: `js/bridge.bundle.js` and `styles/katex.min.css` created

- [ ] **Step 2: Full build** — `trunk build --release`
- [ ] **Step 3: Integration test** — `cargo tauri dev`

Verify: theme applies, folder opens, file tree renders, editor modes work, preview renders markdown with syntax highlighting, KaTeX/Mermaid render, settings window opens, search works.

- [ ] **Step 4: Run all tests** — `cargo test --workspace`
- [ ] **Step 5: Commit vendor bundles**

```bash
git add js/bridge.bundle.js styles/katex.min.css
git commit -m "feat: add vendored JS bridge bundle and KaTeX CSS"
```

---

## Task 15: Cleanup — Remove Solid.js Frontend

- [ ] **Step 1: Verify Leptos frontend is fully functional** — `cargo tauri dev`
- [ ] **Step 2: Remove old files**

```bash
rm -rf src/ public/
rm -f package.json pnpm-lock.yaml tsconfig.json tsconfig.node.json vite.config.ts index.html
```

- [ ] **Step 3: Update .gitignore** — remove node_modules, add dist/ if not present
- [ ] **Step 4: Final build** — `trunk build --release && cargo tauri build`
- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove Solid.js frontend — fully replaced by Leptos"
```
