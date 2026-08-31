//! Effect engine: maps abstract effects to per-frame color decisions
//! for a set of devices.
//!
//! Cross-platform. DXGI-driven screen sync lives in `capture::sampler`,
//! which produces a `Vec<ColorZone>` that `screen_sync.rs` consumes.

pub mod rainbow;
pub mod screen_sync;
pub mod solid;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    Solid { color: Rgb },
    Breathing { color: Rgb, period_ms: u32 },
    Rainbow { period_ms: u32 },
    ScreenSync,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorZone {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}