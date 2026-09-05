//! `xcap`-based screen capture for Linux/macOS development.
//!
//! On Windows we use DXGI (see `dxgi.rs`). This module wraps the `xcap`
//! crate so we have a uniform `grab_frame() -> Result<CapturedFrame>`
//! surface for the screen-sync effect pipeline.
//!
//! `xcap` is gated to non-Windows in `Cargo.toml`, so this module only
//! compiles on those targets.

use super::CapturedFrame;
use anyhow::{anyhow, Context, Result};
use xcap::Monitor;

#[derive(Debug, Clone, thiserror::Error)]
pub enum XCapError {
    #[error("xcap timeout")]
    Timeout,
    #[error("no monitors found")]
    NoMonitors,
    #[error("xcap: {0}")]
    Other(String),
}

impl From<anyhow::Error> for XCapError {
    fn from(e: anyhow::Error) -> Self {
        XCapError::Other(e.to_string())
    }
}

/// Wrapper around `xcap` primary-monitor capture. Cheap to construct,
/// but each `grab_frame` re-encodes BGRA so it's not free — call at
/// your target FPS and not faster.
pub struct XCap {
    monitor: Monitor,
}

impl XCap {
    pub fn new() -> Result<Self> {
        let monitors = Monitor::all().context("xcap: enumerate monitors")?;
        let primary = monitors
            .into_iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .ok_or_else(|| anyhow!("xcap: no primary monitor found"))?;
        Ok(Self { monitor: primary })
    }

    /// Grab a single BGRA frame from the primary monitor.
    pub fn grab_frame(&mut self) -> Result<CapturedFrame, XCapError> {
        let image = self
            .monitor
            .capture_image()
            .map_err(|e| XCapError::Other(format!("xcap capture_image: {e}")))?;
        let width = image.width();
        let height = image.height();
        let rgba = image.into_raw();
        // xcap returns RGBA; we need BGRA for DXGI parity. Byte-swap R↔B
        // in 4-byte chunks — same byte layout the sampler already
        // understands.
        let mut bgra = rgba;
        for chunk in bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        let stride = width * 4;
        Ok(CapturedFrame {
            width,
            height,
            stride,
            data: bgra,
        })
    }
}