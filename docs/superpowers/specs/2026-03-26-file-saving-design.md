# File Saving Design

## Summary

Wire up file saving for markdown edits in RustyNotes. The backend `write_file` command and frontend `tauri_ipc::write_file` already exist but are never called. This design adds save triggers, configurable save modes, new file creation, and a save-before-switch guard.

## Decisions

- **Save modes:** All three available (manual Cmd+S, auto-save after delay, auto-save on focus loss), selectable in settings
- **Save-before-switch:** Adapts to mode — auto-save modes save silently, manual mode shows a prompt
- **Conflict handling:** Skipped for v1 (no external-change detection/prompt)
- **New file creation:** Cmd+N creates untitled buffer, first save opens save dialog
- **Architecture:** All save logic in a dedicated Leptos frontend module (`save.rs`), no backend changes needed

## Config Changes

Add to `AppConfig` in `crates/rustynotes-common/src/lib.rs`:

```rust
pub save_mode: String,          // "manual" | "after_delay" | "on_focus_loss"
pub auto_save_delay_ms: u64,    // default 1000, only used when save_mode = "after_delay"
```

Default: `save_mode = "manual"`, `auto_save_delay_ms = 1000`.

## State Changes

Add to `AppState` in `crates/rustynotes-frontend/src/state.rs`:

```rust
pub save_status: RwSignal<SaveStatus>,             // Idle | Saving | Saved | Error(String)
pub last_save_timestamp: RwSignal<Option<f64>>,    // for watcher suppression
pub pending_file_switch: RwSignal<Option<String>>,  // triggers save-before-switch prompt
```

`SaveStatus` enum:
```rust
pub enum SaveStatus {
    Idle,
    Saving,
    Saved,
    Error(String),
}
```

Existing `is_dirty: RwSignal<bool>` stays as the source of truth for unsaved changes.

## Save Module (`save.rs`)

New file: `crates/rustynotes-frontend/src/save.rs`

### `perform_save(state: &AppState)`

1. Read `active_file_path` — if `None`, open save dialog via `tauri_ipc::save_file_dialog()`, set the path on success
2. Set `save_status = Saving`
3. Set `last_save_timestamp` to current time (for watcher suppression)
4. Call `tauri_ipc::write_file(path, content)`
5. On success: set `is_dirty = false`, `save_status = Saved`
6. On error: set `save_status = Error(msg)`

### `init_save_handlers(state: &AppState)`

Called once at app mount. Sets up:

- **Keyboard (Cmd+S / Ctrl+S):** `keydown` event listener on `document`, calls `perform_save`
- **Keyboard (Cmd+N / Ctrl+N):** `keydown` listener, clears `active_file_path` to `None`, clears `active_file_content`, sets `is_dirty = false`
- **Auto-save timer:** When `save_mode == "after_delay"`, uses `gloo_timers::callback::Interval` to check `is_dirty` every `auto_save_delay_ms` and save if dirty
- **Focus loss:** When `save_mode == "on_focus_loss"`, listens to `visibilitychange` on `document`, saves when page becomes hidden

Timer and event listeners cleaned up via Leptos `on_cleanup`.

### `guard_file_switch(state: &AppState, pending_path: String)`

Called by sidebar/miller/breadcrumb navigation before loading a new file. Instead of returning a bool (which would require blocking on user input), this function handles the full switch flow:

- If not dirty: proceed to load `pending_path` immediately
- If `save_mode == "after_delay"` or `"on_focus_loss"`: auto-save silently, then load `pending_path`
- If `save_mode == "manual"`: set a `pending_file_switch: RwSignal<Option<String>>` signal to show the prompt. The prompt's Save/Discard buttons complete the switch; Cancel clears the signal.

## UI Changes

### Toolbar Save Indicator

Replaces current dirty dot with richer feedback driven by `SaveStatus`:

| State | Display |
|-------|---------|
| `Idle` + not dirty | Nothing |
| `Idle` + dirty | Small dot (current behavior) |
| `Saving` | Subtle pulse/spinner |
| `Saved` | Checkmark, fades after ~1.5s back to `Idle` |
| `Error` | Red indicator with tooltip showing error message |

### Settings Panel

New "Saving" section:

- Dropdown: "Save mode" — Manual (Cmd+S), After delay, On focus loss
- Conditional input: When "After delay" selected, numeric input for delay in seconds (default 1s)
- Persists to `AppConfig` via existing `save_config_cmd` flow

### Save-Before-Switch Prompt (Manual Mode Only)

Lightweight modal/overlay: "You have unsaved changes"
- **Save** — calls `perform_save()` then switches
- **Discard** — switches without saving
- **Cancel** — stays on current file

### New File (Cmd+N)

- Clears editor to empty content
- Title area shows "Untitled"
- First Cmd+S opens save dialog to pick location/name
- After first save, behaves like any other opened file

## Integration Points

### Navigation Guards

Sidebar, miller columns, and breadcrumb file click handlers all call `guard_file_switch(state)` before loading a new file. If it returns `false`, the switch is aborted.

### File Watcher Suppression

When `perform_save` writes a file, the existing watcher fires a `file-changed` event. To prevent reloading our own write:

- Set `last_save_timestamp` right before calling `write_file`
- When `file-changed` arrives for the active file, ignore if within ~500ms of `last_save_timestamp`

### Dependencies

- Add `gloo-timers` to `crates/rustynotes-frontend/Cargo.toml` (WASM-compatible timer for auto-save interval)

### No Backend Changes

- `write_file` Tauri command already exists (`src-tauri/src/commands/fs.rs`)
- `tauri_ipc::write_file` already exists (`crates/rustynotes-frontend/src/tauri_ipc.rs`)
- `save_file_dialog` already exists for the new-file flow

## Out of Scope (v1)

- External file conflict detection/resolution (reload/keep/diff prompt)
- Undo history persistence across saves
- Multiple file tabs / buffer management
