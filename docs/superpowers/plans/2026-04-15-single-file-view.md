# Single-File View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users open a single markdown file in RustyNotes without opening its parent folder, via Finder file association, drag-and-drop, Cmd+O, or CLI argument. Each open spawns a minimal single-file window alongside any existing folder window.

**Architecture:** A new `/file` Leptos route renders a `SingleFileView` with a slim titlebar (traffic lights + filename + save indicator + ••• overflow menu) and the WYSIWYG editor. The Rust backend adds a `FileWindows` Tauri-managed state that maps canonical paths to window labels for duplicate-window focus, an `open_file_in_new_window` command, the `tauri-plugin-single-instance` plugin to route re-launches into the running instance, a macOS `RunEvent::Opened` handler for Finder events, startup-arg parsing, drag-and-drop listeners, and a native macOS File menu that owns the `Cmd+N`/`Cmd+O`/`Cmd+Shift+O`/`Cmd+S` accelerators. Shared UI (filename + save indicator) is extracted from the existing `Toolbar` into `SaveIndicator` to be reused by the slim titlebar.

**Tech Stack:** Rust 2021, Tauri 2, Leptos 0.7 (CSR/WASM), `tauri-plugin-single-instance`, serde, `std::fs::canonicalize`, `uuid`.

**Spec:** `docs/superpowers/specs/2026-04-14-single-file-view-design.md`

---

## File inventory

### Created

- `crates/rustynotes-frontend/src/components/single_file/mod.rs` — `SingleFileView` component (reads `?path=` query param, triggers load, renders slim titlebar + editor)
- `crates/rustynotes-frontend/src/components/single_file/slim_titlebar.rs` — minimal titlebar (traffic lights + filename + save indicator + ••• button)
- `crates/rustynotes-frontend/src/components/single_file/overflow_menu.rs` — dropdown with Switch to Source / Open in folder window / Settings / Export HTML
- `crates/rustynotes-frontend/src/components/save_indicator.rs` — extracted filename + save-status rendering (used by both `Toolbar` and `SlimTitleBar`)
- `src-tauri/src/commands/window_mgmt.rs` — `open_file_in_new_window`, `open_file_dialog`, `open_folder_in_window`; `FileWindows` state; pure helpers for `recent_files` list management (push-dedup-cap, prune-missing)
- `src-tauri/src/menu.rs` — native macOS File menu builder + event emitters + dynamic Open Recent rebuild

### Modified

- `crates/rustynotes-common/src/lib.rs` — add `recent_files: Vec<String>` to `AppConfig`
- `crates/rustynotes-frontend/src/app.rs` — add `/file` route
- `crates/rustynotes-frontend/src/components/mod.rs` — export `single_file` + `save_indicator` modules
- `crates/rustynotes-frontend/src/components/toolbar.rs` — use the extracted `SaveIndicator`
- `crates/rustynotes-frontend/src/components/onboarding/welcome.rs` — add "Open File" button + Recent Files section
- `crates/rustynotes-frontend/src/save.rs` — remove JS `keydown` handlers for `Cmd+S` / `Cmd+N` (moved to menu events); listen for `menu:*` events instead
- `crates/rustynotes-frontend/src/tauri_ipc.rs` — wrappers for new commands + event listeners
- `src-tauri/src/commands/mod.rs` — register `window_mgmt` module
- `src-tauri/src/lib.rs` — register plugin/state/menu/run-event/startup-arg wiring
- `src-tauri/Cargo.toml` — add `tauri-plugin-single-instance` + `uuid`
- `src-tauri/tauri.conf.json` — add `fileAssociations` bundle entry
- `src-tauri/capabilities/default.json` — extend `windows` to include `"file-*"` glob

---

## Task 1: Add `recent_files` field to `AppConfig`

**Files:**
- Modify: `crates/rustynotes-common/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/rustynotes-common/src/lib.rs`:

```rust
    #[test]
    fn test_recent_files_defaults_to_empty() {
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert!(config.recent_files.is_empty());
    }

    #[test]
    fn test_recent_files_roundtrip() {
        let json = r#"{"recent_files":["/a/b.md","/c/d.md"]}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.recent_files, vec!["/a/b.md", "/c/d.md"]);
    }
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test -p rustynotes-common test_recent_files`
Expected: compile error or test failure (field doesn't exist).

- [ ] **Step 3: Add the field to `AppConfig`**

In `crates/rustynotes-common/src/lib.rs`, inside `struct AppConfig`, add after `recent_folders`:

```rust
    #[serde(default)]
    pub recent_files: Vec<String>,
```

In `impl Default for AppConfig`, add to the `Self { ... }` literal:

```rust
            recent_files: Vec::new(),
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test -p rustynotes-common`
Expected: all tests pass, including the two new ones.

- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-common/src/lib.rs
git commit -m "feat: add recent_files field to AppConfig"
```

---

## Task 2: Add pure helpers for `recent_files` list management

**Files:**
- Create: `src-tauri/src/commands/window_mgmt.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Register the new module**

Edit `src-tauri/src/commands/mod.rs`, add to the list of `pub mod` declarations (alphabetical):

```rust
pub mod window_mgmt;
```

- [ ] **Step 2: Create the new module with helper functions and tests**

Create `src-tauri/src/commands/window_mgmt.rs`:

```rust
//! Window-management commands: opening files/folders in new windows,
//! tracking path → window-label associations, and maintaining the
//! `recent_files` list in config.

use std::path::{Path, PathBuf};

/// Push `path` to the front of `list`, dedup by equality, cap at `cap`
/// entries. Returns true if the list changed.
pub fn push_recent(list: &mut Vec<String>, path: String, cap: usize) -> bool {
    if list.first().map(|s| s == &path).unwrap_or(false) {
        return false;
    }
    list.retain(|p| p != &path);
    list.insert(0, path);
    if list.len() > cap {
        list.truncate(cap);
    }
    true
}

/// Remove entries from `list` whose paths no longer exist on disk.
/// Returns true if the list changed.
pub fn prune_missing(list: &mut Vec<String>) -> bool {
    let before = list.len();
    list.retain(|p| Path::new(p).exists());
    list.len() != before
}

/// Canonicalize (resolve symlinks + make absolute). Falls back to the
/// input if the path doesn't exist or can't be canonicalized.
pub fn canonicalize_or_same(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_adds_new_entry_at_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/c".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/c", "/a", "/b"]);
    }

    #[test]
    fn push_recent_moves_existing_entry_to_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
        let changed = push_recent(&mut list, "/b".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/b", "/a", "/c"]);
    }

    #[test]
    fn push_recent_no_op_if_already_first() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/a".to_string(), 10);
        assert!(!changed);
        assert_eq!(list, vec!["/a", "/b"]);
    }

    #[test]
    fn push_recent_caps_length() {
        let mut list: Vec<String> = (0..10).map(|i| format!("/{i}")).collect();
        push_recent(&mut list, "/new".to_string(), 10);
        assert_eq!(list.len(), 10);
        assert_eq!(list[0], "/new");
        assert_eq!(list[9], "/8");
    }

    #[test]
    fn prune_missing_removes_nonexistent() {
        let mut list = vec!["/definitely/not/a/real/path.md".to_string()];
        let changed = prune_missing(&mut list);
        assert!(changed);
        assert!(list.is_empty());
    }

    #[test]
    fn prune_missing_keeps_existing() {
        // Use a path that definitely exists on the build machine.
        let tmp = std::env::temp_dir();
        let mut list = vec![tmp.to_string_lossy().into_owned()];
        let changed = prune_missing(&mut list);
        assert!(!changed);
        assert_eq!(list.len(), 1);
    }
}
```

- [ ] **Step 3: Run tests, verify they pass**

Run: `cargo test -p rustynotes --lib window_mgmt`
Expected: all six tests pass. (The `-p rustynotes` selects the Tauri crate; adjust the crate name if `cargo metadata` shows something different.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/mod.rs src-tauri/src/commands/window_mgmt.rs
git commit -m "feat: add recent_files list helpers with tests"
```

---

## Task 3: Add `uuid` + `tauri-plugin-single-instance` dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml`, under `[dependencies]`, add:

```toml
tauri-plugin-single-instance = { version = "2", features = ["deep-link"] }
uuid = { version = "1", features = ["v4"] }
```

(The `deep-link` feature is the recommended variant; if the build complains about the feature flag, drop it — the base plugin covers our needs.)

- [ ] **Step 2: Verify the build still compiles**

Run: `cargo build -p rustynotes`
Expected: build succeeds (may download new crates on first run).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add tauri-plugin-single-instance and uuid dependencies"
```

---

## Task 4: Add `FileWindows` managed state + `open_file_in_new_window` command

**Files:**
- Modify: `src-tauri/src/commands/window_mgmt.rs`

- [ ] **Step 1: Add the `FileWindows` state struct and the command**

Append to `src-tauri/src/commands/window_mgmt.rs` (after the pure helpers, before the `#[cfg(test)]` module):

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use crate::commands::config::ConfigState;
use crate::config as config_io;

const RECENT_FILES_CAP: usize = 10;

/// Maps canonical file paths to the label of the single-file window
/// currently displaying them. Used to focus-instead-of-spawn when a
/// duplicate open is requested.
pub struct FileWindows {
    map: Mutex<HashMap<PathBuf, String>>,
}

impl FileWindows {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Path) -> Option<String> {
        self.map.lock().unwrap().get(path).cloned()
    }

    pub fn insert(&self, path: PathBuf, label: String) {
        self.map.lock().unwrap().insert(path, label);
    }

    pub fn remove_by_label(&self, label: &str) {
        self.map.lock().unwrap().retain(|_, v| v != label);
    }
}

/// Spawn (or focus) a single-file window for `path`. Canonicalizes,
/// validates existence and UTF-8 readability, dedup-focuses existing
/// windows for the same file, updates `recent_files`.
#[tauri::command]
pub fn open_file_in_new_window(
    app: AppHandle,
    path: String,
    file_windows: tauri::State<FileWindows>,
    config_state: tauri::State<ConfigState>,
) -> Result<(), String> {
    let canonical = canonicalize_or_same(&path);

    // Validate: file exists and is UTF-8 readable.
    if !canonical.exists() {
        return Err(format!("File not found: {}", canonical.display()));
    }
    std::fs::read_to_string(&canonical)
        .map_err(|e| format!("Cannot read file as UTF-8: {e}"))?;

    // Focus existing window if one is already open for this path.
    if let Some(label) = file_windows.get(&canonical) {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_focus();
            return Ok(());
        }
        // Stale entry — the window was closed. Fall through to respawn.
        file_windows.remove_by_label(&label);
    }

    // Build the /file?path=... URL (URL-encoded).
    let encoded = urlencoding::encode(&canonical.to_string_lossy());
    let url = format!("/file?path={encoded}");
    let label = format!("file-{}", Uuid::new_v4().simple());

    let canonical_str = canonical.to_string_lossy().into_owned();
    let filename = canonical
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string());

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(&filename)
        .inner_size(800.0, 650.0)
        .min_inner_size(400.0, 300.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    file_windows.insert(canonical, label);

    // Update recent_files.
    let mut config = config_state.config.lock().unwrap();
    if push_recent(&mut config.recent_files, canonical_str, RECENT_FILES_CAP) {
        config_io::save_config(&config).map_err(|e| e.to_string())?;
        let _ = app.emit("config-changed", config.clone());
    }

    Ok(())
}

