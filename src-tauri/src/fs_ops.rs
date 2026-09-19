//! Filesystem / OS helpers: show-in-folder, folder picker, open URL, stats.

use crate::aria2::Aria2Client;
use crate::state::{lock, AppState};
use crate::util::{cmd_no_window, dir_size, format_bytes, strip_unc_prefix};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};
use tauri::command;
use tauri::Manager;

const FS_CACHE_REFRESH_SECS: u64 = 30;
const DISK_ACTIVITY_FULL_SCALE_MBPS: f64 = 50.0;
const PROCESS_CPU_SMOOTHING: f32 = 0.25;

#[derive(Clone, Default)]
struct FsStatsCache {
    last_refresh: Option<Instant>,
    disk_bytes: u64,
    cache_bytes: u64,
    temp_bytes: u64,
    db_size: u64,
    aria2_session_size: u64,
}

#[derive(Clone, Copy)]
struct DiskRateSample {
    last_at: Instant,
}

impl FsStatsCache {
    fn stale(&self) -> bool {
        self.last_refresh
            .map(|t| t.elapsed() >= Duration::from_secs(FS_CACHE_REFRESH_SECS))
            .unwrap_or(true)
    }
}

static PERF_SYSTEM: Mutex<Option<System>> = Mutex::new(None);
static PERF_FS_CACHE: OnceLock<Mutex<FsStatsCache>> = OnceLock::new();
static PERF_DISK_RATE: Mutex<Option<DiskRateSample>> = Mutex::new(None);
static PERF_PROCESS_CPU: Mutex<Option<f32>> = Mutex::new(None);

/// sysinfo computes CPU deltas from a previous sample, so take one warm sample
/// before the first visible stats poll returns.
fn warm_cpu_sampling(sys: &mut System) {
    sys.refresh_cpu_usage();
    sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();
    sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
}

/// Sum CPU across the app process and every descendant. The Vue UI itself runs
/// in separate WebView2 child processes, so sampling only the Rust process
/// would make the app CPU value look like a constant 0.
fn app_process_tree_cpu_usage(sys: &System, root: Pid) -> f32 {
    let mut pids = vec![root];
    let mut seen = HashSet::new();
    seen.insert(root);

    let mut index = 0;
    while index < pids.len() {
        let parent = pids[index];
        for (pid, proc_) in sys.processes() {
            if proc_.parent() == Some(parent) && seen.insert(*pid) {
                pids.push(*pid);
            }
        }
        index += 1;
    }

    pids
        .into_iter()
        .filter_map(|pid| sys.process(pid))
        .map(|proc_| proc_.cpu_usage())
        .sum()
}

