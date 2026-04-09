# Updater Hardening — Design Spec

**Date:** 2026-04-09
**Scope:** Fix silent update failure + harden the existing DMG-based auto-update system
**Revises:** `2026-04-04-auto-update-design.md` (original implementation)

## Problem

The updater silently fails to detect available updates. Root cause: `last_updated_version` is set in config *before* the install completes, so a failed or incomplete update permanently suppresses that version. Additionally, every failure point in the check/install pipeline returns `None` or a generic string, making failures invisible.

## Goals

1. Fix the `last_updated_version` suppression bug
2. Make every failure point report *what went wrong*
3. Verify the install actually succeeded before declaring success
4. Surface error details to the frontend through the existing event pipeline
5. Consolidate duplicated update logic between background thread and IPC commands

## Non-Goals

- Switching to `tauri-plugin-updater` (keeping the custom DMG-based approach)
- Retry logic or backoff
- Rollback on failure
- Download progress reporting
- Windows/Linux support

## Design

### 1. Replace `last_updated_version` with `dismissed_version`

**Current behavior:** `last_updated_version: Option<String>` in `AppConfig` is set before install and compared in `check_for_update()` — if the remote version matches, the check returns `None` regardless of whether the install succeeded.

**New behavior:**
- Remove `last_updated_version` from `AppConfig`
- Add `dismissed_version: Option<String>` — only set when the user explicitly dismisses the update banner
- `check_for_update()` compares `remote > CARGO_PKG_VERSION` only — no config-based suppression of the check itself
- `dismissed_version` suppresses the *banner*, not the check. The background thread still knows an update exists but won't show it or auto-install it for that version
- `dismissed_version` is cleared automatically when a newer version appears (i.e., dismissed `0.4.0` but `0.5.0` comes out → banner reappears)
- Auto-update (when enabled) skips versions that match `dismissed_version` — the user explicitly said "not now"

**Config migration:** Existing `last_updated_version` values in config files are ignored (field removed from struct, serde skips unknown fields with `#[serde(default)]`).

**Files changed:**
- `crates/rustynotes-common/src/lib.rs` — replace field in `AppConfig`
- `src-tauri/src/updater.rs` — remove `last_updated_version` parameter from `check_for_update()`
- `src-tauri/src/commands/update.rs` — update `dismiss_update` to write `dismissed_version`
- `src-tauri/src/lib.rs` — update background thread to use new field
- `crates/rustynotes-frontend/src/components/toolbar.rs` — no change needed (dismiss IPC call stays the same)

### 2. Typed error pipeline

**Current:** `check_for_update()` returns `Option<UpdateInfo>` with `?` on every fallible operation. `download_and_install()` returns `Result<(), String>`.

