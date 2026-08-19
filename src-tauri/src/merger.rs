//! Media merger management for yt-dlp: MP4Box (bundled by default) and optional
//! ffmpeg. Mirrors the JS-runtime manager so the user can install/select which
//! tool yt-dlp uses to combine separate video+audio streams.

use crate::state::{lock, AppState};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::command;
use tauri::Emitter;
use tauri::Manager;

struct MergerDef {
    id: &'static str,
    name: &'static str,
    exe: &'static str,
    url: &'static str,
    archive: bool,
    archive_name: &'static str,
    size_label: &'static str,
}

const MERGERS: &[MergerDef] = &[
    MergerDef {
        id: "ffmpeg",
        name: "FFmpeg",
        exe: "ffmpeg.exe",
        url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        archive: true,
        archive_name: "ffmpeg.exe",
        size_label: "~90 MB",
    },
];

/// Pseudo-id for a system ffmpeg already on PATH (like system-node).
pub(crate) const SYSTEM_FFMPEG_ID: &str = "system-ffmpeg";

pub(crate) fn merger_def(id: &str) -> Option<&'static MergerDef> {
    MERGERS.iter().find(|def| def.id == id)
}

fn merger_bin_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to resolve app local data dir: {}", e))?;
    Ok(base.join("binaries").join("mergers"))
}

fn merger_dir(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(merger_bin_dir(app)?.join(id))
}

pub(crate) fn merger_path(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    let def = merger_def(id).ok_or_else(|| format!("Unknown merger: {}", id))?;
    Ok(merger_dir(app, id)?.join(def.exe))
}

pub(crate) fn selected_merger_id(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    let settings = lock(&state.proxy_settings)?;
    Ok(settings.media_merger.clone())
}

/// Set the selected merger id in settings (persisted + written back to state).
pub(crate) fn set_selected_merger(app: &tauri::AppHandle, id: Option<String>) -> Result<(), String> {
    let mut settings = {
        let state = app.state::<AppState>();
        let guard = lock(&state.proxy_settings)?;
        guard.clone()
    };
    settings.media_merger = id;
    crate::db::save_settings_to_db(&settings)?;
    let state = app.state::<AppState>();
    let mut guard = lock(&state.proxy_settings)?;
    *guard = settings;
    Ok(())
}

// ─── System ffmpeg detection (pseudo-entry) ──────────────────────────────

#[derive(Clone)]
pub(crate) struct SystemFfmpegInfo {
    pub path: String,
}

/// Detect ffmpeg on PATH (and common locations). Returns the exe path.
pub(crate) fn detect_system_ffmpeg() -> Option<SystemFfmpegInfo> {
    let probe = if cfg!(target_os = "windows") {
        crate::util::cmd_no_window("where.exe").arg("ffmpeg").output().ok()
    } else {
        crate::util::cmd_no_window("which").arg("ffmpeg").output().ok()
    };
    if let Some(output) = probe {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = stdout.lines().next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return Some(SystemFfmpegInfo { path: trimmed.to_string() });
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    for p in [
        PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe"),
        PathBuf::from(r"C:\Program Files\ffmpeg\bin\ffmpeg.exe"),
        PathBuf::from(r"C:\Program Files (x86)\ffmpeg\bin\ffmpeg.exe"),
    ] {
        if p.exists() {
            return Some(SystemFfmpegInfo { path: p.to_string_lossy().into_owned() });
        }
    }
    None
}

/// Resolve the effective merger for a download: the selected merger if usable,
/// else bundled ffmpeg if present, else none.
pub(crate) fn resolved_merger_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let selected = selected_merger_id(app).ok().flatten();

    if selected.as_deref() == Some(SYSTEM_FFMPEG_ID) {
        return detect_system_ffmpeg().map(|i| PathBuf::from(i.path));
    }

    let def = merger_def("ffmpeg")?;
    let path = merger_path(app, def.id).ok()?;
    if path.exists() {
        return Some(path);
    }
    None
}

#[command]
pub fn get_mergers(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let selected = selected_merger_id(&app)?;
    let mut mergers = Vec::new();

    for def in MERGERS {
        let path = merger_path(&app, def.id)?;
        let installed = path.exists();
        mergers.push(json!({
            "id": def.id,
            "name": def.name,
            "installed": installed,
            "selected": selected.as_deref() == Some(def.id),
            "path": if installed {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            },
            "size": def.size_label,
        }));
    }

    let system_ffmpeg = detect_system_ffmpeg().map(|info| {
        json!({
            "path": info.path,
            "selected": selected.as_deref() == Some(SYSTEM_FFMPEG_ID),
        })
    });

    Ok(json!({
        "mergers": mergers,
        "systemFfmpeg": system_ffmpeg,
    }))
}

#[command]
pub fn select_merger(app: tauri::AppHandle, merger_id: String) -> Result<String, String> {
    let message = if merger_id == SYSTEM_FFMPEG_ID {
        let info = detect_system_ffmpeg().ok_or_else(|| "System ffmpeg was not found".to_string())?;
        format!("Using system FFmpeg ({}) for media merging", info.path)
    } else {
        let def = merger_def(&merger_id).ok_or_else(|| format!("Unknown merger: {}", merger_id))?;
        let path = merger_path(&app, def.id)?;
        if !path.exists() {
            return Err(format!("{} is not installed", def.name));
        }
        format!("Using {} for media merging", def.name)
    };

    set_selected_merger(&app, Some(merger_id))?;
    Ok(message)
}