/// Show the system open-file dialog filtered to markdown extensions.
/// Returns the selected path (or None if the user cancels).
#[tauri::command]
pub async fn open_file_dialog(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .pick_file(move |file| {
            let _ = tx.send(file.map(|p| p.to_string()));
        });

    rx.await.map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Add `urlencoding` and `tokio` dependencies**

Edit `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
urlencoding = "2"
tokio = { version = "1", features = ["sync"] }
```

- [ ] **Step 3: Register the commands and state in `lib.rs`**

Edit `src-tauri/src/lib.rs`. In the `run()` function, add to `.manage(...)` chain (after the existing `.manage` calls):

```rust
        .manage(commands::window_mgmt::FileWindows::new())
```

In the `tauri::generate_handler![...]` list, add:

```rust
            commands::window_mgmt::open_file_in_new_window,
            commands::window_mgmt::open_file_dialog,
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p rustynotes`
Expected: builds successfully.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands/window_mgmt.rs src-tauri/src/lib.rs
git commit -m "feat: add open_file_in_new_window and open_file_dialog commands"
```

---

## Task 5: Extract `SaveIndicator` shared component

**Files:**
- Create: `crates/rustynotes-frontend/src/components/save_indicator.rs`
- Modify: `crates/rustynotes-frontend/src/components/mod.rs`
- Modify: `crates/rustynotes-frontend/src/components/toolbar.rs`

- [ ] **Step 1: Create the shared component**

Create `crates/rustynotes-frontend/src/components/save_indicator.rs`:

```rust
//! Shared filename + save-status indicator used by both the full
//! toolbar (folder windows) and the slim titlebar (single-file
//! windows).

use leptos::prelude::*;

use crate::state::{use_app_state, SaveStatus};

/// Extract the filename from a path (last `/`-separated segment).
fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[component]
pub fn SaveIndicator() -> impl IntoView {
    let state = use_app_state();
    let active_file_path = state.active_file_path;
    let is_dirty = state.is_dirty;
    let save_status = state.save_status;

    let active_filename = Memo::new(move |_| {
        active_file_path
            .get()
            .as_deref()
            .map(filename_from_path)
            .map(String::from)
    });

    view! {
        <div class="toolbar-filename">
            {move || {
                let status = save_status.get();
                let dirty = is_dirty.get();
                match status {
                    SaveStatus::Saving => {
                        view! { <span class="save-indicator saving" aria-label="Saving">{"\u{21BB}"}</span> }.into_any()
                    }
                    SaveStatus::Saved => {
                        view! { <span class="save-indicator saved" aria-label="Saved">{"\u{2713}"}</span> }.into_any()
                    }
                    SaveStatus::Error(ref msg) => {
                        let title = msg.clone();
                        view! { <span class="save-indicator error" title=title aria-label="Save error">{"\u{26A0}"}</span> }.into_any()
                    }
                    SaveStatus::Idle if dirty => {
                        view! { <span class="dirty-indicator" aria-label="Unsaved changes" /> }.into_any()
                    }
                    _ => {
                        view! { <span /> }.into_any()
                    }
                }
            }}
            <span
                class="toolbar-filename-text"
                title=move || active_file_path.get().unwrap_or_default()
            >
                {move || {
                    let name = active_filename.get().unwrap_or_default();
                    let path = active_file_path.get();
                    if path.is_none() && name.is_empty() {
                        "Untitled".to_string()
                    } else {
                        name
                    }
                }}
            </span>
        </div>
    }
}
```

- [ ] **Step 2: Register the module**

Edit `crates/rustynotes-frontend/src/components/mod.rs`, add at the top (near other `pub mod` declarations):

```rust
pub mod save_indicator;
```

- [ ] **Step 3: Use the shared component in `toolbar.rs`**

Edit `crates/rustynotes-frontend/src/components/toolbar.rs`. At the top, remove the now-unused helper `filename_from_path` **only if** it's unused after this change (it's still used for the export filename — keep it). Also remove the `stem_from_filename` — still used for export. So only the JSX block for the filename/indicator changes.

Find the block (~line 319–358 of `toolbar.rs`):

```rust
            <Show when=move || active_filename.get().is_some() || active_file_path.get().is_none()>
                <div class="toolbar-filename">
                    {move || {
                        let status = save_status.get();
                        let dirty = is_dirty.get();
                        match status {
                            SaveStatus::Saving => { ... }
                            ...
                        }
                    }}
                    <span
                        class="toolbar-filename-text"
                        title=move || active_file_path.get().unwrap_or_default()
                    >
                        { ... }
                    </span>
                </div>
            </Show>
```

Replace with:

```rust
            <crate::components::save_indicator::SaveIndicator />
```

(The outer `<Show>` is dropped: its condition `active_filename.get().is_some() || active_file_path.get().is_none()` is a tautology — whenever path is Some, filename is Some; whenever path is None, the right-hand side is true. So the wrapper always renders. SaveIndicator handles the "no path" case internally by rendering "Untitled".)

- [ ] **Step 4: Remove now-unused signal bindings from toolbar.rs**

In `toolbar.rs`, after the replacement, the locals `active_filename`, and possibly `is_dirty` become unused in the JSX (but some — like `save_status` — are still used for the Saved→Idle reset effect). Let the compiler guide you: run `cargo build --target wasm32-unknown-unknown -p rustynotes-frontend` and remove anything it flags as `unused_variables`. Leave anything still in use.

- [ ] **Step 5: Build and verify**

Run: `trunk build` from the repo root (or `cargo build --target wasm32-unknown-unknown -p rustynotes-frontend`).
Expected: builds successfully. Any `unused_variables` warnings → remove those bindings.

- [ ] **Step 6: Visual check**

Run: `pnpm tauri dev`
Expected: the toolbar looks identical to before — filename + save/dirty indicator still render correctly when a file is loaded.

- [ ] **Step 7: Commit**

```bash
git add crates/rustynotes-frontend/src/components/save_indicator.rs crates/rustynotes-frontend/src/components/mod.rs crates/rustynotes-frontend/src/components/toolbar.rs
git commit -m "refactor: extract SaveIndicator into shared component"
```

---

## Task 6: Add `/file` route and `SingleFileView` skeleton

**Files:**
- Create: `crates/rustynotes-frontend/src/components/single_file/mod.rs`
- Modify: `crates/rustynotes-frontend/src/components/mod.rs`
- Modify: `crates/rustynotes-frontend/src/app.rs`

- [ ] **Step 1: Register the module**

Edit `crates/rustynotes-frontend/src/components/mod.rs`, add:

```rust
pub mod single_file;
```

- [ ] **Step 2: Create the single-file module**

Create `crates/rustynotes-frontend/src/components/single_file/mod.rs`:

```rust
//! Single-file view — a minimal window that displays one markdown
//! file with no folder context. Spawned when the user opens a file
//! via Finder, Cmd+O, drag-and-drop, or CLI argument.

pub mod overflow_menu;
pub mod slim_titlebar;

use leptos::prelude::*;

use crate::components::editor::WysiwygEditor;
use crate::save;
use crate::state::use_app_state;
use crate::tauri_ipc;
use self::slim_titlebar::SlimTitleBar;

/// Read the `path` query parameter from the current URL.
fn read_path_param() -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    // search is "?path=<encoded>&..." or ""
    let trimmed = search.trim_start_matches('?');
    for pair in trimmed.split('&') {
        if let Some(rest) = pair.strip_prefix("path=") {
            return js_sys::decode_uri_component(rest)
                .ok()
                .and_then(|v| v.as_string());
        }
    }
    None
}

