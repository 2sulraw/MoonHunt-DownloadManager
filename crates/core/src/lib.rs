//! # MoonHunt core
//!
//! Backend library lifted from the old Tauri `src-tauri` backend, decoupled
//! from Tauri: every function that took `tauri::AppHandle` now takes
//! [`ctx::AppHandle`], and UI events flow through an mpsc channel instead of
//! a WebView bridge. The egui frontend lives in the `app` crate.

pub mod addon;
pub mod aria2;
pub mod backend;
pub mod clipboard;
pub mod ctx;
pub mod db;
pub mod extension;
pub mod fs_ops;
pub mod hash;
pub mod install_cancel;
pub mod js_runtime;
pub mod logfile;
pub mod merger;
pub mod protocol;
pub mod spotify;
pub mod state;
pub mod update;
pub mod util;
pub mod ytdlp;

pub use ctx::{AppHandle, AppState, Event, EventSender, EV_WAKE};
pub use ctx::APP_VERSION;
