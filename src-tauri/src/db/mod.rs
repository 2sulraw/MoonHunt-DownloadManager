//! SQLite persistence: settings, download history, schedules.

use crate::aria2::Aria2Client;
use crate::state::{
    db, lock, set_current_rpc_port, AppState, DownloadPreferences, DownloadRecord,
    DownloadSchedule, ProxySettings, DB,
};
use rusqlite::Connection;
use serde_json::{json, Map};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::command;
use tauri::Manager;

/// Lowercase stem (filename without extension) for fuzzy matching.
fn file_stem_lower(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = match base.rfind('.') {
        Some(i) if i > 0 => &base[..i],
        _ => base,
    };
    stem.to_lowercase()
}

/// Sanitization-insensitive key: lowercase alphanumeric characters only.
/// Bridges yt-dlp's on-disk sanitizing vs the predicted filename
/// (e.g. `My Video!.mp4` vs `My Video_.mp4`, emoji stripped, `__` collapsed).
fn sanitized_key(name: &str) -> String {
    file_stem_lower(name)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn find_existing_path(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    for entry in std::fs::read_dir(dir).ok()?.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
                if n.eq_ignore_ascii_case(name) {
                    return Some(path);
                }
            }
        }
        if path.is_dir() {
            if let Some(found) = find_existing_path(&path, name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Fuzzy fallback: same stem (extension changed by remux, e.g. `.mkv` →
/// `.mp4`) or sanitization-insensitive match (yt-dlp rewrote the title).
/// Returns the first hit, preferring exact-stem matches.
fn find_fuzzy_path(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let want_stem = file_stem_lower(name);
    let want_key = sanitized_key(name);
    let mut key_hit: Option<PathBuf> = None;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
                if file_stem_lower(n) == want_stem {
                    return Some(path);
                }
                if key_hit.is_none() && !want_key.is_empty() && sanitized_key(n) == want_key {
                    key_hit = Some(path.clone());
                }
            }
        }
    }
    if key_hit.is_some() {
        return key_hit;
    }
    for entry in std::fs::read_dir(dir).ok()?.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_fuzzy_path(&path, name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_download_file_state(
    status: &str,
    filename: &str,
    stored_path: Option<&str>,
    download_dir: &Path,
) -> (String, Option<String>) {
    let status_lower = status.to_lowercase();
    if !matches!(status_lower.as_str(), "complete" | "stopped") {
        return ("unknown".to_string(), None);
    }

    if let Some(path) = stored_path.filter(|s| !s.trim().is_empty()) {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            return ("present".to_string(), None);
        }
    }

    let name = filename.trim();
    if name.is_empty() {
        return ("removed".to_string(), None);
    }

    // Candidate names: the recorded filename plus the basename of the
    // recorded path (they can disagree for yt-dlp rows).
    let mut candidates: Vec<&str> = vec![name];
    if let Some(path) = stored_path.filter(|s| !s.trim().is_empty()) {
        let base = path.rsplit(['/', '\\']).next().unwrap_or(path).trim();
        if !base.is_empty() && base != name {
            candidates.push(base);
        }
    }

    // Candidate directories: the configured download dir plus the recorded
    // path's parent dir (file may live where it was actually written).
    let mut dirs: Vec<PathBuf> = vec![download_dir.to_path_buf()];
    if let Some(path) = stored_path.filter(|s| !s.trim().is_empty()) {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            let parent_buf = parent.to_path_buf();
            if parent_buf != *download_dir && parent_buf.is_dir() {
                dirs.push(parent_buf);
            }
        }
    }

    for dir in &dirs {
        for cand in &candidates {
            if let Some(found) = find_existing_path(dir, cand, 4) {
                return ("moved".to_string(), Some(found.to_string_lossy().into_owned()));
            }
        }
    }
    for dir in &dirs {
        for cand in &candidates {
            if let Some(found) = find_fuzzy_path(dir, cand, 4) {
                return ("moved".to_string(), Some(found.to_string_lossy().into_owned()));
            }
        }
    }
    ("removed".to_string(), None)
}

