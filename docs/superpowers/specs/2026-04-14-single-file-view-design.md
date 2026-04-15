# Single-File View — Design Spec

**Date:** 2026-04-14
**Scope:** Support opening and editing a single markdown file without opening a folder. Adds Finder file association, drag-and-drop, Cmd+O, and CLI argument entry points. Each single-file open spawns its own minimal window.

## Problem

RustyNotes currently forces the user through a folder. On cold launch the app reopens the last folder or shows a "Open Folder" welcome screen; the main editor (`app-body`) is literally hidden when no folder is open (`app.rs` line 101). There is no way to double-click a `.md` file in Finder and get an editor — the user has to open the containing folder and then navigate to the file.

This is friction for legitimate single-file workflows: reviewing a downloaded README, editing a standalone note on the Desktop, or wanting RustyNotes to be the default macOS handler for `.md` files.

## Goals

1. Open a specific markdown file without requiring the user to open its parent folder.
2. Support four entry points: Finder file association / "Open With…", drag-and-drop, in-app `Cmd+O` / File menu, and `rustynotes <path>` CLI.
3. Each single-file open spawns its own window — folder windows stay untouched.
4. Minimal chrome in single-file windows (traffic lights + filename + save indicator + overflow menu), WYSIWYG-first.
5. Add a native macOS File menu as the home for Open File / Open Folder / Open Recent.
6. Make recently-opened single files discoverable (welcome screen + File > Open Recent).

## Non-Goals