#[component]
pub fn SingleFileView() -> impl IntoView {
    let state = use_app_state();
    save::init_save_handlers(&state);

    // Load config + file on mount.
    {
        let state = state.clone();
        Effect::new(move |_| {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                if let Ok(config) = tauri_ipc::get_config().await {
                    let theme = crate::theme::resolve_theme(&config.theme.active);
                    crate::theme::apply_theme(&theme, Some(&config.theme.overrides));
                    state.app_config.set(Some(config));
                }
                if let Some(path) = read_path_param() {
                    save::load_file(&state, path);
                }
                tauri_ipc::show_current_window();
            });
        });
    }

    // Listen for config changes from other windows.
    {
        let state = state.clone();
        tauri_ipc::listen_config_changed(move |config| {
            let theme = crate::theme::resolve_theme(&config.theme.active);
            crate::theme::apply_theme(&theme, Some(&config.theme.overrides));
            state.app_config.set(Some(config));
        });
    }

    view! {
        <div class="single-file-shell">
            <SlimTitleBar />
            <div class="single-file-content">
                <WysiwygEditor />
            </div>
        </div>
    }
}
```

- [ ] **Step 3: Create a stub `slim_titlebar.rs`**

Create `crates/rustynotes-frontend/src/components/single_file/slim_titlebar.rs`:

```rust
//! Slim title bar for single-file windows — traffic lights,
//! filename, save indicator, overflow menu. Minimal chrome.

use leptos::prelude::*;

use crate::components::save_indicator::SaveIndicator;
use crate::tauri_ipc;

#[component]
pub fn SlimTitleBar() -> impl IntoView {
    let handle_close = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::close_current_window();
    };
    let handle_minimize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::minimize_current_window();
    };
    let handle_maximize = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        tauri_ipc::toggle_maximize_current_window();
    };
    let handle_drag = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            tauri_ipc::start_dragging();
        }
    };
    let handle_dblclick = move |_: web_sys::MouseEvent| {
        tauri_ipc::toggle_maximize_current_window();
    };

    view! {
        <div class="slim-titlebar" on:mousedown=handle_drag on:dblclick=handle_dblclick>
            <div class="titlebar-buttons">
                <button class="titlebar-btn close" on:click=handle_close aria-label="Close" />
                <button class="titlebar-btn minimize" on:click=handle_minimize aria-label="Minimize" />
                <button class="titlebar-btn maximize" on:click=handle_maximize aria-label="Maximize" />
            </div>
            <SaveIndicator />
            <div class="spacer" />
            // Overflow menu placeholder — filled in Task 7
            <button class="slim-titlebar-overflow" aria-label="More" title="More">
                {"\u{22EF}"}
            </button>
        </div>
    }
}
```

- [ ] **Step 4: Create a stub `overflow_menu.rs`**

Create `crates/rustynotes-frontend/src/components/single_file/overflow_menu.rs`:

```rust
//! ••• overflow menu for single-file windows.
//! Filled in by Task 7.

use leptos::prelude::*;

#[component]
pub fn OverflowMenu() -> impl IntoView {
    view! { <div /> }
}
```

- [ ] **Step 5: Register the `/file` route in `app.rs`**

Edit `crates/rustynotes-frontend/src/app.rs`. In the `Routes` block, add:

```rust
                    <Route path=path!("/file") view=SingleFileView />
