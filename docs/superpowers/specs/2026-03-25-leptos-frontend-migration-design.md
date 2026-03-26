# Leptos Frontend Migration Design

Migrate the RustyNotes frontend from Solid.js (TypeScript) to Leptos (Rust/WASM), with a thin JS bridge for irreplaceable JS libraries. Goal: ~95% Rust, ~5% JS (excluding CSS).

## Motivation

- Prefer writing Rust over TypeScript
- Performance gains from WASM-local markdown parsing (no IPC round-trip)
- Rust's type safety and correctness guarantees across the full stack
- Shared types between Tauri backend and Leptos frontend via a common crate

## Approach: Full Leptos Rewrite with JS Bridge

Leptos CSR replaces Solid.js entirely. A single `bridge.js` file (~150 lines) contains all JS library interop. Leptos components never touch JS libraries directly.

Comrak and syntect run in WASM (client-side), eliminating the `parse_markdown` IPC command. KaTeX and Mermaid remain as JS — no Rust equivalents exist.

## Workspace Structure

```
rustynotes/
├── Cargo.toml                  # workspace root
├── Trunk.toml                  # Trunk build config
├── crates/
│   ├── rustynotes-common/      # shared types (Config, FileTree, EditorMode, etc.)
│   └── rustynotes-frontend/    # Leptos CSR app (compiles to WASM)
├── src-tauri/                  # Tauri backend (depends on rustynotes-common)
├── js/
│   ├── bridge.js               # thin JS bridge (~150 lines)
│   └── vendor/                 # pre-bundled JS libraries (see JS Bundling Strategy)
├── static/
│   └── index.html              # Trunk entry point
└── styles/                     # CSS (moved from src/styles/, unchanged)
```

### Crate Responsibilities

**`rustynotes-common`** — shared plain data types between backend and frontend. `Config`, `FileNode`, `SearchResult`, `EditorMode`, `NavMode`, theme definitions. Serde-derives on everything. No platform-specific dependencies. All path fields use `String` (not `PathBuf`) for wasm32 compatibility. Does NOT contain reactive state containers — only the underlying data structs.

**`rustynotes-frontend`** — Leptos 0.7+ CSR app. Depends on `rustynotes-common`, `comrak` (WASM-safe), `syntect` (with `default-features = false, features = ["default-fancy"]`), `gloo-storage` (for localStorage access). Built by Trunk into a WASM bundle. Wraps common types in `RwSignal`s for reactive state.

**`src-tauri`** — unchanged backend, depends on `rustynotes-common` instead of defining its own types. Backend's `FileEntry` converts `PathBuf` to `String` at the serialization boundary. Tauri loads the Trunk-built output as its frontend dist.

### Trunk Configuration

```toml
# Trunk.toml
[build]
target = "static/index.html"
dist = "dist"

[watch]
watch = ["crates/rustynotes-frontend/src", "js", "styles"]
```

### Build Flow

1. `trunk build` compiles `rustynotes-frontend` into `dist/` (WASM + index.html + bridge.js + vendor JS + CSS)
2. `cargo tauri build` bundles `dist/` into the Tauri app

### Tauri Configuration Changes

`src-tauri/tauri.conf.json` must be updated:

```json
{
  "build": {
    "beforeDevCommand": "trunk serve --port 1420",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "trunk build --release",
    "frontendDist": "../dist"
  }
}
```

## JS Bundling Strategy

Vite and node_modules are removed. The JS libraries that `bridge.js` depends on are pre-bundled into self-contained ESM files:

**One-time bundling step** using esbuild (run manually when upgrading library versions):

```bash
# js/bundle-vendor.sh
npx esbuild node_modules/@codemirror/view/dist/index.js \
  --bundle --format=esm --outfile=js/vendor/codemirror.bundle.js
npx esbuild node_modules/@tiptap/core/dist/index.js \
  --bundle --format=esm --outfile=js/vendor/tiptap.bundle.js
# KaTeX and Mermaid are lazy-loaded via dynamic import() from vendor/
npx esbuild node_modules/katex/dist/katex.mjs \
  --bundle --format=esm --outfile=js/vendor/katex.bundle.js
npx esbuild node_modules/mermaid/dist/mermaid.esm.min.mjs \
  --bundle --format=esm --outfile=js/vendor/mermaid.bundle.js
```

The `js/vendor/` directory is committed to the repo. No JS tooling is needed at dev time or in CI — `trunk build` copies these files via `<link data-trunk rel="copy-file">` directives in `index.html`. Version upgrades mean re-running the bundle script and committing the updated files.

`bridge.js` imports from these vendor bundles:

