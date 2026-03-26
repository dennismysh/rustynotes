# Leptos Frontend Migration Design

Migrate the RustyNotes frontend from Solid.js (TypeScript) to Leptos (Rust/WASM), with a thin JS bridge for irreplaceable JS libraries. Goal: ~95% Rust, ~5% JS.

## Motivation

- Prefer writing Rust over TypeScript
- Performance gains from WASM-local markdown parsing (no IPC round-trip)
- Rust's type safety and correctness guarantees across the full stack
- Shared types between Tauri backend and Leptos frontend via a common crate

## Approach: Full Leptos Rewrite with JS Bridge

Leptos CSR replaces Solid.js entirely. A single `bridge.js` file (~100 lines) contains all JS library interop. Leptos components never touch JS libraries directly.

Comrak and syntect run in WASM (client-side), eliminating the `parse_markdown` IPC command. KaTeX and Mermaid remain as JS — no Rust equivalents exist.

## Workspace Structure

```
rustynotes/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── rustynotes-common/      # shared types (Config, FileTree, EditorMode, etc.)
│   └── rustynotes-frontend/    # Leptos CSR app (compiles to WASM)
├── src-tauri/                  # Tauri backend (depends on rustynotes-common)
├── js/
│   └── bridge.js               # thin JS bridge (~100 lines)
├── static/
│   └── index.html              # Trunk entry point
└── styles/                     # CSS (moved from src/styles/, unchanged)
```

### Crate Responsibilities

**`rustynotes-common`** — shared types between backend and frontend. `Config`, `FileNode`, `EditorMode`, `NavMode`, theme definitions. Serde-derives on everything. No platform-specific dependencies.

**`rustynotes-frontend`** — Leptos 0.7+ CSR app. Depends on `rustynotes-common`, `comrak` (WASM-safe), `syntect` (with `default-fancy`). Built by Trunk into a WASM bundle.

**`src-tauri`** — unchanged backend, depends on `rustynotes-common` instead of defining its own types. Tauri loads the Trunk-built output as its frontend dist.

### Build Flow

1. `trunk build` compiles `rustynotes-frontend` into `dist/` (WASM + index.html + bridge.js + CSS)
2. `cargo tauri build` bundles `dist/` into the Tauri app

Vite and node_modules are removed for the app shell. The only JS is `bridge.js` plus vendored scripts for CodeMirror, TipTap, KaTeX, and Mermaid.

## Frontend Component Architecture

```
crates/rustynotes-frontend/src/
├── main.rs                     # Leptos mount, router setup
├── app.rs                      # App shell (layout, mode switching)
├── state.rs                    # Global signals (RwSignal<AppState>)
├── tauri_ipc.rs                # wasm-bindgen externs for __TAURI__ IPC
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
│       ├── welcome.rs
│       └── feature_tip.rs
```

### State Management

Single `RwSignal<AppState>` provided via Leptos context, nearly 1:1 with the current `createRoot` + signals pattern in `state.ts`. The `AppState` struct lives in `rustynotes-common` so the backend can deserialize/serialize the same types.

### Tauri IPC from WASM

A `tauri_ipc.rs` module with `wasm-bindgen` externs that call `window.__TAURI__.core.invoke()`. Each command gets a typed async Rust function:

```rust
pub async fn read_file(path: &str) -> Result<String, JsValue> { ... }
pub async fn list_directory(path: &str) -> Result<Vec<FileNode>, JsValue> { ... }
pub async fn write_file(path: &str, content: &str) -> Result<(), JsValue> { ... }
pub async fn get_config() -> Result<Config, JsValue> { ... }
pub async fn save_config(config: &Config) -> Result<(), JsValue> { ... }
pub async fn search_files(root: &str, query: &str) -> Result<Vec<SearchResult>, JsValue> { ... }
pub async fn export_file(markdown: &str, path: &str, format: &str, theme: bool) -> Result<(), JsValue> { ... }
```

### Rendering Pipeline Change

Currently: `User types -> Leptos signal -> IPC -> Rust comrak -> IPC -> JS post-process (KaTeX/Mermaid/Shiki)`

After migration: `User types -> Leptos signal -> comrak (in WASM) -> syntect (in WASM) -> DOM update -> bridge.js post-process (KaTeX/Mermaid only)`

Preview rendering becomes synchronous from the frontend's perspective. No async IPC round-trip for markdown parsing. Shiki is fully replaced by syntect.

### Settings Window

Stays as a separate Tauri webview window. Loads the same WASM bundle with hash routing (`#/settings`) to render the settings UI. Communication back to the main window uses Tauri events (unchanged from current approach).

## JS Bridge Design

Single file, strict contract. Leptos components never touch JS libraries directly.

### Bridge API

