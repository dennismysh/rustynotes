# File Saving Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up file saving so edits persist to disk, with configurable save modes (manual, auto-delay, focus-loss), new file creation, and save-before-switch guards.

**Architecture:** All save logic lives in a single frontend Leptos module (`save.rs`). The backend `write_file` command and `tauri_ipc::write_file` already exist. Config changes add `save_mode` and `auto_save_delay_ms` to `AppConfig`. Navigation components delegate file-switch decisions to the save module's guard function.

**Tech Stack:** Leptos 0.7 (CSR), Tauri 2 IPC, `gloo-timers` for WASM-compatible intervals, `web-sys` for DOM event listeners.

---

### Task 1: Add `SaveMode` enum and config fields

**Files:**
- Modify: `crates/rustynotes-common/src/lib.rs:30-78` (enums section)
- Modify: `crates/rustynotes-common/src/lib.rs:84-100` (AppConfig struct)
- Modify: `crates/rustynotes-common/src/lib.rs:152-167` (default helpers)
- Modify: `crates/rustynotes-common/src/lib.rs:173-185` (Default impl)
- Modify: `crates/rustynotes-common/src/lib.rs:212-368` (tests)

- [ ] **Step 1: Add `SaveMode` enum**

Add after the `NavMode` impl block (after line 78) in `crates/rustynotes-common/src/lib.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SaveMode {
    Manual,
    AfterDelay,
    OnFocusLoss,
}

impl Default for SaveMode {
    fn default() -> Self {
        Self::Manual
    }
}

impl std::fmt::Display for SaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "manual"),
            Self::AfterDelay => write!(f, "after_delay"),
            Self::OnFocusLoss => write!(f, "on_focus_loss"),
        }
    }
}
```

- [ ] **Step 2: Add config fields to `AppConfig`**

Add two new fields to the `AppConfig` struct:

```rust
#[serde(default)]
pub save_mode: SaveMode,
#[serde(default = "default_auto_save_delay_ms")]
pub auto_save_delay_ms: u64,
```

- [ ] **Step 3: Add default helper**

Add the default helper function alongside the other default helpers:

```rust
pub fn default_auto_save_delay_ms() -> u64 {
    1000
}
```

- [ ] **Step 4: Update `Default` impl for `AppConfig`**

Add to the `Default` impl:

```rust
save_mode: SaveMode::default(),
auto_save_delay_ms: default_auto_save_delay_ms(),
```

- [ ] **Step 5: Add tests for SaveMode serialization and config fields**

Add to the `tests` module:

```rust
#[test]
fn test_save_mode_serde() {
    let mode = SaveMode::AfterDelay;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"after_delay\"");
    let parsed: SaveMode = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, SaveMode::AfterDelay);
}

#[test]
fn test_save_mode_default() {
    assert_eq!(SaveMode::default(), SaveMode::Manual);
}

#[test]
fn test_config_save_mode_defaults() {
    let config: AppConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(config.save_mode, SaveMode::Manual);
    assert_eq!(config.auto_save_delay_ms, 1000);
}

#[test]
fn test_config_with_save_mode() {
    let json = r#"{"save_mode":"after_delay","auto_save_delay_ms":2000}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.save_mode, SaveMode::AfterDelay);
    assert_eq!(config.auto_save_delay_ms, 2000);
}
```

- [ ] **Step 6: Run tests to verify**

Run: `cargo test -p rustynotes-common`
Expected: All tests pass, including new save mode tests.

- [ ] **Step 7: Commit**

```bash
git add crates/rustynotes-common/src/lib.rs
git commit -m "feat: add SaveMode enum and config fields for file saving"
```

---

### Task 2: Add `SaveStatus` enum and new state fields

**Files:**
- Modify: `crates/rustynotes-frontend/src/state.rs`

- [ ] **Step 1: Add `SaveStatus` enum and update `AppState`**

Replace the entire contents of `crates/rustynotes-frontend/src/state.rs` with:

