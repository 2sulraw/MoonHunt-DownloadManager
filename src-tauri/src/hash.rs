//! File hash computation for download verification.

use sha2::Digest;
use std::io::Read;
use tauri::command;

/// Compute file hash using the specified algorithm.
/// Supported: md5, sha1, sha256, sha512
#[command]
pub async fn compute_file_hash(file_path: String, algorithm: String) -> Result<String, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 1024 * 1024];

    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let mut hasher = md5::Md5::new();
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha1" => {
            let mut hasher = sha1::Sha1::new();
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha256" => {
            let mut hasher = sha2::Sha256::new();
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha512" => {
            let mut hasher = sha2::Sha512::new();
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        _ => Err(format!("Unsupported: {}. Use md5, sha1, sha256, or sha512.", algorithm)),
    }
}