async fn resolve_download_path(
    app: &tauri::AppHandle,
    gid: &str,
    file_path: Option<&str>,
    filename: Option<&str>,
) -> Result<PathBuf, String> {
    fn find_file_by_name(dir: &Path, filename: &str, depth: usize) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // case-insensitive match: Windows filesystems are case-insensitive
                    if name.eq_ignore_ascii_case(filename) {
                        return Some(path);
                    }
                }
            } else if path.is_dir() {
                if let Some(found) = find_file_by_name(&path, filename, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    let download_dir = {
        let state = app.state::<AppState>();
        let settings = lock(&state.proxy_settings)?;
        let dir = settings.download_location.clone();
        if dir.trim().is_empty() {
            // Prefer the OS-reported "Downloads" folder over a hardcoded
            // path that rarely exists in real setups.
            dirs::download_dir().unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                { PathBuf::from("C:\\Downloads") }
                #[cfg(not(target_os = "windows"))]
                {
                    dirs::home_dir()
                        .map(|h| h.join("Downloads"))
                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                }
            })
        } else {
            // Canonicalize the user-supplied path to absolute, backslash
            // separators. If the path doesn't exist yet, fall back to a
            // naive forward→backslash replacement so the rest of the
            // function still gets consistent separators.
            let raw = PathBuf::from(&dir);
            match std::fs::canonicalize(&raw) {
                Ok(c) => c,
                Err(_) => PathBuf::from(dir.replace('/', "\\")),
            }
        }
    };

    println!(
        "[show_download_in_folder] GID: {}, file_path: {:?}, filename: {:?}",
        gid, file_path, filename
    );
    println!(
        "[show_download_in_folder] Download dir: {}",
        download_dir.display()
    );

    let aria2_path = match Aria2Client::new() {
        Ok(client) => match client.tell_status(gid).await {
            Ok(details) => details
                .get("files")
                .and_then(|files| files.as_array())
                .and_then(|files| files.first())
                .and_then(|file_item| file_item.get("path"))
                .and_then(|path_value| path_value.as_str())
                .map(|s| s.to_string()),
            Err(e) => {
                println!("[resolve_download_path] aria2 tellStatus failed: {}", e);
                None
            }
        },
        Err(e) => {
            println!("[resolve_download_path] aria2 client failed: {}", e);
            None
        }
    };

    // Resolve the actual file on disk using this priority:
    //   1. The path reported by aria2 (`files[0].path`)
    //   2. The path the frontend cached on the download record
    //   3. Recursive search in the configured download directory by filename
    // If nothing resolves, leave it as None and fall back to the downloads folder.
    let mut selected_path: Option<PathBuf> = None;

    if let Some(p) = aria2_path.as_deref().filter(|s| !s.trim().is_empty()) {
        selected_path = Some(PathBuf::from(p));
    } else if let Some(p) = file_path.filter(|s| !s.trim().is_empty()) {
        selected_path = Some(PathBuf::from(p));
    } else if let Some(name) = filename.filter(|s| !s.trim().is_empty()) {
        selected_path = find_file_by_name(&download_dir, name, 3);
    }

    if selected_path.is_none() {
        println!("[resolve_download_path] No specific file found");
    }

    println!(
        "[resolve_download_path] Selected path from aria2/fallback: {:?}",
        selected_path
    );

    let resolved_path = match selected_path.as_ref() {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => download_dir.join(p),
        None => {
            return Err(format!(
                "Cannot open file: {} does not exist or was moved or removed",
                filename.unwrap_or(gid)
            ))
        }
    };

    println!(
        "[resolve_download_path] Resolved path: {}",
        resolved_path.display()
    );
    println!(
        "[resolve_download_path] Path exists: {}",
        resolved_path.exists()
    );

    // If the recorded path is stale, try to find the file by filename.
    let final_path = if !resolved_path.exists() {
        let found = filename
            .filter(|s| !s.trim().is_empty())
            .and_then(|name| find_file_by_name(&download_dir, name, 5));
        match found {
            Some(path) => path,
            None => {
                println!("[resolve_download_path] Resolved path doesn't exist and no matching filename was found");
                return Err(format!(
                    "Cannot open file: {} does not exist or was moved or removed",
                    filename.unwrap_or(gid)
                ));
            }
        }
    } else {
        resolved_path
    };

    println!(
        "[resolve_download_path] Final path to open: {}",
        final_path.display()
    );

    // Final safety net: never reveal a non-existent or empty path.
    let final_path = if final_path.as_os_str().is_empty() || !final_path.exists() {
        eprintln!(
            "[resolve_download_path] Resolved path is empty or doesn't exist: {:?}.",
            final_path
        );
        return Err(format!(
            "Cannot open file: {} does not exist or was moved or removed",
            filename.unwrap_or(gid)
        ));
    } else {
        final_path
    };

    Ok(final_path)
}

#[command]
pub async fn show_download_in_folder(
    app: tauri::AppHandle,
    gid: String,
    file_path: Option<String>,
    filename: Option<String>,
) -> Result<String, String> {
    let final_path = resolve_download_path(
        &app,
        &gid,
        file_path.as_deref(),
        filename.as_deref(),
    )
    .await?;

    // Reveal and select the item using the platform-native file manager:
    // Explorer's selected-item view on Windows, Finder reveal on macOS,
    // and org.freedesktop.FileManager1/portal on Linux.
    let display_path = strip_unc_prefix(&final_path);
    match tauri_plugin_opener::reveal_item_in_dir(&final_path) {
        Ok(()) => Ok(format!("Revealed: {}", display_path.display())),
        Err(err) => {
            eprintln!(
                "[show_download_in_folder] reveal_item_in_dir failed ({}); opening folder instead",
                err
            );
            let target = if final_path.is_file() {
                final_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| final_path.clone())
            } else {
                final_path.clone()
            };
            tauri_plugin_opener::open_path(&target, None::<&str>)
                .map_err(|e| format!("Failed to open folder {}: {}", target.display(), e))?;
            Ok(format!("Opened folder: {}", target.display()))
        }
    }
}

