//! Govee LAN module.
//!
//! Implements the local-area-network control protocol documented in the
//! Govee LAN Control Developer Guide: UDP multicast discovery on
//! `239.255.255.250` (send to port 4001, listen on port 4002) and
//! per-device control on UDP 4003.
//!
//! The protocol is unauthenticated — anyone on your Wi-Fi can address
//! your lights. That's Govee's design, not ours; see PLAN.md "Risks".

pub mod capabilities;
pub mod device;
pub mod discovery;
pub mod protocol;

use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;

/// A single Govee device as we currently know it on the LAN.
///
/// We use a separate wire-format struct (`LanDeviceDto`) so we don't have
/// to wrangle `Instant` through `serde` — `Instant` doesn't implement
/// `Serialize`/`Deserialize` (it's intentionally opaque).
#[derive(Debug, Clone, Serialize)]
pub struct LanDevice {
    /// Govee's device id (unique per device).
    pub id: String,
    /// Human-friendly name from the Govee cloud account (may be empty).
    pub friendly_name: String,
    /// Model SKU, e.g. "H6046".
    pub sku: String,
    /// Last IP we observed the device at.
    pub ip: IpAddr,
    /// Last time we heard from the device (used for TTL/expiry).
    #[serde(skip)]
    pub last_seen: std::time::Instant,
    /// Per-SKU capabilities discovered from `capabilities::for_sku`.
    pub capabilities: capabilities::Capabilities,
    /// Current cached on/off (None = unknown).
    pub power: Option<bool>,
    /// Current cached brightness (0..=100). None = unknown.
    pub brightness: Option<u8>,
    /// Current cached RGB color. None = unknown.
    pub color: Option<[u8; 3]>,
}

/// Wire-format mirror of `LanDevice` for the JSON API. Strips fields
/// that don't serialize (Instant) and adds derived ones (online, etc.).
#[derive(Debug, Clone, Serialize)]
pub struct LanDeviceDto {
    pub id: String,
    pub friendly_name: String,
    pub sku: String,
    pub ip: IpAddr,
    pub capabilities: capabilities::Capabilities,
    pub power: Option<bool>,
    pub brightness: Option<u8>,
    pub color: Option<[u8; 3]>,
    pub online: bool,
    pub last_seen_ms_ago: u64,
}

impl From<&LanDevice> for LanDeviceDto {
    fn from(d: &LanDevice) -> Self {
        let last_seen_ms_ago = d.last_seen.elapsed().as_millis() as u64;
        Self {
            id: d.id.clone(),
            friendly_name: d.friendly_name.clone(),
            sku: d.sku.clone(),
            ip: d.ip,
            capabilities: d.capabilities.clone(),
            power: d.power,
            brightness: d.brightness,
            color: d.color,
            online: last_seen_ms_ago < 60_000,
            last_seen_ms_ago,
        }
    }
}

#[derive(Default)]
pub struct DeviceRegistry {
    pub devices: HashMap<String, LanDevice>,
}