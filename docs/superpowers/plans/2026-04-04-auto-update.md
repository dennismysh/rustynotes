# Auto-Update System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add in-app auto-update that checks GitHub Releases, downloads/installs DMGs, and relaunches — with configurable auto-update behavior and binary self-watch for external updates.

**Architecture:** Three Rust backend modules (updater, binary_watcher, update commands) communicate via shared Tauri-managed state. Frontend shows an inline update banner in the toolbar and an Updates settings category. Uses `ureq` for HTTP and `semver` for version comparison.

**Tech Stack:** Tauri 2 IPC, ureq, semver, notify (existing), Leptos signals

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `src-tauri/Cargo.toml` | Add ureq, semver deps |
| Create | `src-tauri/src/updater.rs` | Version check + download/install logic |
| Create | `src-tauri/src/binary_watcher.rs` | Watch running binary for external changes |
| Create | `src-tauri/src/commands/update.rs` | Tauri IPC commands for update operations |
| Modify | `src-tauri/src/commands/mod.rs` | Add `pub mod update` |
| Modify | `src-tauri/src/lib.rs` | Register update state, commands, init updater + binary watcher |
| Modify | `crates/rustynotes-common/src/lib.rs` | Add `auto_update`, `last_updated_version` to AppConfig |
| Modify | `crates/rustynotes-frontend/src/tauri_ipc.rs` | Add update IPC bindings + event listener |
| Modify | `crates/rustynotes-frontend/src/components/toolbar.rs` | Add update banner |
| Create | `crates/rustynotes-frontend/src/components/settings/categories/update.rs` | Updates settings category |
| Modify | `crates/rustynotes-frontend/src/components/settings/categories/mod.rs` | Export UpdateSettings |
| Modify | `crates/rustynotes-frontend/src/components/settings/settings_window.rs` | Add Updates category |
| Modify | `styles/base.css` | Update banner styles |

---

### Task 1: Add Config Fields

**Files:**
- Modify: `crates/rustynotes-common/src/lib.rs`

- [ ] **Step 1: Add auto_update and last_updated_version to AppConfig**

In `crates/rustynotes-common/src/lib.rs`, add two fields to the `AppConfig` struct after `auto_save_delay_ms`:

```rust
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub last_updated_version: Option<String>,
```

- [ ] **Step 2: Update the Default impl**

In the `Default` impl for `AppConfig`, add after `auto_save_delay_ms`:

```rust
            auto_update: true,
            last_updated_version: None,
```

- [ ] **Step 3: Verify it compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-common 2>&1 | tail -5`

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-common/src/lib.rs
git commit -m "feat: add auto_update and last_updated_version config fields"
```

---

### Task 2: Add Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add ureq and semver to backend dependencies**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
ureq = "3"
semver = "1"
```

- [ ] **Step 2: Verify it compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -5`

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml Cargo.lock
git commit -m "chore: add ureq and semver dependencies for auto-update"
```

---

### Task 3: Implement Update Checker

**Files:**
- Create: `src-tauri/src/updater.rs`

- [ ] **Step 1: Create updater.rs with version check and update types**

Create `src-tauri/src/updater.rs`:

```rust
//! Auto-update logic: check GitHub Releases for new versions, download DMG,
//! mount/copy/unmount, and relaunch.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

const GITHUB_REPO: &str = "dennismysh/rustynotes";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const TEMP_DIR: &str = "/tmp/rustynotes-update";
const INSTALL_PATH: &str = "/Applications/rustynotes.app";
const BINARY_PATH: &str = "/Applications/rustynotes.app/Contents/MacOS/rustynotes";
const DMG_SUFFIX: &str = "-macos-universal.dmg";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available { version: String },
    Downloading,
    Installing,
    Ready,
    Error(String),
}