```javascript
import { EditorView, ... } from './vendor/codemirror.bundle.js';
import { Editor, ... } from './vendor/tiptap.bundle.js';
// KaTeX and Mermaid lazy-loaded:
// const katex = await import('./vendor/katex.bundle.js');
```

## Frontend Component Architecture

```
crates/rustynotes-frontend/src/
├── main.rs                     # Leptos mount, router setup (leptos_router, hash mode)
├── app.rs                      # App shell (layout, mode switching)
├── state.rs                    # Global signals (RwSignal wrapping common types)
├── tauri_ipc.rs                # wasm-bindgen externs for __TAURI__ IPC
├── bridge.rs                   # wasm-bindgen externs for bridge.js
├── theme.rs                    # CSS custom property application via web-sys
├── components/
│   ├── toolbar.rs
│   ├── editor/
│   │   ├── source_editor.rs    # Mounts CodeMirror via bridge
│   │   ├── wysiwyg_editor.rs   # Mounts TipTap via bridge
│   │   └── split_pane.rs
│   ├── preview/
│   │   └── preview.rs          # comrak + syntect in WASM, KaTeX/Mermaid via bridge
│   ├── navigation/
│   │   ├── sidebar.rs
│   │   ├── miller_columns.rs
│   │   └── breadcrumb.rs
│   ├── settings/
│   │   ├── settings_window.rs
│   │   ├── settings_sidebar.rs
│   │   └── categories/
│   │       ├── appearance.rs
│   │       ├── editor.rs
│   │       ├── preview.rs
│   │       └── advanced.rs
│   └── onboarding/
│       ├── welcome.rs          # uses gloo-storage for localStorage state
│       └── feature_tip.rs
```

### State Management

Single `RwSignal<AppState>` provided via Leptos context, nearly 1:1 with the current `createRoot` + signals pattern in `state.ts`. The plain data structs (`Config`, `FileNode`, etc.) live in `rustynotes-common`. The `AppState` struct with its `RwSignal` wrappers lives in `rustynotes-frontend/src/state.rs` — it is frontend-specific and not shared with the backend.

### Routing

Uses `leptos_router` with hash-based routing:

- `#/` — main editor view
- `#/settings` — settings UI (loaded in the separate Tauri settings window)

Both the main window and settings window load the same WASM bundle; the route determines which component tree renders.

### Tauri IPC from WASM

A `tauri_ipc.rs` module with `wasm-bindgen` externs that call `window.__TAURI__.core.invoke()`. Complete command list matching the current backend:

```rust
// File system
pub async fn read_file(path: &str) -> Result<String, JsValue> { ... }
pub async fn write_file(path: &str, content: &str) -> Result<(), JsValue> { ... }
pub async fn list_directory(path: &str) -> Result<Vec<FileNode>, JsValue> { ... }
pub async fn resolve_wikilink(root: &str, name: &str) -> Result<Option<String>, JsValue> { ... }
pub async fn search_files(root: &str, query: &str) -> Result<Vec<SearchResult>, JsValue> { ... }
pub async fn watch_folder(path: &str) -> Result<(), JsValue> { ... }

// Config
pub async fn get_config() -> Result<Config, JsValue> { ... }
pub async fn save_config_cmd(config_data: &str) -> Result<(), JsValue> { ... }
pub async fn open_settings() -> Result<(), JsValue> { ... }

// Export
pub async fn export_file(markdown: &str, path: &str, format: &str, theme: bool) -> Result<(), JsValue> { ... }

// Tauri plugin APIs (dialog, window)
pub async fn open_folder_dialog() -> Result<Option<String>, JsValue> { ... }
pub async fn save_file_dialog(default_name: &str) -> Result<Option<String>, JsValue> { ... }
pub fn show_current_window() { ... }

// Event listeners
pub fn listen_file_changed(on_change: impl Fn(String) + 'static) { ... }
pub fn listen_config_changed(on_change: impl Fn(Config) + 'static) { ... }
```

Dialog and window APIs call into `__TAURI__.dialog` and `__TAURI__.window` plugin namespaces via the same `wasm-bindgen` / `js_sys` pattern as IPC commands.

### Rendering Pipeline Change

Currently: `User types -> Solid.js signal -> IPC -> Rust comrak -> IPC -> JS post-process (KaTeX/Mermaid/Shiki)`

After migration: `User types -> Leptos signal -> comrak (in WASM) -> syntect (in WASM) -> DOM update -> bridge.js post-process (KaTeX/Mermaid only)`

Preview rendering becomes synchronous from the frontend's perspective. No async IPC round-trip for markdown parsing. Shiki is fully replaced by syntect.

### Settings Window

