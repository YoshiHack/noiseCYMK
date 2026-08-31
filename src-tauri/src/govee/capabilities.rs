//! Per-SKU capability map.
//!
//! Govee's LAN protocol exposes a fixed surface (color, brightness, on/off)
//! but individual SKUs differ in *which* commands actually work and how
//! reliably. This module centralizes those quirks so the device-control
//! layer doesn't have to special-case each SKU.

use serde::{Deserialize, Serialize};

/// What a given Govee SKU is known to support on the LAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// SKU model number, e.g. "H6046".
    pub sku: String,
    /// Whether `cmd=turn` (0/1) is accepted.
    pub power: bool,
    /// Whether `cmd=brightness` is accepted (0..=100).
    pub brightness: bool,
    /// Whether `cmd=color` (raw RGB) is accepted.
    pub color_rgb: bool,
    /// Whether `cmd=colorwc` (RGB + Kelvin) is accepted.
    pub color_wc: bool,
    /// Human description (e.g. "RGBIC LED light bars").
    pub description: &'static str,
    /// Whether the device supports segmented (multi-zone) effects on the LAN.
    /// As of this writing, Govee's LAN API does NOT expose per-segment color
    /// for RGBIC devices; we expose this as a documented limitation rather
    /// than a workaround.
    pub segments_via_lan: bool,
}

impl Capabilities {
    /// Look up known capabilities by SKU. Falls back to a conservative
    /// default ("color_rgb + brightness + power only") for any SKU we
    /// haven't characterized yet — that way discovery still works on
    /// devices we don't recognize, and missing features fail safely.
    pub fn for_sku(sku: &str) -> Self {
        match sku {
            // RGBIC LED light bars (pair)
            "H6046" => Self {
                sku: sku.into(),
                power: true,
                brightness: true,
                color_rgb: true,
                color_wc: false,
                description: "Govee RGBIC LED Light Bars",
                segments_via_lan: false,
            },
            // RGBIC LED strip light
            "H610A" => Self {
                sku: sku.into(),
                power: true,
                brightness: true,
                color_rgb: true,
                color_wc: true,
                description: "Govee RGBIC LED Strip",
                segments_via_lan: false,
            },
            // TV Backlight 3 Lite (camera + RGBIC strip)
            "H6609" => Self {
                sku: sku.into(),
                power: true,
                brightness: true,
                color_rgb: true,
                color_wc: true,
                description: "Govee TV Backlight 3 Lite",
                segments_via_lan: false,
            },
            _ => Self {
                sku: sku.into(),
                power: true,
                brightness: true,
                color_rgb: true,
                color_wc: false,
                description: "Unknown Govee SKU (conservative defaults)",
                segments_via_lan: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_skus_resolve() {
        let c = Capabilities::for_sku("H6046");
        assert!(c.color_rgb);
        assert!(!c.color_wc);
        assert!(!c.segments_via_lan);
    }

    #[test]
    fn unknown_sku_falls_back_safely() {
        let c = Capabilities::for_sku("H9999");
        // Conservative defaults: basic stuff yes, fancier features no.
        assert!(c.power);
        assert!(c.brightness);
        assert!(c.color_rgb);
        assert!(!c.color_wc);
        assert!(!c.segments_via_lan);
    }

    #[test]
    fn h610a_supports_colorwc() {
        let c = Capabilities::for_sku("H610A");
        assert!(c.color_wc);
    }
}