```rust
use leptos::prelude::*;
use rustynotes_common::{AppConfig, EditorMode, FileNode, NavMode};

#[derive(Clone, Debug, PartialEq)]
pub enum SaveStatus {
    Idle,
    Saving,
    Saved,
    Error(String),
}

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
    // Save-related state
    pub save_status: RwSignal<SaveStatus>,
    pub last_save_timestamp: RwSignal<Option<f64>>,
    pub pending_file_switch: RwSignal<Option<String>>,
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
            save_status: RwSignal::new(SaveStatus::Idle),
            last_save_timestamp: RwSignal::new(None),
            pending_file_switch: RwSignal::new(None),
        }
    }
}

pub fn provide_app_state() {
    provide_context(AppState::new());
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/src/state.rs
git commit -m "feat: add SaveStatus enum and save-related state fields"
```

---

### Task 3: Add `gloo-timers` dependency

**Files:**
- Modify: `crates/rustynotes-frontend/Cargo.toml`

- [ ] **Step 1: Add dependency**

Add to `[dependencies]` in `crates/rustynotes-frontend/Cargo.toml`:

```toml
gloo-timers = { version = "0.3", features = ["futures"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/Cargo.toml
git commit -m "chore: add gloo-timers dependency for auto-save interval"
```

---

### Task 4: Create the save module (`save.rs`)

**Files:**
- Create: `crates/rustynotes-frontend/src/save.rs`
- Modify: `crates/rustynotes-frontend/src/main.rs` (add `mod save;`)

- [ ] **Step 1: Create `save.rs` with `perform_save`**

Create `crates/rustynotes-frontend/src/save.rs`:

