//! NoiseCMYK core library.
//!
//! Cross-platform where it can be; DXGI-based screen capture and the
//! LAN HTTP server are Windows-only and feature-gated so the rest of
//! the code still compiles on Linux/macOS for development and CI.

pub mod capture;
pub mod effects;
pub mod govee;
pub mod http;
pub mod state;

use state::AppState;

/// Tauri command surface. Each function is exposed to the React frontend
/// via `invoke('command_name', { ... })`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub mod commands {
    use super::govee::capabilities::Capabilities;
    use super::govee::device::DeviceClient;
    use super::govee::discovery;
    use super::state::AppState;
    use crate::effects::{ColorZone, Effect};
    use serde::{Deserialize, Serialize};
    use tauri::State;

    #[derive(Debug, Serialize)]
    pub struct DeviceSummary {
        pub id: String,
        pub sku: String,
        pub ip: String,
        pub friendly_name: String,
        pub online: bool,
        pub power: Option<bool>,
        pub brightness: Option<u8>,
        pub color: Option<[u8; 3]>,
        pub description: String,
    }

    #[tauri::command]
    pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceSummary>, String> {
        let guard = state.devices.read();
        let out = guard
            .devices
            .values()
            .map(|d| DeviceSummary {
                id: d.id.clone(),
                sku: d.sku.clone(),
                ip: d.ip.to_string(),
                friendly_name: d.friendly_name.clone(),
                online: true, // registry = online by definition for now
                power: d.power,
                brightness: d.brightness,
                color: d.color,
                description: d.capabilities.description.to_string(),
            })
            .collect();
        Ok(out)
    }

    #[tauri::command]
    pub async fn rescan(state: State<'_, AppState>) -> Result<usize, String> {
        let conflict_safe = state.settings.read().conflict_safe;
        let registry = discovery::scan_once(conflict_safe)
            .await
            .map_err(|e| e.to_string())?;
        let count = registry.devices.len();
        *state.devices.write() = registry;
        Ok(count)
    }

    #[tauri::command]
    pub async fn set_color(
        id: String,
        r: u8,
        g: u8,
        b: u8,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let (ip, caps) = lookup(&state, &id).map_err(|e| e.to_string())?;
        DeviceClient::connect(ip)
            .await
            .map_err(|e| e.to_string())?
            .set_color(r, g, b, &caps, None)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_brightness(
        id: String,
        pct: u8,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let (ip, caps) = lookup(&state, &id).map_err(|e| e.to_string())?;
        DeviceClient::connect(ip)
            .await
            .map_err(|e| e.to_string())?
            .set_brightness(pct, &caps)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_power(
        id: String,
        on: bool,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let (ip, caps) = lookup(&state, &id).map_err(|e| e.to_string())?;
        DeviceClient::connect(ip)
            .await
            .map_err(|e| e.to_string())?
            .set_power(on, &caps)
            .await
            .map_err(|e| e.to_string())
    }

    fn lookup(
        state: &State<'_, AppState>,
        id: &str,
    ) -> anyhow::Result<(std::net::IpAddr, Capabilities)> {
        let guard = state.devices.read();
        let dev = guard
            .devices
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("unknown device id: {id}"))?;
        Ok((dev.ip, dev.capabilities.clone()))
    }

    // Effect commands wire into the screen-sync scheduler when the
    // chosen effect is ScreenSync; for the simple effects (solid /
    // breathing / rainbow) we just record the current choice and the
    // scheduler reads it next tick.
    #[derive(Debug, Deserialize)]
    pub struct StartEffectArgs {
        pub effect: Effect,
    }

    /// Live screen-sync loop handle. Stored on `AppState` so `stop_effect`
    /// can drop it.
    static SCREEN_SYNC_LOOP: once_cell::sync::Lazy<
        parking_lot::Mutex<Option<crate::effects::scheduler::ScreenSyncLoop>>,
    > = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

    #[tauri::command]
    pub async fn start_effect(args: StartEffectArgs, state: State<'_, AppState>) -> Result<(), String> {
        use crate::capture::sampler::Rect;
        use crate::effects::scheduler::{ScreenSyncLoop, SyncTarget};

        if matches!(args.effect, Effect::ScreenSync) {
            // Build a default single full-screen zone and target every
            // discovered device. Real zone mapping comes from the UI
            // (Phase 3); for now this proves the end-to-end pipeline.
            let zones = vec![Rect { x: 0, y: 0, w: 1, h: 1 }];
            let targets: Vec<SyncTarget> = {
                let reg = state.devices.read();
                reg.devices
                    .values()
                    .map(|d| SyncTarget {
                        device_id: d.id.clone(),
                        ip: d.ip,
                        supports_colorwc: d.capabilities.color_wc,
                    })
                    .collect()
            };

            let loop_handle = ScreenSyncLoop::start(state.inner().clone(), targets, zones, 30)
                .map_err(|e| e.to_string())?;
            *SCREEN_SYNC_LOOP.lock() = Some(loop_handle);
        }
        Ok(())
    }

    #[tauri::command]
    pub async fn stop_effect() -> Result<(), String> {
        if let Some(handle) = SCREEN_SYNC_LOOP.lock().take() {
            handle.stop();
        }
        Ok(())
    }

    /// Pretend the sampler ran on a test fixture — used by the UI in dev
    /// to show what color zones would produce without needing a real DXGI
    /// capture yet.
    #[tauri::command]
    pub fn preview_sample(_zones: Vec<[u32; 4]>) -> Vec<ColorZone> {
        vec![ColorZone { r: 128, g: 64, b: 192 }]
    }
}

/// Library entry point (called by `main.rs` and by mobile entry points).
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::rescan,
            commands::set_color,
            commands::set_brightness,
            commands::set_power,
            commands::start_effect,
            commands::stop_effect,
            commands::preview_sample,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NoiseCMYK");
}