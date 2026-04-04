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

    let body_str = response
        .into_body()
        .read_to_string()
        .ok()?;

    let body: serde_json::Value = serde_json::from_str(&body_str).ok()?;

    let tag = body["tag_name"].as_str()?;
    let remote_version = tag.trim_start_matches('v');

    let remote = semver::Version::parse(remote_version).ok()?;
    let current = semver::Version::parse(CURRENT_VERSION).ok()?;

    if remote <= current {
        return None;
    }

    if let Some(last) = last_updated_version {
        if last == remote_version {
            return None;
        }
    }

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
pub fn download_and_install(url: &str) -> Result<(), String> {
    let temp_dir = Path::new(TEMP_DIR);
    let dmg_path = temp_dir.join("rustynotes.dmg");
    let mount_point = temp_dir.join("mount");

    let _ = fs::remove_dir_all(temp_dir);
    fs::create_dir_all(temp_dir).map_err(|e| format!("create temp dir: {e}"))?;

    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("download: {e}"))?;

    let bytes = response
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("read body: {e}"))?;

    let mut file = fs::File::create(&dmg_path).map_err(|e| format!("create dmg: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("write dmg: {e}"))?;
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
        .map_err(|e| format!("hdiutil attach: {e}"))?;

    if !mount_status.status.success() {
        return Err(format!(
            "hdiutil attach failed: {}",
            String::from_utf8_lossy(&mount_status.stderr)
        ));
    }

    let mounted_app = mount_point.join("rustynotes.app");
    if !mounted_app.exists() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap()])
            .output();
        return Err("rustynotes.app not found in DMG".to_string());
    }

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

    let _ = Command::new("hdiutil")
        .args(["detach", mount_point.to_str().unwrap()])
        .output();
    let _ = fs::remove_dir_all(temp_dir);

    Ok(())
}

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
        let result = check_for_update(None);
        let _ = result;
    }
}