/// Returns a per-download file-state badge for the Downloads UI.
///
/// `present` means the recorded path still exists, `moved` means the file was
/// found elsewhere under the configured download directory, and `removed`
/// means no matching file can be found.
#[command]
pub fn check_download_file_states(
    app: tauri::AppHandle,
    gids: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let download_dir = {
        let state = app.state::<AppState>();
        let settings = lock(&state.proxy_settings)?;
        let dir = settings.download_location.clone();
        if dir.trim().is_empty() {
            dirs::download_dir().unwrap_or_else(|| PathBuf::from("C:/Downloads"))
        } else {
            PathBuf::from(dir)
        }
    };

    let mut results = Vec::new();
    for gid in gids {
        let Some(mut row) = get_download_by_gid(app.clone(), gid.clone())? else {
            continue;
        };

        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let filename = row.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let stored_path = row
            .get("path")
            .and_then(|v| v.as_str());
        let (file_state, resolved_path) =
            resolve_download_file_state(status, filename, stored_path, &download_dir);

        row["fileState"] = json!(file_state);

        if let Some(new_path) = resolved_path {
            row["path"] = json!(new_path.clone());
            let record = DownloadRecord {
                gid: row.get("gid").and_then(|v| v.as_str()).unwrap_or(&gid).to_string(),
                filename: row.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                url: row.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                size: row.get("size").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                status: row.get("status").and_then(|v| v.as_str()).unwrap_or("complete").to_string(),
                progress: row.get("progress").and_then(|v| v.as_i64()).unwrap_or(100) as i32,
                added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(0),
                path: Some(new_path),
                connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as i32),
                source: row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                ytdlp_token: row.get("ytdlp_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
                proxy: row.get("proxy").and_then(|v| v.as_str()).map(|s| s.to_string()),
                size_bytes: None,
            };
            upsert_download(app.clone(), record)?;
        }

        results.push(row);
    }

    Ok(results)
}

pub fn get_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
// Get the path to the SQLite database file.
    Ok(app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("settings.db"))
}

#[command]
pub fn open_app_db_folder(app: tauri::AppHandle) -> Result<String, String> {
    let db_path = get_db_path(&app)?;
    let dir = db_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Failed to resolve DB folder".to_string())?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create DB folder {}: {}", dir.display(), e))?;
    tauri_plugin_opener::open_path(&dir, None::<&str>)
        .map_err(|e| format!("Failed to open DB folder {}: {}", dir.display(), e))?;
    Ok(format!("Opened DB folder: {}", dir.display()))
}

#[command]
pub fn list_app_db_folder(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app)?;
    let dir = db_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Failed to resolve DB folder".to_string())?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create DB folder {}: {}", dir.display(), e))?;

    let mut files = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read DB folder {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        files.push(json!({
            "name": name,
            "size": size,
            "is_db": name.ends_with(".db") || name.ends_with(".db-wal") || name.ends_with(".db-shm")
        }));
    }
    files.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Ok(json!({
        "folder": dir.to_string_lossy().into_owned(),
        "files": files,
    }))
}

