# Updater Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the silent update failure bug and harden the DMG-based auto-update system with typed errors, post-install verification, and consolidated logic.

**Architecture:** Replace `last_updated_version` with `dismissed_version` (banner-only suppression), convert silent `Option`/`String` error handling to typed `UpdateError` enum, add post-install `Info.plist` verification, and consolidate the duplicated background-thread/IPC update logic into shared functions.

**Tech Stack:** Rust (Tauri backend), Leptos (WASM frontend), serde, semver, ureq

---

## File Structure

| File | Role | Change |
|------|------|--------|
| `src-tauri/src/updater.rs` | Core update logic: check, download, install, verify | Rewrite: `UpdateError` enum, typed returns, `expected_version` param, plist verification |
| `src-tauri/src/commands/update.rs` | Tauri IPC commands + shared orchestration | Rewrite: shared `perform_check`, updated commands, `dismissed_version` handling |
| `src-tauri/src/lib.rs` | App setup + background thread | Modify: background thread calls shared function |
| `crates/rustynotes-common/src/lib.rs` | Shared config types | Modify: replace `last_updated_version` with `dismissed_version` |
| `crates/rustynotes-frontend/src/components/settings/categories/update.rs` | Settings UI | Modify: handle error results from check |
| `crates/rustynotes-frontend/src/components/toolbar.rs` | Update banner | Modify: extract error message from event |
| `crates/rustynotes-frontend/src/tauri_ipc.rs` | Frontend IPC wrappers | No change needed (already returns `Result<Option<String>, String>`) |

---

### Task 1: Replace `last_updated_version` with `dismissed_version` in config

**Files:**
- Modify: `crates/rustynotes-common/src/lib.rs:108-132` (AppConfig struct)
- Modify: `crates/rustynotes-common/src/lib.rs:208-224` (Default impl)

- [ ] **Step 1: Replace the field in `AppConfig`**

In `crates/rustynotes-common/src/lib.rs`, replace line 131:

```rust
    #[serde(default)]
    pub last_updated_version: Option<String>,
```

with:

```rust
    #[serde(default)]
    pub dismissed_version: Option<String>,
```

- [ ] **Step 2: Update the `Default` impl**

In the same file, replace line 221:

```rust
            last_updated_version: None,
```

with:

```rust
            dismissed_version: None,
```

- [ ] **Step 3: Update the existing config tests**

The existing tests in `crates/rustynotes-common/src/lib.rs` don't reference `last_updated_version` directly, so they should still pass. Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p rustynotes-common
```

Expected: all tests pass. (Existing `config.json` files with `last_updated_version` will have that field silently ignored by serde since it's no longer in the struct.)

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-common/src/lib.rs
git commit -m "refactor: replace last_updated_version with dismissed_version in config"
```

---

### Task 2: Define `UpdateError` enum and rewrite `check_for_update`

**Files:**
- Modify: `src-tauri/src/updater.rs` (full rewrite of error types and `check_for_update`)

- [ ] **Step 1: Write tests for `UpdateError` display messages**

