//! Clipboard listener — polls system clipboard for downloadable URLs.
//!
//! Detection rules (matching the frontend detectResource utility):
//! 1. Content must be ≤ 2048 chars.
//! 2. Split into lines, filter empty lines.
//! 3. Max 200 lines.
//! 4. Each line must start with a recognized protocol tag.
//!    Supports: http://, https://, ftp://, magnet:, ed2k://, thunder://

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tauri::command;

static LISTENER_RUNNING: AtomicBool = AtomicBool::new(false);

const MAX_CHARS: usize = 2048;
const MAX_LINES: usize = 200;

const PROTOCOL_TAGS: &[&str] = &[
    "http://", "https://", "ftp://", "magnet:", "ed2k://", "thunder://",
];

fn line_matches_protocol(line: &str) -> bool {
    let lower = line.to_lowercase();
    PROTOCOL_TAGS.iter().any(|tag| lower.starts_with(tag) && line.len() > tag.len())
}

/// Check if text contains downloadable URL(s) using protocol-based detection.
/// Returns the first valid URL if found.
pub fn detect_and_extract_url(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_CHARS {
        return None;
    }

    let lines: Vec<&str> = trimmed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() || lines.len() > MAX_LINES {
        return None;
    }

    // Every line must match a protocol
    for line in &lines {
        if !line_matches_protocol(line) {
            return None;
        }
    }

    // Return first valid URL
    lines.first().map(|s| s.to_string())
}

/// Start the clipboard polling listener (non-blocking, spawns a thread).
#[command]
pub async fn start_clipboard_listener(app: tauri::AppHandle) -> Result<String, String> {
    if LISTENER_RUNNING.load(Ordering::Relaxed) {
        return Ok("already running".to_string());
    }
    LISTENER_RUNNING.store(true, Ordering::Relaxed);

    std::thread::spawn(move || {
        let mut last_text = String::new();
        loop {
            if !LISTENER_RUNNING.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1500));

            let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                Ok(t) => t.to_string(),
                Err(_) => continue,
            };

            if text == last_text || text.trim().is_empty() {
                continue;
            }
            last_text = text.clone();

            if let Some(url) = detect_and_extract_url(&text) {
                let _ = app.emit("clipboard-url-detected", url);
            }
        }
    });

    Ok("started".to_string())
}

/// Stop the clipboard polling listener.
#[command]
pub fn read_clipboard_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .map(|t| t.to_string())
        .map_err(|e| format!("Failed to read clipboard: {}", e))
}