```rust
//! File save logic — perform_save, keyboard shortcuts, auto-save timer,
//! focus-loss handler, and file-switch guard.

use leptos::prelude::*;
use rustynotes_common::SaveMode;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

use crate::state::{use_app_state, AppState, SaveStatus};
use crate::tauri_ipc;

// ---------------------------------------------------------------------------
// Core save function
// ---------------------------------------------------------------------------

/// Save the current editor content to disk. If no file path is set (new file),
/// opens a save dialog first.
pub async fn perform_save(state: &AppState) {
    let content = state.active_file_content.get_untracked();
    let path = state.active_file_path.get_untracked();

    let path = match path {
        Some(p) => p,
        None => {
            // New file — open save dialog
            match tauri_ipc::save_file_dialog("Untitled.md").await {
                Ok(Some(p)) => {
                    state.active_file_path.set(Some(p.clone()));
                    p
                }
                Ok(None) => return, // user cancelled
                Err(e) => {
                    state.save_status.set(SaveStatus::Error(e));
                    return;
                }
            }
        }
    };

    state.save_status.set(SaveStatus::Saving);

    // Record timestamp so the file watcher ignores our own write
    let now = js_sys::Date::now();
    state.last_save_timestamp.set(Some(now));

    match tauri_ipc::write_file(&path, &content).await {
        Ok(()) => {
            state.is_dirty.set(false);
            state.save_status.set(SaveStatus::Saved);
        }
        Err(e) => {
            state.save_status.set(SaveStatus::Error(e));
        }
    }
}

// ---------------------------------------------------------------------------
// Save handlers (keyboard, auto-save, focus loss)
// ---------------------------------------------------------------------------

/// Initialize all save-related event handlers. Call once at app mount.
pub fn init_save_handlers(state: &AppState) {
    init_keyboard_shortcuts(state);
    init_auto_save(state);
    init_focus_loss_save(state);
}

fn is_mac() -> bool {
    js_sys::eval("navigator.platform.includes('Mac')")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn init_keyboard_shortcuts(state: &AppState) {
    let state = state.clone();
    let mac = is_mac();

    let handler = Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let Ok(ke) = ev.dyn_into::<KeyboardEvent>() else { return };
        let modifier = if mac { ke.meta_key() } else { ke.ctrl_key() };
        if !modifier {
            return;
        }

        match ke.key().as_str() {
            "s" => {
                ke.prevent_default();
                let state = state.clone();
                leptos::task::spawn_local(async move {
                    perform_save(&state).await;
                });
            }
            "n" => {
                ke.prevent_default();
                state.active_file_path.set(None);
                state.active_file_content.set(String::new());
                state.is_dirty.set(false);
                state.save_status.set(SaveStatus::Idle);
            }
            _ => {}
        }
    });

    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback(
            "keydown",
            handler.as_ref().unchecked_ref(),
        );
    }
    handler.forget();
}

fn init_auto_save(state: &AppState) {
    let state = state.clone();

    // Reactive effect: when config changes, (re)start or stop the auto-save timer.
    // The effect owns the interval handle via a local variable.
    Effect::new(move |prev_handle: Option<Option<gloo_timers::callback::Interval>>| {
        // Drop previous interval if any
        drop(prev_handle.flatten());

        let config = state.app_config.get();
        let Some(config) = config else {
            return None;
        };

        if config.save_mode != SaveMode::AfterDelay {
            return None;
        }

        let delay = config.auto_save_delay_ms.max(200); // floor at 200ms
        let state = state.clone();

        let interval = gloo_timers::callback::Interval::new(delay as u32, move || {
            if state.is_dirty.get_untracked() {
                let state = state.clone();
                leptos::task::spawn_local(async move {
                    perform_save(&state).await;
                });
            }
        });

        Some(interval)
    });
}

fn init_focus_loss_save(state: &AppState) {
    let state = state.clone();

    let handler = Closure::<dyn Fn()>::new(move || {
        let config = state.app_config.get_untracked();
        let is_focus_mode = config
            .as_ref()
            .map(|c| c.save_mode == SaveMode::OnFocusLoss)
            .unwrap_or(false);

        if !is_focus_mode || !state.is_dirty.get_untracked() {
            return;
        }

        // Check that the page is actually hidden (not just blurred)
        let hidden = web_sys::window()
            .and_then(|w| w.document())
            .map(|d| d.hidden())
            .unwrap_or(false);

        if hidden {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                perform_save(&state).await;
            });
        }
    });

    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ = document.add_event_listener_with_callback(
            "visibilitychange",
            handler.as_ref().unchecked_ref(),
        );
    }
    handler.forget();
}

// ---------------------------------------------------------------------------
// File-switch guard
// ---------------------------------------------------------------------------

/// Call before switching to a new file. Handles save-before-switch logic
/// based on the current save mode.
///
/// - Not dirty: loads `pending_path` immediately.
/// - Auto-save modes: saves silently, then loads.
/// - Manual mode: sets `pending_file_switch` signal to show the prompt UI.
pub fn guard_file_switch(state: &AppState, pending_path: String) {
    if !state.is_dirty.get_untracked() {
        load_file(state, pending_path);
        return;
    }

    let config = state.app_config.get_untracked();
    let save_mode = config
        .as_ref()
        .map(|c| c.save_mode.clone())
        .unwrap_or_default();

    match save_mode {
        SaveMode::AfterDelay | SaveMode::OnFocusLoss => {
            // Auto-save silently, then switch
            let state = state.clone();
            leptos::task::spawn_local(async move {
                perform_save(&state).await;
                load_file(&state, pending_path);
            });
        }
        SaveMode::Manual => {
            // Show the save-before-switch prompt
            state.pending_file_switch.set(Some(pending_path));
        }
    }
}

/// Load a file by path into the editor state.
pub fn load_file(state: &AppState, path: String) {
    state.active_file_path.set(Some(path.clone()));
    let state = state.clone();
    leptos::task::spawn_local(async move {
        match tauri_ipc::read_file(&path).await {
            Ok(content) => {
                state.active_file_content.set(content);
                state.is_dirty.set(false);
                state.save_status.set(SaveStatus::Idle);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to read file: {e}").into());
            }
        }
    });
}
```

- [ ] **Step 2: Register the module in `main.rs`**

Add `mod save;` to `crates/rustynotes-frontend/src/main.rs`:

```rust
mod app;
mod bridge;
mod components;
mod save;
mod state;
mod tauri_ipc;
mod theme;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors (may show unused warnings — that's fine).

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/src/save.rs crates/rustynotes-frontend/src/main.rs
git commit -m "feat: add save module with perform_save, keyboard shortcuts, auto-save, and file-switch guard"
```

---

### Task 5: Initialize save handlers at app mount

**Files:**
- Modify: `crates/rustynotes-frontend/src/app.rs:34-66` (MainView component)

- [ ] **Step 1: Call `init_save_handlers` in `MainView`**

In `crates/rustynotes-frontend/src/app.rs`, add the import at the top:

```rust
use crate::save;
```

Then in the `MainView` component, add the init call right after `let state = use_app_state();`:

```rust
// Initialize save handlers (keyboard shortcuts, auto-save timer, focus-loss)
save::init_save_handlers(&state);
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/src/app.rs
git commit -m "feat: initialize save handlers at app mount"
```

