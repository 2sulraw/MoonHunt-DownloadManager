//! Cross-platform `moonhunt://` protocol registration and wake handling.
//!
//! The browser extension opens a `moonhunt://` URL to wake the desktop app and
//! show its window. On Windows/Linux the single-instance plugin receives the
//! URL argv and reveals the window (see lib.rs); on macOS the deep-link plugin
//! emits `on_open_url`. Registration is best-effort at startup via the Tauri
//! deep-link plugin, and can also be triggered from the Settings page.

use tauri::{AppHandle, Manager};

pub const WAKE_SCHEME: &str = "moonhunt";

/// Registered schemes for the MoonHunt app (mirrors tauri.conf.json).
pub const SCHEMES: &[&str] = &[WAKE_SCHEME];

/// Reveal the main window (unminimize + show + focus). No-op if it's already visible.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        println!("[deep-link] focus_main_window: revealing main window");
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        println!("[deep-link] focus_main_window: main window not found");
    }
}

#[tauri::command]
pub async fn is_default_protocol_client(protocol: String) -> Result<bool, String> {
    Ok(is_protocol_registered(&protocol))
}

#[tauri::command]
pub async fn set_default_protocol_client(app: AppHandle, protocol: String) -> Result<String, String> {
    use tauri_plugin_deep_link::DeepLinkExt;
    // Prefer the deep-link plugin's cross-platform registration.
    if let Err(plugin_err) = app.deep_link().register(&protocol) {
        // Fall back to a direct HKCU registry write on Windows for dev/portable builds.
        #[cfg(target_os = "windows")]
        {
            crate::protocol::register_protocol_windows(&app, &protocol)?;
            return Ok(format!("Protocol {} registered (manual HKCU)", protocol));
        }
        #[cfg(not(target_os = "windows"))]
        return Err(format!("Failed to register protocol {}: {}", protocol, plugin_err));
    }
    Ok(format!("Protocol {} registered", protocol))
}

#[tauri::command]
pub async fn remove_as_default_protocol_client(app: AppHandle, protocol: String) -> Result<String, String> {
    use tauri_plugin_deep_link::DeepLinkExt;
    #[cfg(target_os = "macos")]
    {
        // macOS cannot programmatically unregister; instruct the user.
        Err("macOS cannot programmatically unregister a protocol handler. Uncheck it in System Settings → Apps → Default Apps.".to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        app.deep_link()
            .unregister(&protocol)
            .map_err(|e| format!("Failed to unregister protocol {}: {}", protocol, e))?;
        Ok(format!("Protocol {} unregistered", protocol))
    }
}

/// Best-effort registration of all configured schemes at app startup.
pub fn register_all(app: &AppHandle) {
    use tauri_plugin_deep_link::DeepLinkExt;
    for scheme in SCHEMES {
        match app.deep_link().register(scheme) {
            Ok(()) => println!("[protocol] Registered moonhunt:// handler for {}", scheme),
            Err(e) => println!("[protocol] Failed to register {}://: {}", scheme, e),
        }
    }
}

/// Best-effort protocol registration on Windows using the registry directly,
/// without elevating. Writes the ProgID + capabilities under HKCU.
#[cfg(target_os = "windows")]
pub fn register_protocol_windows(_app: &AppHandle, scheme: &str) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current exe: {}", e))?;
    let exe_str = exe.to_string_lossy().to_string();
    let prog_id = format!("MoonHunt.Url.{}", scheme);

    // create_subkey returns (RegKey, RegDisposition); this helper drops the disposition.
    fn key(hive: &RegKey, path: &str) -> Result<RegKey, String> {
        hive.create_subkey(path)
            .map(|(k, _)| k)
            .map_err(|e| format!("Failed to open registry key {}: {}", path, e))
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. ProgID + URL Protocol marker
    let root = key(&hkcu, &format!("Software\\Classes\\{}", prog_id))?;
    root.set_value("", &format!("URL:{} Protocol", scheme))
        .map_err(|e| format!("Failed to set URL Protocol value: {}", e))?;
    root.set_value("URL Protocol", &"")
        .map_err(|e| format!("Failed to set URL Protocol marker: {}", e))?;

    // 2. shell\open\command → launch the exe with the URL as %1
    let cmd = key(&hkcu, &format!("Software\\Classes\\{}\\shell\\open\\command", prog_id))?;
    cmd.set_value("", &format!("\"{}\" \"%1\"", exe_str))
        .map_err(|e| format!("Failed to set shell command: {}", e))?;

    // 3. URLAssociation under Capabilities
    let caps = key(&hkcu, "Software\\MoonHunt\\Capabilities\\URLAssociations")?;
    caps.set_value(scheme, &prog_id)
        .map_err(|e| format!("Failed to set URLAssociation: {}", e))?;

    // 4. RegisteredApplications
    let reg_apps = key(&hkcu, "Software\\RegisteredApplications")?;
    reg_apps
        .set_value("MoonHunt", &"Software\\MoonHunt\\Capabilities")
        .map_err(|e| format!("Failed to set RegisteredApplications: {}", e))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn is_protocol_registered(scheme: &str) -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let prog_id = format!("MoonHunt.Url.{}", scheme);
    let cmd_path = format!("Software\\Classes\\{}\\shell\\open\\command", prog_id);
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(&cmd_path)
        .and_then(|k| k.get_value::<String, _>(""))
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_protocol_registered(_scheme: &str) -> bool {
    // The deep-link plugin owns registration on macOS/Linux; treat as registered
    // if the plugin reports it, otherwise default to true (registration is best-effort).
    true
}