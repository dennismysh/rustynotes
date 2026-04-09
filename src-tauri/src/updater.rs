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
    fn test_update_error_display_network() {
        let err = UpdateError::NetworkError("connection refused".into());
        assert_eq!(
            err.to_string(),
            "Couldn\u{2019}t reach GitHub \u{2014} check your connection"
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
            "Update installed but version doesn\u{2019}t match \u{2014} try updating manually"
        );
    }

    #[test]
    fn test_update_error_display_all_variants() {
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