#[command]
pub async fn open_download_file(
    app: tauri::AppHandle,
    gid: String,
    file_path: Option<String>,
    filename: Option<String>,
) -> Result<String, String> {
    let final_path = resolve_download_path(
        &app,
        &gid,
        file_path.as_deref(),
        filename.as_deref(),
    )
    .await?;
    let display_path = strip_unc_prefix(&final_path);
    tauri_plugin_opener::open_path(&final_path, None::<&str>)
        .map_err(|e| format!("Failed to open file {}: {}", display_path.display(), e))?;
    Ok(format!("Opened: {}", display_path.display()))
}

#[command]
pub fn open_external(_app: tauri::AppHandle, url: String) -> Result<(), String> {
// Open a URL in the system default browser.
    // Use platform-native opener to launch the default browser.
    #[cfg(target_os = "windows")]
    {
        cmd_no_window("explorer")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open url with explorer: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        cmd_no_window("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open url with open: {}", e))?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        cmd_no_window("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open url with xdg-open: {}", e))?;
        Ok(())
    }
}


#[command]
pub async fn open_cache_folder(app: tauri::AppHandle) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Failed to resolve cache dir: {}", e))?;
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    tauri_plugin_opener::open_path(&cache_dir, None::<&str>)
        .map_err(|e| format!("Failed to open cache folder {}: {}", cache_dir.display(), e))?;
    Ok(format!("Opened cache folder: {}", cache_dir.display()))
}

#[command]
pub async fn get_system_stats(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || collect_system_stats(app))
        .await
        .map_err(|e| format!("Failed to collect system stats: {e}"))?
}

fn collect_fs_stats(app: &tauri::AppHandle) -> Result<FsStatsCache, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve data dir: {}", e))?;
    let disk_bytes = dir_size(&data_dir);

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Failed to resolve cache dir: {}", e))?;
    let cache_bytes = dir_size(&cache_dir);

    let aria2_cache = data_dir.join("aria2-session.txt");
    let aria2_cache_size = if aria2_cache.exists() {
        std::fs::metadata(&aria2_cache).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    // The live DB is settings.db (see db::get_db_path); moonhunt.db is a legacy
    // name from older builds. Track whichever exists so the reported size matches
    // what init_db actually opens.
    let settings_db = data_dir.join("settings.db");
    let legacy_db = data_dir.join("moonhunt.db");
    let db_file = if settings_db.exists() { settings_db } else { legacy_db };
    let db_size = if db_file.exists() {
        std::fs::metadata(&db_file).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let temp_dir = app
        .path()
        .temp_dir()
        .map_err(|e| format!("Failed to resolve temp dir: {}", e))?;
    let temp_bytes = dir_size(&temp_dir);

    Ok(FsStatsCache {
        last_refresh: Some(Instant::now()),
        disk_bytes,
        cache_bytes,
        temp_bytes,
        db_size,
        aria2_session_size: aria2_cache_size,
    })
}

fn format_rate(bytes_per_sec: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec.max(0.0) as u64))
}

fn disk_activity_percent(read_rate: f64, write_rate: f64) -> f64 {
    let full_scale = DISK_ACTIVITY_FULL_SCALE_MBPS * 1024.0 * 1024.0;
    let percent = (read_rate + write_rate) / full_scale * 100.0;
    (percent.min(100.0) * 10.0).round() / 10.0
}

