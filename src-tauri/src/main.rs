// src-tauri/src/main.rs
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    // Apply WebView2 GPU flag BEFORE Tauri starts the webview process.
    // This must happen in main() because the env var is read once when the
    // webview2 process is spawned — there's no way to change it mid-run.
    apply_webview2_flags();
    moonhunt::run()
}

/// Read the small `webview2_flags.json` file in the app data dir and set
/// the WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS env var accordingly.
fn apply_webview2_flags() {
    let Some(mut path) = dirs::data_local_dir() else {
        return;
    };
    path.push("com.werlos.moonhunt");
    path.push("webview2_flags.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else { return };
    if val.get("disable_hardware_acceleration").and_then(|v| v.as_bool()) == Some(true) {
        // Append --disable-gpu to whatever Tauri set, or set it fresh
        let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        let combined = if existing.is_empty() {
            "--disable-gpu".to_string()
        } else {
            format!("{} --disable-gpu", existing)
        };
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", combined);
        eprintln!("[webview2] Hardware acceleration disabled");
    }
}