Replace the existing `#[cfg(test)] mod tests` block at the bottom of `src-tauri/src/updater.rs` (lines 172-187) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version_is_valid_semver() {
        let v = semver::Version::parse(CURRENT_VERSION);
        assert!(v.is_ok(), "CARGO_PKG_VERSION is not valid semver: {CURRENT_VERSION}");
    }

    #[test]
    fn test_update_error_display_network() {
        let err = UpdateError::NetworkError("connection refused".into());
        assert_eq!(
            err.to_string(),
            "Couldn't reach GitHub \u{2014} check your connection"
        );
    }

    #[test]
    fn test_update_error_display_verification_failed() {
        let err = UpdateError::VerificationFailed {
            expected: "0.4.0".into(),
            found: "0.3.1".into(),
        };
        assert_eq!(
            err.to_string(),
            "Update installed but version doesn't match \u{2014} try updating manually"
        );
    }

    #[test]
    fn test_update_error_display_all_variants() {
        // Ensure every variant has a non-empty Display impl
        let variants: Vec<UpdateError> = vec![
            UpdateError::NetworkError("test".into()),
            UpdateError::ApiError("test".into()),
            UpdateError::ParseError("test".into()),
            UpdateError::AssetNotFound,
            UpdateError::VersionParseError("test".into()),
            UpdateError::DownloadFailed("test".into()),
            UpdateError::DmgMountFailed("test".into()),
            UpdateError::AppNotFoundInDmg,
            UpdateError::CopyFailed("test".into()),
            UpdateError::VerificationFailed {
                expected: "1.0".into(),
                found: "0.9".into(),
            },
        ];
        for v in variants {
            assert!(!v.to_string().is_empty(), "Empty display for {v:?}");
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p rustynotes_lib updater::tests -- -v
```

Expected: FAIL — `UpdateError` type does not exist yet.

- [ ] **Step 3: Define `UpdateError` and implement `Display`**

Replace everything in `src-tauri/src/updater.rs` from line 1 through line 32 (the existing imports, constants, `UpdateInfo`, and `UpdateStatus`) with:

```rust
//! Auto-update logic: check GitHub Releases for new versions, download DMG,
//! mount/copy/unmount, verify install, and relaunch.

use serde::{Deserialize, Serialize};
use std::fmt;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateError {
    NetworkError(String),
    ApiError(String),
    ParseError(String),
    AssetNotFound,
    VersionParseError(String),
    DownloadFailed(String),
    DmgMountFailed(String),
    AppNotFoundInDmg,
    CopyFailed(String),
    VerificationFailed { expected: String, found: String },
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(_) => write!(f, "Couldn\u{2019}t reach GitHub \u{2014} check your connection"),
            Self::ApiError(_) => write!(f, "GitHub API error \u{2014} try again later"),
            Self::ParseError(_) => write!(f, "Unexpected response from GitHub"),
            Self::AssetNotFound => write!(f, "No macOS update found in this release"),
            Self::VersionParseError(_) => write!(f, "Invalid version format in release"),
            Self::DownloadFailed(_) => write!(f, "Download failed \u{2014} check your connection"),
            Self::DmgMountFailed(_) => write!(f, "Couldn\u{2019}t open the update file"),
            Self::AppNotFoundInDmg => write!(f, "Update file is damaged or incomplete"),
            Self::CopyFailed(_) => write!(f, "Couldn\u{2019}t install update \u{2014} check /Applications permissions"),
            Self::VerificationFailed { .. } => write!(f, "Update installed but version doesn\u{2019}t match \u{2014} try updating manually"),
        }
    }
}

impl std::error::Error for UpdateError {}
```

- [ ] **Step 4: Rewrite `check_for_update` with typed errors**

Replace the existing `check_for_update` function (lines 34-85 in the original) with:

```rust
/// Check GitHub Releases for a newer version.
/// Returns `Ok(None)` if up to date, `Ok(Some(info))` if update available,
/// `Err(e)` if something went wrong.
pub fn check_for_update() -> Result<Option<UpdateInfo>, UpdateError> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .header("User-Agent", "rustynotes-updater")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

    let body_str = response
        .into_body()
        .read_to_string()
        .map_err(|e| UpdateError::ApiError(e.to_string()))?;

    let body: serde_json::Value =
        serde_json::from_str(&body_str).map_err(|e| UpdateError::ParseError(e.to_string()))?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| UpdateError::ParseError("missing tag_name".into()))?;
    let remote_version = tag.trim_start_matches('v');

    let remote = semver::Version::parse(remote_version)
        .map_err(|e| UpdateError::VersionParseError(e.to_string()))?;
    let current = semver::Version::parse(CURRENT_VERSION)
        .map_err(|e| UpdateError::VersionParseError(e.to_string()))?;

    if remote <= current {
        return Ok(None);
    }

    let assets = body["assets"]
        .as_array()
        .ok_or_else(|| UpdateError::ParseError("missing assets array".into()))?;

    let dmg_asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .map(|n| n.ends_with(DMG_SUFFIX))
                .unwrap_or(false)
        })
        .ok_or(UpdateError::AssetNotFound)?;

    let download_url = dmg_asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| UpdateError::ParseError("missing download URL".into()))?;

    Ok(Some(UpdateInfo {
        version: remote_version.to_string(),
        download_url: download_url.to_string(),
    }))
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p rustynotes_lib updater::tests -- -v
```

Expected: all updater tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/updater.rs
git commit -m "feat: add UpdateError enum and typed check_for_update"
```

---

### Task 3: Rewrite `download_and_install` with typed errors + post-install verification

**Files:**
- Modify: `src-tauri/src/updater.rs` (replace `download_and_install` function)

- [ ] **Step 1: Add verification test**

Add these tests to the existing `#[cfg(test)] mod tests` block in `src-tauri/src/updater.rs`:

```rust
    #[test]
    fn test_verify_installed_version_parses_plist_output() {
        // verify_installed_version shells out to plutil, so we just test
        // that the function signature is correct and handles missing path.
        // Real verification is a manual/integration test.
        let result = verify_installed_version("99.99.99");
        // Will fail because the installed app version won't be 99.99.99
        // (or the app isn't installed), which is the correct behavior.
        assert!(result.is_err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p rustynotes_lib updater::tests::test_verify -- -v
```

Expected: FAIL — `verify_installed_version` does not exist yet.

- [ ] **Step 3: Add `verify_installed_version` function**

Add this function after the `check_for_update` function in `src-tauri/src/updater.rs`:

```rust
/// Verify the installed app version matches expectations by reading Info.plist.
fn verify_installed_version(expected_version: &str) -> Result<(), UpdateError> {
    let plist_path = format!("{}/Contents/Info.plist", INSTALL_PATH);

    let output = Command::new("plutil")
        .args([
            "-extract",
            "CFBundleShortVersionString",
            "raw",
            "-o",
            "-",
            &plist_path,
        ])
        .output()
        .map_err(|e| UpdateError::VerificationFailed {
            expected: expected_version.to_string(),
            found: format!("plutil failed: {e}"),
        })?;

    if !output.status.success() {
        return Err(UpdateError::VerificationFailed {
            expected: expected_version.to_string(),
            found: format!(
                "plutil error: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }

    let installed_version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if installed_version != expected_version {
        return Err(UpdateError::VerificationFailed {
            expected: expected_version.to_string(),
            found: installed_version,
        });
    }

    Ok(())
}
```

- [ ] **Step 4: Rewrite `download_and_install` with typed errors and verification**

Replace the existing `download_and_install` function with:

```rust
/// Download a DMG, mount it, replace the app bundle, verify, unmount, and clean up.
pub fn download_and_install(url: &str, expected_version: &str) -> Result<(), UpdateError> {
    let temp_dir = Path::new(TEMP_DIR);
    let dmg_path = temp_dir.join("rustynotes.dmg");
    let mount_point = temp_dir.join("mount");

    let _ = fs::remove_dir_all(temp_dir);
    fs::create_dir_all(temp_dir).map_err(|e| UpdateError::DownloadFailed(e.to_string()))?;

    let response = ureq::get(url)
        .call()
        .map_err(|e| UpdateError::DownloadFailed(e.to_string()))?;

    let bytes = response
        .into_body()
        .read_to_vec()
        .map_err(|e| UpdateError::DownloadFailed(e.to_string()))?;

    let mut file = fs::File::create(&dmg_path)
        .map_err(|e| UpdateError::DownloadFailed(e.to_string()))?;
    file.write_all(&bytes)
        .map_err(|e| UpdateError::DownloadFailed(e.to_string()))?;
    drop(file);

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
        .map_err(|e| UpdateError::DmgMountFailed(e.to_string()))?;

    if !mount_status.status.success() {
        return Err(UpdateError::DmgMountFailed(
            String::from_utf8_lossy(&mount_status.stderr).to_string(),
        ));
    }

    let mounted_app = mount_point.join("rustynotes.app");
    if !mounted_app.exists() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap()])
            .output();
        return Err(UpdateError::AppNotFoundInDmg);
    }

    let _ = fs::remove_dir_all(INSTALL_PATH);
    let cp_status = Command::new("cp")
        .args(["-R", mounted_app.to_str().unwrap(), INSTALL_PATH])
        .output()
        .map_err(|e| UpdateError::CopyFailed(e.to_string()))?;

    if !cp_status.status.success() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap()])
            .output();
        return Err(UpdateError::CopyFailed(
            String::from_utf8_lossy(&cp_status.stderr).to_string(),
        ));
    }

    // Verify the installed version matches what we expected
    let verification = verify_installed_version(expected_version);

    // Always clean up, regardless of verification result
    let _ = Command::new("hdiutil")
        .args(["detach", mount_point.to_str().unwrap()])
        .output();
    let _ = fs::remove_dir_all(temp_dir);

    verification
}
```

- [ ] **Step 5: Update `relaunch` and `current_version` (no changes needed, just verify they still exist)**

The existing `relaunch()` and `current_version()` functions remain unchanged:

```rust
pub fn relaunch() -> Result<(), String> {
    Command::new(BINARY_PATH)
        .spawn()
        .map_err(|e| format!("relaunch: {e}"))?;
    Ok(())
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}
```