Stays as a separate Tauri webview window. Loads the same WASM bundle with hash routing (`#/settings`) to render the settings UI. Communication back to the main window uses Tauri events (unchanged from current approach).

### Onboarding State

The current `localStorage`-based onboarding system (welcome state, feature tip tracking) uses `gloo-storage` from WASM for `localStorage` access. This is a direct replacement — `gloo-storage::LocalStorage::get/set` instead of `window.localStorage.getItem/setItem`.

## JS Bridge Design

Single file, strict contract. Leptos components never touch JS libraries directly.

### Bridge API

```javascript
// js/bridge.js

// === Editors ===
// onChange receives the full content string: onChange(content: string)
mountCodeMirror(element, content, options, onChange)   -> handle
updateCodeMirror(handle, content)                      -> void
focusCodeMirror(handle)                                -> void
destroyCodeMirror(handle)                              -> void

mountTipTap(element, content, options, onChange)        -> handle
// onChange receives markdown string: onChange(markdown: string)
updateTipTap(handle, content)                          -> void
getTipTapMarkdown(handle)                              -> string
focusTipTap(handle)                                    -> void
destroyTipTap(handle)                                  -> void

// === Renderers (post-processing) ===
renderKatex(element, latex, displayMode)               -> void
renderMermaid(element, code, theme)                    -> Promise<void>
```

**`options` parameter for `mountCodeMirror`:**
```javascript
{
  lineNumbers: bool,       // show line numbers (default true)
  highlightActiveLine: bool, // highlight current line (default true)
  theme: string,           // "light" | "dark"
  keymaps: string[],       // additional keymaps (e.g., ["search", "history"])
}
```

**`options` parameter for `mountTipTap`:**
```javascript
{
  theme: string,           // "light" | "dark"
  taskLists: bool,         // enable task list extension (default true)
  codeBlocks: bool,        // enable code block extension with lowlight (default true)
}
```

**`onChange` callback payload:** Both editors pass the full document content as a string. For CodeMirror, this is `view.state.doc.toString()`. For TipTap, this is `editor.storage.markdown.getMarkdown()`. The Leptos side receives a `String` and updates the content signal directly.

### Leptos-side Bindings

```rust
// bridge.rs
#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    fn mountCodeMirror(el: &HtmlElement, content: &str, options: &JsValue, on_change: &Closure<dyn Fn(String)>) -> JsValue;
    fn updateCodeMirror(handle: &JsValue, content: &str);
    fn focusCodeMirror(handle: &JsValue);
    fn destroyCodeMirror(handle: &JsValue);

    fn mountTipTap(el: &HtmlElement, content: &str, options: &JsValue, on_change: &Closure<dyn Fn(String)>) -> JsValue;
    fn updateTipTap(handle: &JsValue, content: &str);
    fn getTipTapMarkdown(handle: &JsValue) -> String;
    fn focusTipTap(handle: &JsValue);
    fn destroyTipTap(handle: &JsValue);

    fn renderKatex(el: &HtmlElement, latex: &str, display_mode: bool);
    #[wasm_bindgen(catch)]
    async fn renderMermaid(el: &HtmlElement, code: &str, theme: &str) -> Result<(), JsValue>;
}
```

### Lifecycle Pattern

Editor components follow mount/destroy lifecycle. The `Closure` for `onChange` must be stored alongside the handle and dropped in `on_cleanup`:

```rust
#[component]
fn SourceEditor() -> impl IntoView {
    let container_ref = create_node_ref::<Div>();
    let handle = store_value(None::<JsValue>);
    let on_change_closure = store_value(None::<Closure<dyn Fn(String)>>);

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            let closure = Closure::wrap(Box::new(move |content: String| {
                // Update Leptos signal with new content
                set_content.set(content);
            }) as Box<dyn Fn(String)>);

            let h = mountCodeMirror(&el, &content.get(), &options, &closure);
            handle.set_value(Some(h));
            on_change_closure.set_value(Some(closure));  // prevent GC
        }
    });

    on_cleanup(move || {
        if let Some(h) = handle.get_value() {
            destroyCodeMirror(&h);
        }
        // Closure dropped here — prevents wasm-bindgen leak
        on_change_closure.set_value(None);
    });

    view! { <div node_ref=container_ref class="editor-container"/> }
}
```

### Bridge Constraints

- No state management — receives content, fires `onChange` callbacks with full content string
- No styling — editors styled via CSS (same as today)
- No initialization of comrak or syntect — those run in WASM directly
- KaTeX and Mermaid are lazy-loaded inside the bridge via dynamic `import()`

## Data Flow

### Path 1: WASM-local (no IPC) — new with this migration

