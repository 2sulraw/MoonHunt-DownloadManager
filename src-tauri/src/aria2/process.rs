//! aria2c process lifecycle: resolve binary, kill, spawn with settings.

use crate::state::{set_current_rpc_port, ProxySettings, ARIA2_RPC_TOKEN};
use crate::util::cmd_no_window;
use std::fs;
use tauri::Manager;

/// Kill existing aria2c processes so a fresh instance can bind the RPC port.
pub fn kill_aria2c() {
    #[cfg(target_os = "windows")]
    {
        let _ = cmd_no_window("taskkill")
            .args(["/IM", "aria2c.exe", "/F"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-f", "aria2c"])
            .output();
    }
}

fn resolve_aria2c_bin() -> Result<std::path::PathBuf, String> {
    // Tauri bundles the sidecar under the target-triple name; dev builds use a
    // plain binary. Try both next to the app exe, then the dev binaries dir.
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Cannot find current exe: {}", e))?
        .parent()
        .ok_or("Cannot get exe parent dir")?
        .to_path_buf();
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Platform-specific candidates: bundled sidecar name, plain name, dev dir
    #[cfg(target_os = "windows")]
    let candidates = [
        exe_dir.join("aria2c-x86_64-pc-windows-msvc.exe"),
        exe_dir.join("aria2c.exe"),
        manifest_dir.join("binaries").join("aria2c.exe"),
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        exe_dir.join("aria2c-x86_64-apple-darwin"),
        exe_dir.join("aria2c"),
        manifest_dir.join("binaries").join("aria2c"),
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        exe_dir.join("aria2c-x86_64-unknown-linux-gnu"),
        exe_dir.join("aria2c"),
        manifest_dir.join("binaries").join("aria2c-x86_64-unknown-linux-gnu"),
        std::path::PathBuf::from("/usr/lib/moonhunt/aria2c"),
    ];

    let found = candidates
        .into_iter()
        .find(|p| p.exists());

    // On Linux, also fall back to system PATH if no bundled binary was found.
    #[cfg(target_os = "linux")]
    let found = found.or_else(|| {
        std::process::Command::new("which")
            .arg("aria2c")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !path.is_empty() {
                        let p = std::path::PathBuf::from(path);
                        if p.exists() { return Some(p); }
                    }
                }
                None
            })
    });

    found.ok_or_else(|| "aria2c binary not found".to_string())
}

fn build_aria2_args(
    settings: &ProxySettings,
    session_path: &str,
) -> Vec<String> {
    let mut args = vec![
        "--enable-rpc".to_string(),
        format!("--rpc-listen-port={}", settings.rpc_port),
        "--rpc-allow-origin-all=true".to_string(),
        format!("--max-concurrent-downloads={}", settings.max_concurrent),
        format!("--dir={}", settings.download_location),
        // Session is saved (for aria2's in-place resume) but never re-loaded via
        // --input-file. Re-importing the session file resurrects transfers that
        // ALREADY COMPLETED as fresh `active` tasks with the same gid, which then
        // collide with the completed row in the DB and make finished downloads
        // appear "active" again after a relaunch. In-flight resumes still work
        // through aria2's --continue + the frontend resume action.
        "--save-session".to_string(),
        session_path.to_string(),
        "--save-session-interval=30".to_string(),
        "--log-level=notice".to_string(),
        "--console-log-level=notice".to_string(),
        "--continue=true".to_string(),
    ];

    // Wire the "Verify Certificates" toggle to aria2c.
    // When disabled, skip the OCSP/CRL revocation check that fails instantly
    // on unstable connections (SChannel error 80092013).
    if !settings.verify_ssl {
        args.push("--check-certificate=false".to_string());
    }

    // aria2 rejects `--rpc-secret=` (empty string) and exits — only pass the
    // flag when a non-empty token is configured.
    if !ARIA2_RPC_TOKEN.is_empty() {
        args.push(format!("--rpc-secret={}", ARIA2_RPC_TOKEN));
    }

    let global_speed_limit = settings.global_max_download_speed();
    if global_speed_limit > 0 {
        args.push(format!(
            "--max-download-limit={}",
            global_speed_limit * 1024
        ));
    } else {
        args.push("--max-download-limit=0".to_string());
    }

    // NOTE: aria2c does NOT support socks5:// in --all-proxy — it crashes on startup.
    // For SOCKS5 proxies, skip passing proxy to aria2 entirely; downloads will use
    // the reqwest fallback which handles SOCKS5 natively via the system proxy.
    // Only pass HTTP proxies to aria2.
    if settings.proxy_type == "http" && !settings.proxy_host.is_empty() {
        args.push(format!(
            "--all-proxy=http://{}:{}",
            settings.proxy_host, settings.proxy_port
        ));
    }

    args
}

/// Spawn aria2c with the given settings. Optionally kill existing processes first.
pub fn spawn_aria2(
    settings: &ProxySettings,
    app: &tauri::AppHandle,
    kill_existing: bool,
) -> Result<String, String> {
    if kill_existing {
        kill_aria2c();
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?;

    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data directory: {}", e))?;

    let session_file = data_dir.join("aria2-session.txt");
    let session_path = session_file
        .to_str()
        .ok_or("Invalid session file path")?
        .replace('\\', "/");

    set_current_rpc_port(settings.rpc_port);

    let args = build_aria2_args(settings, &session_path);

    println!(
        "[*] Starting aria2c: max_speed={} KB/s, dir={}, concurrent={}",
        settings.max_download_speed, settings.download_location, settings.max_concurrent
    );

    let bin_path = resolve_aria2c_bin()?;
    println!("[*] Spawning aria2c from: {}", bin_path.display());

    let mut cmd = cmd_no_window(&bin_path.to_string_lossy());
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn aria2c: {}", e))?;

    println!("[✓] Aria2 started! PID: {:?}", child.id());
    drop(child);
    Ok("success".to_string())
}