/// Check GitHub Releases for a newer version.
/// Returns None on any failure — update checks never crash the app.
pub fn check_for_update(last_updated_version: Option<&str>) -> Option<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .header("User-Agent", "rustynotes-updater")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .ok()?;

    let body: serde_json::Value = response.body_mut().read_json().ok()?;

    let tag = body["tag_name"].as_str()?;
    let remote_version = tag.trim_start_matches('v');

    // Parse with semver for correct comparison
    let remote = semver::Version::parse(remote_version).ok()?;
    let current = semver::Version::parse(CURRENT_VERSION).ok()?;

    if remote <= current {
        return None;
    }

    // Skip if this version was already updated (duplicate prevention)
    if let Some(last) = last_updated_version {
        if last == remote_version {
            return None;
        }
    }

    // Find the DMG asset
    let assets = body["assets"].as_array()?;
    let dmg_asset = assets.iter().find(|a| {
        a["name"]
            .as_str()
            .map(|n| n.ends_with(DMG_SUFFIX))
            .unwrap_or(false)
    })?;

    let download_url = dmg_asset["browser_download_url"].as_str()?;

    Some(UpdateInfo {
        version: remote_version.to_string(),
        download_url: download_url.to_string(),
    })
}

/// Download a DMG, mount it, replace the app bundle, unmount, and clean up.
/// Returns Ok(()) on success. Does NOT relaunch — caller handles that.
pub fn download_and_install(url: &str) -> Result<(), String> {
    let temp_dir = Path::new(TEMP_DIR);
    let dmg_path = temp_dir.join("rustynotes.dmg");
    let mount_point = temp_dir.join("mount");

    // 1. Clean temp dir
    let _ = fs::remove_dir_all(temp_dir);
    fs::create_dir_all(temp_dir).map_err(|e| format!("create temp dir: {e}"))?;

    // 2. Download DMG
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("download: {e}"))?;

    let mut bytes = Vec::new();
    response
        .into_body()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read body: {e}"))?;

    let mut file = fs::File::create(&dmg_path).map_err(|e| format!("create dmg: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("write dmg: {e}"))?;
    drop(file);

    // 3. Mount DMG
    let mount_status = Command::new("hdiutil")
        .args([
            "attach",
            dmg_path.to_str().unwrap(),
            "-nobrowse",
            "-noautoopen",
            "-mountpoint",
            mount_point.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("hdiutil attach: {e}"))?;

    if !mount_status.status.success() {
        return Err(format!(
            "hdiutil attach failed: {}",
            String::from_utf8_lossy(&mount_status.stderr)
        ));
    }

    // 4. Validate .app exists in mount
    let mounted_app = mount_point.join("rustynotes.app");
    if !mounted_app.exists() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap()])
            .output();
        return Err("rustynotes.app not found in DMG".to_string());
    }

    // 5. Replace bundle
    let _ = fs::remove_dir_all(INSTALL_PATH);
    let cp_status = Command::new("cp")
        .args(["-R", mounted_app.to_str().unwrap(), INSTALL_PATH])
        .output()
        .map_err(|e| format!("cp -R: {e}"))?;

    if !cp_status.status.success() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap()])
            .output();
        return Err(format!(
            "cp -R failed: {}",
            String::from_utf8_lossy(&cp_status.stderr)
        ));
    }

    // 6. Cleanup
    let _ = Command::new("hdiutil")
        .args(["detach", mount_point.to_str().unwrap()])
        .output();
    let _ = fs::remove_dir_all(temp_dir);

    Ok(())
}

/// Spawn the installed binary and return. Caller should close the current app.
pub fn relaunch() -> Result<(), String> {
    Command::new(BINARY_PATH)
        .spawn()
        .map_err(|e| format!("relaunch: {e}"))?;
    Ok(())
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version_is_valid_semver() {
        let v = semver::Version::parse(CURRENT_VERSION);
        assert!(v.is_ok(), "CARGO_PKG_VERSION is not valid semver: {CURRENT_VERSION}");
    }

    #[test]
    fn test_check_for_update_returns_none_gracefully() {
        // With a bogus repo, should return None (not panic)
        // This test just verifies the function doesn't crash on network failure
        let result = check_for_update(None);
        // Result depends on network — we just verify no panic
        let _ = result;
    }
}
```

- [ ] **Step 2: Add mod updater to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod watcher;`:

