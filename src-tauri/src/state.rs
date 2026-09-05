//! Shared application state.
//!
//! Holds the device registry, settings, and effect scheduler handle so
//! Tauri commands and the background effect loop can read/write them
//! safely.

use crate::govee::DeviceRegistry;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// If true, don't fight Govee Home for UDP 4002 (slower discovery).
    pub conflict_safe: bool,
    /// If true, expose the LAN HTTP control endpoint. Off by default.
    pub http_enabled: bool,
    /// Port for the LAN HTTP server.
    pub http_port: u16,
    /// Bearer token required for HTTP control.
    pub http_token: String,
    /// Target frame rate for the screen-capture effect.
    pub capture_fps: u32,
    /// Folder for persistent settings.json.
    pub data_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            conflict_safe: false,
            http_enabled: false,
            http_port: 7878,
            http_token: generate_token(),
            capture_fps: 30,
            data_dir: default_data_dir(),
        }
    }
}

fn default_data_dir() -> PathBuf {
    // %APPDATA%/NoiseCMYK on Windows, ~/.config/noiseCYMK elsewhere.
    #[cfg(target_os = "windows")]
    {
        if let Some(roaming) = std::env::var_os("APPDATA") {
            return PathBuf::from(roaming).join("NoiseCMYK");
        }
    }
    if let Some(home) = dirs_home() {
        return home.join(".config").join("noiseCYMK");
    }
    PathBuf::from(".noiseCYMK")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn generate_token() -> String {
    // 24 random bytes → 32-char base64. Not cryptographic, just enough
    // entropy that you can't guess a neighbor's token over the LAN.
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut s = format!("{nanos:x}");
    s.push_str(&format!("{:x}", std::process::id()));
    s.truncate(32);
    s
}

#[derive(Clone)]
pub struct AppState {
    pub devices: Arc<RwLock<DeviceRegistry>>,
    pub settings: Arc<RwLock<Settings>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(DeviceRegistry::default())),
            settings: Arc::new(RwLock::new(Settings::default())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}