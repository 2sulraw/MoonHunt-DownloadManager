//! Addon host: install/list/uninstall/enable addons plus generic host
//! commands addons use (`addon_fetch_url`, `addon_yt_search`).
//!
//! Addons run in-process: a zip carrying `manifest.json` + a single ESM
//! `addon.js` entry. The frontend dynamic-imports the entry with full
//! `invoke` access; the backend only validates, stores and exposes data.

use crate::util::cmd_no_window;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const MAX_ADDON_ZIP_BYTES: u64 = 50 * 1024 * 1024;
const MAX_FETCH_BYTES: usize = 2 * 1024 * 1024;
const ADDON_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    pub entry: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub url_patterns: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub min_app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonSource {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledAddon {
    pub manifest: AddonManifest,
    pub enabled: bool,
    pub source: AddonSource,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub installed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddonRegistry {
    #[serde(default)]
    addons: Vec<InstalledAddon>,
    /// Addon ids already offered as first-run seed (never re-seed after removal).
    #[serde(default)]
    seeded: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtSearchResult {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub uploader: Option<String>,
    pub webpage_url: String,
}

fn addons_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?
        .join("addons");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create addons dir: {}", e))?;
    Ok(dir)
}

fn registry_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(addons_dir(app)?.join("registry.json"))
}

fn read_registry(app: &tauri::AppHandle) -> AddonRegistry {
    let path = match registry_path(app) {
        Ok(p) => p,
        Err(_) => return AddonRegistry { addons: vec![], seeded: vec![] },
    };
    let text = fs::read_to_string(path).unwrap_or_default();
    if text.trim().is_empty() {
        return AddonRegistry { addons: vec![], seeded: vec![] };
    }
    serde_json::from_str(&text).unwrap_or(AddonRegistry { addons: vec![], seeded: vec![] })
}

fn write_registry(app: &tauri::AppHandle, registry: &AddonRegistry) -> Result<(), String> {
    let path = registry_path(app)?;
    let text =
        serde_json::to_string_pretty(registry).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write registry: {}", e))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn validate_manifest(m: &AddonManifest) -> Result<(), String> {
    let id_re = Regex::new(r"^[a-z0-9][a-z0-9-]{1,63}$").map_err(|e| e.to_string())?;
    if !id_re.is_match(&m.id) {
        return Err(
            "Invalid addon id (lowercase letters, digits, dashes, 2-64 chars)".to_string(),
        );
    }
    if m.name.trim().is_empty() || m.name.len() > 80 {
        return Err("Addon name must be 1-80 chars".to_string());
    }
    if semver::Version::parse(&m.version).is_err() {
        return Err("Addon version must be semver (e.g. 1.0.0)".to_string());
    }
    if m.entry.trim().is_empty() || !m.entry.ends_with(".js") || m.entry.contains("..") {
        return Err("Addon entry must be a .js file inside the package".to_string());
    }
    if m.id == "general" || m.id == "builtin" {
        return Err("Addon id is reserved".to_string());
    }
    Ok(())
}

/// Extract validated zip bytes into `addons/<id>/`. Returns the manifest.
fn install_zip_bytes(
    app: &tauri::AppHandle,
    bytes: &[u8],
    source: AddonSource,
) -> Result<InstalledAddon, String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = format!("{:x}", hasher.finalize());

    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid zip: {}", e))?;

    // Manifest first.
    let manifest: AddonManifest = {
        let mut file = zip
            .by_name("manifest.json")
            .map_err(|_| "Zip must contain manifest.json at its root".to_string())?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
        serde_json::from_str(&text).map_err(|e| format!("Invalid manifest.json: {}", e))?
    };
    validate_manifest(&manifest)?;

    // Entry must exist.
    zip.by_name(&manifest.entry)
        .map_err(|_| format!("Entry file '{}' missing from zip", manifest.entry))?;

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| format!("Zip read error: {}", e))?;
        if file.is_dir() {
            continue;
        }
        let rel = match file.enclosed_name() {
            Some(p) => p.to_string_lossy().to_string(),
            None => return Err(format!("Unsafe path in zip: {}", file.name())),
        };
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("Extract error: {}", e))?;
        files.push((rel, buf));
    }
    write_addon_files(app, &manifest.id, &files)?;
    finish_install(app, manifest, source, hash)
}