**New:** Both functions return `Result<_, UpdateError>` with a typed enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateError {
    // check_for_update errors
    NetworkError(String),       // ureq call failed
    ApiError(String),           // non-200 response or body read failed
    ParseError(String),         // JSON parse or missing fields
    AssetNotFound,              // no DMG asset matching suffix
    VersionParseError(String),  // semver parse failed

    // download_and_install errors
    DownloadFailed(String),     // DMG download failed
    DmgMountFailed(String),     // hdiutil attach failed
    AppNotFoundInDmg,           // rustynotes.app not in mounted volume
    CopyFailed(String),         // cp -R failed
    VerificationFailed {        // post-install version mismatch
        expected: String,
        found: String,
    },
}
```

`check_for_update()` signature changes to:
```rust
pub fn check_for_update() -> Result<Option<UpdateInfo>, UpdateError>
```

`Ok(None)` = checked successfully, you're up to date. `Err(e)` = something broke.

`download_and_install()` signature changes to:
```rust
pub fn download_and_install(url: &str, expected_version: &str) -> Result<(), UpdateError>
```

The `expected_version` parameter is used for post-install verification (section 3).

**User-facing messages:** `UpdateError` implements `Display` with user-friendly text:

| Variant | Display |
|---------|---------|
| `NetworkError` | "Couldn't reach GitHub — check your connection" |
| `ApiError` | "GitHub API error — try again later" |
| `ParseError` | "Unexpected response from GitHub" |
| `AssetNotFound` | "No macOS update found in this release" |
| `VersionParseError` | "Invalid version format in release" |
| `DownloadFailed` | "Download failed — check your connection" |
| `DmgMountFailed` | "Couldn't open the update file" |
| `AppNotFoundInDmg` | "Update file is damaged or incomplete" |
| `CopyFailed` | "Couldn't install update — check /Applications permissions" |
| `VerificationFailed` | "Update installed but version doesn't match — try updating manually" |

### 3. Post-install verification

After `cp -R` succeeds, before cleanup:

1. Read `/Applications/rustynotes.app/Contents/Info.plist`
2. Extract `CFBundleShortVersionString` via `plutil -extract CFBundleShortVersionString raw -o - <path>`
3. Compare to `expected_version`
4. If mismatch → return `Err(UpdateError::VerificationFailed { expected, found })`
5. If match → proceed to cleanup and emit `Ready`

This catches cases where `cp -R` exits 0 but the bundle wasn't actually replaced (macOS file protection, SIP, permission issues).

### 4. Frontend error surfacing

No new UI components. Changes to existing behavior:

- **`UpdateStatus::Error`** already exists and carries a `String` — change it to carry the `UpdateError`'s `Display` output so messages are specific
- **Toolbar banner** already renders error state — it will now show "Couldn't reach GitHub — check your connection" instead of a generic error
- **Settings "Check now"** result text will show the same specific messages
- **Settings panel** `check_for_update_cmd` IPC return type changes: currently returns `Option<String>` (version or None). Change to return `Result<Option<String>, String>` so errors propagate to the UI rather than looking like "up to date"

### 5. Background thread consolidation

**Current:** `lib.rs` background thread (lines 75-126) duplicates the check + auto-install logic that also exists in `commands/update.rs`.

**New:** Extract a shared function in `commands/update.rs`:

```rust
/// Perform an update check and optionally auto-apply.
/// Used by both the background thread and the manual IPC command.
pub fn perform_check_and_maybe_apply(
    app: &AppHandle,
    state: &UpdateState,
    config_state: &commands::config::ConfigState,
) { ... }
```

The background thread in `lib.rs` calls this function in its loop. The `check_for_update` IPC command calls the check portion. The `apply_update` IPC command calls the install portion. One code path for each operation.

## Files Changed

| File | Change |
|------|--------|
| `crates/rustynotes-common/src/lib.rs` | Replace `last_updated_version` with `dismissed_version` |
| `src-tauri/src/updater.rs` | `UpdateError` enum, typed returns, post-install verification, remove `last_updated_version` param |
| `src-tauri/src/commands/update.rs` | Shared `perform_check_and_maybe_apply`, update IPC commands for new error types, update `dismiss_update` |
| `src-tauri/src/lib.rs` | Background thread calls shared function |
| `crates/rustynotes-frontend/src/components/toolbar.rs` | No structural change — error messages improve automatically |
| `crates/rustynotes-frontend/src/components/settings/categories/update.rs` | Handle error results from check command |
| `crates/rustynotes-frontend/src/tauri_ipc.rs` | Update `check_for_update_cmd` return handling if needed |

## Testing

1. **Unit: `UpdateError` display** — verify each variant produces the expected user-facing string
2. **Unit: version comparison** — `check_for_update` with mocked responses: newer version, same version, older version, malformed JSON, missing asset
3. **Unit: `dismissed_version` logic** — dismissed version suppresses banner but not check; new version clears dismissed
4. **Unit: verification** — mock `plutil` output, test match and mismatch
5. **Manual: end-to-end** — create a test release, verify detection → download → verify → ready flow
6. **Manual: error states** — disconnect network during check, corrupt DMG, read-only /Applications
