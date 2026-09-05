//! Effect engine: maps abstract effects to per-frame color decisions
//! for a set of devices.
//!
//! Cross-platform. Screen capture is DXGI on Windows and `xcap` on
//! Linux/macOS — both produce a BGRA frame that the sampler averages
//! into `ColorZone`s. The screen-sync scheduler in `scheduler.rs` then
//! runs CMYK decomposition to drive each device with a different
//! channel.

pub mod cmyk;
pub mod rainbow;
pub mod scheduler;
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