/// Write validated file list into `addons/<id>/` (wipe first).
fn write_addon_files(
    app: &tauri::AppHandle,
    id: &str,
    files: &[(String, Vec<u8>)],
) -> Result<(), String> {
    let target = addons_dir(app)?.join(id);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| format!("Failed to clear old install: {}", e))?;
    }
    fs::create_dir_all(&target).map_err(|e| format!("Failed to create addon dir: {}", e))?;
    for (rel, bytes) in files {
        let rel_path = Path::new(rel);
        if rel_path.is_absolute()
            || rel_path.components().any(|c| {
                matches!(c, std::path::Component::ParentDir | std::path::Component::Prefix(_))
            })
        {
            return Err(format!("Unsafe path in package: {}", rel));
        }
        let out_path = target.join(rel_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&out_path, bytes).map_err(|e| format!("Write error: {}", e))?;
    }
    Ok(())
}

fn finish_install(
    app: &tauri::AppHandle,
    manifest: AddonManifest,
    source: AddonSource,
    hash: String,
) -> Result<InstalledAddon, String> {
    let installed = InstalledAddon {
        manifest,
        enabled: true,
        source,
        hash,
        installed_at: now_secs(),
    };
    let mut registry = read_registry(app);
    registry.addons.retain(|a| a.manifest.id != installed.manifest.id);
    registry.addons.push(installed.clone());
    write_registry(app, &registry)?;
    Ok(installed)
}

/// Open the runtime addons folder in the file manager (created on demand).
#[tauri::command]
pub fn open_addons_folder(app: tauri::AppHandle) -> Result<String, String> {
    let dir = addons_dir(&app)?;
    tauri_plugin_opener::open_path(&dir, None::<&str>)
        .map_err(|e| format!("Failed to open addons folder {}: {}", dir.display(), e))?;
    Ok(format!("Opened addons folder: {}", dir.display()))
}

#[tauri::command]
pub async fn install_addon_from_path(
    app: tauri::AppHandle,
    path: String,
) -> Result<InstalledAddon, String> {
    let meta = fs::metadata(&path).map_err(|e| format!("Cannot read file: {}", e))?;
    if meta.len() > MAX_ADDON_ZIP_BYTES {
        return Err("Addon file too large (max 50 MB)".to_string());
    }
    if !path.to_lowercase().ends_with(".zip") {
        return Err("Addon file must be a .zip package".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("Cannot read file: {}", e))?;
    install_zip_bytes(
        &app,
        &bytes,
        AddonSource {
            kind: "file".to_string(),
            detail: Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
        },
    )
}

#[tauri::command]
pub async fn install_addon_from_url(
    app: tauri::AppHandle,
    url: String,
) -> Result<InstalledAddon, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("URL must start with http(s)://".to_string());
    }
    let client = reqwest::Client::builder()
        .user_agent(ADDON_UA)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    if bytes.len() as u64 > MAX_ADDON_ZIP_BYTES {
        return Err("Addon file too large (max 50 MB)".to_string());
    }
    install_zip_bytes(
        &app,
        &bytes,
        AddonSource {
            kind: "url".to_string(),
            detail: url,
        },
    )
}

#[tauri::command]
pub fn list_addons(app: tauri::AppHandle) -> Result<Vec<InstalledAddon>, String> {
    ensure_seeded(&app)?;
    let mut registry = read_registry(&app);
    // Normalize records from the old separate official track: same shape as
    // any file-installed addon.
    let mut dirty = false;
    for a in registry.addons.iter_mut() {
        if a.source.kind == "official" {
            a.source.kind = "file".to_string();
            a.source.detail = format!("{}.zip", a.manifest.id);
            dirty = true;
        }
    }
    if dirty {
        write_registry(&app, &registry)?;
    }
    Ok(registry.addons)
}