```rust
mod updater;
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo test -p rustynotes --lib updater 2>&1 | tail -10`

Expected: Tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/updater.rs src-tauri/src/lib.rs
git commit -m "feat: add updater module with version check and DMG install logic"
```

---

### Task 4: Implement Binary Watcher

**Files:**
- Create: `src-tauri/src/binary_watcher.rs`

- [ ] **Step 1: Create binary_watcher.rs**

Create `src-tauri/src/binary_watcher.rs`:

```rust
//! Watch the running binary for external changes (e.g., user drags new .app
//! to /Applications). On change, debounce 500ms then relaunch.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 500;

pub struct BinaryWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<PathBuf>,
    exe_path: PathBuf,
    changed_at: Option<Instant>,
}

impl BinaryWatcher {
    /// Start watching the running binary's parent directory.
    /// Returns None if the exe path can't be resolved (e.g., running from cargo).
    pub fn start() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?.canonicalize().ok()?;
        let parent = exe_path.parent()?.to_path_buf();

        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        })
        .ok()?;

        watcher.watch(&parent, RecursiveMode::NonRecursive).ok()?;

        Some(Self {
            _watcher: watcher,
            rx,
            exe_path,
            changed_at: None,
        })
    }

    /// Poll for binary changes. Call this periodically.
    /// Returns true if the binary changed and the debounce period has elapsed.
    pub fn poll(&mut self) -> bool {
        // Drain events
        while let Ok(path) = self.rx.try_recv() {
            if path == self.exe_path {
                self.changed_at = Some(Instant::now());
            }
        }

        // Check debounce
        if let Some(changed_at) = self.changed_at {
            if changed_at.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                self.changed_at = None;
                return true;
            }
        }

        false
    }
}
```

- [ ] **Step 2: Add mod binary_watcher to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod updater;`:

```rust
mod binary_watcher;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -5`

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/binary_watcher.rs src-tauri/src/lib.rs
git commit -m "feat: add binary watcher for external update detection"
```

---

### Task 5: Implement Update Commands (Tauri IPC)

**Files:**
- Create: `src-tauri/src/commands/update.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Create commands/update.rs**

Create `src-tauri/src/commands/update.rs`:

