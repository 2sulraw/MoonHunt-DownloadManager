//! JS runtime management for yt-dlp: install QuickJS/Node/Bun/Deno into
//! app-local data and let the user select which one yt-dlp should use.

use crate::state::{lock,};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct RuntimeDef {
    id: &'static str,
    name: &'static str,
    exe: &'static str,
    url: &'static str,
    archive: bool,
    archive_name: &'static str,
    size_label: &'static str,
}

fn get_runtimes() -> Vec<RuntimeDef> {
    if cfg!(target_os = "windows") {
        vec![
            RuntimeDef {
                id: "quickjs",
                name: "QuickJS",
                exe: "qjs-windows-x86_64.exe",
                url: "https://github.com/quickjs-ng/quickjs/releases/latest/download/qjs-windows-x86_64.exe",
                archive: false,
                archive_name: "",
                size_label: "~2 MB",
            },
            RuntimeDef {
                id: "node",
                name: "Node.js",
                exe: "node.exe",
                url: "https://nodejs.org/dist/latest-v24.x/win-x64/node.exe",
                archive: false,
                archive_name: "",
                size_label: "~88 MB",
            },
            RuntimeDef {
                id: "bun",
                name: "Bun",
                exe: "bun.exe",
                url: "https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip",
                archive: true,
                archive_name: "bun.exe",
                size_label: "~37 MB zip / ~100 MB installed",
            },
            RuntimeDef {
                id: "deno",
                name: "Deno",
                exe: "deno.exe",
                url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip",
                archive: true,
                archive_name: "deno.exe",
                size_label: "~41 MB zip / ~100 MB installed",
            },
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            RuntimeDef {
                id: "quickjs",
                name: "QuickJS",
                exe: "qjs-darwin-universal",
                url: "https://github.com/quickjs-ng/quickjs/releases/latest/download/qjs-darwin-universal",
                archive: false,
                archive_name: "",
                size_label: "~2 MB",
            },
            RuntimeDef {
                id: "node",
                name: "Node.js",
                exe: "node",
                url: "https://nodejs.org/dist/latest-v24.x/node-v24.12.0-darwin-arm64.tar.gz",
                archive: true,
                archive_name: "node",
                size_label: "~45 MB",
            },
            RuntimeDef {
                id: "bun",
                name: "Bun",
                exe: "bun",
                url: "https://github.com/oven-sh/bun/releases/latest/download/bun-darwin-aarch64.zip",
                archive: true,
                archive_name: "bun",
                size_label: "~37 MB zip / ~100 MB installed",
            },
            RuntimeDef {
                id: "deno",
                name: "Deno",
                exe: "deno",
                url: "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip",
                archive: true,
                archive_name: "deno",
                size_label: "~41 MB zip / ~100 MB installed",
            },
        ]
    } else {
        // Linux
        vec![
            RuntimeDef {
                id: "quickjs",
                name: "QuickJS",
                exe: "qjs-linux-x86_64",
                url: "https://github.com/quickjs-ng/quickjs/releases/latest/download/qjs-linux-x86_64",
                archive: false,
                archive_name: "",
                size_label: "~2 MB",
            },
            RuntimeDef {
                id: "node",
                name: "Node.js",
                exe: "node",
                url: "https://nodejs.org/dist/latest-v24.x/node-v24.12.0-linux-x64.tar.gz",
                archive: true,
                archive_name: "node",
                size_label: "~45 MB",
            },
            RuntimeDef {
                id: "bun",
                name: "Bun",
                exe: "bun",
                url: "https://github.com/oven-sh/bun/releases/latest/download/bun-linux-x64.zip",
                archive: true,
                archive_name: "bun",
                size_label: "~37 MB zip / ~100 MB installed",
            },
            RuntimeDef {
                id: "deno",
                name: "Deno",
                exe: "deno",
                url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip",
                archive: true,
                archive_name: "deno",
                size_label: "~41 MB zip / ~100 MB installed",
            },
        ]
    }
}