---

### Task 6: Wire navigation components to use `guard_file_switch`

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/navigation/sidebar.rs`
- Modify: `crates/rustynotes-frontend/src/components/navigation/miller_columns.rs`
- Modify: `crates/rustynotes-frontend/src/components/navigation/breadcrumb.rs`

- [ ] **Step 1: Update sidebar `TreeNode`**

In `crates/rustynotes-frontend/src/components/navigation/sidebar.rs`, add the import:

```rust
use crate::save;
```

Replace the file-opening logic in `handle_click` (lines 92-111). The current code:

```rust
let handle_click = move |_| {
    if entry_is_dir {
        expanded.update(|v| *v = !*v);
    } else {
        let path = path_for_click.clone();
        state.active_file_path.set(Some(path.clone()));
        leptos::task::spawn_local(async move {
            match tauri_ipc::read_file(&path).await {
                Ok(content) => {
                    state.active_file_content.set(content);
                    state.is_dirty.set(false);
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to read file: {e}").into(),
                    );
                }
            }
        });
    }
};
```

Replace with:

```rust
let handle_click = move |_| {
    if entry_is_dir {
        expanded.update(|v| *v = !*v);
    } else {
        save::guard_file_switch(&state, path_for_click.clone());
    }
};
```

Do the same for the `handle_keydown` closure — replace the `else` branch (lines 121-137) that duplicates the file-open logic with:

```rust
} else {
    save::guard_file_switch(&state, entry_path.clone());
}
```

- [ ] **Step 2: Update miller columns**

In `crates/rustynotes-frontend/src/components/navigation/miller_columns.rs`, add the import:

```rust
use crate::save;
```

In `handle_click` (around line 100-127), replace the file-opening `else` branch:

```rust
} else {
    // Select this file, trim columns after
    selected_paths.update(|sp| {
        sp.truncate(col_index + 1);
        sp[col_index] = Some(entry.path.clone());
    });
    columns.update(|cols| {
        cols.truncate(col_index + 1);
    });

    // Open the file
    let path = entry.path.clone();
    state.active_file_path.set(Some(path.clone()));
    leptos::task::spawn_local(async move {
        match tauri_ipc::read_file(&path).await {
            Ok(content) => {
                state.active_file_content.set(content);
                state.is_dirty.set(false);
            }
            Err(e) => {
                web_sys::console::error_1(
                    &format!("Failed to read file: {e}").into(),
                );
            }
        }
    });
}
```

Replace with:

```rust
} else {
    // Select this file, trim columns after
    selected_paths.update(|sp| {
        sp.truncate(col_index + 1);
        sp[col_index] = Some(entry.path.clone());
    });
    columns.update(|cols| {
        cols.truncate(col_index + 1);
    });

    save::guard_file_switch(&state, entry.path.clone());
}
```

- [ ] **Step 3: Update breadcrumb**

In `crates/rustynotes-frontend/src/components/navigation/breadcrumb.rs`, add the import:

```rust
use crate::save;
```

In `handle_dropdown_item_click` (around lines 138-154), replace the file-opening `else` branch:

```rust
} else {
    let path = entry.path.clone();
    state.active_file_path.set(Some(path.clone()));
    leptos::task::spawn_local(async move {
        match tauri_ipc::read_file(&path).await {
            Ok(content) => {
                state.active_file_content.set(content);
                state.is_dirty.set(false);
            }
            Err(e) => {
                web_sys::console::error_1(
                    &format!("Failed to read file: {e}").into(),
                );
            }
        }
    });
}
```

Replace with:

```rust
} else {
    save::guard_file_switch(&state, entry.path.clone());
}
```

- [ ] **Step 4: Remove unused `tauri_ipc` import if needed**

Check each file — if `tauri_ipc` is no longer used directly (sidebar still uses it for other things), remove the unused import to avoid warnings.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add crates/rustynotes-frontend/src/components/navigation/sidebar.rs \
       crates/rustynotes-frontend/src/components/navigation/miller_columns.rs \
       crates/rustynotes-frontend/src/components/navigation/breadcrumb.rs
git commit -m "feat: wire navigation components to save guard for file switching"
```

---