pub fn init_db(app: &tauri::AppHandle) -> Result<(), String> {
// Initialize SQLite — creates tables if they dont exist.
    if DB.get().is_some() {
        return Ok(());
    }

    let db_path = get_db_path(app)?;
    // SQLite cannot create missing parent directories. On a fresh install
    // %LOCALAPPDATA%\com.werlos.moonhunt\ does not exist yet, so Connection::open
    // fails with SQLITE_CANTOPEN → setup() errors → the app panics and exits,
    // showing only a white window flash on first launch. Create the dir first.
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL for DB: {}", e))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| format!("Failed to set DB synchronous mode: {}", e))?;
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| format!("Failed to set DB busy timeout: {}", e))?;

    // Create settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    ).map_err(|e| format!("Failed to create settings table: {}", e))?;

    // Create downloads table (ensure new columns source and ytdlp_token exist for persistence)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS downloads (
            gid TEXT PRIMARY KEY,
            filename TEXT,
            url TEXT,
            size TEXT,
            status TEXT,
            progress INTEGER,
            added_at INTEGER,
            path TEXT,
            connections INTEGER,
            proxy TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create downloads table: {}", e))?;

    // Migration: add columns if missing (safe to run on every start)
    let mut col_stmt = conn.prepare("PRAGMA table_info(downloads)").map_err(|e| format!("Failed to inspect downloads table: {}", e))?;
    let cols_iter = col_stmt.query_map([], |row| row.get::<_, String>(1)).map_err(|e| format!("Failed to read table info: {}", e))?;
    let mut cols = Vec::new();
    for c in cols_iter {
        if let Ok(name) = c {
            cols.push(name);
        }
    }
    drop(col_stmt);

    if !cols.iter().any(|c| c == "connections") {
        if cols.iter().any(|c| c == "segments") {
            conn.execute("ALTER TABLE downloads RENAME COLUMN segments TO connections", [])
                .map_err(|e| format!("Failed to rename segments column to connections: {}", e))?;
            println!("[~] Renamed 'segments' column to 'connections' in downloads table");
        } else {
            conn.execute("ALTER TABLE downloads ADD COLUMN connections INTEGER", [])
                .map_err(|e| format!("Failed to add connections column: {}", e))?;
            println!("[~] Added missing column 'connections' to downloads table");
        }
    } else if cols.iter().any(|c| c == "segments") {
        conn.execute("UPDATE downloads SET connections = segments WHERE connections IS NULL", [])
            .map_err(|e| format!("Failed to backfill connections column: {}", e))?;
    }

    if !cols.iter().any(|c| c == "source") {
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN source TEXT", []);
        println!("[~] Added missing column 'source' to downloads table");
    }
    if !cols.iter().any(|c| c == "ytdlp_token") {
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN ytdlp_token TEXT", []);
        println!("[~] Added missing column 'ytdlp_token' to downloads table");
    }

    if !cols.iter().any(|c| c == "proxy") {
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN proxy TEXT", []);
        println!("[~] Added missing column 'proxy' to downloads table");
    }

    // Create schedules table for download scheduling
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schedules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            download_gid TEXT,
            url TEXT NOT NULL,
            filename TEXT NOT NULL,
            action TEXT NOT NULL,
            scheduled_time INTEGER NOT NULL,
            repeat_interval INTEGER DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        )",
        [],
    ).map_err(|e| format!("Failed to create schedules table: {}", e))?;

    // Create download_events table for per-download activity log
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            gid TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            detail TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create download_events table: {}", e))?;
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_events_gid_ts ON download_events(gid, timestamp)", []);

    // The old per-5s speed_sample recording is gone (replaced by drop
    // detection); purge the noisy rows so the log only shows real events.
    let _ = conn.execute("DELETE FROM download_events WHERE event_type = 'speed_sample'", []);

    // Migration: add size_bytes column for raw byte counts
    if !cols.iter().any(|c| c == "size_bytes") {
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN size_bytes INTEGER", []);
    }

    // Migration: persist failure reasons so the UI can show *why* a download
    // errored (yt-dlp failures otherwise leave no trace in the row).
    if !cols.iter().any(|c| c == "error_message") {
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN error_message TEXT", []);
    }

    DB.set(Mutex::new(conn)).map_err(|_| "Database already initialized".to_string())?;
    Ok(())
}

pub fn load_settings_from_db() -> Result<ProxySettings, String> {
// Load saved ProxySettings from the database.
    let conn = db()?;
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| format!("DB query error: {}", e))?;

    let result: Option<String> = stmt.query_row(["all_settings"], |row| row.get(0))
        .ok();

    match result {
        Some(json_str) => {
            serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed to deserialize settings: {}", e))
        }
        None => Ok(ProxySettings::default()),
    }
}

pub fn save_settings_to_db(settings: &ProxySettings) -> Result<(), String> {
// Persist ProxySettings to the database.
    let conn = db()?;
    let json_str = serde_json::to_string(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params!["all_settings", &json_str],
    ).map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(())
}

#[command]
pub fn get_download_preferences() -> Result<Option<DownloadPreferences>, String> {
    let conn = db()?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| format!("DB query error: {}", e))?;

    let result: Option<String> = stmt
        .query_row(["download_preferences"], |row| row.get(0))
        .ok();

    match result {
        Some(json_str) => serde_json::from_str(&json_str)
            .map(Some)
            .map_err(|e| format!("Failed to deserialize download preferences: {}", e)),
        None => Ok(None),
    }
}