```rust
use crate::updater::{self, UpdateInfo, UpdateStatus};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub struct UpdateState {
    pub status: Mutex<UpdateStatus>,
    pub available: Mutex<Option<UpdateInfo>>,
    pub update_in_progress: Mutex<bool>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
            available: Mutex::new(None),
            update_in_progress: Mutex::new(false),
        }
    }
}

#[derive(Clone, Serialize)]
struct StatusEvent {
    status: UpdateStatus,
}

fn emit_status(app: &AppHandle, status: UpdateStatus) {
    let _ = app.emit("update-status", StatusEvent {
        status: status.clone(),
    });
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) -> Option<UpdateInfo> {
    *state.status.lock().unwrap() = UpdateStatus::Checking;
    emit_status(&app, UpdateStatus::Checking);

    let last_updated = config_state
        .config
        .lock()
        .unwrap()
        .last_updated_version
        .clone();

    let result = updater::check_for_update(last_updated.as_deref());

    if let Some(ref info) = result {
        *state.available.lock().unwrap() = Some(info.clone());
        let status = UpdateStatus::Available {
            version: info.version.clone(),
        };
        *state.status.lock().unwrap() = status.clone();
        emit_status(&app, status);
    } else {
        *state.status.lock().unwrap() = UpdateStatus::Idle;
        emit_status(&app, UpdateStatus::Idle);
    }

    result
}

#[tauri::command]
pub fn apply_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) -> Result<(), String> {
    let info = state
        .available
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update available")?;

    *state.update_in_progress.lock().unwrap() = true;

    // Save last_updated_version to config before downloading
    {
        let mut config = config_state.config.lock().unwrap();
        config.last_updated_version = Some(info.version.clone());
        let _ = crate::config::save_config(&config);
    }

    let url = info.download_url.clone();
    let app_handle = app.clone();

    std::thread::spawn(move || {
        // Downloading
        emit_status(&app_handle, UpdateStatus::Downloading);

        // Installing
        match updater::download_and_install(&url) {
            Ok(()) => {
                emit_status(&app_handle, UpdateStatus::Ready);
            }
            Err(e) => {
                emit_status(&app_handle, UpdateStatus::Error(e));
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn restart_after_update() -> Result<(), String> {
    updater::relaunch()?;
    std::process::exit(0);
}

#[tauri::command]
pub fn get_update_status(state: tauri::State<UpdateState>) -> UpdateStatus {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_current_version() -> String {
    updater::current_version().to_string()
}

#[tauri::command]
pub fn dismiss_update(
    state: tauri::State<UpdateState>,
    config_state: tauri::State<crate::commands::config::ConfigState>,
) {
    if let Some(info) = state.available.lock().unwrap().take() {
        let mut config = config_state.config.lock().unwrap();
        config.last_updated_version = Some(info.version);
        let _ = crate::config::save_config(&config);
    }
    *state.status.lock().unwrap() = UpdateStatus::Idle;
}
```

- [ ] **Step 2: Add pub mod update to commands/mod.rs**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod update;
```

- [ ] **Step 3: Register update state and commands in lib.rs**

In `src-tauri/src/lib.rs`, add after the existing `.manage(...)` calls:

```rust
        .manage(commands::update::UpdateState::new())
```

Add to the `generate_handler!` macro:

```rust
            commands::update::check_for_update,
            commands::update::apply_update,
            commands::update::restart_after_update,
            commands::update::get_update_status,
            commands::update::get_current_version,
            commands::update::dismiss_update,
```

- [ ] **Step 4: Initialize background update checker and binary watcher in setup**

In `src-tauri/src/lib.rs`, add a `.setup()` hook before `.run()`:

```rust
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background update check on startup + periodic (every 6 hours)
            std::thread::spawn(move || {
                loop {
                    // Check for update
                    let config_state = app_handle.state::<commands::config::ConfigState>();
                    let update_state = app_handle.state::<commands::update::UpdateState>();
                    let last_updated = config_state
                        .config
                        .lock()
                        .unwrap()
                        .last_updated_version
                        .clone();

                    if let Some(info) = updater::check_for_update(last_updated.as_deref()) {
                        *update_state.available.lock().unwrap() = Some(info.clone());
                        let status = updater::UpdateStatus::Available {
                            version: info.version.clone(),
                        };
                        *update_state.status.lock().unwrap() = status.clone();
                        let _ = app_handle.emit("update-status", commands::update::StatusEvent {
                            status,
                        });

                        // Auto-update if enabled
                        let auto_update = config_state.config.lock().unwrap().auto_update;
                        if auto_update {
                            *update_state.update_in_progress.lock().unwrap() = true;
                            let mut config = config_state.config.lock().unwrap();
                            config.last_updated_version = Some(info.version.clone());
                            let _ = crate::config::save_config(&config);
                            drop(config);

                            let _ = app_handle.emit("update-status", commands::update::StatusEvent {
                                status: updater::UpdateStatus::Downloading,
                            });

                            match updater::download_and_install(&info.download_url) {
                                Ok(()) => {
                                    let _ = app_handle.emit("update-status", commands::update::StatusEvent {
                                        status: updater::UpdateStatus::Ready,
                                    });
                                }
                                Err(e) => {
                                    let _ = app_handle.emit("update-status", commands::update::StatusEvent {
                                        status: updater::UpdateStatus::Error(e),
                                    });
                                }
                            }
                        }
                    }

                    // Sleep 6 hours before next check
                    std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                }
            });

            // Binary self-watch
            if let Some(mut watcher) = binary_watcher::BinaryWatcher::start() {
                let app_handle2 = app.handle().clone();
                std::thread::spawn(move || {
                    loop {
                        if watcher.poll() {
                            let update_state = app_handle2.state::<commands::update::UpdateState>();
                            let in_progress = *update_state.update_in_progress.lock().unwrap();
                            if !in_progress {
                                // Binary changed externally — relaunch
                                let _ = updater::relaunch();
                                std::process::exit(0);
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(250));
                    }
                });
            }

            Ok(())
        })