```

Add to the imports at the top (near the other component uses):

```rust
use crate::components::single_file::SingleFileView;
```

- [ ] **Step 6: Add minimal CSS**

Edit `styles/base.css`. Append:

```css
.single-file-shell {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}
.single-file-content {
  flex: 1 1 auto;
  overflow: auto;
  min-height: 0;
}
.slim-titlebar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 10px;
  height: 36px;
  flex: 0 0 auto;
  -webkit-app-region: drag;
  user-select: none;
  background: var(--surface-0, #1e1e1e);
  border-bottom: 1px solid var(--border-subtle, #2a2a2a);
}
.slim-titlebar .titlebar-buttons { -webkit-app-region: no-drag; }
.slim-titlebar .spacer { flex: 1; }
.slim-titlebar-overflow {
  -webkit-app-region: no-drag;
  background: transparent;
  border: 0;
  color: var(--fg-muted, #888);
  font-size: 18px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
}
.slim-titlebar-overflow:hover { background: var(--surface-1, #2a2a2a); color: var(--fg-default, #ddd); }
```

(Token names follow the existing theme naming; if any don't exist verbatim, use whatever the current theme uses — check `styles/base.css` for actual tokens like `--surface-0`, `--fg-muted`.)

- [ ] **Step 7: Verify the route works**

Run: `pnpm tauri dev`.
Then in the running app's address bar (or via browser devtools console): `location.hash = ''; location.pathname = '/file'; location.search = '?path=' + encodeURIComponent('/tmp/test.md')`. Or simpler: close the app, create `/tmp/test.md` with some markdown, and temporarily hard-code a navigation in `main.rs` to `/file?path=/tmp/test.md`.
Expected: the slim titlebar + WYSIWYG editor render; file loads if it exists.

(This is a smoke test — the real entry path through `open_file_in_new_window` comes in later tasks.)

- [ ] **Step 8: Commit**

```bash
git add crates/rustynotes-frontend/src/components/single_file/ crates/rustynotes-frontend/src/components/mod.rs crates/rustynotes-frontend/src/app.rs styles/base.css
git commit -m "feat: add /file route and SingleFileView skeleton"
```

---

## Task 7: Build the `OverflowMenu` component

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/single_file/overflow_menu.rs`
- Modify: `crates/rustynotes-frontend/src/components/single_file/slim_titlebar.rs`
- Modify: `crates/rustynotes-frontend/src/tauri_ipc.rs`

- [ ] **Step 1: Add IPC wrapper for `open_folder_in_window`**

We'll need a backend command for "Open in folder window" that takes a file path and opens its parent directory. That command is added in Task 8; for now, stub the IPC wrapper in `tauri_ipc.rs` following the pattern used by existing wrappers (look at `open_settings()` as a reference — it uses `js_sys::eval` or a JS bridge):

```rust
pub async fn open_folder_in_window(path: &str) -> Result<(), String> {
    // Use the same invocation pattern as other async Tauri commands
    // already in this file — e.g., `read_file`, `save_config_cmd`.
    // They typically wrap `window.__TAURI__.core.invoke(cmd, args)`
    // via `wasm_bindgen`. Copy whichever helper is already used
    // (often named like `invoke_async` or inline JS eval).
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"path".into(), &path.into()).ok();
    invoke_async("open_folder_in_window", &args).await
}
```

Check the file for the actual helper — follow whatever pattern `read_file` / `write_file` / `save_config_cmd` use. The name `invoke_async` is a placeholder; use whatever the existing code calls it.

- [ ] **Step 2: Replace the stub `OverflowMenu` with the real component**

Edit `crates/rustynotes-frontend/src/components/single_file/overflow_menu.rs`:

```rust
//! ••• overflow menu for single-file windows.

use leptos::prelude::*;
use rustynotes_common::EditorMode;

use crate::state::use_app_state;
use crate::tauri_ipc;

#[component]
pub fn OverflowMenu() -> impl IntoView {
    let state = use_app_state();
    let open = RwSignal::new(false);
    let editor_mode = state.editor_mode;
    let active_file_path = state.active_file_path;
    let active_file_content = state.active_file_content;

    let toggle = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        open.update(|v| *v = !*v);
    };

    let handle_source = move |_| {
        editor_mode.set(EditorMode::Source);
        open.set(false);
    };

    let handle_folder = move |_| {
        open.set(false);
        let path = active_file_path.get_untracked();
        if let Some(p) = path {
            leptos::task::spawn_local(async move {
                let _ = tauri_ipc::open_folder_in_window(&p).await;
            });
        }
    };

    let handle_settings = move |_| {
        open.set(false);
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::open_settings().await;
        });
    };

    let handle_export = move |_| {
        open.set(false);
        leptos::task::spawn_local(async move {
            let path = active_file_path.get_untracked();
            let Some(ref current_path) = path else { return };
            let content = active_file_content.get_untracked();

            let default_name = {
                let stem = current_path
                    .rsplit('/')
                    .next()
                    .and_then(|name| name.rfind('.').map(|i| &name[..i]))
                    .unwrap_or("export");
                format!("{stem}.html")
            };

            if let Ok(Some(save_path)) = tauri_ipc::save_file_dialog(&default_name).await {
                let _ = tauri_ipc::export_file(&content, &save_path, "html", true).await;
            }
        });
    };

    view! {
        <div class="overflow-menu-wrapper">
            <button class="slim-titlebar-overflow" on:click=toggle aria-label="More" title="More">
                {"\u{22EF}"}
            </button>
            <Show when=move || open.get()>
                <div class="overflow-menu">
                    <button class="overflow-item" on:click=handle_source>"Switch to Source mode"</button>
                    <button class="overflow-item" on:click=handle_folder>"Open in folder window"</button>
                    <div class="overflow-sep" />
                    <button class="overflow-item" on:click=handle_settings>"Settings…"</button>
                    <button class="overflow-item" on:click=handle_export>"Export HTML…"</button>
                </div>
            </Show>
        </div>
    }
}
```

- [ ] **Step 3: Wire `OverflowMenu` into the slim titlebar**

Edit `crates/rustynotes-frontend/src/components/single_file/slim_titlebar.rs`. Add import:

```rust
use super::overflow_menu::OverflowMenu;
```

Replace the placeholder `<button class="slim-titlebar-overflow" ...>...</button>` with:

```rust
            <OverflowMenu />
```

- [ ] **Step 4: Add CSS for the overflow menu**

Append to `styles/base.css`:

```css
.overflow-menu-wrapper { position: relative; -webkit-app-region: no-drag; }
.overflow-menu {
  position: absolute;
  right: 0;
  top: 32px;
  min-width: 200px;
  background: var(--surface-1, #2a2a2a);
  border: 1px solid var(--border-subtle, #3a3a3a);
  border-radius: 6px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.3);
  padding: 4px;
  z-index: 100;
}
.overflow-menu .overflow-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 6px 10px;
  border: 0;
  background: transparent;
  color: var(--fg-default, #ddd);
  font-size: 13px;
  cursor: pointer;
  border-radius: 4px;
}
.overflow-menu .overflow-item:hover { background: var(--surface-2, #333); }
.overflow-menu .overflow-sep {
  height: 1px;
  background: var(--border-subtle, #3a3a3a);
  margin: 4px 0;
}
```

- [ ] **Step 5: Build + verify**

Run: `pnpm tauri dev`.
If you can navigate to `/file?path=/tmp/test.md` (e.g., temporary test nav): click the ••• button → menu opens with four items. Clicking "Settings" opens settings window.

- [ ] **Step 6: Commit**

```bash
git add crates/rustynotes-frontend/src/components/single_file/overflow_menu.rs crates/rustynotes-frontend/src/components/single_file/slim_titlebar.rs crates/rustynotes-frontend/src/tauri_ipc.rs styles/base.css
git commit -m "feat: add OverflowMenu to slim titlebar"
```

---

## Task 8: Add `open_folder_in_window` backend command

**Files:**
- Modify: `src-tauri/src/commands/window_mgmt.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the command**

Append to `src-tauri/src/commands/window_mgmt.rs` (after `open_file_dialog`):

```rust
/// Open the parent directory of `file_path` as a folder in the main
/// window. If the main window already exists, focus it and emit an
/// event so the frontend can swap folders + reveal the file in the
/// sidebar. If it doesn't exist, create it with the folder preset via
/// the existing main-window URL and let the frontend handle loading.
#[tauri::command]
pub fn open_folder_in_window(app: AppHandle, path: String) -> Result<(), String> {
    let canonical = canonicalize_or_same(&path);
    let parent = canonical
        .parent()
        .ok_or_else(|| "File has no parent directory".to_string())?
        .to_string_lossy()
        .into_owned();
    let filename = canonical
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
        let _ = app.emit(
            "open-folder-with-file",
            serde_json::json!({ "folder": parent, "file": filename }),
        );
        return Ok(());
    }

    // No main window — create it. (Routing into the / route; the
    // frontend's MainView will notice the recent_folders update and
    // auto-open it, or we can emit the event once the window is ready.)
    WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("/".into()))
        .title("RustyNotes")
        .inner_size(1100.0, 750.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    // Update recent_folders so MainView auto-opens on mount.
    let config_state = app.state::<ConfigState>();
    let mut config = config_state.config.lock().unwrap();
    if !config.recent_folders.iter().any(|f| f == &parent) {
        config.recent_folders.insert(0, parent.clone());
        if config.recent_folders.len() > 10 {
            config.recent_folders.truncate(10);
        }
        config_io::save_config(&config).map_err(|e| e.to_string())?;
    }

    // Emit once the window is ready; MainView also listens for this.
    let _ = app.emit(
        "open-folder-with-file",
        serde_json::json!({ "folder": parent, "file": filename }),
    );

    Ok(())
}
```

- [ ] **Step 2: Register the command**

Edit `src-tauri/src/lib.rs`, add to `tauri::generate_handler![...]`:

```rust
            commands::window_mgmt::open_folder_in_window,
```

- [ ] **Step 3: (Frontend) listen for the event in `MainView`**

Edit `crates/rustynotes-frontend/src/app.rs`. In `MainView`, after the existing `listen_config_changed` block, add:

```rust
    {
        let state = state.clone();
        tauri_ipc::listen_event("open-folder-with-file", move |payload_json| {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload_json) {
                if let Some(folder) = v.get("folder").and_then(|v| v.as_str()) {
                    let folder = folder.to_string();
                    let state = state.clone();
                    leptos::task::spawn_local(async move {
                        crate::save::open_folder(&state, folder).await;
                    });
                }
            }
        });
    }
```

(If a generic `listen_event` helper doesn't exist in `tauri_ipc.rs`, add one following the `listen_config_changed` pattern — it's a thin wrapper around `window.__TAURI__.event.listen`.)

- [ ] **Step 4: Verify build**

Run: `cargo build -p rustynotes` and `trunk build`.
Expected: both build cleanly.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/window_mgmt.rs src-tauri/src/lib.rs crates/rustynotes-frontend/src/app.rs crates/rustynotes-frontend/src/tauri_ipc.rs
git commit -m "feat: add open_folder_in_window command for overflow-menu use"
```

---

## Task 9: Wire the `tauri-plugin-single-instance` plugin

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register the plugin with a file-arg callback**

Edit `src-tauri/src/lib.rs`. In `run()`, add to `.plugin(...)` chain (after `opener` / `dialog`):

```rust
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Forwarded args from a second-launch attempt.
            // Skip argv[0] (the executable path).
            for arg in argv.iter().skip(1) {
                if arg.starts_with("-") {
                    continue;
                }
                let path = arg.clone();
                let app = app.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    let file_windows = app.state::<commands::window_mgmt::FileWindows>();
                    let config_state = app.state::<commands::config::ConfigState>();
                    let _ = commands::window_mgmt::open_file_in_new_window(
                        app.clone(),
                        path,
                        file_windows,
                        config_state,
                    );
                });
            }
        }))
```

(Double-check the exact callback signature from `tauri-plugin-single-instance` docs — it may take `(app, argv, cwd)` or just `(app, argv)`. Adjust accordingly.)

- [ ] **Step 2: Verify build**

Run: `cargo build -p rustynotes`.
Expected: builds cleanly.

- [ ] **Step 3: Manual test**

Run: `pnpm tauri dev`.
Once the app is up, from another terminal: `pnpm tauri dev` again.
Expected: no second app instance spawns; the first instance receives the args (visible in Rust logs if you add a `println!` in the callback — optional).

*This task doesn't yet do anything visible because Task 13 is what triggers the file-open path. The test here is just that single-instance gates further launches.*

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register single-instance plugin"
```

---

## Task 10: macOS `RunEvent::Opened` handler for Finder file-open events

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Attach a run-event handler**

In `src-tauri/src/lib.rs`, change the terminal `.run(tauri::generate_context!())` call to use `.build` + run with a handler:

```rust
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Opened { urls } = event {
                for url in urls {
                    if url.scheme() == "file" {
                        if let Ok(path_buf) = url.to_file_path() {
                            let path = path_buf.to_string_lossy().into_owned();
                            let app = app.clone();
                            let _ = tauri::async_runtime::spawn_blocking(move || {
                                let file_windows = app.state::<commands::window_mgmt::FileWindows>();
                                let config_state = app.state::<commands::config::ConfigState>();
                                let _ = commands::window_mgmt::open_file_in_new_window(
                                    app.clone(),
                                    path,
                                    file_windows,
                                    config_state,
                                );
                            });
                        }
                    }
                }
            }
        });
```

- [ ] **Step 2: Verify build**

Run: `cargo build -p rustynotes`.
Expected: builds cleanly.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: handle macOS RunEvent::Opened for Finder file-open events"
```

---

## Task 11: Startup-arg parsing + cold-launch file handling

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Parse argv on startup + route files; suppress auto-folder-open**

In `src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, *before* the existing `std::thread::spawn(move || { /* update check */ ... })`, add:

```rust
            // Cold-launch CLI argument handling.
            // If exactly one path argument was provided and it refers to
            // an existing UTF-8 file, spawn a single-file window and
            // suppress the main window's auto-folder-open for this launch.
            let mut startup_paths: Vec<String> = std::env::args()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .collect();

            let has_file_arg = !startup_paths.is_empty();

            if has_file_arg {
                // Close the auto-created main window so we don't flash
                // the welcome screen before the single-file window opens.
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.close();
                }
                let app_handle_args = app.handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    for path in startup_paths.drain(..) {
                        let file_windows = app_handle_args
                            .state::<commands::window_mgmt::FileWindows>();
                        let config_state = app_handle_args
                            .state::<commands::config::ConfigState>();
                        let _ = commands::window_mgmt::open_file_in_new_window(
                            app_handle_args.clone(),
                            path,
                            file_windows,
                            config_state,
                        );
                    }
                });
            }