#[command]
pub async fn save_download_preferences(preferences: DownloadPreferences) -> Result<String, String> {
    let conn = db()?;
    let json_str = serde_json::to_string(&preferences)
        .map_err(|e| format!("Failed to serialize download preferences: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params!["download_preferences", &json_str],
    )
    .map_err(|e| format!("Failed to save download preferences: {}", e))?;

    Ok("Download preferences saved".to_string())
}

#[command]
pub fn get_download_categories() -> Result<Option<Vec<String>>, String> {
    let conn = db()?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| format!("DB query error: {}", e))?;

    let result: Option<String> = stmt
        .query_row(["download_categories"], |row| row.get(0))
        .ok();

    match result {
        Some(json_str) => serde_json::from_str(&json_str)
            .map(Some)
            .map_err(|e| format!("Failed to deserialize download categories: {}", e)),
        None => Ok(None),
    }
}

#[command]
pub async fn save_download_categories(categories: Vec<String>) -> Result<String, String> {
    let conn = db()?;
    let json_str = serde_json::to_string(&categories)
        .map_err(|e| format!("Failed to serialize download categories: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params!["download_categories", &json_str],
    )
    .map_err(|e| format!("Failed to save download categories: {}", e))?;

    Ok("Download categories saved".to_string())
}

#[command]
pub async fn save_all_settings(settings: ProxySettings, app: tauri::AppHandle) -> Result<String, String> {
    save_settings_to_db(&settings)?;
    
    // Update app state
    {
        let state = app.state::<AppState>();
        let mut guard = lock(&state.proxy_settings)?;
        *guard = settings.clone();
    }
    
    set_current_rpc_port(settings.rpc_port);
    
    Ok("Settings saved".to_string())
}

#[command]
pub fn get_all_settings(app: tauri::AppHandle) -> Result<ProxySettings, String> {
    let state = app.state::<AppState>();
    let guard = lock(&state.proxy_settings)?;
    Ok(guard.clone())
}

// Download DB commands using rusqlite
#[command]
pub fn get_download_by_gid(_app: tauri::AppHandle, gid: String) -> Result<Option<serde_json::Value>, String> {
// Look up a download record by its aria2 GID.
    let conn = db()?;
    let mut stmt = conn.prepare("SELECT gid, filename, url, size, status, progress, added_at, path, connections, source, ytdlp_token, proxy, error_message FROM downloads WHERE gid = ?1")
        .map_err(|e| format!("Query error: {}", e))?;

    let result = stmt.query_row([&gid], |row| {
        Ok(serde_json::json!({
            "gid": row.get::<_, String>(0)?,
            "filename": row.get::<_, String>(1)?,
            "url": row.get::<_, String>(2)?,
            "size": row.get::<_, String>(3)?,
            "status": row.get::<_, String>(4)?,
            "progress": row.get::<_, i32>(5)?,
            "added_at": row.get::<_, i64>(6)?,
            "path": row.get::<_, Option<String>>(7)?,
            "connections": row.get::<_, Option<i32>>(8)?,
            "source": row.get::<_, Option<String>>(9)?,
            "ytdlp_token": row.get::<_, Option<String>>(10)?,
            "proxy": row.get::<_, Option<String>>(11)?,
                "errorMessage": row.get::<_, Option<String>>(12)?,
        }))
    }).ok();
    
    Ok(result)
}

#[command]
pub fn upsert_download(_app: tauri::AppHandle, record: DownloadRecord) -> Result<(), String> {
// Insert or update a download record in the database.
    let conn = db()?;
    
    conn.execute(
        "INSERT INTO downloads (gid, filename, url, size, status, progress, added_at, path, connections, source, ytdlp_token, proxy, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(gid) DO UPDATE SET
           filename = excluded.filename,
           url = excluded.url,
           size = excluded.size,
           status = excluded.status,
           progress = excluded.progress,
           added_at = CASE
             WHEN downloads.added_at IS NOT NULL AND downloads.added_at > 0 THEN downloads.added_at
             ELSE excluded.added_at
           END,
           path = excluded.path,
           connections = excluded.connections,
           source = excluded.source,
           ytdlp_token = excluded.ytdlp_token,
           proxy = excluded.proxy,
           size_bytes = COALESCE(excluded.size_bytes, downloads.size_bytes)",
        rusqlite::params![
            record.gid,
            record.filename,
            record.url,
            record.size,
            record.status,
            record.progress,
            record.added_at,
            record.path,
            record.connections,
            record.source,
            record.ytdlp_token,
            record.proxy,
            record.size_bytes,
        ],
    ).map_err(|e| format!("Insert error: {}", e))?;
    
    Ok(())
}

/// Store a failure reason on a download row (capped). Upserts never touch
/// this column, so progress writes can't wipe it and it only ever belongs to
/// terminal error rows.
pub fn set_download_error_message(gid: &str, message: &str) -> Result<(), String> {
    const MAX: usize = 2000;
    let trimmed = message.trim();
    let capped = if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        format!("{}…", trimmed[..MAX].trim())
    };
    let conn = db()?;
    conn.execute(
        "UPDATE downloads SET error_message = ?1 WHERE gid = ?2",
        rusqlite::params![capped, gid],
    )
    .map_err(|e| format!("Failed to store error message: {}", e))?;
    Ok(())
}