/// Absolute file-system path of an installed addon's entry file, so the
/// frontend can dynamic-import it.
#[tauri::command]
pub fn addon_entry_path(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let registry = read_registry(&app);
    let addon = registry
        .addons
        .iter()
        .find(|a| a.manifest.id == id)
        .ok_or_else(|| "Addon not found".to_string())?;
    if !addon.enabled {
        return Err("Addon is disabled".to_string());
    }
    let path = addons_dir(&app)?
        .join(&addon.manifest.id)
        .join(&addon.manifest.entry);
    if !path.exists() {
        return Err("Addon entry file missing".to_string());
    }
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn uninstall_addon(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let mut registry = read_registry(&app);
    let pos = registry
        .addons
        .iter()
        .position(|a| a.manifest.id == id)
        .ok_or_else(|| "Addon not found".to_string())?;
    registry.addons.remove(pos);
    write_registry(&app, &registry)?;
    let dir = addons_dir(&app)?.join(&id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Failed to remove addon files: {}", e))?;
    }
    Ok(format!("Addon '{}' removed", id))
}

#[tauri::command]
pub fn set_addon_enabled(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> Result<String, String> {
    let mut registry = read_registry(&app);
    let addon = registry
        .addons
        .iter_mut()
        .find(|a| a.manifest.id == id)
        .ok_or_else(|| "Addon not found".to_string())?;
    addon.enabled = enabled;
    write_registry(&app, &registry)?;
    Ok(if enabled { "Addon enabled" } else { "Addon disabled" }.to_string())
}

/// Generic page fetch for addons (CORS-free). Returns text, capped at 2 MB.
#[tauri::command]
pub async fn addon_fetch_url(url: String) -> Result<String, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("URL must start with http(s)://".to_string());
    }
    let client = reqwest::Client::builder()
        .user_agent(ADDON_UA)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Fetch failed: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    let capped = &bytes[..bytes.len().min(MAX_FETCH_BYTES)];
    Ok(String::from_utf8_lossy(capped).to_string())
}

/// First YouTube result for a query via yt-dlp `ytsearch1:`.
#[tauri::command]
pub async fn addon_yt_search(
    app: tauri::AppHandle,
    query: String,
) -> Result<YtSearchResult, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Query cannot be empty".to_string());
    }
    let bin = crate::ytdlp::yt_dlp_bin_path(&app)?;
    let bin_str = if bin.exists() {
        bin.to_string_lossy().to_string()
    } else if cfg!(target_os = "windows") {
        "yt-dlp.exe".to_string()
    } else {
        "yt-dlp".to_string()
    };
    let target = format!("ytsearch1:{}", query);
    // Same hardening as format scans: browser UA/impersonation, player
    // clients, JS runtime, and — critically — the user's configured proxy
    // (system proxy included), or restricted networks kill the search with
    // connection errors.
    let mut cmd = cmd_no_window(&bin_str);
    {
        let mut args: Vec<String> = vec![
            "--no-config".to_string(),
            "-j".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "--socket-timeout".to_string(),
            "30".to_string(),
        ];
        crate::ytdlp::push_common_yt_flags(&mut args, &app)?;
        crate::ytdlp::push_yt_proxy_args(&mut args, &app)?;
        args.push(target);
        cmd.args(&args);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let line = err.lines().next().unwrap_or("yt-dlp search failed");
        return Err(line.to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with('{'))
        .ok_or_else(|| "No YouTube results".to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(line).map_err(|_| "Failed to parse search result".to_string())?;
    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
    if id.is_empty() {
        return Err("No YouTube results".to_string());
    }
    Ok(YtSearchResult {
        title: v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or(&query)
            .to_string(),
        duration: v.get("duration").and_then(|x| x.as_u64()),
        uploader: v
            .get("uploader")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        webpage_url: v
            .get("webpage_url")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id)),
        id,
    })
}

// ---------------------------------------------------------------------------
// First-run seed + community authoring support.
//
// There is exactly one addon concept. The reference addon is bundled only
// as raw files; on first run it is zipped in-memory and run through the
// SAME install_zip_bytes pipeline as any community zip, so the result is
// indistinguishable (same validation, same storage, same registry).
// ---------------------------------------------------------------------------

/// Current seed addon id (must match the embedded manifest).
const SEED_ID: &str = "moonhunt-spotify-playlist-bridge";
/// Previous ids of the same addon, removed once on rename.
const LEGACY_SEED_IDS: &[&str] = &["spotify-manager"];
fn seed_files() -> Vec<(&'static str, &'static str)> {
    vec![(
        "addon.js",
        include_str!("../embedded-addons/moonhunt-spotify-playlist-bridge/addon.js"),
    )]
}