#[command]
pub async fn install_merger(app: tauri::AppHandle, merger_id: String) -> Result<String, String> {
    use futures_util::StreamExt;

    let def = merger_def(&merger_id).ok_or_else(|| format!("Unknown merger: {}", merger_id))?;
    let cancel_key = format!("merger:{}", def.id);
    crate::install_cancel::clear(&cancel_key);
    let dir = merger_dir(&app, def.id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create merger dir: {}", e))?;

    let out_path = dir.join(def.exe);
    let temp_path = dir.join(format!(".download-{}", def.id));
    let client = crate::util::configured_http_client(&app)?;

    println!("[~] Downloading {} from {}", def.name, def.url);
    let resp = client
        .get(def.url)
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {}", def.name, e))?;
    if !resp.status().is_success() {
        return Err(format!("{} download error: {}", def.name, resp.status()));
    }
    if crate::install_cancel::is_cancelled(&cancel_key) {
        crate::install_cancel::clear(&cancel_key);
        return Err("Install cancelled".to_string());
    }

    let total = resp.content_length().unwrap_or(0);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)
        .map_err(|e| format!("Failed to create temp download: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(item) = stream.next().await {
        if crate::install_cancel::is_cancelled(&cancel_key) {
            drop(file);
            let _ = std::fs::remove_file(&temp_path);
            crate::install_cancel::clear(&cancel_key);
            return Err("Install cancelled".to_string());
        }

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
            "merger_install_progress",
            json!({
                "mergerId": def.id,
                "downloaded": downloaded,
                "total": total,
                "percent": percent,
            }),
        );
    }
    if crate::install_cancel::is_cancelled(&cancel_key) {
        drop(file);
        let _ = std::fs::remove_file(&temp_path);
        crate::install_cancel::clear(&cancel_key);
        return Err("Install cancelled".to_string());
    }
    file.flush()
        .map_err(|e| format!("Failed to flush download: {}", e))?;
    drop(file);

    if crate::install_cancel::is_cancelled(&cancel_key) {
        let _ = std::fs::remove_file(&temp_path);
        crate::install_cancel::clear(&cancel_key);
        return Err("Install cancelled".to_string());
    }

    if downloaded == 0 {
        let _ = std::fs::remove_file(&temp_path);
        crate::install_cancel::clear(&cancel_key);
        return Err(format!("{} download was empty", def.name));
    }

    if def.archive {
        extract_merger_zip(&temp_path, &out_path, def.archive_name)?;
        std::fs::remove_file(&temp_path)
            .map_err(|e| format!("Failed to remove temp archive: {}", e))?;
    } else {
        if out_path.exists() {
            std::fs::remove_file(&out_path)
                .map_err(|e| format!("Failed to replace existing {}: {}", def.name, e))?;
        }
        std::fs::rename(&temp_path, &out_path)
            .map_err(|e| format!("Failed to save {}: {}", def.name, e))?;
    }

    if crate::install_cancel::is_cancelled(&cancel_key) {
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&temp_path);
        crate::install_cancel::clear(&cancel_key);
        return Err("Install cancelled".to_string());
    }

    crate::install_cancel::clear(&cancel_key);
    println!("[✓] {} saved to {}", def.name, out_path.display());
    Ok(format!("Installed {} at {}", def.name, out_path.display()))
}

#[command]
pub fn cancel_merger_install(merger_id: String) -> Result<String, String> {
    let def = merger_def(&merger_id).ok_or_else(|| format!("Unknown merger: {}", merger_id))?;
    crate::install_cancel::request_cancel(&format!("merger:{}", def.id));
    Ok(format!("Cancelling {} install", def.name))
}

#[command]
pub fn remove_merger(app: tauri::AppHandle, merger_id: String) -> Result<String, String> {
    if merger_id == SYSTEM_FFMPEG_ID {
        return Err("System ffmpeg cannot be removed".to_string());
    }

    let def = merger_def(&merger_id).ok_or_else(|| format!("Unknown merger: {}", merger_id))?;
    let dir = merger_dir(&app, def.id)?;
    if !dir.exists() {
        return Err(format!("{} is not installed", def.name));
    }

    std::fs::remove_dir_all(&dir)
        .map_err(|e| format!("Failed to remove {}: {}", def.name, e))?;

    let selected_id = selected_merger_id(&app)?;
    if selected_id.as_deref() == Some(def.id) {
        set_selected_merger(&app, None)?;
    }

    Ok(format!("Removed {}", def.name))
}

fn extract_merger_zip(zip_path: &Path, out_path: &Path, wanted: &str) -> Result<(), String> {
    let extract_dir = out_path
        .parent()
        .ok_or_else(|| "Merger dir not available".to_string())?
        .join(".extract");

    let result = (|| -> Result<(), String> {
        if extract_dir.exists() {
            std::fs::remove_dir_all(&extract_dir)
                .map_err(|e| format!("Failed to clean extract dir: {}", e))?;
        }
        std::fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to create extract dir: {}", e))?;

        let status = crate::util::cmd_no_window("tar")
            .arg("-xf")
            .arg(zip_path)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .map_err(|e| format!("Failed to extract archive: {}", e))?;
        if !status.success() {
            return Err(format!("Archive extraction failed with code {:?}", status.code()));
        }

        let found = find_file_in_dir(&extract_dir, wanted)
            .ok_or_else(|| format!("{} not found in archive", wanted))?;
        std::fs::copy(&found, out_path)
            .map_err(|e| format!("Failed to copy {}: {}", wanted, e))?;
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&extract_dir);
    result
}

fn find_file_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.eq_ignore_ascii_case(name)) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_in_dir(&path, name) {
                return Some(found);
            }
        }
    }
    None
}
