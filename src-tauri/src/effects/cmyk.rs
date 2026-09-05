//! CMYK-mapped color zones — the heart of NoiseCMYK.
//!
//! The screen-sync effect pulls RGB zone averages from the sampler, but
//! we want each device to react to a *different* component of the color
//! (Cyan / Magenta / Yellow / Key-black). That's what turns "ambient
//! lighting" into "the screen as a 4-channel printer".
//!
//! Mapping (per device index in the mapping list):
//!   0 → C   (1 - R)
//!   1 → M   (1 - G)
//!   2 → Y   (1 - B)
//!   3 → K   (min(C, M, Y))
//!   4+ → K
//!
//! Each CMYK channel is reconstructed back into RGB before being sent
//! to a single-zone Govee device, so the light still takes a normal
//! `color` command — we just *compute* it from a CMYK decomposition.

use crate::effects::ColorZone;

/// CMYK color decomposition, all components in 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cmyk {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

impl Cmyk {
    /// Decompose an RGB zone into CMYK using the standard printer
    /// formula: K = 1 - max(R,G,B); the remaining channels are then
    /// normalized against (1 - K).
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        let k = 1.0 - r.max(g).max(b);
        let denom = (1.0 - k).max(f64::EPSILON);
        Self {
            c: (1.0 - r - k) / denom,
            m: (1.0 - g - k) / denom,
            y: (1.0 - b - k) / denom,
            k,
        }
        .clamp01()
    }

    /// Pull out one channel by index (0=C, 1=M, 2=Y, 3+=K).
    pub fn channel(&self, idx: usize) -> f64 {
        match idx {
            0 => self.c,
            1 => self.m,
            2 => self.y,
            _ => self.k,
        }
    }

    fn clamp01(mut self) -> Self {
        self.c = self.c.clamp(0.0, 1.0);
        self.m = self.m.clamp(0.0, 1.0);
        self.y = self.y.clamp(0.0, 1.0);
        self.k = self.k.clamp(0.0, 1.0);
        self
    }

    /// Reconstruct a single channel into the RGB color that, when
    /// printed, would deposit that much of that ink. Single-channel
    /// display is intentionally punchy so the effect reads clearly on
    /// a wall of Govee strips.
    pub fn channel_to_rgb(channel: char, amount: f64) -> ColorZone {
        let a = amount.clamp(0.0, 1.0);
        match channel {
            'C' => ColorZone { r: 0, g: ((1.0 - a) * 255.0) as u8, b: ((1.0 - a) * 255.0) as u8 },
            'M' => ColorZone { r: ((1.0 - a) * 255.0) as u8, g: 0, b: ((1.0 - a) * 255.0) as u8 },
            'Y' => ColorZone { r: ((1.0 - a) * 255.0) as u8, g: ((1.0 - a) * 255.0) as u8, b: 0 },
            // K: dimmer warm-white. Pure K would just be black on Govee,
            // so we mix with a tiny amount of warm to keep the light
            // visible against bright rooms.
            'K' => {
                let v = ((1.0 - a) * 200.0) as u8;
                ColorZone { r: v, g: (v as f64 * 0.85) as u8, b: (v as f64 * 0.7) as u8 }
            }
            _ => ColorZone { r: 0, g: 0, b: 0 },
        }
    }
}

/// Pick which CMYK channel a device at `device_index` is bound to.
pub fn channel_for_device(device_index: usize) -> char {
    match device_index % 4 {
        0 => 'C',
        1 => 'M',
        2 => 'Y',
        _ => 'K',
    }
}