fn seed_manifest() -> &'static str {
    include_str!("../embedded-addons/moonhunt-spotify-playlist-bridge/manifest.json")
}

/// Raw bundled files for the starter template (export only, never installed).
fn template_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("manifest.json", include_str!("../embedded-addons/template/manifest.json")),
        ("addon.js", include_str!("../embedded-addons/template/addon.js")),
        ("README.md", include_str!("../embedded-addons/template/README.md")),
    ]
}

fn build_seed_zip() -> Result<Vec<u8>, String> {
    let manifest: AddonManifest =
        serde_json::from_str(seed_manifest()).map_err(|e| format!("Seed manifest error: {}", e))?;
    validate_manifest(&manifest)?;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("manifest.json", opts)
            .map_err(|e| format!("Seed zip error: {}", e))?;
        use std::io::Write;
        zip.write_all(seed_manifest().as_bytes())
            .map_err(|e| format!("Seed zip error: {}", e))?;
        for (name, content) in seed_files() {
            zip.start_file(name, opts)
                .map_err(|e| format!("Seed zip error: {}", e))?;
            zip.write_all(content.as_bytes())
                .map_err(|e| format!("Seed zip error: {}", e))?;
        }
        zip.finish().map_err(|e| format!("Seed zip error: {}", e))?;
    }
    Ok(buf.into_inner())
}

/// Install the seed addon once via the standard zip pipeline. Never
/// re-installs after the user removes it. Renames are migrated once:
/// legacy installs of the same addon are removed with their files.
fn ensure_seeded(app: &tauri::AppHandle) -> Result<(), String> {
    let mut registry = read_registry(app);
    if registry.seeded.iter().any(|s| s == SEED_ID) {
        return Ok(());
    }
    registry.seeded.push(SEED_ID.to_string());
    for legacy in LEGACY_SEED_IDS {
        if let Some(pos) = registry.addons.iter().position(|a| a.manifest.id == *legacy) {
            registry.addons.remove(pos);
            let dir = addons_dir(app)?.join(legacy);
            if dir.exists() {
                let _ = fs::remove_dir_all(&dir);
            }
        }
        if !registry.seeded.iter().any(|s| s == *legacy) {
            registry.seeded.push(legacy.to_string());
        }
    }
    let already_there = registry.addons.iter().any(|a| a.manifest.id == SEED_ID);
    if !already_there {
        let bytes = build_seed_zip()?;
        // Same pipeline as a community file install; go through validation,
        // extraction and registration identically.
        install_zip_bytes(
            app,
            &bytes,
            AddonSource {
                kind: "file".to_string(),
                detail: format!("{}.zip", SEED_ID),
            },
        )?;
        registry = read_registry(app);
        if !registry.seeded.iter().any(|s| s == SEED_ID) {
            registry.seeded.push(SEED_ID.to_string());
        }
    }
    write_registry(app, &registry)?;
    Ok(())
}

const MAX_ADDON_FILES: usize = 500;

/// Install an addon from a folder (community dev loop / template testing).
#[tauri::command]
pub fn install_addon_from_dir(
    app: tauri::AppHandle,
    path: String,
) -> Result<InstalledAddon, String> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err("Path is not a folder".to_string());
    }
    let manifest_text =
        fs::read_to_string(root.join("manifest.json")).map_err(|_| "Folder must contain manifest.json".to_string())?;
    let manifest: AddonManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("Invalid manifest.json: {}", e))?;
    validate_manifest(&manifest)?;

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total: u64 = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("Read error: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Read error: {}", e))?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p
                .strip_prefix(root)
                .map_err(|_| "Unsafe path".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&p).map_err(|e| format!("Read error: {}", e))?;
            total += bytes.len() as u64;
            if total > MAX_ADDON_ZIP_BYTES {
                return Err("Addon folder too large (max 50 MB)".to_string());
            }
            files.push((rel, bytes));
            if files.len() > MAX_ADDON_FILES {
                return Err("Addon folder has too many files (max 500)".to_string());
            }
        }
    }
    if !files.iter().any(|(p, _)| p == &manifest.entry) {
        return Err(format!("Entry file '{}' missing from folder", manifest.entry));
    }
    write_addon_files(&app, &manifest.id, &files)?;
    finish_install(
        &app,
        manifest,
        AddonSource {
            kind: "folder".to_string(),
            detail: path,
        },
        String::new(),
    )
}