```
User types -> Leptos signal update -> comrak parse (in WASM) -> syntect highlight (in WASM) -> DOM update
```

Preview rendering with no async IPC round-trip. Perf win for live preview.

### Path 2: Tauri IPC (file system, config, export, dialogs)

```
Leptos component -> tauri_ipc::read_file() -> wasm-bindgen -> __TAURI__.core.invoke() -> Rust backend -> response -> deserialize into shared types
```

File I/O, config persistence, file watching, export, wikilink resolution, and native dialogs stay as Tauri commands/plugin APIs. `rustynotes-common` means responses deserialize directly into the same Rust structs.

### Path 3: JS bridge (editors, KaTeX, Mermaid)

```
Leptos component -> wasm-bindgen extern -> bridge.js -> JS library -> DOM mutation
Content changes: JS library -> onChange(content_string) -> wasm-bindgen Closure -> Leptos signal update
```

### Tauri Event Listening from WASM

Both `file-changed` and `config-changed` events are registered:

```rust
pub fn listen_file_changed(on_change: impl Fn(String) + 'static) {
    let closure = Closure::wrap(Box::new(on_change) as Box<dyn Fn(String)>);
    // js_sys call to __TAURI__.event.listen("file-changed", closure)
    closure.forget(); // long-lived listener, intentional leak
}

pub fn listen_config_changed(on_change: impl Fn(Config) + 'static) {
    let closure = Closure::wrap(Box::new(move |payload: JsValue| {
        if let Ok(config) = serde_wasm_bindgen::from_value(payload) {
            on_change(config);
        }
    }) as Box<dyn Fn(JsValue)>);
    // js_sys call to __TAURI__.event.listen("config-changed", closure)
    closure.forget();
}
```

### Backend Changes

- `parse_markdown` command — remove `#[tauri::command]` registration. Keep the function for the HTML exporter.
- Backend depends on `rustynotes-common` instead of defining its own types.
- Backend's `FileEntry` converts `PathBuf` fields to `String` at serialization boundaries (the common crate's `FileNode` uses `String` for wasm32 compatibility).
- All other commands unchanged.

## Testing Strategy

- **`rustynotes-common`** — standard `#[cfg(test)]` unit tests. Serialization round-trips, type invariants. `cargo test`.
- **`rustynotes-frontend`** — `wasm-pack test --headless --chrome` for DOM-touching components. Pure logic (comrak/syntect rendering) testable with `cargo test`.
- **JS bridge** — tested indirectly via frontend WASM tests that mount editors and verify callbacks.
- **`src-tauri` backend** — unchanged, existing tests continue to work.
- **Integration** — `cargo tauri dev` for manual end-to-end.

## Known Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Trunk + Tauri dev loop slower than Vite HMR | DX regression — WASM recompile is seconds not milliseconds | `trunk serve --watch` with incremental compilation. Accept the tradeoff. |
| comrak + syntect WASM binary size | ~2MB WASM binary (proven in BeanieAndPen) | Acceptable for desktop app. `wasm-opt -Oz` can trim further. Tauri bundles are already 10MB+. |
| `wasm-bindgen` closure memory leaks | Editor `onChange` callbacks create `Closure` objects that must be explicitly managed | Store `Closure` alongside editor handle; drop both in `on_cleanup`. Event listeners use `closure.forget()` (intentional, app-lifetime). |
| CodeMirror/TipTap version upgrades | Bridge API coupled to their APIs | Bridge functions are thin wrappers — update bridge.js, not Rust code. Re-run `bundle-vendor.sh` and commit. |
| Leptos pre-1.0 API churn | APIs can shift between versions | Pin to specific version. Previously navigated with BeanieAndPen (0.8 view! macro changes). |
| syntect WASM compatibility | Oniguruma doesn't compile to wasm32 | Known solved: `syntect = { version = "5", default-features = false, features = ["default-fancy"] }` |
| TipTap markdown round-trip fidelity | Bridge must support reading markdown back out, not just pushing content in | `getTipTapMarkdown(handle)` in bridge API + onChange passes markdown string |

## What Stays Unchanged

- All CSS and theme files — carried over as-is
- Tauri capabilities, permissions
- Tauri backend commands (minus `parse_markdown` IPC registration)
- App window management, settings window pattern
- File watcher, export system

## Final Code Distribution

Excluding CSS (framework-agnostic, carries over unchanged):

| Component | Language | Approx. Lines |
|---|---|---|
| Tauri backend | Rust | ~865 |
| Leptos frontend | Rust | ~1,600–2,000 |
| JS bridge + vendor bundling | JavaScript | ~150 |
| **Rust share of app code** | | **~93–95%** |
| **JS share of app code** | | **~5–7%** |