#[command]
pub fn remove_download_from_db(_app: tauri::AppHandle, gid: String) -> Result<(), String> {// Delete a download record from the database by GID.
    let conn = db()?;
    conn.execute("DELETE FROM downloads WHERE gid = ?1", rusqlite::params![gid])
        .map_err(|e| format!("Delete error: {}", e))?;
    Ok(())
}

#[command]
pub fn get_all_downloads_from_db(_app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
// Get all download records (history view).
    let conn = db()?;
    let mut stmt = conn.prepare("SELECT gid, filename, url, size, status, progress, added_at, path, connections, source, ytdlp_token, proxy, error_message FROM downloads ORDER BY added_at DESC")
        .map_err(|e| format!("Query error: {}", e))?;

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "gid": row.get::<_, String>(0)?,
            "filename": row.get::<_, String>(1)?,
            "url": row.get::<_, String>(2)?,
            "size": row.get::<_, String>(3)?,
            "status": row.get::<_, String>(4)?,
            "progress": row.get::<_, i32>(5)?,
            "added_at": row.get::<_, i64>(6)?,
            "path": row.get::<_, Option<String>>(7)?,
            "connections": row.get::<_, Option<i32>>(8)?,
            "source": row.get::<_, Option<String>>(9)?,
            "ytdlp_token": row.get::<_, Option<String>>(10)?,
            "proxy": row.get::<_, Option<String>>(11)?,
                "errorMessage": row.get::<_, Option<String>>(12)?,
        }))
    }).map_err(|e| format!("Query error: {}", e))?;
    
    for row in rows {
        if let Ok(val) = row {
            results.push(val);
        }
    }
    
    Ok(results)
}

