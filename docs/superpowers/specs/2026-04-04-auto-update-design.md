# RustyNotes Auto-Update System

**Date:** 2026-04-04
**Scope:** In-app update detection, download, install, relaunch, and binary self-watch

## Goal

Detect new releases from GitHub, download and install the updated DMG, and relaunch — with configurable auto-update behavior and external update detection.

## Architecture

Three backend modules + frontend UI:

| Module | File | Responsibility |
|--------|------|----------------|
| Updater | `src-tauri/src/updater.rs` | Version check, DMG download, mount/copy/unmount, relaunch |
| Binary Watcher | `src-tauri/src/binary_watcher.rs` | Detect external binary changes, debounced relaunch |
| Update Commands | `src-tauri/src/commands/update.rs` | Tauri IPC: `check_for_update`, `apply_update`, `get_update_status` |
| Frontend Banner | `crates/rustynotes-frontend/src/components/toolbar.rs` | Inline update banner in toolbar |
| Settings | `crates/rustynotes-frontend/src/components/settings/categories/update.rs` | Updates settings category |

### Shared State

`UpdateState` managed by Tauri (`Mutex<UpdateInfo>`):

```rust
pub struct UpdateInfo {
    pub available_version: Option<String>,
    pub download_url: Option<String>,
    pub status: UpdateStatus,
    pub update_in_progress: bool,
}

pub enum UpdateStatus {
    Idle,
    Checking,
    Downloading,
    Installing,
    Ready,       // installed, waiting for restart
    Error(String),
}
```

### Dependencies

| Crate | Purpose |
|-------|---------|
| `ureq` (v3) | HTTP client for GitHub API + DMG download |
| `semver` | Proper version comparison (avoids lexicographic gotcha) |
| `notify` | Already in deps — reused for binary self-watch |
| `serde_json` | Already in deps — parse GitHub Releases JSON |

## Version Check — `check_for_update()`

- Endpoint: `GET https://api.github.com/repos/dennismysh/rustynotes/releases/latest`
- Headers: `User-Agent: rustynotes-updater`, `Accept: application/vnd.github.v3+json`
- Version source: `env!("CARGO_PKG_VERSION")` baked in at compile time
- Comparison: Parse both versions with `semver::Version`. If remote > current and remote != `last_updated_version`, an update is available.
- Asset selection: First asset whose name ends with `-macos-universal.dmg`
- Error handling: Returns `None` on any failure (network, parse, etc.) — update checks never crash the app
- Duplicate prevention: If `available_version == config.last_updated_version`, suppress the banner and skip auto-install

## Download & Install — `apply_update(url)`

Runs on a background thread via `std::thread::spawn`. Emits Tauri events (`update-status`) for each phase.

1. Set `update_in_progress = true` in `UpdateState`
2. Set `config.last_updated_version = Some(new_version)` and persist config
3. Clean temp dir: `/tmp/rustynotes-update/`, rm + recreate
4. Download DMG: `ureq::get(url)` → read full body into `Vec<u8>` → write to `/tmp/rustynotes-update/rustynotes.dmg`
5. Mount DMG: `hdiutil attach <dmg> -nobrowse -noautoopen -mountpoint /tmp/rustynotes-update/mount`
6. Validate: Check `mount/rustynotes.app` exists. If not, detach + emit `Error`
7. Replace bundle: `rm -rf /Applications/rustynotes.app` then `cp -R mount/rustynotes.app /Applications/rustynotes.app`
8. Cleanup: `hdiutil detach`, `rm -rf /tmp/rustynotes-update/`
9. Emit `Ready` status (frontend shows "Restart" button)
10. If auto-update mode: automatically proceed to relaunch
11. On user click or auto: Spawn `/Applications/rustynotes.app/Contents/MacOS/rustynotes`, close current window

## Binary Self-Watch

Detects updates applied externally (e.g., user drags new .app to /Applications).

**Init (on app startup):**
- `exe = std::env::current_exe().canonicalize()`
- `watcher = notify::recommended_watcher(tx)`
- `watcher.watch(exe.parent(), NonRecursive)`

**Detection:**
- On file change event matching the executable path: record `binary_changed_at = Some(Instant::now())`

**Relaunch (debounced):**
- Checked periodically (via Tauri event loop or polling thread)
- If `binary_changed_at.elapsed() >= 500ms` AND `!update_in_progress`:
  - Save any unsaved work (trigger `perform_save` if dirty)
  - Spawn the new binary via `Command::new(exe)`
  - Close the current window

**Coordination with updater:**
- The `update_in_progress` flag prevents the binary watcher from relaunching during a self-update (the updater handles its own relaunch)

## Scheduling

| Trigger | When |
|---------|------|
| Startup | Background thread immediately on app init |
| Periodic | Every 6 hours (tracked via `Instant` comparison) |
| Manual | User clicks "Check for updates" in settings |

## Frontend UI

### Update Banner (toolbar)

Inline pill between the toolbar buttons and the filename. Only visible when there's an update.

**States:**
- **Update available (ask-first mode):** "v0.3.0 available" + "Update" button
- **Downloading:** "Downloading..." with subtle progress
- **Ready:** "Update ready" + "Restart" button
- **Error:** "Update failed" (tooltip shows error details)

Styled as a small inline element in `--accent` color. Dismissible with "x" (sets `last_updated_version` to suppress for this version).

### Settings — Updates Category

New category in settings sidebar:
- **Current version** display: `env!("CARGO_PKG_VERSION")`
- **Auto-update toggle** (default: on) — when on, downloads silently, only prompts for restart
- **"Check for updates" button** — triggers manual check, shows result inline

## Config Additions

Added to `AppConfig` in `crates/rustynotes-common/src/lib.rs`:

```rust
pub auto_update: bool,                    // default: true
pub last_updated_version: Option<String>, // suppress repeated prompts
```

## Testing Strategy

1. **Version comparison:** Unit test `semver` parsing with edge cases (0.9.0 vs 0.10.0, same version, newer version)
2. **GitHub API parsing:** Unit test JSON response parsing with sample payload
3. **DMG operations:** Integration test on macOS (hdiutil mount/unmount with a test DMG)
4. **Binary watcher:** Manual test — replace the binary while the app is running, verify relaunch
5. **Frontend:** Manual smoke test — verify banner appears, buttons work, settings toggle functions
6. **End-to-end:** Create a test release on GitHub, verify the full flow from detection to restart

## Out of Scope

- Windows/Linux support (macOS only for now)
- Download progress percentage (just show "Downloading...")
- Release notes display in the app
- Delta/patch updates (always full DMG replacement)