```javascript
// js/bridge.js

// === Editors ===
mountCodeMirror(element, content, options, onChange)   -> handle
updateCodeMirror(handle, content)                      -> void
destroyCodeMirror(handle)                              -> void

mountTipTap(element, content, options, onChange)        -> handle
updateTipTap(handle, content)                          -> void
destroyTipTap(handle)                                  -> void

// === Renderers (post-processing) ===
renderKatex(element, latex, displayMode)               -> void
renderMermaid(element, code, theme)                    -> Promise<void>
```

### Leptos-side Bindings

```rust
#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    fn mountCodeMirror(el: &HtmlElement, content: &str, ...) -> JsValue;
    fn updateCodeMirror(handle: &JsValue, content: &str);
    fn destroyCodeMirror(handle: &JsValue);
    fn mountTipTap(el: &HtmlElement, content: &str, ...) -> JsValue;
    fn updateTipTap(handle: &JsValue, content: &str);
    fn destroyTipTap(handle: &JsValue);
    fn renderKatex(el: &HtmlElement, latex: &str, display_mode: bool);
    #[wasm_bindgen(catch)]
    async fn renderMermaid(el: &HtmlElement, code: &str, theme: &str) -> Result<(), JsValue>;
}
```

### Lifecycle Pattern

Editor components follow mount/destroy lifecycle:

```rust
#[component]
fn SourceEditor() -> impl IntoView {
    let container_ref = create_node_ref::<Div>();
    let handle = store_value(None::<JsValue>);

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            let h = mountCodeMirror(&el, &content.get(), ...);
            handle.set_value(Some(h));
        }
    });

    on_cleanup(move || {
        if let Some(h) = handle.get_value() {
            destroyCodeMirror(&h);
        }
    });

    view! { <div node_ref=container_ref class="editor-container"/> }
}
```

### Bridge Constraints

- No state management — receives content, fires `onChange` callbacks
- No styling — editors styled via CSS (same as today)
- No initialization of comrak or syntect — those run in WASM directly
- KaTeX and Mermaid are lazy-loaded inside the bridge

## Data Flow

### Path 1: WASM-local (no IPC) — new with this migration

```
User types -> Leptos signal update -> comrak parse (in WASM) -> syntect highlight (in WASM) -> DOM update
```

Preview rendering with no async IPC round-trip. Perf win for live preview.

### Path 2: Tauri IPC (file system, config, export)

```
Leptos component -> tauri_ipc::read_file() -> wasm-bindgen -> __TAURI__.core.invoke() -> Rust backend -> response -> deserialize into shared types
```

File I/O, config persistence, file watching, export, and wikilink resolution stay as Tauri commands. `rustynotes-common` means responses deserialize directly into the same Rust structs.

### Path 3: JS bridge (editors, KaTeX, Mermaid)

```
Leptos component -> wasm-bindgen extern -> bridge.js -> JS library -> DOM mutation
Content changes: JS library -> onChange callback -> wasm-bindgen closure -> Leptos signal update
```

### Tauri Event Listening from WASM

```rust
pub fn listen_file_changed(on_change: impl Fn(String) + 'static) {
    let closure = Closure::wrap(Box::new(on_change) as Box<dyn Fn(String)>);
    // js_sys call to __TAURI__.event.listen("file-changed", closure)
}
```

### Backend Changes

- `parse_markdown` command — remove `#[tauri::command]` registration. Keep the function for the HTML exporter.
- Backend depends on `rustynotes-common` instead of defining its own types.
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
| `wasm-bindgen` closure memory leaks | Editor onChange callbacks create `Closure` objects | mount/destroy lifecycle handles this — `destroyCodeMirror` drops the closure. |
| CodeMirror/TipTap version upgrades | Bridge API coupled to their APIs | Bridge functions are thin wrappers — update bridge.js, not Rust code. Surface area is ~6 functions. |
| Leptos pre-1.0 API churn | APIs can shift between versions | Pin to specific version. Previously navigated with BeanieAndPen (0.8 view! macro changes). |
| syntect WASM compatibility | Oniguruma doesn't compile to wasm32 | Known solved: `default-features = false, features = ["default-fancy"]`. |

## What Stays Unchanged

- All CSS and theme files — carried over as-is
- Tauri config, capabilities, permissions
- Tauri backend commands (minus `parse_markdown` IPC registration)
- App window management, settings window pattern
- File watcher, export system

## Final Code Distribution

| Component | Language | Approx. Lines |
|---|---|---|
| Tauri backend | Rust | ~865 |
| Leptos frontend | Rust | ~1,400 |
| JS bridge | JavaScript | ~100 |
| CSS/themes | CSS | ~985 |
| **Total Rust** | | **~95%** |
| **Total JS** | | **~5%** |