/// Copy the starter template into a user-chosen folder.
#[tauri::command]
pub fn export_addon_template(dir: String) -> Result<String, String> {
    let target = Path::new(&dir);
    fs::create_dir_all(target).map_err(|e| format!("Cannot create folder: {}", e))?;
    // Refuse to clobber a non-empty folder.
    let non_empty = fs::read_dir(target)
        .map(|mut it| it.next().is_some())
        .unwrap_or(false);
    if non_empty {
        return Err("Folder is not empty — pick an empty folder".to_string());
    }
    let all = template_files();
    for (rel, content) in &all {
        let out_path = target.join(rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&out_path, content.as_bytes()).map_err(|e| format!("Write error: {}", e))?;
    }
    Ok(format!("Template written to {}", dir))
}

const MAX_ENTRY_BYTES: u64 = 5 * 1024 * 1024;

/// Entry file text for in-process loading (frontend imports via Blob URL).
#[tauri::command]
pub fn addon_read_entry(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let registry = read_registry(&app);
    let addon = registry
        .addons
        .iter()
        .find(|a| a.manifest.id == id)
        .ok_or_else(|| "Addon not found".to_string())?;
    if !addon.enabled {
        return Err("Addon is disabled".to_string());
    }
    let path = addons_dir(&app)?
        .join(&addon.manifest.id)
        .join(&addon.manifest.entry);
    let meta = fs::metadata(&path).map_err(|_| "Addon entry file missing".to_string())?;
    if meta.len() > MAX_ENTRY_BYTES {
        return Err("Addon entry too large".to_string());
    }
    fs::read_to_string(&path).map_err(|_| "Addon entry file missing".to_string())
}

/// Zip an installed addon to a user-chosen folder (share / back up / re-install).
#[tauri::command]
pub fn export_addon_package(
    app: tauri::AppHandle,
    id: String,
    dir: String,
) -> Result<String, String> {
    let registry = read_registry(&app);
    let addon = registry
        .addons
        .iter()
        .find(|a| a.manifest.id == id)
        .ok_or_else(|| "Addon not found".to_string())?;
    let root = addons_dir(&app)?.join(&addon.manifest.id);
    if !root.is_dir() {
        return Err("Addon files missing".to_string());
    }
    let out_name = format!("{}-{}.zip", addon.manifest.id, addon.manifest.version);
    let out_path = Path::new(&dir).join(&out_name);

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total: u64 = 0;
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
        let entries = fs::read_dir(&d).map_err(|e| format!("Read error: {}", e))?;
        for entry in entries {
            let p = entry.map_err(|e| format!("Read error: {}", e))?.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p
                .strip_prefix(&root)
                .map_err(|_| "Unsafe path".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&p).map_err(|e| format!("Read error: {}", e))?;
            total += bytes.len() as u64;
            if total > MAX_ADDON_ZIP_BYTES {
                return Err("Addon too large to export".to_string());
            }
            files.push((rel, bytes));
            if files.len() > MAX_ADDON_FILES {
                return Err("Addon has too many files".to_string());
            }
        }
    }
    if !files.iter().any(|(p, _)| p == "manifest.json") {
        // Older installs predated manifest-on-disk: heal from the registry,
        // which always holds the validated manifest.
        let healed = serde_json::to_vec_pretty(&addon.manifest)
            .map_err(|e| format!("Addon package is corrupt: {}", e))?;
        files.push(("manifest.json".to_string(), healed));
    }

    let out_file = fs::File::create(&out_path).map_err(|e| format!("Cannot write zip: {}", e))?;
    let mut zip = zip::ZipWriter::new(out_file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    for (rel, bytes) in &files {
        use std::io::Write;
        zip.start_file(rel, opts)
            .map_err(|e| format!("Zip error: {}", e))?;
        zip.write_all(bytes)
            .map_err(|e| format!("Zip error: {}", e))?;
    }
    zip.finish().map_err(|e| format!("Zip error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}