fn runtime_def(id: &str) -> Option<RuntimeDef> {
    get_runtimes().into_iter().find(|def| def.id == id)
}

pub(crate) const SYSTEM_NODE_ID: &str = "system-node";

#[derive(Clone)]
struct SystemNodeInfo {
    version: String,
    path: String,
}

struct SystemNodeCache {
    found_at: Instant,
    info: Option<SystemNodeInfo>,
}

const SYSTEM_NODE_CACHE_TTL: Duration = Duration::from_secs(10);
static SYSTEM_NODE_CACHE: OnceLock<Mutex<Option<SystemNodeCache>>> = OnceLock::new();

fn system_node_info() -> Option<SystemNodeInfo> {
    let now = Instant::now();
    let cache = SYSTEM_NODE_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().ok()?;
    if let Some(cached) = guard.as_ref() {
        if now.duration_since(cached.found_at) < SYSTEM_NODE_CACHE_TTL {
            return cached.info.clone();
        }
    }

    let info = detect_system_node();
    *guard = Some(SystemNodeCache {
        found_at: now,
        info: info.clone(),
    });
    info
}

fn detect_system_node() -> Option<SystemNodeInfo> {
    let version_output = crate::util::cmd_no_window("node")
        .arg("--version")
        .output()
        .ok()?;
    if !version_output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();
    if version.is_empty() {
        return None;
    }

    let path = if cfg!(target_os = "windows") {
        crate::util::cmd_no_window("where.exe").arg("node").output().ok()
    } else {
        crate::util::cmd_no_window("which").arg("node").output().ok()
    };
    let path = path
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|output| output.lines().next().map(|line| line.trim().to_string()))
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| "node".to_string());

    Some(SystemNodeInfo { version, path })
}

fn runtime_bin_dir(app: &crate::ctx::AppHandle) -> Result<PathBuf, String> {
    let base = app
        .app_local_data_dir()
        .map_err(|e| format!("Failed to resolve app local data dir: {}", e))?;
    Ok(base.join("binaries").join("runtimes"))
}

fn runtime_dir(app: &crate::ctx::AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(runtime_bin_dir(app)?.join(id))
}

pub(crate) fn runtime_path(app: &crate::ctx::AppHandle, id: &str) -> Result<PathBuf, String> {
    let def = runtime_def(id).ok_or_else(|| format!("Unknown JS runtime: {}", id))?;
    Ok(runtime_dir(app, id)?.join(def.exe))
}

fn selected_runtime_id(app: &crate::ctx::AppHandle) -> Result<Option<String>, String> {
    let state = app.state();
    let settings = lock(&state.proxy_settings)?;
    Ok(settings.js_runtime.clone())
}

