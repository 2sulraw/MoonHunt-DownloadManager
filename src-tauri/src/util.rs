//! Small shared helpers (process spawn, filenames, byte formatting).

use crate::state::{lock, AppState};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Manager;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Create a Command that won't open a console window (Windows only).
#[cfg(target_os = "windows")]
pub fn cmd_no_window(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Create a Command (no special handling on non-Windows).
#[cfg(not(target_os = "windows"))]
pub fn cmd_no_window(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// Format byte count as human-readable string (e.g. "1.5 GB").
pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(UNITS.len() - 1);
    let value = bytes as f64 / 1024f64.powi(i as i32);
    if i == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", value, UNITS[i])
    }
}

/// Extract a reasonable filename from a URL. Strips query strings and fragments.
pub fn extract_filename_from_url(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    let without_fragment = without_query.split('#').next().unwrap_or(without_query);
    let last = without_fragment.rsplit('/').next().unwrap_or("").trim();
    if last.is_empty() || last == "/" {
        "download".to_string()
    } else {
        last.to_string()
    }
}

/// Basic filename sanitization for Windows and cross-platform safety.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect()
}

/// Decode a Xunlei `thunder://` link into its real download URL.
///
/// Xunlei wraps the real URL as base64(`AA` + url + `ZZ`). Marker-less
/// payloads are accepted when they already decode to an http/https/ftp URL.
pub fn decode_thunder_url(url: &str) -> Option<String> {
    const PREFIX: &str = "thunder://";
    let trimmed = url.trim();
    if !trimmed
        .get(..PREFIX.len())
        .map_or(false, |p| p.eq_ignore_ascii_case(PREFIX))
    {
        return None;
    }
    let payload = trimmed.get(PREFIX.len()..)?.trim();
    if payload.is_empty() {
        return None;
    }

    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload))
        .ok()?;
    let inner = std::str::from_utf8(&decoded).ok()?;

    let real = inner
        .strip_prefix("AA")
        .and_then(|s| s.strip_suffix("ZZ"))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            if inner.starts_with("http://")
                || inner.starts_with("https://")
                || inner.starts_with("ftp://")
            {
                Some(inner)
            } else {
                None
            }
        })?;
    Some(real.to_string())
}

/// Recursively calculate the total size of a directory in bytes.
pub fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                total += dir_size(&entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// `\\?\C:\Downloads\foo.mp4` → `C:\Downloads\foo.mp4`
/// `\\?\UNC\host\share\...` → `\\host\share\...`
pub fn strip_unc_prefix(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        if let Some(unc_rest) = rest.strip_prefix("UNC\\") {
            std::path::PathBuf::from(format!(r"\\{}", unc_rest))
        } else {
            std::path::PathBuf::from(rest)
        }
    } else {
        path.to_path_buf()
    }
}

/// Build a reqwest HTTP client with no proxy configuration.
pub fn no_proxy_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Build a reqwest HTTP client using the app's configured proxy settings.
pub fn configured_http_client(app: &tauri::AppHandle) -> Result<reqwest::Client, String> {
    let settings = {
        let state = app.state::<AppState>();
        let guard = lock(&state.proxy_settings)?;
        guard.clone()
    };

    let mut builder = reqwest::Client::builder();
    if !settings.verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }

    if settings.proxy_type != "none" && !settings.proxy_host.is_empty() {
        let scheme = if settings.proxy_type == "socks5" {
            "socks5"
        } else {
            "http"
        };
        let proxy_url = format!(
            "{}://{}:{}",
            scheme, settings.proxy_host, settings.proxy_port
        );
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| format!("Invalid configured proxy URL: {}", e))?;
        let proxy = if !settings.proxy_user.is_empty() || !settings.proxy_pass.is_empty() {
            proxy.basic_auth(&settings.proxy_user, &settings.proxy_pass)
        } else {
            proxy
        };
        builder = builder.proxy(proxy);
    } else {
        builder = builder.no_proxy();
    }

    builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_format_bytes_fractional() {
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_extract_filename_from_url_simple() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip"),
            "file.zip"
        );
    }

    #[test]
    fn test_extract_filename_from_url_with_query() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip?token=abc"),
            "file.zip"
        );
    }

    #[test]
    fn test_extract_filename_from_url_with_fragment() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip#section"),
            "file.zip"
        );
    }

    #[test]
    fn test_extract_filename_from_url_no_file() {
        assert_eq!(
            extract_filename_from_url("https://example.com/"),
            "download"
        );
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file:name.txt"), "file_name.txt");
        assert_eq!(sanitize_filename("no/slash.txt"), "no_slash.txt");
        assert_eq!(sanitize_filename("clean.txt"), "clean.txt");
    }

    #[test]
    fn test_decode_thunder_url() {
        assert_eq!(
            decode_thunder_url("thunder://QUFodHRwczovL2V4YW1wbGUuY29tL2ZpbGUuemlwWlo=")
                .as_deref(),
            Some("https://example.com/file.zip")
        );
    }

    #[test]
    fn test_decode_thunder_url_ignores_http() {
        assert_eq!(decode_thunder_url("https://example.com/file.zip"), None);
    }

    #[test]
    fn test_decode_thunder_url_without_padding() {
        assert_eq!(
            decode_thunder_url("thunder://QUFodHRwczovL2V4YW1wbGUuY29tL2ZpbGUuemlwWlo")
                .as_deref(),
            Some("https://example.com/file.zip")
        );
    }

    #[test]
    fn test_decode_thunder_url_rejects_garbage_payload() {
        assert_eq!(decode_thunder_url("thunder://bm90LWEtdXJs"), None);
    }

    #[test]
    fn test_strip_unc_prefix_with_unc() {
        let path = std::path::Path::new(r"\\?\C:\Downloads\file.mp4");
        let stripped = strip_unc_prefix(path);
        assert_eq!(stripped.to_string_lossy(), r"C:\Downloads\file.mp4");
    }

    #[test]
    fn test_strip_unc_prefix_without_unc() {
        let path = std::path::Path::new(r"C:\Downloads\file.mp4");
        let stripped = strip_unc_prefix(path);
        assert_eq!(stripped.to_string_lossy(), r"C:\Downloads\file.mp4");
    }
}