- Session restoration across app restarts (reopening yesterday's single-file windows).
- External-change detection for single-file windows (no folder-level file watcher applied).
- Window tab-merging (macOS "Merge All Windows").
- Windows / Linux file-association installers (macOS is primary).
- Converting every editor operation into a new window (Approach 3 rejected).

## User flows

### Flow 1 — Finder double-click (file association)

1. User double-clicks `notes.md` in Finder.
2. macOS dispatches the open-file event to RustyNotes (registered via `fileAssociations` in `tauri.conf.json`).
3. If the app isn't running, it starts; if it is, the single-instance plugin forwards the args to the running instance.
4. Backend spawns a new window with label `file-<uuid>` and URL `/file?path=<url-encoded-abs-path>`.
5. Frontend mounts `SingleFileView`, reads `path` from the query string, fetches file content, renders in WYSIWYG.
6. Path is added to `recent_files` in config (dedup, cap at 10).

### Flow 2 — Cmd+O (any window) or File > Open File…

1. User hits `Cmd+O` or clicks File → Open File….
2. Native open dialog filtered to `.md` / `.markdown`.
3. Same as Flow 1 from step 4 onward — always spawns a new window, regardless of which window invoked it.

### Flow 3 — Drag-and-drop

1. User drags a `.md` file onto the Dock icon OR onto a window's title bar area.
2. Tauri file-drop event fires with the path(s).
3. For each path: same as Flow 1 from step 4 onward. Cap at 10 paths per drop.

### Flow 4 — CLI argument

1. `rustynotes path/to/file.md`.
2. If RustyNotes is already running, the single-instance plugin forwards args; otherwise it's the initial launch.
3. Same as Flow 1 from step 4 onward.

### Flow 5 — Cold launch, no args

Current behavior preserved: reopen the last folder (or show the Welcome screen if none). No single-file window is created.

### Flow 6 — Cold launch, with file arg

App starts with a file argument (Finder opening it on a fresh boot). The "auto-reopen last folder" behavior is **skipped** for this launch — the user's intent is clearly "just this file." A single-file window is spawned; no folder window is created for this launch. Next plain launch (no args) brings the folder window back.

## Architecture

### Frontend (Leptos)

**New route** in `app.rs`:

```rust
<Route path=path!("/file") view=SingleFileView />
```

**New component tree** under `components/single_file/`:

- `mod.rs` — `SingleFileView` component. Reads `path` query param from `window.location.search`, triggers file load via `save::load_file`, mounts `<SlimTitleBar>` + WYSIWYG editor.
- `slim_titlebar.rs` — traffic lights + filename + save indicator + ••• overflow menu. Close/minimize/maximize handlers reuse `tauri_ipc::{close_current_window, minimize_current_window, toggle_maximize_current_window}`.
- `overflow_menu.rs` — dropdown with "Switch to Source mode", "Open in folder window", "Settings", "Export HTML…".

**Small refactor:** extract the save indicator + filename rendering from `toolbar.rs` into a shared component (`components/save_indicator.rs` or similar) used by both the full Toolbar and the SlimTitleBar, to avoid drift.

**Reused as-is:**

- `AppState` context (save logic, theme, config, editor state signals).
- `save::perform_save`, `save::load_file` — both already path-agnostic; they don't require a folder.
- Editor components (`WysiwygEditor`, `SourceEditor`).
- `theme::apply_theme`, update-status listener, settings window wiring.

**Per-window state:** each window has its own fresh `AppState` (via `provide_context`) — single-file windows don't share in-memory editor state with the folder window. **Config is shared** through the backend (`get_config` on mount, `save_config_cmd` on change, `config_changed` event broadcasts to all windows), so theme changes and `recent_files` updates propagate everywhere.

### Backend (Rust / Tauri)

**New Tauri plugin:** `tauri-plugin-single-instance`. On second launch, forwards `(argv, cwd)` to the running instance via a callback instead of spawning a second process.

**`tauri.conf.json` additions:**

- `fileAssociations`: `[{ "ext": ["md", "markdown"], "role": "Editor" }]`.
- The single-file window uses the same `visible: false` + `titleBarStyle: Overlay` + `hiddenTitle: true` setup as the main window (per the "Tauri v2 Window Visibility for Flash Prevention" lesson). Window is shown after theme applies.

**New Tauri commands** (in `src-tauri/src/commands/`):

- `open_file_in_new_window(path: String)` — canonicalizes path, validates existence and UTF-8 readability, generates window label `file-<uuid>`. If a single-file window with the same canonicalized path already exists, calls `set_focus()` on it and returns. Otherwise creates a window with URL `/file?path=<url-encoded>`, updates `recent_files` in config.
- `open_file_dialog()` — wraps the dialog plugin; used by the File menu and the in-app `Cmd+O` path.

**File-open event router** (in `lib.rs` `setup`):

- **Startup:** parse `std::env::args` directly (simpler than the `cli` plugin for a single positional path). If exactly one argument that points to an existing UTF-8 file → spawn single-file window and **skip auto-reopening last folder** for this launch.
- **Single-instance callback:** receives `(argv, cwd)` on second-launch attempts. Resolve each path relative to `cwd`, route through `open_file_in_new_window`.
- **macOS `RunEvent::Opened { urls }`:** for `file://` URLs, route through `open_file_in_new_window`.

**Path → window-label map** lives in Tauri-managed state:

```rust
pub struct FileWindows {
    map: Mutex<HashMap<PathBuf, String>>, // canonical path → window label
}
```

Maintained by `open_file_in_new_window` (insert) and by a window-close hook (remove).

**Drag-and-drop:** the main window and any single-file windows listen for Tauri's `FileDrop` event. When files are dropped on *any* window → route through `open_file_in_new_window` for each (dropping on a single-file window spawns another new window rather than replacing its content — consistent with the "single-file always = new window" rule). Dock-icon drops arrive via `RunEvent::Opened`.

### macOS native File menu

Wired in `lib.rs` via `tauri::menu::{Menu, MenuBuilder, Submenu, MenuItem}`. The File submenu is prepended to Tauri's default menu (which provides Edit / View / Window). Menu items emit events the frontend listens for:

| Menu item         | Shortcut      | Emits event             | Handler                                              |
|-------------------|---------------|-------------------------|------------------------------------------------------|
| New File          | `Cmd+N`       | `menu:new-file`         | Context-dependent: in a folder window, clear the active file and create an untitled buffer in place (existing behavior). In a single-file window, spawn a *new* untitled single-file window (replacing the current file would be destructive). With no window open (menu-only invocation), spawn a new untitled single-file window. |
| Open File…        | `Cmd+O`       | `menu:open-file`        | `open_file_dialog()` → `open_file_in_new_window()`   |
| Open Folder…      | `Cmd+Shift+O` | `menu:open-folder`      | Existing `open_folder` logic; routes to main window if open, else creates it |
| Open Recent ▸     | —             | `menu:open-recent:<path>` | Dynamic submenu; rebuilt on `config_changed`       |
| Save              | `Cmd+S`       | `menu:save`             | Frontend's existing Cmd+S path                       |
| Export HTML…      | —             | `menu:export`           | Frontend's existing export path                      |

**Keyboard accelerator ownership:** menu items registered with Tauri hold the canonical accelerator binding for `Cmd+N`, `Cmd+O`, `Cmd+Shift+O`, `Cmd+S`. The existing JS `keydown` handlers in `save.rs` (`Cmd+S`, `Cmd+N`) must be removed in favor of listening for the corresponding `menu:*` events, so the same shortcut isn't handled twice. `Cmd+1–4` (editor-mode switch) and `Cmd+K` (search) and `Cmd+,` (settings) remain JS-handled since they have no menu-item counterpart.

**Open Recent submenu** is built from `recent_files` + `recent_folders`. Sections:

```
Recent Files
  - ~/Downloads/readme.md
  - ~/Desktop/scratch.md
  - ...
  ─────────────
Recent Folders
  - Documents/notes/
  - Projects/mybook/
  - ...
  ─────────────
Clear Recent
```

Menu is rebuilt on `config_changed`. "Clear Recent" empties both lists in config.

### Tauri capabilities

Per the "Tauri v2 Capability Window Scope" lesson, capabilities are per-window-label. Extend the default capability's `windows` array to include `"file-*"` (glob match). Grant single-file windows the same IPC surface as the main window, *except* `watch_folder` and `list_directory` which aren't used there — omitting them is defense-in-depth, not strictly required.

## Data model

### `AppConfig` addition

```rust
pub struct AppConfig {
    // ... existing fields
    pub recent_files: Vec<String>,  // NEW — max 10, most-recent first
}
```

Serde default: empty `Vec`. No migration logic needed — absent field deserializes to default (per the "Saved Config Overrides Rust Default Changes" lesson, existing users just get an empty list on first read).

### Recent-files management

- On successful file load via `open_file_in_new_window` → canonicalize path, push to front of `recent_files`, dedup, cap at 10, persist config.
- On attempt to open a path that no longer exists (stale entry in config) → remove from `recent_files`, show a transient "File not found" message on the welcome screen if relevant, persist config.
- Pruning stale entries: lazily when `recent_files` is read for display.

### Query-param protocol

Single-file windows get URL `/file?path=<url-encoded-absolute-path>`. The frontend reads `window.location.search` once on mount. Relative or missing path → redirect to `/` (folder window fallback). URL-encoding handles spaces, `#`, `?`, unicode.

## Welcome screen updates

Current layout: title → welcome blurb → "Open Folder" CTA → `Recent` (folders only) → shortcut hints.

New layout:

```
Welcome to RustyNotes
[blurb]
[Open Folder]  [Open File]    ← new button

Recent
  Folders
    - Documents/notes/
    - Projects/mybook/
    - ...
  Files
    - readme.md           ~/Downloads/
    - scratch.md          ~/Desktop/
    - ...

[shortcut hints]
```

- Two subsections under one "Recent" heading, max 5 each.
- Each recent-file row: filename (bold) + dim parent path. Click → spawn single-file window for that path.
- Missing files are silently pruned the next time the list is read.
- The new "Open File" button triggers the same path as Cmd+O / File > Open File.

## Single-file window chrome

Top strip (the entire strip is `-webkit-app-region: drag` except buttons):

```
[● ● ●]    notes.md  [●]                                    [⋯]
 traffic   filename + dirty/save indicator                  overflow menu
 lights
```

- **Traffic lights** — reuse existing close/minimize/maximize handlers. The close button triggers the Save/Discard/Cancel modal if dirty.
- **Filename** — left-aligned basename of the current path (consistent with macOS document-window title convention).
- **Save indicator** — same Saving/Saved/Error/dirty-dot rendering as the main toolbar, extracted into the shared component.
- **••• overflow menu** items:
  - **Switch to Source mode** — toggles `editor_mode` locally in that window's `AppState`. Doesn't touch saved config.
  - **Open in folder window** — spawns (or focuses) a folder window at the file's parent directory, with the file selected in the sidebar.
  - **Settings** — opens the existing settings window.
  - **Export HTML…** — existing export path.

Keyboard shortcuts stay live in single-file windows: `Cmd+S` (save), `Cmd+,` (settings), `Cmd+O` (open another file → new window), `Cmd+1–4` (mode switch — overrides the WYSIWYG default for that window), `Cmd+N` (new untitled — opens a new single-file window with no path set, save dialog on first save).

## Edge cases

- **Unsaved on close.** Tauri's `CloseRequested` event is intercepted. If `is_dirty`, show the existing Save/Discard/Cancel modal; on Cancel, `event.prevent_close()`. Applies to traffic-light close, Cmd+W, and app-quit.
- **Duplicate open.** The path → window-label map handles this. Canonicalize the incoming path, look up, focus if found, else spawn.
- **File moved/deleted while window open.** No folder watcher runs for single-file windows in v1. Saving either succeeds or errors via the existing save-error UI. External-change detection is v2.
- **Non-markdown file via CLI/drag.** File association filter only applies to Finder routing, not CLI/drag. Policy: accept any UTF-8 file, open in WYSIWYG. If not UTF-8 or non-existent, mount an error view ("Can't open this file") in the window with a Close button instead of the editor.
- **Symlinks / aliases.** Canonicalize via `std::fs::canonicalize` before using path as the window-identity key and before writing to `recent_files`. Prevents duplicate windows for the same underlying file.
- **Multiple paths at once.** Spawn one window per path, capped at 10.
- **Path with special characters.** URL-encode when building the query string; URL-decode in `SingleFileView`.
- **Folder dropped on app.** Routed to the existing `open_folder` logic — spawn/focus the folder window with that path. (Minor convenience; deferrable if it adds scope.)
- **macOS dock "Reopen" (clicking dock icon with no visible windows).** Show the folder window. Don't resurrect old single-file windows.
- **"Switch to Source mode"** in overflow — window-lifetime only. Doesn't persist across launches, doesn't affect the global `config.editor_mode`.
- **"Open in folder window"** — opens parent directory as a folder window. If one already exists for that directory, focus it and reveal the file in the sidebar.

## Manual test plan

1. Build and run. Double-click a `.md` file in Finder while the app is not running — opens a single-file window; no folder window appears.
2. Quit. Double-click the same file while the app is running (with a folder window open) — spawns a second window; folder window stays.
3. Double-click the same file a third time — focuses the existing single-file window; no duplicate.
4. `Cmd+O` from any window — opens dialog, creates a new single-file window with chosen file.
5. Drag a `.md` file onto the Dock icon — same result as #1.
6. `rustynotes path/to/file.md` from the terminal — opens single-file window.
7. In a single-file window: edit content → `Cmd+S` saves in place; dirty dot clears. `Cmd+W` with dirty state → Save/Discard/Cancel modal appears.
8. Overflow menu: "Switch to Source mode" — editor switches to CodeMirror, title bar stays minimal. "Open in folder window" — spawns (or focuses) a folder window rooted at parent dir with the file selected.
9. File menu → Open Recent: shows both recent files and recent folders. Clicking a file opens a single-file window; clicking a folder opens the folder window.
10. Welcome screen (no recent folder): shows the "Open File" button and both Recent Files + Recent Folders sections.
11. Delete a file on disk that's in `recent_files`, relaunch: the entry is silently pruned from the list.
12. File path with spaces and unicode (`~/Desktop/my notes — 日本語.md`): opens correctly; filename renders correctly.
13. Theme change applied in folder window's settings: single-file window updates live (via `config_changed` event).
14. Launch app with CLI arg and no recent folder: single-file window appears, Welcome screen does not.
15. Launch app with CLI arg when a recent folder exists in config: single-file window appears, folder window is **not** auto-opened.
16. Plain launch the next day: folder window reopens normally; no stale single-file window from yesterday.