```

- [ ] **Step 2: Verify build**

Run: `cargo build -p rustynotes`.
Expected: builds cleanly.

- [ ] **Step 3: Manual test**

Build the app (`pnpm tauri build`), then from a terminal:

```bash
open -a /path/to/RustyNotes.app --args /tmp/test.md
```

Expected: app launches with a single-file window showing `/tmp/test.md`; no folder window appears.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: route startup file args into single-file windows"
```

---

## Task 12: Register `fileAssociations` in `tauri.conf.json`

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add the bundle section**

Edit `src-tauri/tauri.conf.json`. In the `bundle` object, add:

```json
    "fileAssociations": [
      {
        "ext": ["md", "markdown"],
        "description": "Markdown file",
        "role": "Editor"
      }
    ],
```

- [ ] **Step 2: Build installer**

Run: `pnpm tauri build`.
Expected: a `.app` is produced with the `.md` / `.markdown` associations registered in its `Info.plist`.

- [ ] **Step 3: Manual test (macOS)**

Install the built app (drag into Applications). In Finder, right-click a `.md` file → Get Info → "Open with" should now include RustyNotes. Set it as default, then double-click a `.md` file.
Expected: single-file window opens.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: register .md/.markdown file associations"
```

---

## Task 13: Drag-and-drop listener on all windows

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `on_drop` handling to both window builders**

File drops are handled by Tauri's webview drop handlers. In `src-tauri/src/lib.rs`'s `.setup`, after the main window is available, attach a drop handler. A clean place is a helper function `attach_drop_handler(window: &tauri::WebviewWindow)` that listens for `WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. })`.

Add to `src-tauri/src/lib.rs`:

```rust
fn attach_drop_handler(window: &tauri::WebviewWindow) {
    let app_handle = window.app_handle().clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
            let paths: Vec<String> = paths
                .iter()
                .take(10)
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn_blocking(move || {
                for path in paths {
                    // Distinguish folder vs file drop.
                    if std::path::Path::new(&path).is_dir() {
                        // Ignore folder drops for now — spec marks this as deferrable v2 polish.
                        continue;
                    }
                    let file_windows = app_handle.state::<commands::window_mgmt::FileWindows>();
                    let config_state = app_handle.state::<commands::config::ConfigState>();
                    let _ = commands::window_mgmt::open_file_in_new_window(
                        app_handle.clone(),
                        path,
                        file_windows,
                        config_state,
                    );
                }
            });
        }
    });
}
```

- [ ] **Step 2: Call it for the main window during `.setup`**

In `.setup`, after the (implicitly-created) main window exists, add:

```rust
            if let Some(main) = app.get_webview_window("main") {
                attach_drop_handler(&main);
            }
```

- [ ] **Step 3: Call it for newly-spawned file windows**

In `src-tauri/src/commands/window_mgmt.rs`, modify `open_file_in_new_window` — after the `.build()` call creates the window, capture the result and attach:

```rust
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(&filename)
        .inner_size(800.0, 650.0)
        .min_inner_size(400.0, 300.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    crate::attach_drop_handler(&window);
```

(Make sure `attach_drop_handler` is declared `pub fn` in `lib.rs` so `commands/window_mgmt.rs` can reach it.)

- [ ] **Step 4: Verify build**

Run: `cargo build -p rustynotes`.
Expected: builds cleanly.

- [ ] **Step 5: Manual test**

Run `pnpm tauri dev`. Drag a `.md` file onto the running window.
Expected: a new single-file window opens for that file.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands/window_mgmt.rs
git commit -m "feat: handle file drag-and-drop onto windows"
```

---

## Task 14: Build the native macOS File menu

**Files:**
- Create: `src-tauri/src/menu.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create the menu module**

Create `src-tauri/src/menu.rs`:

```rust
//! Native macOS File menu. Items emit events the frontend listens to.
//! The Open Recent submenu is rebuilt dynamically on `config-changed`.

use tauri::menu::{
    AboutMetadata, Menu, MenuBuilder, MenuId, MenuItemBuilder, PredefinedMenuItem, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::commands::config::ConfigState;

pub fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let file_submenu = build_file_submenu(app)?;

    let menu = MenuBuilder::new(app)
        // macOS convention: the first submenu is the app menu.
        .items(&[
            &SubmenuBuilder::new(app, "RustyNotes")
                .items(&[
                    &PredefinedMenuItem::about(
                        app,
                        Some("About RustyNotes"),
                        Some(AboutMetadata::default()),
                    )?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::show_all(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ])
                .build()?,
            &file_submenu,
            &SubmenuBuilder::new(app, "Edit")
                .items(&[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ])
                .build()?,
            &SubmenuBuilder::new(app, "Window")
                .items(&[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::close_window(app, None)?,
                ])
                .build()?,
        ])
        .build()?;

    Ok(menu)
}

fn build_file_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let new_file = MenuItemBuilder::new("New File")
        .id("file.new")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_file = MenuItemBuilder::new("Open File\u{2026}")
        .id("file.open-file")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let open_folder = MenuItemBuilder::new("Open Folder\u{2026}")
        .id("file.open-folder")
        .accelerator("CmdOrCtrl+Shift+O")
        .build(app)?;
    let recent = build_open_recent_submenu(app)?;
    let save = MenuItemBuilder::new("Save")
        .id("file.save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let export = MenuItemBuilder::new("Export HTML\u{2026}")
        .id("file.export")
        .build(app)?;

    let submenu = SubmenuBuilder::new(app, "File")
        .items(&[
            &new_file,
            &open_file,
            &open_folder,
            &PredefinedMenuItem::separator(app)?,
            &recent,
            &PredefinedMenuItem::separator(app)?,
            &save,
            &export,
        ])
        .build()?;

    Ok(submenu)
}

/// Build the Open Recent submenu from the current config.
/// Items carry an id of `recent.file:<index>` or `recent.folder:<index>`
/// so we can look up the path in the config when the user clicks.
fn build_open_recent_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let config_state = app.state::<ConfigState>();
    let config = config_state.config.lock().unwrap().clone();

    let mut builder = SubmenuBuilder::new(app, "Open Recent");

    if !config.recent_files.is_empty() {
        // Labelled separator isn't first-class in Tauri menus — we inline
        // a disabled "Recent Files" item instead to mark the section.
        let header = MenuItemBuilder::new("Recent Files")
            .id("recent.header-files")
            .enabled(false)
            .build(app)?;
        builder = builder.item(&header);
        for (i, path) in config.recent_files.iter().enumerate().take(10) {
            let filename = std::path::Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            let item = MenuItemBuilder::new(filename)
                .id(format!("recent.file:{i}"))
                .build(app)?;
            builder = builder.item(&item);
        }
        builder = builder.separator();
    }

    if !config.recent_folders.is_empty() {
        let header = MenuItemBuilder::new("Recent Folders")
            .id("recent.header-folders")
            .enabled(false)
            .build(app)?;
        builder = builder.item(&header);
        for (i, folder) in config.recent_folders.iter().enumerate().take(10) {
            let folder_name = std::path::Path::new(folder)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| folder.clone());
            let item = MenuItemBuilder::new(folder_name)
                .id(format!("recent.folder:{i}"))
                .build(app)?;
            builder = builder.item(&item);
        }
        builder = builder.separator();
    }

    if config.recent_files.is_empty() && config.recent_folders.is_empty() {
        let empty = MenuItemBuilder::new("No Recent Items")
            .id("recent.empty")
            .enabled(false)
            .build(app)?;
        builder = builder.item(&empty);
    } else {
        let clear = MenuItemBuilder::new("Clear Recent")
            .id("recent.clear")
            .build(app)?;
        builder = builder.item(&clear);
    }

    builder.build()
}