### Task 7: Update toolbar save indicator

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/toolbar.rs:195-242` (view)

- [ ] **Step 1: Import SaveStatus and add status-driven indicator**

In `crates/rustynotes-frontend/src/components/toolbar.rs`, add the import:

```rust
use crate::state::SaveStatus;
```

Add a new signal for the save status and the "Saved" fade timer. After the existing signal declarations (around line 50), add:

```rust
let save_status = state.save_status;
```

- [ ] **Step 2: Replace the dirty indicator in the view**

Replace the current dirty indicator block (lines 199-211):

```rust
<Show when=move || active_filename.get().is_some()>
    <div class="toolbar-filename">
        <Show when=move || is_dirty.get()>
            <span class="dirty-indicator" aria-label="Unsaved changes" />
        </Show>
        <span
            class="toolbar-filename-text"
            title=move || active_file_path.get().unwrap_or_default()
        >
            {move || active_filename.get().unwrap_or_default()}
        </span>
    </div>
</Show>
```

Replace with:

```rust
<Show when=move || active_filename.get().is_some()>
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
</Show>
```

- [ ] **Step 3: Add "Saved" fade-back effect**

Add an `Effect` in the `Toolbar` component body (after the existing Effects) that resets `SaveStatus::Saved` back to `Idle` after 1.5 seconds:

```rust
// Reset "Saved" status back to Idle after 1.5s
Effect::new(move |_| {
    if save_status.get() == SaveStatus::Saved {
        let save_status = save_status;
        leptos::task::spawn_local(async move {
            sleep_ms(1500).await;
            // Only reset if still in Saved state
            if save_status.get_untracked() == SaveStatus::Saved {
                save_status.set(SaveStatus::Idle);
            }
        });
    }
});
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/components/toolbar.rs
git commit -m "feat: update toolbar with save status indicator and Untitled display"
```

---

### Task 8: Add save-before-switch prompt UI

**Files:**
- Modify: `crates/rustynotes-frontend/src/app.rs` (add prompt overlay to MainView)

- [ ] **Step 1: Add the prompt component in the MainView**

In `crates/rustynotes-frontend/src/app.rs`, update the imports:

```rust
use crate::save;
use crate::state::SaveStatus;
```

In the `MainView` view, add the save-before-switch prompt overlay just before the closing `</div>` of `app-container`:

```rust
view! {
    <div class="app-container">
        <Toolbar />
        <Show
            when=has_folder
            fallback=|| view! { <WelcomeEmptyState /> }
        >
            {nav_view}
            <div class="main-content">
                {editor_view}
            </div>
        </Show>
        // Save-before-switch prompt
        <Show when=move || state.pending_file_switch.get().is_some()>
            <div class="modal-overlay">
                <div class="modal-dialog">
                    <p>"You have unsaved changes"</p>
                    <div class="modal-actions">
                        <button
                            class="modal-btn primary"
                            on:click=move |_| {
                                let pending = state.pending_file_switch.get_untracked();
                                state.pending_file_switch.set(None);
                                if let Some(path) = pending {
                                    let state = state.clone();
                                    leptos::task::spawn_local(async move {
                                        save::perform_save(&state).await;
                                        save::load_file(&state, path);
                                    });
                                }
                            }
                        >
                            "Save"
                        </button>
                        <button
                            class="modal-btn"
                            on:click=move |_| {
                                let pending = state.pending_file_switch.get_untracked();
                                state.pending_file_switch.set(None);
                                state.is_dirty.set(false);
                                state.save_status.set(SaveStatus::Idle);
                                if let Some(path) = pending {
                                    save::load_file(&state, path);
                                }
                            }
                        >
                            "Discard"
                        </button>
                        <button
                            class="modal-btn"
                            on:click=move |_| {
                                state.pending_file_switch.set(None);
                            }
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    </div>
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/src/app.rs
git commit -m "feat: add save-before-switch prompt modal for manual save mode"
```

---

### Task 9: Add CSS for save indicator and modal

**Files:**
- Modify: The main CSS file (find via `glob src-tauri/*.css` or `styles/*.css`)

- [ ] **Step 1: Find the CSS file**

Run: `find . -name '*.css' -not -path '*/node_modules/*' | head -20`

Identify the main stylesheet.

- [ ] **Step 2: Add save indicator styles**

Add to the CSS file:

```css
/* Save status indicators */
.save-indicator {
    display: inline-flex;
    align-items: center;
    margin-right: 6px;
    font-size: 12px;
    line-height: 1;
}

.save-indicator.saving {
    color: var(--text-muted);
    animation: spin 1s linear infinite;
}

.save-indicator.saved {
    color: var(--accent);
    animation: fade-out 1.5s ease forwards;
}

.save-indicator.error {
    color: var(--error, #e74c3c);
    cursor: help;
}

@keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
}

@keyframes fade-out {
    0%, 60% { opacity: 1; }
    100% { opacity: 0; }
}

/* Save-before-switch modal */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.modal-dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px 24px;
    min-width: 300px;
    max-width: 400px;
}