/// Drive: convert a list of zones + a device index into the RGB color
/// that device should display.
///
/// `device_index` selects the channel (C/M/Y/K); `zone_index` selects
/// which screen zone to sample from.
pub fn drive(zone: ColorZone, device_index: usize, zone_index: usize) -> ColorZone {
    // For the screen-sync effect we sample the *same* zone for every
    // device and let the CMYK decomposition do the work — that's the
    // whole point. If a caller wants per-device zones they can call
    // `screen_sync::apply` instead.
    let _ = zone_index;
    let cmyk = Cmyk::from_rgb(zone.r, zone.g, zone.b);
    let channel = channel_for_device(device_index);
    Cmyk::channel_to_rgb(channel, cmyk.channel(device_index % 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_red_decomposes_correctly() {
        // R=255, G=0, B=0 → K=0, C=0, M=1, Y=1
        let cmyk = Cmyk::from_rgb(255, 0, 0);
        assert!((cmyk.k - 0.0).abs() < 0.01);
        assert!((cmyk.c - 0.0).abs() < 0.01);
        assert!(cmyk.m > 0.95);
        assert!(cmyk.y > 0.95);
    }

    #[test]
    fn pure_white_decomposes_to_k_only() {
        // R=G=B=255 → K=0, all channels ~0
        let cmyk = Cmyk::from_rgb(255, 255, 255);
        assert!((cmyk.k - 0.0).abs() < 0.01);
        assert!((cmyk.c).abs() < 0.01);
        assert!((cmyk.m).abs() < 0.01);
        assert!((cmyk.y).abs() < 0.01);
    }

    #[test]
    fn pure_black_decomposes_to_full_k() {
        let cmyk = Cmyk::from_rgb(0, 0, 0);
        assert!((cmyk.k - 1.0).abs() < 0.01);
    }

    #[test]
    fn channel_routing_is_deterministic() {
        assert_eq!(channel_for_device(0), 'C');
        assert_eq!(channel_for_device(1), 'M');
        assert_eq!(channel_for_device(2), 'Y');
        assert_eq!(channel_for_device(3), 'K');
        assert_eq!(channel_for_device(4), 'C'); // wraps
    }

    #[test]
    fn drive_pulls_different_channels_per_device() {
        // Pure red (R=255) decomposes to CMYK(0, 1, 1, 0).
        // Device 0 (C) gets channel c=0 → channel_to_rgb('C', 0) → green-ish (g=b=255).
        // Device 1 (M) gets channel m=1 → channel_to_rgb('M', 1) → black (r=g=b=0).
        // We verify the channels diverge, not the specific colour, because
        // printer-style CMYK is *subtractive* and a single saturated
        // input makes the other channels black.
        let zone = ColorZone { r: 255, g: 0, b: 0 };
        let c_device = drive(zone, 0, 0);
        let m_device = drive(zone, 1, 0);
        assert!(
            c_device.g > c_device.r,
            "C device should be green-ish (c≈0 → high g/b), got {:?}",
            c_device
        );
        assert!(
            m_device.r == 0 && m_device.g == 0 && m_device.b == 0,
            "M device should be black (m=1 → suppressed r+b), got {:?}",
            m_device
        );
    }

    #[test]
    fn drive_yields_white_to_cmyk_only() {
        // Pure white R=G=B=255 → CMYK(0, 0, 0, 0).
        // Every channel is 0, so every device should show full brightness on
        // the two channels each CMY component drives.
        let zone = ColorZone { r: 255, g: 255, b: 255 };
        let c_device = drive(zone, 0, 0); // C channel = 0 → (0, 255, 255) cyan-cyan
        let m_device = drive(zone, 1, 0); // M channel = 0 → (255, 0, 255) magenta
        assert_eq!(c_device.r, 0);
        assert!(c_device.g > 250 && c_device.b > 250);
        assert_eq!(m_device.g, 0);
        assert!(m_device.r > 250 && m_device.b > 250);
    }

    #[test]
    fn channel_to_rgb_handles_full_range() {
        let black = Cmyk::channel_to_rgb('K', 1.0);
        assert_eq!(black.r, 0);
        let white = Cmyk::channel_to_rgb('C', 0.0);
        assert!(white.r < 5);
        assert!(white.g > 250);
        assert!(white.b > 250);
    }
}