/// Handle a menu click by dispatching to the right backend command
/// or emitting an event for the frontend to handle.
pub fn handle_menu_event(app: &AppHandle, id: &MenuId) {
    let id = id.0.as_str();
    match id {
        "file.new" => {
            let _ = app.emit("menu:new-file", ());
        }
        "file.open-file" => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(Some(path)) = crate::commands::window_mgmt::open_file_dialog(
                    app_handle.clone(),
                )
                .await
                {
                    let file_windows = app_handle
                        .state::<crate::commands::window_mgmt::FileWindows>();
                    let config_state = app_handle.state::<ConfigState>();
                    let _ = crate::commands::window_mgmt::open_file_in_new_window(
                        app_handle.clone(),
                        path,
                        file_windows,
                        config_state,
                    );
                }
            });
        }
        "file.open-folder" => {
            let _ = app.emit("menu:open-folder", ());
        }
        "file.save" => {
            let _ = app.emit("menu:save", ());
        }
        "file.export" => {
            let _ = app.emit("menu:export", ());
        }
        "recent.clear" => {
            let config_state = app.state::<ConfigState>();
            let mut config = config_state.config.lock().unwrap();
            config.recent_files.clear();
            config.recent_folders.clear();
            let _ = crate::config::save_config(&config);
            let _ = app.emit("config-changed", config.clone());
        }
        other if other.starts_with("recent.file:") => {
            if let Some(idx) = other
                .strip_prefix("recent.file:")
                .and_then(|s| s.parse::<usize>().ok())
            {
                let config_state = app.state::<ConfigState>();
                let path = config_state
                    .config
                    .lock()
                    .unwrap()
                    .recent_files
                    .get(idx)
                    .cloned();
                if let Some(path) = path {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let file_windows = app_handle
                            .state::<crate::commands::window_mgmt::FileWindows>();
                        let config_state = app_handle.state::<ConfigState>();
                        let _ = crate::commands::window_mgmt::open_file_in_new_window(
                            app_handle.clone(),
                            path,
                            file_windows,
                            config_state,
                        );
                    });
                }
            }
        }
        other if other.starts_with("recent.folder:") => {
            if let Some(idx) = other
                .strip_prefix("recent.folder:")
                .and_then(|s| s.parse::<usize>().ok())
            {
                let config_state = app.state::<ConfigState>();
                let folder = config_state
                    .config
                    .lock()
                    .unwrap()
                    .recent_folders
                    .get(idx)
                    .cloned();
                if let Some(folder) = folder {
                    let _ = app.emit(
                        "open-folder-with-file",
                        serde_json::json!({ "folder": folder, "file": "" }),
                    );
                }
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Register the module in `lib.rs`**

Edit `src-tauri/src/lib.rs`. Add at the top with the other `mod` declarations:

```rust
mod menu;
```

In `.setup(|app| { ... })`, after the drop handler attachment, add:

```rust
            let menu_obj = menu::build_menu(app.handle())?;
            app.set_menu(menu_obj)?;
            let app_handle_menu = app.handle().clone();
            app.on_menu_event(move |_, event| {
                menu::handle_menu_event(&app_handle_menu, event.id());
            });
```

- [ ] **Step 3: Rebuild menu when config changes**

We need to rebuild the Open Recent submenu when `recent_files` or `recent_folders` change. In `commands::config::save_config_cmd`, after the existing `let _ = app.emit("config-changed", config_data);`, add:

```rust
    if let Ok(new_menu) = crate::menu::build_menu(&app) {
        let _ = app.set_menu(new_menu);
    }
```

Also do the same inside `commands::window_mgmt::open_file_in_new_window` after the `save_config` + `emit` block (wrap in a helper if copy-paste hurts).

- [ ] **Step 4: Verify build**

Run: `cargo build -p rustynotes`.
Expected: builds cleanly. (Check API signatures — `tauri::menu::*` APIs changed between 2.0 betas; refer to the installed version's docs if any name doesn't resolve.)

- [ ] **Step 5: Manual test**

Run: `pnpm tauri dev`.
Expected: a native File menu appears. "New File" / "Open File…" / "Open Folder…" / "Save" / "Open Recent" / "Export HTML…" all show. Clicking each emits something (not necessarily a visible effect yet — that's Task 15).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/menu.rs src-tauri/src/lib.rs src-tauri/src/commands/config.rs src-tauri/src/commands/window_mgmt.rs
git commit -m "feat: add native macOS File menu with Open Recent submenu"
```

---

## Task 15: Wire menu events to frontend handlers + remove conflicting JS keydown handlers

**Files:**
- Modify: `crates/rustynotes-frontend/src/save.rs`
- Modify: `crates/rustynotes-frontend/src/components/toolbar.rs`
- Modify: `crates/rustynotes-frontend/src/components/single_file/mod.rs`
- Modify: `crates/rustynotes-frontend/src/tauri_ipc.rs`

- [ ] **Step 1: Add menu-event listener helpers**

In `crates/rustynotes-frontend/src/tauri_ipc.rs`, add (follow the `listen_config_changed` pattern):

```rust
pub fn listen_menu_event(event_name: &str, cb: impl Fn() + 'static) {
    // Generic payload-less menu event listener.
    let cb = std::rc::Rc::new(cb);
    let cb = cb.clone();
    listen_event(event_name, move |_payload| cb());
}
```

(If `listen_event` doesn't exist, add it first — it's a thin wrapper around `window.__TAURI__.event.listen` that takes a string payload. Use the existing `listen_config_changed` implementation as a template.)

- [ ] **Step 2: Remove `Cmd+S` and `Cmd+N` from the JS keydown handler in `save.rs`**

Edit `crates/rustynotes-frontend/src/save.rs`. In `init_keyboard_shortcuts`, remove the `"s"` and `"n"` arms of the `match ke.key().as_str()` block. Keep the `"1"`/`"2"`/`"3"`/`"4"` arms.

Rationale: those shortcuts are now owned by the native menu's accelerators; handling them in both places causes double-invocation on macOS.

- [ ] **Step 3: Register menu-event listeners in `init_save_handlers`**

At the end of `init_save_handlers` in `save.rs`, after the existing init calls, add:

```rust
    init_menu_listeners(state);
}

fn init_menu_listeners(state: &AppState) {
    let state = state.clone();
    tauri_ipc::listen_menu_event("menu:save", {
        let state = state.clone();
        move || {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                perform_save(&state).await;
            });
        }
    });

    tauri_ipc::listen_menu_event("menu:new-file", {
        let state = state.clone();
        move || {
            // Same logic as the old Cmd+N handler.
            state.active_file_path.set(None);
            state.active_file_content.set(String::new());
            state.is_dirty.set(false);
            state.save_status.set(SaveStatus::Idle);
            state.rendered_html.set(String::new());
        }
    });
```

(Closing brace of `init_save_handlers` moves below `init_menu_listeners` — adjust syntax accordingly.)

- [ ] **Step 4: Wire `menu:open-folder` and `menu:export` in the main toolbar component**

Edit `crates/rustynotes-frontend/src/components/toolbar.rs`. In `Toolbar`, after the other listeners, add:

```rust
    {
        let state = state.clone();
        tauri_ipc::listen_menu_event("menu:open-folder", move || {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                if let Ok(Some(folder)) = tauri_ipc::open_folder_dialog().await {
                    crate::save::open_folder(&state, folder).await;
                }
            });
        });
    }

    tauri_ipc::listen_menu_event("menu:export", {
        let file_path = state.active_file_path;
        let content = state.active_file_content;
        move || {
            let Some(ref path) = file_path.get_untracked() else { return };
            let content = content.get_untracked();
            let stem_name = path.rsplit('/').next().unwrap_or(path);
            let stem = stem_name.rfind('.').map(|i| &stem_name[..i]).unwrap_or(stem_name);
            let default_name = format!("{stem}.html");
            leptos::task::spawn_local(async move {
                if let Ok(Some(save_path)) = tauri_ipc::save_file_dialog(&default_name).await {
                    let _ = tauri_ipc::export_file(&content, &save_path, "html", true).await;
                }
            });
        }
    });
```

- [ ] **Step 5: Wire `menu:export` inside `SingleFileView` too**

Edit `crates/rustynotes-frontend/src/components/single_file/mod.rs`. After the existing listeners in `SingleFileView`, add the same `menu:export` listener as in Step 4.

(`menu:save` and `menu:new-file` are handled in `init_save_handlers`, which `SingleFileView` already calls. `menu:open-folder` is handled in `Toolbar`, which doesn't exist in single-file windows — if the user hits it from a single-file window, the event fires for *every* window listening, and only the folder window responds. If no folder window exists, we need a fallback. Handle that by adding the listener also in `SingleFileView` with a backend command that spawns the main window if absent.)

Actually, simpler: add the `menu:open-folder` listener in `main.rs` via a Tauri-side handler that dispatches to the right window. For now, add it in `SingleFileView` too:

```rust
    {
        let state = state.clone();
        tauri_ipc::listen_menu_event("menu:open-folder", move || {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                if let Ok(Some(folder)) = tauri_ipc::open_folder_dialog().await {
                    // Route through the backend so it either focuses
                    // the main window or spawns one with the folder.
                    let _ = tauri_ipc::open_folder_in_main_window(&folder).await;
                }
            });
        });
    }
```

And add a new backend command `open_folder_in_main_window(folder: String)` that:
1. If main window exists, focuses it and emits `open-folder-with-file` with file=""
2. Else, creates the main window at `/`, sets `recent_folders[0] = folder`, saves config

Add this to `src-tauri/src/commands/window_mgmt.rs`:

```rust
#[tauri::command]
pub fn open_folder_in_main_window(
    app: AppHandle,
    folder: String,
    config_state: tauri::State<ConfigState>,
) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
        let _ = app.emit(
            "open-folder-with-file",
            serde_json::json!({ "folder": folder, "file": "" }),
        );
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("/".into()))
        .title("RustyNotes")
        .inner_size(1100.0, 750.0)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    let mut config = config_state.config.lock().unwrap();
    if !config.recent_folders.first().map(|f| f == &folder).unwrap_or(false) {
        config.recent_folders.retain(|f| f != &folder);
        config.recent_folders.insert(0, folder);
        if config.recent_folders.len() > 10 {
            config.recent_folders.truncate(10);
        }
        config_io::save_config(&config).map_err(|e| e.to_string())?;
        let _ = app.emit("config-changed", config.clone());
    }

    Ok(())
}
```

Register in `invoke_handler![...]`:

```rust
            commands::window_mgmt::open_folder_in_main_window,