fn collect_system_stats(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    // Keep one System instance between live polls so CPU percentages are
    // calculated from the previous sample instead of returning a fresh 0.
    let mut sys_guard = PERF_SYSTEM
        .lock()
        .map_err(|_| "Failed to lock perf stats".to_string())?;
    let already_warmed = sys_guard.is_some();
    let sys = sys_guard.get_or_insert_with(|| {
        let mut sys = System::new();
        warm_cpu_sampling(&mut sys);
        sys
    });
    sys.refresh_memory();
    if already_warmed {
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
    }

    let pid = Pid::from_u32(std::process::id());
    let process_cpu = app_process_tree_cpu_usage(sys, pid);
    sys.refresh_pids_specifics(
        &[pid],
        ProcessRefreshKind::new().with_memory().with_disk_usage(),
    );

    let (mem_bytes, disk_usage) = if let Some(proc_) = sys.process(pid) {
        (proc_.memory(), proc_.disk_usage())
    } else {
        (0u64, Default::default())
    };

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let cpu_pct = sys.global_cpu_info().cpu_usage();
    let cores = sys.cpus().len();

    // Live samples can briefly show one-off process spikes (for example while
    // scanning cache folders), so smooth the app CPU before it reaches the UI.
    let process_cpu = {
        let mut cpu_guard = PERF_PROCESS_CPU
            .lock()
            .map_err(|_| "Failed to lock perf cpu stats".to_string())?;
        let smoothed = match *cpu_guard {
            Some(prev) => prev + (process_cpu - prev) * PROCESS_CPU_SMOOTHING,
            None => process_cpu,
        };
        *cpu_guard = Some(smoothed);
        smoothed
    };

    // sysinfo reports process disk counters since the last refresh, so divide
    // by the elapsed live-poll interval to get per-second read/write rates.
    let (read_rate, write_rate) = {
        let now = Instant::now();
        let mut rate_guard = PERF_DISK_RATE
            .lock()
            .map_err(|_| "Failed to lock perf disk rate".to_string())?;
        let rates = if let Some(prev) = rate_guard.as_ref() {
            let dt = now.duration_since(prev.last_at).as_secs_f64().max(0.001);
            (
                disk_usage.read_bytes as f64 / dt,
                disk_usage.written_bytes as f64 / dt,
            )
        } else {
            (0.0, 0.0)
        };
        *rate_guard = Some(DiskRateSample { last_at: now });
        rates
    };
    let disk_percent = disk_activity_percent(read_rate, write_rate);

    // Directory/drive sizes are expensive to walk, so cache them briefly and
    // keep the live poll cheap for memory/CPU.
    let mut fs_guard = PERF_FS_CACHE
        .get_or_init(|| Mutex::new(FsStatsCache::default()))
        .lock()
        .map_err(|_| "Failed to lock perf disk stats".to_string())?;
    if fs_guard.stale() {
        *fs_guard = collect_fs_stats(&app)?;
    }
    let fs = &*fs_guard;

    Ok(json!({
        "memory": {
            "used_bytes": mem_bytes,
            "used_human": format_bytes(mem_bytes),
            "total_bytes": total_mem,
            "total_human": format_bytes(total_mem),
            "system_used_bytes": used_mem,
            "system_used_human": format_bytes(used_mem),
            "percent": if total_mem > 0 { mem_bytes as f64 / total_mem as f64 * 100.0 } else { 0.0 }
        },
        "cpu": {
            "usage_percent": (cpu_pct * 10.0).round() / 10.0,
            "process_percent": ((process_cpu / cores.max(1) as f32).min(100.0) * 10.0).round() / 10.0,
            "cores": cores
        },
        "disk": {
            "usage_percent": disk_percent,
            "read_bytes_per_sec": read_rate,
            "write_bytes_per_sec": write_rate,
            "read_human": format_rate(read_rate),
            "write_human": format_rate(write_rate),
            "total_human": format_rate(read_rate + write_rate),
            "app_data_bytes": fs.disk_bytes,
            "app_data_human": format_bytes(fs.disk_bytes),
            "db_bytes": fs.db_size,
            "db_human": format_bytes(fs.db_size),
            "aria2_session_bytes": fs.aria2_session_size,
            "aria2_session_human": format_bytes(fs.aria2_session_size)
        },
        "cache": {
            "cache_dir_bytes": fs.cache_bytes,
            "cache_dir_human": format_bytes(fs.cache_bytes),
            "temp_dir_bytes": fs.temp_bytes,
            "temp_dir_human": format_bytes(fs.temp_bytes)
        }
    }))
}

/// Persist the WebView2 hardware-acceleration flag to a tiny JSON file in the
/// app data dir, then ask the user to restart. The flag is read by main.rs
/// at process startup — it cannot be applied mid-run.
#[command]
pub async fn set_hardware_acceleration(
    _app: tauri::AppHandle,
    enabled: bool,
) -> Result<String, String> {
    let mut path = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get local app data dir".to_string())?;
    path.push("com.werlos.moonhunt");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create app dir: {}", e))?;
    path.push("webview2_flags.json");

    let payload = json!({ "disable_hardware_acceleration": !enabled });
    std::fs::write(&path, serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to serialize: {}", e))?)
        .map_err(|e| format!("Failed to write flag file: {}", e))?;
    println!("[webview2] Flag file written: {} (GPU enabled = {})", path.display(), enabled);

    Ok(format!("Restart MoonHunt for GPU changes to take effect (hardware acceleration {})", if enabled { "enabled" } else { "disabled" }))
}