pub fn get_js_runtimes(app: crate::ctx::AppHandle) -> Result<serde_json::Value, String> {
    let selected = selected_runtime_id(&app)?;
    let mut runtimes = Vec::new();

    for def in get_runtimes() {
        let path = runtime_path(&app, def.id)?;
        let installed = path.exists();
        runtimes.push(json!({
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

    let system_node = system_node_info().map(|node| {
        json!({
            "version": node.version,
            "path": node.path,
            "selected": selected.as_deref() == Some(SYSTEM_NODE_ID),
        })
    });

    Ok(json!({
        "runtimes": runtimes,
        "systemNode": system_node,
    }))
}

pub fn select_js_runtime(app: crate::ctx::AppHandle, runtime_id: String) -> Result<String, String> {
    let message = if runtime_id == SYSTEM_NODE_ID {
        let node = system_node_info().ok_or_else(|| "System Node.js was not found".to_string())?;
        format!("Using Node.js ({}) for YouTube JavaScript solving", node.version)
    } else {
        let def = runtime_def(&runtime_id).ok_or_else(|| format!("Unknown JS runtime: {}", runtime_id))?;
        let path = runtime_path(&app, def.id)?;
        if !path.exists() {
            return Err(format!("{} is not installed", def.name));
        }
        format!("Using {} for YouTube JavaScript solving", def.name)
    };

    let mut settings = {
        let state = app.state();
        let guard = lock(&state.proxy_settings)?;
        guard.clone()
    };
    settings.js_runtime = Some(runtime_id.clone());
    crate::db::save_settings_to_db(&settings)?;

    {
        let state = app.state();
        let mut guard = lock(&state.proxy_settings)?;
        *guard = settings;
    }

    Ok(message)
}

pub fn remove_js_runtime(app: crate::ctx::AppHandle, runtime_id: String) -> Result<String, String> {
    if runtime_id == SYSTEM_NODE_ID {
        return Err("System Node.js cannot be removed".to_string());
    }

    let def = runtime_def(&runtime_id).ok_or_else(|| format!("Unknown JS runtime: {}", runtime_id))?;
    let dir = runtime_dir(&app, def.id)?;
    if !dir.exists() {
        return Err(format!("{} is not installed", def.name));
    }

    std::fs::remove_dir_all(&dir)
        .map_err(|e| format!("Failed to remove {}: {}", def.name, e))?;

    let selected_id = selected_runtime_id(&app)?;
    if selected_id.as_deref() == Some(def.id) {
        let mut settings = {
            let state = app.state();
            let guard = lock(&state.proxy_settings)?;
            guard.clone()
        };
        settings.js_runtime = None;
        crate::db::save_settings_to_db(&settings)?;

        let state = app.state();
        let mut guard = lock(&state.proxy_settings)?;
        *guard = settings;
    }

    Ok(format!("Removed {}", def.name))
}

pub async fn install_js_runtime(
    app: crate::ctx::AppHandle,
    runtime_id: String,
) -> Result<String, String> {
    use futures_util::StreamExt;

    let def = runtime_def(&runtime_id).ok_or_else(|| format!("Unknown JS runtime: {}", runtime_id))?;
    let cancel_key = format!("js-runtime:{}", def.id);
    crate::install_cancel::clear(&cancel_key);
    let dir = runtime_dir(&app, def.id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create runtime dir: {}", e))?;

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
            "js_runtime_install_progress",
            json!({
                "runtimeId": def.id,
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
        extract_runtime_archive(&temp_path, &out_path, def.archive_name)?;
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

    // On Unix, ensure the downloaded binary is executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
    }

    crate::install_cancel::clear(&cancel_key);
    println!("[âœ“] {} saved to {}", def.name, out_path.display());
    Ok(format!("Installed {} at {}", def.name, out_path.display()))
}

pub fn cancel_js_runtime_install(runtime_id: String) -> Result<String, String> {
    let def = runtime_def(&runtime_id).ok_or_else(|| format!("Unknown JS runtime: {}", runtime_id))?;
    crate::install_cancel::request_cancel(&format!("js-runtime:{}", def.id));
    Ok(format!("Cancelling {} install", def.name))
}

fn extract_runtime_archive(archive_path: &Path, out_path: &Path, wanted: &str) -> Result<(), String> {
    if is_zip_archive(archive_path) {
        extract_runtime_zipfile(archive_path, out_path, wanted)
    } else {
        extract_runtime_targz(archive_path, out_path, wanted)
    }
}

fn is_zip_archive(path: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic).map(|_| magic))
        .map(|m| &m[..2] == b"PK")
        .unwrap_or(false)
}

fn extract_runtime_zipfile(zip_path: &Path, out_path: &Path, wanted: &str) -> Result<(), String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;
        if entry.is_dir() {
            continue;
        }
        let base = entry
            .name()
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or("");
        if !base.eq_ignore_ascii_case(wanted) {
            continue;
        }
        let mut out = std::fs::File::create(out_path)
            .map_err(|e| format!("Failed to create {}: {}", wanted, e))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("Failed to extract {}: {}", wanted, e))?;
        return Ok(());
    }
    Err(format!("{} not found in archive", wanted))
}

fn extract_runtime_targz(zip_path: &Path, out_path: &Path, wanted: &str) -> Result<(), String> {
    let extract_dir = out_path
        .parent()
        .ok_or_else(|| "Runtime dir not available".to_string())?
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