#[command]
pub fn create_schedule(schedule: DownloadSchedule) -> Result<DownloadSchedule, String> {
// Create a new download schedule.
    let conn = db()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut stmt = conn.prepare(
        "INSERT INTO schedules (download_gid, url, filename, action, scheduled_time, repeat_interval, enabled, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    ).map_err(|e| format!("Failed to prepare insert: {}", e))?;

    let id = stmt.insert(rusqlite::params![
        schedule.download_gid,
        schedule.url,
        schedule.filename,
        schedule.action,
        schedule.scheduled_time,
        schedule.repeat_interval.unwrap_or(0),
        if schedule.enabled { 1 } else { 0 },
        now,
    ]).map_err(|e| format!("Failed to insert schedule: {}", e))?;

    Ok(DownloadSchedule {
        id: Some(id),
        download_gid: schedule.download_gid,
        url: schedule.url,
        filename: schedule.filename,
        action: schedule.action,
        scheduled_time: schedule.scheduled_time,
        repeat_interval: schedule.repeat_interval,
        enabled: schedule.enabled,
        created_at: now,
    })
}

#[command]
pub async fn execute_schedule(schedule_id: i64) -> Result<String, String> {
    // Phase 1: all synchronous DB work, then drop the connection
    //          so the MutexGuard doesn't cross an await point.
    let (action, url, filename, gid, scheduled_time, repeat_interval): (String, String, String, Option<String>, i64, Option<i64>) = {
        let conn = db()?;
        let mut stmt = conn.prepare(
            "SELECT action, url, filename, download_gid, scheduled_time, repeat_interval
             FROM schedules WHERE id = ?1 AND enabled = 1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let row = stmt.query_row([schedule_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        }).map_err(|e| format!("Schedule not found: {}", e))?;

        row
    }; // conn dropped here

    // Phase 2: async HTTP calls — no DB guard held
    let client = Aria2Client::new()?;
    match action.as_str() {
        "start" => {
            let mut options = Map::new();
            options.insert("out".to_string(), json!(filename));
            options.insert("split".to_string(), json!("1"));
            options.insert("max-connection-per-server".to_string(), json!("1"));
            options.insert("min-split-size".to_string(), json!("1M"));
            options.insert("allow-overwrite".to_string(), json!("false"));
            options.insert("auto-file-renaming".to_string(), json!("true"));
            let _ = client.add_uri(&[url], options).await;
        }
        "pause" => {
            if let Some(ref gid_val) = gid {
                let _ = client.pause(gid_val).await;
            }
        }
        "resume" => {
            if let Some(ref gid_val) = gid {
                let _ = client.unpause(gid_val).await;
            }
        }
        _ => return Err(format!("Unknown action: {}", action)),
    }

    // Phase 3: handle repeat — fresh connection, synchronous
    if let Some(repeat) = repeat_interval {
        if repeat > 0 {
            let conn = db()?;
            conn.execute(
                "UPDATE schedules SET scheduled_time = ?1 WHERE id = ?2",
                [scheduled_time + repeat, schedule_id],
            ).map_err(|e| format!("Failed to update schedule time: {}", e))?;
        }
    }

    Ok(format!("Schedule {} executed successfully", schedule_id))
}

#[command]
pub fn get_due_schedules() -> Result<Vec<DownloadSchedule>, String> {
// Find schedules whose time has passed and are enabled.
    let conn = db()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut stmt = conn.prepare(
        "SELECT id, download_gid, url, filename, action, scheduled_time, repeat_interval, enabled, created_at 
         FROM schedules WHERE enabled = 1 AND scheduled_time <= ?1 ORDER BY scheduled_time ASC"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let schedules = stmt.query_map([now], |row| {
        Ok(DownloadSchedule {
            id: row.get(0)?,
            download_gid: row.get(1)?,
            url: row.get(2)?,
            filename: row.get(3)?,
            action: row.get(4)?,
            scheduled_time: row.get(5)?,
            repeat_interval: row.get(6)?,
            enabled: row.get::<_, i32>(7)? == 1,
            created_at: row.get(8)?,
        })
    }).map_err(|e| format!("Failed to query due schedules: {}", e))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(schedules)
}

// ── Download event log ───────────────────────────────────────────

pub fn insert_download_event(gid: &str, event_type: &str, detail: &str) {
    let Ok(conn) = db() else { return };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let _ = conn.execute(
        "INSERT INTO download_events (gid, timestamp, event_type, detail) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![gid, now_ms, event_type, detail],
    );
}

pub fn get_download_events(gid: &str) -> Result<Vec<serde_json::Value>, String> {
    let conn = db()?;
    let mut stmt = conn
        .prepare("SELECT id, gid, timestamp, event_type, detail FROM download_events WHERE gid = ?1 ORDER BY timestamp ASC")
        .map_err(|e| format!("Failed to query events: {}", e))?;
    let rows = stmt
        .query_map([gid], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "gid": row.get::<_, String>(1)?,
                "timestamp": row.get::<_, i64>(2)?,
                "eventType": row.get::<_, String>(3)?,
                "detail": row.get::<_, Option<String>>(4)?,
            }))
        })
        .map_err(|e| format!("Failed to query events: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