.modal-dialog p {
    margin: 0 0 16px;
    color: var(--text);
    font-size: 14px;
}

.modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
}

.modal-btn {
    padding: 6px 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
}

.modal-btn:hover {
    background: var(--surface-hover, var(--border));
}

.modal-btn.primary {
    background: var(--accent);
    color: var(--accent-fg, #fff);
    border-color: var(--accent);
}

.modal-btn.primary:hover {
    opacity: 0.9;
}
```

- [ ] **Step 3: Verify the app builds**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add <css-file-path>
git commit -m "style: add CSS for save indicator and save-before-switch modal"
```

---

### Task 10: Add saving settings category

**Files:**
- Create: `crates/rustynotes-frontend/src/components/settings/categories/saving.rs`
- Modify: `crates/rustynotes-frontend/src/components/settings/categories/mod.rs`
- Modify: `crates/rustynotes-frontend/src/components/settings/settings_window.rs`

- [ ] **Step 1: Create saving settings panel**

Create `crates/rustynotes-frontend/src/components/settings/categories/saving.rs`:

```rust
//! Saving settings panel — save mode and auto-save delay.

use leptos::prelude::*;
use rustynotes_common::{AppConfig, SaveMode};

use crate::components::settings::shared::{SettingRow, SettingSelect, SettingSlider};
use crate::tauri_ipc;

#[component]
pub fn SavingSettings() -> impl IntoView {
    let config = RwSignal::new(Option::<AppConfig>::None);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tauri_ipc::get_config().await {
                Ok(c) => config.set(Some(c)),
                Err(e) => {
                    web_sys::console::error_1(&format!("get_config: {e}").into());
                }
            }
        });
    });

    let update = move |updater: Box<dyn FnOnce(&mut AppConfig)>| {
        if let Some(mut c) = config.get_untracked() {
            updater(&mut c);
            config.set(Some(c.clone()));
            leptos::task::spawn_local(async move {
                if let Err(e) = tauri_ipc::save_config_cmd(c).await {
                    web_sys::console::error_1(&format!("save_config: {e}").into());
                }
            });
        }
    };

    let save_mode = Signal::derive(move || {
        config
            .get()
            .map(|c| c.save_mode.to_string())
            .unwrap_or_else(|| "manual".into())
    });

    let auto_save_delay = Signal::derive(move || {
        config
            .get()
            .map(|c| (c.auto_save_delay_ms as f64) / 1000.0)
            .unwrap_or(1.0)
    });

    let is_after_delay = Signal::derive(move || {
        config
            .get()
            .map(|c| c.save_mode == SaveMode::AfterDelay)
            .unwrap_or(false)
    });

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Saving"</h2>
            <p class="settings-category-subtitle">"When and how files are saved"</p>

            <SettingRow label="Save Mode" description="When to save your changes">
                <SettingSelect
                    value=save_mode
                    options=vec![
                        ("manual".into(), "Manual (Cmd+S)".into()),
                        ("after_delay".into(), "After Delay".into()),
                        ("on_focus_loss".into(), "On Focus Loss".into()),
                    ]
                    on_change=move |v| {
                        let mode = match v.as_str() {
                            "after_delay" => SaveMode::AfterDelay,
                            "on_focus_loss" => SaveMode::OnFocusLoss,
                            _ => SaveMode::Manual,
                        };
                        update(Box::new(move |c| { c.save_mode = mode; }));
                    }
                />
            </SettingRow>

            <Show when=move || is_after_delay.get()>
                <SettingRow label="Auto-save Delay" description="Seconds between edits and auto-save">
                    <SettingSlider
                        value=auto_save_delay
                        min=0.2
                        max=10.0
                        step=0.1
                        unit="s".to_string()
                        on_change=move |v| {
                            let ms = (v * 1000.0) as u64;
                            update(Box::new(move |c| { c.auto_save_delay_ms = ms; }));
                        }
                    />
                </SettingRow>
            </Show>
        </div>
    }
}
```

- [ ] **Step 2: Register in categories mod**

Update `crates/rustynotes-frontend/src/components/settings/categories/mod.rs`:

```rust
pub mod advanced;
pub mod appearance;
pub mod editor;
pub mod preview;
pub mod saving;

pub use advanced::AdvancedSettings;
pub use appearance::AppearanceSettings;
pub use editor::EditorSettings;
pub use preview::PreviewSettings;
pub use saving::SavingSettings;
```

- [ ] **Step 3: Add saving tab to settings window**

In `crates/rustynotes-frontend/src/components/settings/settings_window.rs`, add the import:

```rust
use crate::components::settings::categories::{
    AdvancedSettings, AppearanceSettings, EditorSettings, PreviewSettings, SavingSettings,
};
```

Add the "Saving" category to the `categories()` function (after "Editor"):

```rust
SettingsCategory { id: "saving", label: "Saving", icon: "\u{1F4BE}" },
```

Add the match arm in the view (in the `move || match active_category.get().as_str()` block):

```rust
"saving"   => view! { <SavingSettings /> }.into_any(),
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/components/settings/categories/saving.rs \
       crates/rustynotes-frontend/src/components/settings/categories/mod.rs \
       crates/rustynotes-frontend/src/components/settings/settings_window.rs
git commit -m "feat: add Saving category to settings window"
```

---

### Task 11: Suppress file watcher for own writes

**Files:**
- Modify: `crates/rustynotes-frontend/src/app.rs` (where file-changed listener is set up, or wherever it's initialized)

- [ ] **Step 1: Find where `listen_file_changed` is called**

Run: `grep -rn "listen_file_changed" crates/rustynotes-frontend/src/`

This will show where the file-changed event listener is registered.

- [ ] **Step 2: Add timestamp check**

Wherever the `listen_file_changed` callback processes an event for the active file, add a check:

```rust
// Ignore file-changed events within 500ms of our own save
let last_save = state.last_save_timestamp.get_untracked();
if let Some(ts) = last_save {
    let now = js_sys::Date::now();
    if (now - ts) < 500.0 {
        return; // Our own write, ignore
    }
}
```

If `listen_file_changed` is not currently connected to any reload logic, this task is a no-op — just document the suppression pattern for when reload is implemented.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p rustynotes-frontend --target wasm32-unknown-unknown`
Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/src/
git commit -m "feat: suppress file watcher events from own saves"
```

---

### Task 12: Manual smoke test

**Files:** None (testing only)

- [ ] **Step 1: Build and run the app**

Run: `cd "/Users/dennis/programming projects/rustynotes" && pnpm tauri dev`

- [ ] **Step 2: Test manual save (Cmd+S)**

1. Open a folder and click a `.md` file
2. Make an edit in the editor
3. Verify the dirty indicator (dot) appears in the toolbar
4. Press Cmd+S
5. Verify the indicator changes to a checkmark then fades
6. Close and reopen the file — verify changes persisted

- [ ] **Step 3: Test auto-save after delay**

1. Open Settings > Saving
2. Change save mode to "After Delay"
3. Set delay to 1 second
4. Make an edit
5. Wait ~1 second
6. Verify the dirty indicator clears automatically

- [ ] **Step 4: Test new file (Cmd+N)**

1. Press Cmd+N
2. Verify editor clears and shows "Untitled"
3. Type some content
4. Press Cmd+S
5. Verify save dialog appears
6. Save the file and verify it appears in the sidebar

- [ ] **Step 5: Test save-before-switch (manual mode)**

1. Ensure save mode is "Manual"
2. Make an edit to the current file (dirty indicator shows)
3. Click a different file in the sidebar
4. Verify the "You have unsaved changes" prompt appears
5. Test all three buttons: Save, Discard, Cancel

- [ ] **Step 6: Test focus-loss save**

1. Change save mode to "On Focus Loss"
2. Make an edit
3. Switch to another application
4. Switch back — verify changes were saved (dirty indicator gone)
