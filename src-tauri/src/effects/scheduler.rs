//! Screen-sync effect scheduler: glue between the capture pipeline and
//! the per-device LAN clients.
//!
//! Owns the background loop that pulls frames, runs the CMYK
//! decomposition, and dispatches color commands to each device at the
//! user's target FPS.

use crate::capture::{CaptureError, ScreenCapture};
use crate::capture::sampler::Rect;
use crate::effects::{cmyk, ColorZone};
use crate::govee::device::DeviceClient;
use crate::state::AppState;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// What the scheduler needs to know about a device to drive it from
/// the screen-sync loop. We cache the device's `IpAddr` and color
/// capability so the scheduler doesn't have to round-trip the registry
/// every frame.
#[derive(Debug, Clone)]
pub struct SyncTarget {
    pub device_id: String,
    pub ip: IpAddr,
    pub supports_colorwc: bool,
}

/// Per-frame work item: which zone to sample + which target gets it.
#[derive(Debug, Clone)]
pub struct SyncJob {
    pub target: SyncTarget,
    pub zone_index: usize,
}

/// Live screen-sync loop handle. Drop to stop the loop.
pub struct ScreenSyncLoop {
    stop: Arc<Notify>,
}

impl ScreenSyncLoop {
    /// Start a screen-sync loop that pulls frames, runs CMYK mix, and
    /// pushes per-device color commands at `fps` frames per second.
    ///
    /// Returns immediately. The loop runs until `stop()` is called or
    /// the process exits.
    pub fn start(
        state: AppState,
        targets: Vec<SyncTarget>,
        zones: Vec<Rect>,
        fps: u32,
    ) -> Result<Self, CaptureError> {
        let capture = ScreenCapture::new().map_err(|e| {
            log::warn!("screen capture unavailable: {e}");
            CaptureError::from(e)
        })?;
        let stop = Arc::new(Notify::new());
        let stop_clone = stop.clone();
        let frame_interval = Duration::from_millis((1000 / fps.max(1)) as u64);

        tokio::spawn(async move {
            run_loop(capture, state, targets, zones, frame_interval, stop_clone).await;
        });

        Ok(Self { stop })
    }

    pub fn stop(&self) {
        self.stop.notify_waiters();
    }
}

async fn run_loop(
    mut capture: ScreenCapture,
    state: AppState,
    targets: Vec<SyncTarget>,
    zones: Vec<Rect>,
    frame_interval: Duration,
    stop: Arc<Notify>,
) {
    let mut ticker = tokio::time::interval(frame_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = stop.notified() => {
                log::info!("screen-sync loop stopping");
                return;
            }
            _ = ticker.tick() => {
                let zone_colors = match capture.grab_zones(&zones) {
                    Ok(zs) => zs,
                    Err(CaptureError::Timeout) => continue,
                    Err(e) => {
                        log::warn!("screen capture failed: {e}");
                        // Give the GPU a moment to recover before retrying.
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };

                for (idx, target) in targets.iter().enumerate() {
                    let zone_color = zone_colors.first().copied().unwrap_or(ColorZone { r: 0, g: 0, b: 0 });
                    let rgb = cmyk::drive(zone_color, idx, 0);
                    push_color(&state, target, rgb).await;
                }
            }
        }
    }
}

async fn push_color(state: &AppState, target: &SyncTarget, rgb: ColorZone) {
    let (ip, caps) = {
        let reg = state.devices.read();
        let dev = match reg.devices.get(&target.device_id) {
            Some(d) => (d.ip, d.capabilities.clone()),
            None => return,
        };
        dev
    };
    if let Ok(client) = DeviceClient::connect(ip).await {
        let _ = client
            .set_color(rgb.r, rgb.g, rgb.b, &caps, None)
            .await;
    }
}

// Keep the existing screen_sync module's helpers (ZoneMapping /
// apply) re-exported through this module so callers don't need to
// know about the file split.
pub use super::screen_sync::{ZoneMapping as _ZoneMapping, apply as apply_mappings};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_target_debug() {
        let t = SyncTarget {
            device_id: "ABC".into(),
            ip: "192.168.1.50".parse().unwrap(),
            supports_colorwc: true,
        };
        assert_eq!(t.device_id, "ABC");
        assert!(t.supports_colorwc);
    }
}