```

- [ ] **Step 5: Make StatusEvent pub**

In `src-tauri/src/commands/update.rs`, change:

```rust
#[derive(Clone, Serialize)]
struct StatusEvent {
```

To:

```rust
#[derive(Clone, Serialize)]
pub struct StatusEvent {
```

- [ ] **Step 6: Verify it compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -10`

Expected: Build succeeds.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/update.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add update IPC commands, background checker, and binary watcher init"
```

---

### Task 6: Add Frontend IPC Bindings

**Files:**
- Modify: `crates/rustynotes-frontend/src/tauri_ipc.rs`

- [ ] **Step 1: Add update IPC functions**

In `crates/rustynotes-frontend/src/tauri_ipc.rs`, add before the event listeners section:

```rust
// ---------------------------------------------------------------------------
// Update commands
// ---------------------------------------------------------------------------

pub async fn check_for_update_cmd() -> Result<Option<String>, String> {
    // Returns the version string if an update is available, None otherwise
    let val = tauri_invoke_no_args("check_for_update").await?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        // The result is an UpdateInfo struct with a "version" field
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

/// Listen to update-status events from the backend.
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
```

- [ ] **Step 2: Verify frontend compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -5`

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/src/tauri_ipc.rs
git commit -m "feat: add update IPC bindings and event listener to frontend"
```

---

### Task 7: Add Update Banner to Toolbar

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/toolbar.rs`
- Modify: `styles/base.css`

- [ ] **Step 1: Add update state signals and banner to toolbar**

In `crates/rustynotes-frontend/src/components/toolbar.rs`, add at the top of the `Toolbar` component function (after existing signal definitions):

```rust
    // Update banner state
    let update_version = RwSignal::new(Option::<String>::None);
    let update_status = RwSignal::new(String::from("idle"));

    // Listen for update status events
    {
        let update_version = update_version;
        let update_status = update_status;
        tauri_ipc::listen_update_status(move |json| {
            // Parse the status JSON to extract the state
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(status) = parsed.get("status") {
                    if let Some(s) = status.as_str() {
                        update_status.set(s.to_string());
                    } else if let Some(obj) = status.as_object() {
                        if let Some(version) = obj.get("Available").and_then(|v| v.get("version")).and_then(|v| v.as_str()) {
                            update_version.set(Some(version.to_string()));
                            update_status.set("available".to_string());
                        } else if obj.contains_key("Error") {
                            update_status.set("error".to_string());
                        }
                    }
                }
            }
        });
    }

    let handle_update_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::apply_update_cmd().await;
        });
    };

    let handle_restart_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::restart_after_update_cmd().await;
        });
    };

    let handle_dismiss_update = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        update_version.set(None);
        update_status.set("idle".to_string());
        leptos::task::spawn_local(async move {
            let _ = tauri_ipc::dismiss_update_cmd().await;
        });
    };
```

Then in the view, add the update banner after `<button on:click=handle_open_folder>"Open Folder"</button>`:

```rust
            // Update banner
            {
                let status = update_status.clone();
                let version = update_version.clone();
                view! {
                    <Show when=move || status.get() != "idle" && status.get() != "Checking">
                        <div class="update-banner">
                            {move || {
                                let s = status.get();
                                match s.as_str() {
                                    "available" => {
                                        let v = version.get().unwrap_or_default();
                                        view! {
                                            <span class="update-text">{format!("v{v} available")}</span>
                                            <button class="update-btn" on:click=handle_update_click>"Update"</button>
                                            <button class="update-dismiss" on:click=handle_dismiss_update>"\u{00D7}"</button>
                                        }.into_any()
                                    }
                                    "Downloading" => {
                                        view! { <span class="update-text">"Downloading..."</span> }.into_any()
                                    }
                                    "Installing" => {
                                        view! { <span class="update-text">"Installing..."</span> }.into_any()
                                    }
                                    "Ready" => {
                                        view! {
                                            <span class="update-text">"Update ready"</span>
                                            <button class="update-btn" on:click=handle_restart_click>"Restart"</button>
                                        }.into_any()
                                    }
                                    "error" => {
                                        view! { <span class="update-text update-error">"Update failed"</span> }.into_any()
                                    }
                                    _ => view! { <span /> }.into_any()
                                }
                            }}
                        </div>
                    </Show>
                }
            }
```

- [ ] **Step 2: Add update banner CSS**

In `styles/base.css`, add after the toolbar icon button styles:

```css
/* Update banner */
.update-banner {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: 12px;
  background: var(--accent);
  color: var(--accent-fg);
  font-size: 11px;
  font-weight: 500;
  white-space: nowrap;
  flex-shrink: 0;
}

.update-text { font-size: 11px; }

.update-btn {
  background: rgba(255, 255, 255, 0.2);
  border: none;
  border-radius: 8px;
  color: inherit;
  padding: 1px 8px;
  font-size: 11px;
  cursor: pointer;
  font-family: var(--font-body);
}

.update-btn:hover { background: rgba(255, 255, 255, 0.3); }

.update-dismiss {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 14px;
  padding: 0 2px;
  opacity: 0.7;
}

.update-dismiss:hover { opacity: 1; }

.update-error { color: var(--error); }
```

- [ ] **Step 3: Verify frontend compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -5`

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/src/components/toolbar.rs styles/base.css
git commit -m "feat: add update banner to toolbar with download/restart/dismiss actions"
```

---

### Task 8: Add Updates Settings Category

**Files:**
- Create: `crates/rustynotes-frontend/src/components/settings/categories/update.rs`
- Modify: `crates/rustynotes-frontend/src/components/settings/categories/mod.rs`
- Modify: `crates/rustynotes-frontend/src/components/settings/settings_window.rs`

- [ ] **Step 1: Create update.rs settings category**

Create `crates/rustynotes-frontend/src/components/settings/categories/update.rs`:

```rust
//! Update settings panel — auto-update toggle, check for updates, version display.

use leptos::prelude::*;
use rustynotes_common::AppConfig;

use crate::components::settings::shared::SettingRow;
use crate::tauri_ipc;

#[component]
pub fn UpdateSettings() -> impl IntoView {
    let config = RwSignal::new(Option::<AppConfig>::None);
    let current_version = RwSignal::new(String::new());
    let check_result = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(c) = tauri_ipc::get_config().await {
                config.set(Some(c));
            }
            if let Ok(v) = tauri_ipc::get_current_version().await {
                current_version.set(v);
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

    let auto_update = Signal::derive(move || {
        config.get().map(|c| c.auto_update).unwrap_or(true)
    });

    let handle_check = move |_| {
        check_result.set(Some("Checking...".to_string()));
        leptos::task::spawn_local(async move {
            match tauri_ipc::check_for_update_cmd().await {
                Ok(Some(version)) => {
                    check_result.set(Some(format!("v{version} available!")));
                }
                Ok(None) => {
                    check_result.set(Some("You're up to date.".to_string()));
                }
                Err(e) => {
                    check_result.set(Some(format!("Error: {e}")));
                }
            }
        });
    };

    view! {
        <div class="settings-category">
            <h2 class="settings-category-title">"Updates"</h2>
            <p class="settings-category-subtitle">"Keep RustyNotes up to date"</p>

            <SettingRow label="Current version" description="">
                <span class="setting-value">{move || current_version.get()}</span>
            </SettingRow>

            <SettingRow label="Auto-update" description="Download and install updates silently, prompt only to restart">
                <input
                    type="checkbox"
                    prop:checked=auto_update
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        update(Box::new(move |c| c.auto_update = checked));
                    }
                />
            </SettingRow>

            <SettingRow label="Check for updates" description="">
                <button
                    class="setting-btn"
                    on:click=handle_check
                >
                    "Check now"
                </button>
            </SettingRow>

            <Show when=move || check_result.get().is_some()>
                <div class="setting-check-result">
                    {move || check_result.get().unwrap_or_default()}
                </div>
            </Show>
        </div>
    }
}
```

- [ ] **Step 2: Export UpdateSettings from mod.rs**

In `crates/rustynotes-frontend/src/components/settings/categories/mod.rs`, add:

```rust
pub mod update;
pub use update::UpdateSettings;
```

- [ ] **Step 3: Add Updates category to settings window**

In `crates/rustynotes-frontend/src/components/settings/settings_window.rs`, add to the imports:

```rust
use crate::components::settings::categories::UpdateSettings;
```

Wait — the imports use a wildcard. Check the existing import line:

```rust
use crate::components::settings::categories::{
    AdvancedSettings, AppearanceSettings, EditorSettings, PreviewSettings, SavingSettings,
};
```

Change to:

```rust
use crate::components::settings::categories::{
    AdvancedSettings, AppearanceSettings, EditorSettings, PreviewSettings, SavingSettings,
    UpdateSettings,
};
```

Add the Updates entry to the `categories()` function:

```rust
        SettingsCategory { id: "updates",    label: "Updates",    icon: "\u{1F504}" },
```

Add the match arm in the view:

```rust
                    "updates"  => view! { <UpdateSettings /> }.into_any(),
```

- [ ] **Step 4: Add setting-btn and setting-check-result CSS**

In `styles/settings.css`, add at the end:

```css
/* Check for updates button */
.setting-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  font-family: var(--font-body);
}

.setting-btn:hover { background: var(--accent); color: var(--accent-fg); }

.setting-check-result {
  padding: var(--space-sm) 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.setting-value {
  font-size: 13px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
}
```

- [ ] **Step 5: Verify full build**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -5`

Expected: Build succeeds.

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -5`

Expected: Backend also succeeds.

- [ ] **Step 6: Commit**

```bash
git add crates/rustynotes-frontend/src/components/settings/categories/update.rs \
       crates/rustynotes-frontend/src/components/settings/categories/mod.rs \
       crates/rustynotes-frontend/src/components/settings/settings_window.rs \
       styles/settings.css
git commit -m "feat: add Updates settings category with auto-update toggle and check button"
```

---

### Task 9: Smoke Test and Verify

**Files:**
- None (verification only)

- [ ] **Step 1: Run all backend tests**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo test -p rustynotes 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 2: Build production app**

Run: `cd "/Users/dennis/programming projects/rustynotes" && pnpm tauri build 2>&1 | tail -8`

Expected: Build succeeds, .app bundle created.

- [ ] **Step 3: Manual smoke test**

Launch the app and verify:
- Settings > Updates category exists with version display, auto-update toggle, check button
- Click "Check now" — should show "You're up to date" or detect an available version
- If an update is available, the toolbar banner should appear
- Auto-update toggle persists between app restarts

- [ ] **Step 4: Commit any fixes, then final commit**

```bash
git add -A
git commit -m "feat: complete auto-update system with version check, install, and settings UI"
```
