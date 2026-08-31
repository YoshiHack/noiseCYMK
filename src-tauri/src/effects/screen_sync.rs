//! ScreenSync effect — pipes captured-frame color zones to devices.
//!
//! The capture loop lives in `capture::dxgi`; this module just owns the
//! scheduler glue that decides which device gets which zone.

use super::{ColorZone, Effect};
use crate::capture::sampler::Rect;

/// Maps each device to one screen zone. The `zone_index` selects into
/// the `Vec<ColorZone>` produced by the sampler.
#[derive(Debug, Clone)]
pub struct ZoneMapping {
    pub device_id: String,
    pub zone_index: usize,
}

/// Holds the current zone mapping for the screen-sync effect. Stored on
/// `AppState` so the effect loop can read it without locking the device
/// registry.
#[derive(Default)]
pub struct ScreenSyncState {
    pub mappings: Vec<ZoneMapping>,
    pub zones: Vec<Rect>,
}

pub fn is_screen_sync(effect: &Effect) -> bool {
    matches!(effect, Effect::ScreenSync)
}

pub fn apply(zones: &[ColorZone], mappings: &[ZoneMapping]) -> Vec<(String, ColorZone)> {
    mappings
        .iter()
        .filter_map(|m| {
            zones
                    .get(m.zone_index)
                    .map(|z| (m.device_id.clone(), *z))
            })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Rgb;

    #[test]
    fn applies_zones_to_devices() {
        let zones = vec![
            ColorZone { r: 255, g: 0, b: 0 },
            ColorZone { r: 0, g: 255, b: 0 },
        ];
        let mappings = vec![
            ZoneMapping {
                device_id: "A".into(),
                zone_index: 0,
            },
            ZoneMapping {
                device_id: "B".into(),
                zone_index: 1,
            },
        ];
        let out = apply(&zones, &mappings);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "A");
        assert_eq!(out[0].1.r, 255);
        assert_eq!(out[1].1.g, 255);
    }

    #[test]
    fn missing_zone_is_skipped() {
        let zones = vec![ColorZone { r: 1, g: 2, b: 3 }];
        let mappings = vec![ZoneMapping {
            device_id: "A".into(),
            zone_index: 99,
        }];
        let out = apply(&zones, &mappings);
        assert!(out.is_empty());
    }

    #[test]
    fn detects_screen_sync() {
        assert!(is_screen_sync(&Effect::ScreenSync));
        assert!(!is_screen_sync(&Effect::Solid {
            color: Rgb(1, 2, 3)
        }));
    }
}