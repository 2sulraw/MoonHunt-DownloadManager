use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Emitter;
use tauri::Manager;

use futures_util::StreamExt;

const RELEASES_API: &str =
    "https://api.github.com/repos/2sulraw/MoonHunt-DownloadManager/releases/latest";

#[derive(Serialize, Clone)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UpdateStatus {
    UpToDate,
    #[serde(rename_all = "camelCase")]
    Available { version: String, url: String, asset_url: String },
    NoReleases,
    #[serde(rename_all = "camelCase")]
    Error { message: String },
}

/// True when the remote tag is a semver newer than the local version.
/// Non-parsable versions (pre-release tag conventions, garbage) are never "newer".
fn is_newer(local: &str, tag: &str) -> bool {
    let (Ok(local), Ok(remote)) = (
        semver::Version::parse(local),
        semver::Version::parse(tag.trim_start_matches('v')),
    ) else {
        return false;
    };
    remote > local
}

/// Pick the Windows NSIS installer asset (`.exe`) from a release's assets list.
/// Falls back to the first `.exe`, then the first URL that ends with `.exe`/`.msi`.
fn exe_asset_url(body: &serde_json::Value) -> String {
    body.get("assets")
        .and_then(|a| a.as_array())
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| {
                    a.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.ends_with(".exe"))
                        .unwrap_or(false)
                })
                .or_else(|| assets.first())
        })
        .and_then(|a| a.get("browser_download_url"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<UpdateStatus, String> {
    let resp = reqwest::Client::new()
        .get(RELEASES_API)
        .header("User-Agent", "moonhunt-app") // GitHub API requires a UA
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match resp.status() {
        s if s == reqwest::StatusCode::NOT_FOUND => return Ok(UpdateStatus::NoReleases),
        s if s.is_success() => {}
        s => return Ok(UpdateStatus::Error { message: s.to_string() }),
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let Some(tag) = body.get("tag_name").and_then(|t| t.as_str()) else {
        return Ok(UpdateStatus::NoReleases);
    };
    let local = app.package_info().version.to_string();
    if is_newer(&local, tag) {
        let url = body
            .get("html_url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        let asset_url = exe_asset_url(&body);
        Ok(UpdateStatus::Available {
            version: tag.to_string(),
            url,
            asset_url,
        })
    } else {
        Ok(UpdateStatus::UpToDate)
    }
}

/// Cache filename for an installer asset URL. The URL's last segment is the
/// versioned asset name (e.g. `moonhunt_0.1.5_x64-setup.exe`), so it's a
/// reliable per-version cache key.
fn installer_filename(asset_url: &str) -> String {
    // Take the last path segment only when the URL has an actual path after
    // the host; otherwise fall back to a sane default.
    asset_url
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "MoonHunt-setup.exe".to_string())
}

/// Spawn the downloaded NSIS installer with UAC elevation.
///
/// Since v0.0.2 the setup is `perMachine` and its manifest requires admin,
/// so a plain `Command::spawn` (CreateProcess) fails with OS error 740
/// (`ERROR_ELEVATION_REQUIRED`) — CreateProcess cannot trigger the UAC
/// prompt. `runas` uses ShellExecuteEx with the `runas` verb, which can.
///
/// Returns true when a new process was started. A cancellation (user clicks
/// "No" on the UAC prompt) yields an IO error whose raw_os_error is 1223
/// (`ERROR_CANCELLED`) — that's not a failure of our code, so we surface it
/// as Ok(false) to avoid alarming "update failed" UI for a user choice.
fn spawn_installer_elevated(installer: &std::path::Path) -> Result<bool, String> {
    match std::process::Command::new("explorer.exe")
        .arg(installer)
        .spawn()
    {
        Ok(_) => return Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Failed to launch installer: {}", e)),
    }
    // Fallback for exotic setups without explorer: direct CreateProcess via
    // runas; reports ERROR_CANCELLED (user declined UAC) as Ok(false).
    use runas::Command as RunasCommand;
    match RunasCommand::new(installer).status() {
        Ok(_) => Ok(true),
        Err(e) if e.raw_os_error() == Some(1223) => Ok(false),
        Err(e) => Err(format!("Failed to launch installer: {}", e)),
    }
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, asset_url: String) -> Result<String, String> {
    if asset_url.is_empty() {
        return Err("No installer asset available for this release".into());
    }
    let dir = app
        .path()
        .temp_dir()
        .map_err(|e| e.to_string())?
        .join("Moonhunt");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let name = installer_filename(&asset_url);
    let out_path = dir.join(&name);
    let temp_path = dir.join(format!(".{}.download", name));

    // Same version already downloaded → relaunch it instantly, no re-download,
    // and don't force-exit (the user may have canceled the first time).
    if out_path.exists() && out_path.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
        spawn_installer_elevated(&out_path)?;
        return Ok(out_path.to_string_lossy().into_owned());
    }

    let client = crate::util::configured_http_client(&app)?;
    let resp = client
        .get(&asset_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Update download error: {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        downloaded += chunk.len() as u64;
        let percent = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let _ = app.emit(
            "update-download-progress",
            serde_json::json!({ "downloaded": downloaded, "total": total, "percent": percent }),
        );
    }
    file.flush().map_err(|e| format!("Failed to flush: {}", e))?;
    drop(file);

    if downloaded == 0 {
        let _ = std::fs::remove_file(&temp_path);
        return Err("Update download was empty".into());
    }

    std::fs::rename(&temp_path, &out_path)
        .map_err(|e| format!("Failed to finalize download: {}", e))?;

    // Fresh (new-version) download → launch it elevated and let the running
    // instance exit so the new build can replace it. NSIS needs a moment to
    // unpack. If the user declines the UAC prompt we keep the app open.
    match spawn_installer_elevated(&out_path) {
        Ok(true) => {}
        Ok(false) => return Ok(out_path.to_string_lossy().into_owned()),
        Err(e) => return Err(e),
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2500));
        handle.exit(0);
    });

    Ok(out_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_patch_is_update() {
        assert!(is_newer("0.1.1", "v0.1.2"));
    }

    #[test]
    fn older_or_equal_is_not_update() {
        assert!(!is_newer("0.1.2", "v0.1.1"));
        assert!(!is_newer("0.1.1", "v0.1.1"));
    }

    #[test]
    fn major_outranks_patch_count() {
        assert!(!is_newer("0.11.0", "v0.2.0"));
        assert!(is_newer("0.1.9", "v0.2.0"));
    }

    #[test]
    fn unparsable_tag_is_not_update() {
        assert!(!is_newer("0.1.1", "vbeta"));
        assert!(!is_newer("not-a-version", "v0.1.2"));
    }

    #[test]
    fn exe_asset_url_prefers_installer() {
        let body = serde_json::json!({
            "assets": [
                { "name": "moonhunt.msi", "browser_download_url": "https://x/moonhunt.msi" },
                { "name": "moonhunt_0.1.3_x64-setup.exe", "browser_download_url": "https://x/setup.exe" },
            ]
        });
        assert_eq!(exe_asset_url(&body), "https://x/setup.exe");
    }

    #[test]
    fn exe_asset_url_falls_back_to_first() {
        let body = serde_json::json!({
            "assets": [
                { "name": "moonhunt.msi", "browser_download_url": "https://x/moonhunt.msi" },
            ]
        });
        assert_eq!(exe_asset_url(&body), "https://x/moonhunt.msi");
    }

    #[test]
    fn exe_asset_url_empty_when_no_assets() {
        assert_eq!(exe_asset_url(&serde_json::Value::Null), "");
    }

    #[test]
    fn available_serializes_camel_case_fields() {
        let v = UpdateStatus::Available {
            version: "v0.1.5".into(),
            url: "https://x".into(),
            asset_url: "https://a/exe".into(),
        };
        let s = serde_json::to_value(&v).unwrap();
        // enum-level rename_all camelCases the variant tag; the per-variant
        // rename_all must camelCase the struct fields too.
        assert_eq!(s["status"], "available");
        assert_eq!(s["assetUrl"], "https://a/exe");
        assert_eq!(s["url"], "https://x");
        assert_eq!(s["version"], "v0.1.5");
    }

    #[test]
    fn installer_filename_uses_versioned_asset_name() {
        let url = "https://github.com/2sulraw/Test/releases/download/v0.1.5/moonhunt_0.1.5_x64-setup.exe";
        assert_eq!(installer_filename(url), "moonhunt_0.1.5_x64-setup.exe");
        // different version → different cache key
        let url2 = "https://github.com/2sulraw/Test/releases/download/v0.1.6/moonhunt_0.1.6_x64-setup.exe";
        assert_ne!(installer_filename(url), installer_filename(url2));
        // last non-empty segment of the URL path
        assert_eq!(installer_filename("https://github.com/x/y.exe"), "y.exe");
    }
}