//! Cooperative cancellation for long-running binary/runtime installs.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

fn registry() -> &'static Mutex<HashSet<String>> {
    static REGISTRY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn request_cancel(key: &str) {
    if let Ok(mut set) = registry().lock() {
        set.insert(key.to_string());
    }
}

pub fn is_cancelled(key: &str) -> bool {
    registry()
        .lock()
        .map(|set| set.contains(key))
        .unwrap_or(false)
}

pub fn clear(key: &str) {
    if let Ok(mut set) = registry().lock() {
        set.remove(key);
    }
}