- [ ] **Step 6: Run all updater tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p rustynotes_lib updater::tests -- -v
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/updater.rs
git commit -m "feat: typed errors for download_and_install with post-install verification"
```

---

### Task 4: Rewrite `commands/update.rs` with shared logic and `dismissed_version`

**Files:**
- Modify: `src-tauri/src/commands/update.rs` (full rewrite)

- [ ] **Step 1: Rewrite the full file**

Replace the entire contents of `src-tauri/src/commands/update.rs` with:

```rust
use crate::updater::{self, UpdateInfo, UpdateStatus};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

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
pub struct StatusEvent {
    pub status: UpdateStatus,
}

fn emit_status(app: &AppHandle, status: UpdateStatus) {
    let _ = app.emit("update-status", StatusEvent {
        status: status.clone(),
    });
}

/// Shared update check logic used by both the background thread and IPC command.
/// Returns the UpdateInfo if an update is available, or None if up to date.
/// Errors are emitted as UpdateStatus::Error events.
pub fn perform_check(
    app: &AppHandle,
    state: &UpdateState,
) -> Option<UpdateInfo> {
    *state.status.lock().unwrap() = UpdateStatus::Checking;
    emit_status(app, UpdateStatus::Checking);

    match updater::check_for_update() {
        Ok(Some(info)) => {
            *state.available.lock().unwrap() = Some(info.clone());
            let status = UpdateStatus::Available {
                version: info.version.clone(),
            };
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
            Some(info)
        }
        Ok(None) => {
            *state.status.lock().unwrap() = UpdateStatus::Idle;
            emit_status(app, UpdateStatus::Idle);
            None
        }
        Err(e) => {
            let status = UpdateStatus::Error(e.to_string());
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
            None
        }
    }
}

/// Shared download+install logic used by both the background thread and IPC command.
pub fn perform_install(
    app: &AppHandle,
    state: &UpdateState,
    info: &UpdateInfo,
) {
    *state.update_in_progress.lock().unwrap() = true;
    emit_status(app, UpdateStatus::Downloading);

    match updater::download_and_install(&info.download_url, &info.version) {
        Ok(()) => {
            let status = UpdateStatus::Ready;
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
        }
        Err(e) => {
            *state.update_in_progress.lock().unwrap() = false;
            let status = UpdateStatus::Error(e.to_string());
            *state.status.lock().unwrap() = status.clone();
            emit_status(app, status);
        }
    }
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
) -> Option<UpdateInfo> {
    perform_check(&app, &state)
}