```

Add IPC wrapper in `tauri_ipc.rs`:

```rust
pub async fn open_folder_in_main_window(folder: &str) -> Result<(), String> {
    invoke_with_arg("open_folder_in_main_window", "folder", folder).await
}
```

- [ ] **Step 6: Verify build**

Run: `cargo build -p rustynotes` and `trunk build`.
Expected: clean builds.

- [ ] **Step 7: Manual test**

Run: `pnpm tauri dev`.
- File → New File → untitled loads.
- File → Open File… → dialog opens, selecting a file spawns a new window.
- File → Save → saves.
- File → Export HTML… → dialog opens.
- Cmd+N, Cmd+O, Cmd+S, Cmd+Shift+O all trigger the same actions.

- [ ] **Step 8: Commit**

```bash
git add crates/rustynotes-frontend/src/save.rs crates/rustynotes-frontend/src/components/toolbar.rs crates/rustynotes-frontend/src/components/single_file/mod.rs crates/rustynotes-frontend/src/tauri_ipc.rs src-tauri/src/commands/window_mgmt.rs src-tauri/src/lib.rs
git commit -m "feat: wire menu events; remove duplicate Cmd+S/Cmd+N keydown handlers"
```

---

## Task 16: Add capability for `file-*` windows

**Files:**
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Extend the windows glob**

Edit `src-tauri/capabilities/default.json`:

```json
  "windows": ["main", "settings", "file-*"],
```

- [ ] **Step 2: Verify**

Run: `pnpm tauri dev`. Open a file via Cmd+O.
Expected: the single-file window's `invoke()` calls (e.g., `read_file`, `get_config`) succeed without silent permission failures.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/capabilities/default.json
git commit -m "feat: grant file-* windows IPC permissions"
```

---

## Task 17: Welcome screen — Open File button + Recent Files section

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/onboarding/welcome.rs`

- [ ] **Step 1: Add the Open File button and Recent Files section**

Edit `crates/rustynotes-frontend/src/components/onboarding/welcome.rs`. Inside `WelcomeEmptyState`, after the existing `recent_folders` memo, add:

```rust
    let recent_files = Memo::new(move |_| {
        state
            .app_config
            .get()
            .map(|c| c.recent_files)
            .unwrap_or_default()
    });
```

Add a new handler next to `open_folder`:

```rust
    let state_for_open_file = state.clone();
    let open_file = move |_| {
        let set_first_run = set_first_run;
        let state = state_for_open_file.clone();
        leptos::task::spawn_local(async move {
            if is_first_run.get_untracked() {
                mark_welcomed(set_first_run);
            }
            if let Ok(Some(path)) = tauri_ipc::open_file_dialog().await {
                let _ = tauri_ipc::open_file_in_new_window(&path).await;
                // Opening a file via welcome happens from the main window;
                // the newly-opened single-file window is independent.
                // Leave the welcome state as-is.
            }
        });
    };
```

Add the IPC wrapper in `tauri_ipc.rs` if not already present:

```rust
pub async fn open_file_dialog() -> Result<Option<String>, String> {
    invoke_no_args("open_file_dialog").await
}

pub async fn open_file_in_new_window(path: &str) -> Result<(), String> {
    invoke_with_arg("open_file_in_new_window", "path", path).await
}
```

Then in the `view!` block, replace the existing `<button class="empty-state-cta" ...>"Open Folder"</button>` with:

```rust
            <div class="empty-state-actions">
                <button class="empty-state-cta" on:click=open_folder>
                    "Open Folder"
                </button>
                <button class="empty-state-cta secondary" on:click=open_file>
                    "Open File"
                </button>
            </div>
```

And update the `<Show when=...>` block for recents to show both files and folders as subsections:

```rust
            <Show when=move || !recent_folders.get().is_empty() || !recent_files.get().is_empty()>
                <div class="recent-folders">
                    <h2 class="recent-folders-heading">"Recent"</h2>

                    <Show when=move || !recent_folders.get().is_empty()>
                        <h3 class="recent-subheading">"Folders"</h3>
                        <ul class="recent-folders-list">
                            {
                                let state_for_list = state.clone();
                                view! {
                                <For
                                    each=move || {
                                        recent_folders.get().into_iter().take(5).collect::<Vec<_>>()
                                    }
                                    key=|folder| folder.clone()
                                    children=move |folder| {
                                        let folder_for_click = folder.clone();
                                        let folder_for_title = folder.clone();
                                        let folder_for_path = folder.clone();
                                        let folder_for_name = folder.clone();
                                        let set_first_run = set_first_run;
                                        let state_for_recent = state_for_list.clone();
                                        let handle_recent = move |_| {
                                            let folder = folder_for_click.clone();
                                            let state = state_for_recent.clone();
                                            leptos::task::spawn_local(async move {
                                                if is_first_run.get_untracked() {
                                                    mark_welcomed(set_first_run);
                                                }
                                                crate::save::open_folder(&state, folder).await;
                                            });
                                        };
                                        let display_name = folder_for_name
                                            .rsplit('/')
                                            .next()
                                            .unwrap_or(&folder_for_name)
                                            .to_string();
                                        view! {
                                            <li>
                                                <button
                                                    class="recent-folder-item"
                                                    on:click=handle_recent
                                                    title=folder_for_title.clone()
                                                >
                                                    <span class="recent-folder-icon" aria-hidden="true">"\u{2013}"</span>
                                                    <span class="recent-folder-name">{display_name}</span>
                                                    <span class="recent-folder-path">{folder_for_path}</span>
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                                }
                            }
                        </ul>
                    </Show>

                    <Show when=move || !recent_files.get().is_empty()>
                        <h3 class="recent-subheading">"Files"</h3>
                        <ul class="recent-folders-list">
                            {
                                let state_for_files = state.clone();
                                view! {
                                <For
                                    each=move || {
                                        recent_files.get().into_iter().take(5).collect::<Vec<_>>()
                                    }
                                    key=|file| file.clone()
                                    children=move |file| {
                                        let file_for_click = file.clone();
                                        let file_for_title = file.clone();
                                        let file_for_name = file.clone();
                                        let _state_for_item = state_for_files.clone();
                                        let handle_recent_file = move |_| {
                                            let file = file_for_click.clone();
                                            leptos::task::spawn_local(async move {
                                                let _ = tauri_ipc::open_file_in_new_window(&file).await;
                                            });
                                        };
                                        let filename = file_for_name
                                            .rsplit('/')
                                            .next()
                                            .unwrap_or(&file_for_name)
                                            .to_string();
                                        let parent = file_for_name
                                            .rsplit('/')
                                            .skip(1)
                                            .next()
                                            .unwrap_or("")
                                            .to_string();
                                        view! {
                                            <li>
                                                <button
                                                    class="recent-folder-item"
                                                    on:click=handle_recent_file
                                                    title=file_for_title.clone()
                                                >
                                                    <span class="recent-folder-icon" aria-hidden="true">"\u{2013}"</span>
                                                    <span class="recent-folder-name">{filename}</span>
                                                    <span class="recent-folder-path">{parent}</span>
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                                }
                            }
                        </ul>
                    </Show>
                </div>
            </Show>
```

- [ ] **Step 2: Add CSS**

Append to `styles/base.css`:

```css
.empty-state-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}
.empty-state-cta.secondary {
  background: transparent;
  border: 1px solid var(--border-subtle, #3a3a3a);
  color: var(--fg-default, #ddd);
}
.empty-state-cta.secondary:hover {
  background: var(--surface-1, #2a2a2a);
}
.recent-subheading {
  font-size: 11px;
  font-weight: 600;
  color: var(--fg-muted, #888);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin: 10px 0 4px;
}
```

- [ ] **Step 3: Verify build**

Run: `trunk build`.
Expected: clean build.

- [ ] **Step 4: Manual test**

Run: `pnpm tauri dev`. (If recent_folders is populated, a folder auto-opens — back out with no folder to see the welcome screen. Easiest: temporarily edit config to clear `recent_folders`.)
Expected: welcome screen shows both buttons; after you've opened a file once, Recent → Files has that entry next time you see the welcome screen.

- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/components/onboarding/welcome.rs crates/rustynotes-frontend/src/tauri_ipc.rs styles/base.css
git commit -m "feat: add Open File button and Recent Files to welcome screen"
```

---

## Task 18: Unsaved-changes guard on single-file window close

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/single_file/mod.rs`

- [ ] **Step 1: Intercept `CloseRequested` from the frontend**

Tauri 2 exposes window close events via `window.__TAURI__.webviewWindow.getCurrent().onCloseRequested(cb)`. Add a helper in `tauri_ipc.rs`:

```rust
/// Register a callback for the current window's CloseRequested event.
/// The callback receives a function to prevent default (cancel close).
pub fn on_close_requested(cb: impl Fn(js_sys::Function) + 'static) {
    // Bridge to window.__TAURI__.webviewWindow.getCurrent().onCloseRequested
    // ... (follow the pattern used for listen_event).
}
```

(Implementation detail: the full shim is a small JS bridge. Alternative simpler approach: emit a custom event from the backend side when a close is requested — but intercepting the default close behavior needs the JS-side API. Use the existing `bridge.js` to add a helper.)

Simpler approach — handle it backend-side. Edit `src-tauri/src/commands/window_mgmt.rs`, after building the window in `open_file_in_new_window`:

```rust
    let window_for_close = window.clone();
    let app_for_close = app.clone();
    let label_for_close = label.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Ask the frontend whether it's safe to close.
            // The frontend replies via a "close-confirmed" event.
            api.prevent_close();
            let _ = app_for_close.emit_to(
                tauri::EventTarget::WebviewWindow { label: label_for_close.clone() },
                "confirm-close",
                (),
            );
        }
    });
```

(Keep the existing drop-handler `attach_drop_handler(&window)` call — both handlers can coexist via `on_window_event` closures chaining if Tauri supports it. If not, move both into one closure.)

Actually `on_window_event` only accepts one closure — merge them:

```rust
    let app_for_events = app.clone();
    let label_for_events = label.clone();
    window.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                let paths: Vec<String> = paths
                    .iter()
                    .take(10)
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                let app_handle = app_for_events.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    for path in paths {
                        if std::path::Path::new(&path).is_dir() { continue; }
                        let fw = app_handle.state::<FileWindows>();
                        let cs = app_handle.state::<ConfigState>();
                        let _ = open_file_in_new_window(app_handle.clone(), path, fw, cs);
                    }
                });
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = app_for_events.emit_to(
                    tauri::EventTarget::WebviewWindow { label: label_for_events.clone() },
                    "confirm-close",
                    (),
                );
            }
            tauri::WindowEvent::Destroyed => {
                let fw = app_for_events.state::<FileWindows>();
                fw.remove_by_label(&label_for_events);
            }
            _ => {}
        }
    });