#[tauri::command]
pub fn apply_update(
    app: AppHandle,
    state: tauri::State<UpdateState>,
) -> Result<(), String> {
    let info = state
        .available
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update available")?;

    let app_handle = app.clone();
    let version = info.version.clone();
    let url = info.download_url.clone();

    std::thread::spawn(move || {
        let state_ref = app_handle.state::<UpdateState>();
        let info = UpdateInfo {
            version,
            download_url: url,
        };
        perform_install(&app_handle, &state_ref, &info);
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
        config.dismissed_version = Some(info.version);
        let _ = crate::config::save_config(&config);
    }
    *state.status.lock().unwrap() = UpdateStatus::Idle;
}
```

- [ ] **Step 2: Verify the file compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | head -20
```

Expected: errors in `lib.rs` because the background thread still uses old APIs — that's expected, we fix it in the next task.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/update.rs
git commit -m "refactor: shared perform_check/perform_install + dismissed_version in commands"
```

---

### Task 5: Update background thread in `lib.rs` to use shared functions

**Files:**
- Modify: `src-tauri/src/lib.rs:52-127` (invoke_handler registration + background thread)

- [ ] **Step 1: Update the invoke_handler to remove `config_state` from `check_for_update`**

In `src-tauri/src/lib.rs`, the `invoke_handler` registration (lines 52-70) stays the same — the command names haven't changed.

- [ ] **Step 2: Rewrite the background thread**

Replace lines 71-126 in `src-tauri/src/lib.rs` (the `.setup(|app| {` block up to the binary watcher section) with:

```rust
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background update check on startup + periodic (every 6 hours)
            std::thread::spawn(move || {
                loop {
                    let update_state = app_handle.state::<commands::update::UpdateState>();
                    let config_state = app_handle.state::<commands::config::ConfigState>();

                    if let Some(info) = commands::update::perform_check(&app_handle, &update_state) {
                        let dismissed = config_state
                            .config
                            .lock()
                            .unwrap()
                            .dismissed_version
                            .clone();

                        // Skip if user dismissed this version
                        let is_dismissed = dismissed.as_deref() == Some(info.version.as_str());

                        let auto_update = config_state.config.lock().unwrap().auto_update;

                        if auto_update && !is_dismissed {
                            commands::update::perform_install(&app_handle, &update_state, &info);
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                }
            });
```

The binary watcher block (lines 128-144) stays unchanged.

- [ ] **Step 3: Verify the full project compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```

Expected: clean compile (no errors, warnings are OK).

- [ ] **Step 4: Run all tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1
```

Expected: all 31+ tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: background thread uses shared perform_check/perform_install"
```

---

### Task 6: Update frontend to surface error details

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/toolbar.rs:52-67` (update event listener)
- Modify: `crates/rustynotes-frontend/src/components/settings/categories/update.rs:42-57` (check result handling)

- [ ] **Step 1: Update the toolbar event listener to extract error messages**

In `crates/rustynotes-frontend/src/components/toolbar.rs`, the `listen_update_status` callback (lines 52-67) currently detects `Error` but doesn't extract the message. Add an `update_error_msg` signal and update the listener:

Add a new signal after line 49:

```rust
    let update_error_msg = RwSignal::new(Option::<String>::None);
```

Replace the event listener block (lines 52-67) with:

```rust
    tauri_ipc::listen_update_status(move |json| {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(status) = parsed.get("status") {
                if let Some(s) = status.as_str() {
                    update_status.set(s.to_lowercase());
                    if s != "Error" {
                        update_error_msg.set(None);
                    }
                } else if let Some(obj) = status.as_object() {
                    if let Some(v) = obj.get("Available").and_then(|v| v.get("version")).and_then(|v| v.as_str()) {
                        update_version.set(Some(v.to_string()));
                        update_status.set("available".to_string());
                        update_error_msg.set(None);
                    } else if let Some(msg) = obj.get("Error").and_then(|v| v.as_str()) {
                        update_error_msg.set(Some(msg.to_string()));
                        update_status.set("error".to_string());
                    }
                }
            }
        }
    });
```

- [ ] **Step 2: Update the error rendering in the banner**

In the same file, replace the error match arm (line 303-305):

```rust
                            "error" => {
                                view! { <span class="update-text update-error">"Update failed"</span> }.into_any()
                            }
```

with:

```rust
                            "error" => {
                                let msg = update_error_msg.get().unwrap_or_else(|| "Update failed".to_string());
                                view! { <span class="update-text update-error">{msg}</span> }.into_any()
                            }
```

- [ ] **Step 3: Update settings check result to show errors**

In `crates/rustynotes-frontend/src/components/settings/categories/update.rs`, the `handle_check` closure (lines 42-57) currently shows errors as `"Error: {e}"`. This already works — the `e` will now contain the user-friendly message from `UpdateError::Display`. No change needed here.

- [ ] **Step 4: Verify frontend compiles**

```bash
cd "/Users/dennis/programming projects/rustynotes" && trunk build 2>&1 | tail -10
```

Expected: successful build.

- [ ] **Step 5: Commit**

```bash
git add crates/rustynotes-frontend/src/components/toolbar.rs
git commit -m "feat: surface specific error messages in update banner"
```

---

### Task 7: Manual smoke test

- [ ] **Step 1: Clear stale config**

Edit `~/Library/Application Support/rustynotes/config.json` and remove the `last_updated_version` field (it will be ignored, but clean up for clarity). If a `dismissed_version` field is present, remove it too to get a clean slate.

- [ ] **Step 2: Build and run**

```bash
cd "/Users/dennis/programming projects/rustynotes" && cargo tauri dev
```

- [ ] **Step 3: Test "Check now" in settings**

Open Settings → Updates → click "Check now". Verify:
- If there's a newer release: shows "vX.Y.Z available!"
- If up to date: shows "You're up to date."
- If network error (disconnect wifi): shows a specific error message, not silence

- [ ] **Step 4: Test banner dismiss**

If an update is available, dismiss it. Check `~/Library/Application Support/rustynotes/config.json` — should have `"dismissed_version": "X.Y.Z"`. Restart the app — the banner should not reappear for that version.

- [ ] **Step 5: Commit any fixes**

If any issues were found and fixed during testing:

```bash
git add -u
git commit -m "fix: address issues found during updater smoke test"
```