```

(Remove the `crate::attach_drop_handler(&window)` call — now redundant. Also consider pulling this into a `attach_single_file_window_handlers` helper.)

- [ ] **Step 2: Frontend — listen for `confirm-close` and prompt if dirty**

Edit `crates/rustynotes-frontend/src/components/single_file/mod.rs`. In `SingleFileView`, add a signal for the close prompt:

```rust
    let confirm_close_open = RwSignal::new(false);
```

Register the listener:

```rust
    {
        let confirm_close_open = confirm_close_open;
        let state = state.clone();
        tauri_ipc::listen_event("confirm-close", move |_| {
            if state.is_dirty.get_untracked() {
                confirm_close_open.set(true);
            } else {
                // Not dirty — safe to close.
                tauri_ipc::destroy_current_window();
            }
        });
    }
```

Add the `destroy_current_window` IPC helper in `tauri_ipc.rs`:

```rust
pub fn destroy_current_window() {
    let _ = js_sys::eval(
        "window.__TAURI__.webviewWindow.getCurrent().destroy()",
    );
}
```

Render a Save/Discard/Cancel modal at the bottom of the `view!` block:

```rust
            <Show when=move || confirm_close_open.get()>
                <div class="modal-overlay">
                    <div class="modal-dialog">
                        <p>"You have unsaved changes"</p>
                        <div class="modal-actions">
                            <button class="modal-btn primary" on:click={
                                let state = state.clone();
                                let confirm_close_open = confirm_close_open;
                                move |_| {
                                    let state = state.clone();
                                    confirm_close_open.set(false);
                                    leptos::task::spawn_local(async move {
                                        crate::save::perform_save(&state).await;
                                        tauri_ipc::destroy_current_window();
                                    });
                                }
                            }>"Save"</button>
                            <button class="modal-btn" on:click={
                                let state = state.clone();
                                let confirm_close_open = confirm_close_open;
                                move |_| {
                                    state.is_dirty.set(false);
                                    confirm_close_open.set(false);
                                    tauri_ipc::destroy_current_window();
                                }
                            }>"Discard"</button>
                            <button class="modal-btn" on:click={
                                let confirm_close_open = confirm_close_open;
                                move |_| confirm_close_open.set(false)
                            }>"Cancel"</button>
                        </div>
                    </div>
                </div>
            </Show>
```

- [ ] **Step 3: Verify build**

Run: `cargo build -p rustynotes` and `trunk build`.
Expected: clean.

- [ ] **Step 4: Manual test**

Run `pnpm tauri dev`. Open a file in a single-file window, edit it, click the close button.
Expected: modal appears. Save → file saves and window closes. Discard → window closes, changes lost. Cancel → window stays.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/window_mgmt.rs crates/rustynotes-frontend/src/components/single_file/mod.rs crates/rustynotes-frontend/src/tauri_ipc.rs
git commit -m "feat: prompt to save on single-file window close with unsaved changes"
```

---

## Task 19: Error view for invalid files

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/single_file/mod.rs`

- [ ] **Step 1: Add error-state rendering**

The backend already rejects non-UTF-8 / non-existent files before spawning the window (per Task 4), so this only matters if a file becomes unreadable *after* the window opens (deleted while open). Add a check in `SingleFileView`:

Add a signal for error state:

```rust
    let load_error = RwSignal::new(Option::<String>::None);
```

In the load block, if `tauri_ipc::read_file` errors, set `load_error`:

Replace:

```rust
                if let Some(path) = read_path_param() {
                    save::load_file(&state, path);
                }
```

With:

```rust
                if let Some(path) = read_path_param() {
                    match tauri_ipc::read_file(&path).await {
                        Ok(content) => {
                            state.active_file_path.set(Some(path));
                            state.suppress_dirty.set(true);
                            state.active_file_content.set(content);
                            state.is_dirty.set(false);
                            // Existing suppress-dirty timeout pattern from save::load_file
                            let state2 = state.clone();
                            gloo_timers::callback::Timeout::new(100, move || {
                                state2.suppress_dirty.set(false);
                            }).forget();
                        }
                        Err(e) => {
                            load_error.set(Some(e));
                        }
                    }
                } else {
                    load_error.set(Some("No file path provided.".to_string()));
                }
```

Update the `view!` block to branch on `load_error`:

```rust
    view! {
        <div class="single-file-shell">
            <SlimTitleBar />
            <div class="single-file-content">
                <Show when=move || load_error.get().is_some()
                    fallback=|| view! { <WysiwygEditor /> }>
                    <div class="single-file-error">
                        <h2>"Can't open this file"</h2>
                        <p>{move || load_error.get().unwrap_or_default()}</p>
                        <button on:click=move |_| tauri_ipc::close_current_window()>"Close"</button>
                    </div>
                </Show>
            </div>
        </div>
    }
```

- [ ] **Step 2: Add CSS**

Append to `styles/base.css`:

```css
.single-file-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 40px;
  height: 100%;
  text-align: center;
  color: var(--fg-muted, #888);
}
.single-file-error h2 { margin: 0; font-size: 18px; color: var(--fg-default, #ddd); }
.single-file-error button {
  padding: 6px 16px;
  background: var(--surface-1, #2a2a2a);
  border: 1px solid var(--border-subtle, #3a3a3a);
  color: var(--fg-default, #ddd);
  border-radius: 4px;
  cursor: pointer;
}
```

- [ ] **Step 3: Manual test**

Start the app. Open a file that exists, then delete it on disk, then close the window — the close path doesn't hit this. For *this* task, simulate by passing a bad path directly in the URL: modify the test harness to navigate to `/file?path=/nonexistent`.
Expected: error view with Close button appears.

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/src/components/single_file/mod.rs styles/base.css
git commit -m "feat: show error view when single-file window can't load its file"
```

---

## Task 20: Prune stale `recent_files` on read

**Files:**
- Modify: `src-tauri/src/commands/config.rs`

- [ ] **Step 1: Prune in `get_config`**

Edit `src-tauri/src/commands/config.rs`. Modify `get_config`:

```rust
#[tauri::command]
pub fn get_config(state: tauri::State<ConfigState>) -> AppConfig {
    let mut config = state.config.lock().unwrap();
    let changed = crate::commands::window_mgmt::prune_missing(&mut config.recent_files)
        | crate::commands::window_mgmt::prune_missing(&mut config.recent_folders);
    if changed {
        let _ = crate::config::save_config(&config);
    }
    config.clone()
}
```

- [ ] **Step 2: Verify build**

Run: `cargo build -p rustynotes`.
Expected: clean.

- [ ] **Step 3: Manual test**

Add a non-existent path to `~/.config/rustynotes/config.json`'s `recent_files` by editing the file directly, then launch the app and open the welcome screen.
Expected: the stale entry is gone.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/config.rs
git commit -m "feat: prune stale recent_files and recent_folders on config read"
```

---

## Task 21: Execute manual test plan from spec

Run through every item in the spec's Manual Test Plan (section 16 items). Document any failures as GitHub issues or follow-up tasks. This task doesn't produce code — it validates everything above.

- [ ] **Step 1: Build release**

```bash
pnpm tauri build
```

Install the resulting `.app` into `/Applications`.

- [ ] **Step 2: Run each test from the spec**

From `docs/superpowers/specs/2026-04-14-single-file-view-design.md` "Manual test plan" section, execute items 1–16. For each:
- If it passes, continue.
- If it fails, note the behavior and open a follow-up task (do not fix here — this is validation).

- [ ] **Step 3: If all pass, commit a CHANGELOG entry**

Edit (or create) `CHANGELOG.md` with a new entry describing the feature.

```bash
git add CHANGELOG.md
git commit -m "docs: changelog entry for single-file view"
```

---

## Follow-ups out of scope

Items the spec explicitly defers to v2 — do **not** implement here:
- Session restoration (reopening yesterday's single-file windows on cold launch)
- External-change detection (file watcher) for single-file windows
- Window tab-merging
- Windows / Linux file-association installers
- Folder-dropped-on-app routing (currently ignores folder drops)
