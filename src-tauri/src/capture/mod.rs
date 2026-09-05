//! Screen-capture surface.
//!
//! Provides:
//! - `sampler` — gamma-correct BGRA → ColorZone averaging (cross-platform)
//! - `dxgi` — DXGI Desktop Duplication wrapper (Windows only)
//! - `ScreenCapture` — cross-platform enum that picks DXGI on Windows and
//!   `xcap` elsewhere, so the screen-sync effect pipeline is exercisable
//!   end-to-end on Linux/macOS development machines.

pub mod sampler;

#[cfg(target_os = "windows")]
pub mod dxgi;

#[cfg(not(target_os = "windows"))]
pub mod xcap;

use anyhow::Result;
use thiserror::Error;

use crate::effects::ColorZone;
use sampler::Rect;

/// A captured frame: BGRA, 4 bytes per pixel, padded to a 4-byte stride.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("screen capture not supported on this platform")]
    Unsupported,
    #[error("DXGI session lost — recreate capture")]
    DxgiLost,
    #[error("capture timed out — no new frame")]
    Timeout,
    #[error("capture failed: {0}")]
    Other(String),
}

impl From<anyhow::Error> for CaptureError {
    fn from(e: anyhow::Error) -> Self {
        CaptureError::Other(e.to_string())
    }
}

/// Cross-platform screen-capture entry point used by the screen-sync
/// effect. Picks the right backend at runtime based on cfg.
pub enum ScreenCapture {
    #[cfg(target_os = "windows")]
    Dxgi(dxgi::DxgiCapture),
    #[cfg(not(target_os = "windows"))]
    Xcap(xcap::XCap),
}

impl ScreenCapture {
    /// Build a screen capture for the primary monitor. Returns
    /// `Err(CaptureError::Unsupported)` on platforms that have no
    /// available backend (very unusual — every desktop OS has one).
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            Ok(ScreenCapture::Dxgi(dxgi::DxgiCapture::new()?))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(ScreenCapture::Xcap(xcap::XCap::new()?))
        }
    }

    /// Block for up to ~one frame interval and return a captured BGRA
    /// frame, or a typed error on timeout / platform failure.
    pub fn grab_frame(&mut self) -> Result<CapturedFrame, CaptureError> {
        match self {
            #[cfg(target_os = "windows")]
            ScreenCapture::Dxgi(c) => match c.grab_frame() {
                Ok(f) => Ok(f),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("timeout") {
                        Err(CaptureError::Timeout)
                    } else if msg.contains("session") || msg.contains("lost") {
                        Err(CaptureError::DxgiLost)
                    } else {
                        Err(CaptureError::Other(msg))
                    }
                }
            },
            #[cfg(not(target_os = "windows"))]
            ScreenCapture::Xcap(c) => match c.grab_frame() {
                Ok(f) => Ok(f),
                Err(xcap::XCapError::Timeout) => Err(CaptureError::Timeout),
                Err(other) => Err(CaptureError::Other(other.to_string())),
            },
        }
    }

    /// Convenience: grab a frame, average the supplied zones, return the
    /// gamma-corrected `ColorZone`s. Used by the screen-sync effect.
    pub fn grab_zones(&mut self, zones: &[Rect]) -> Result<Vec<ColorZone>, CaptureError> {
        let frame = self.grab_frame()?;
        Ok(sampler::average_zones(&frame.data, frame.stride, frame.height, zones))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_is_construction() {
        let r = Rect { x: 10, y: 20, w: 100, h: 50 };
        assert_eq!(r.x, 10);
        assert_eq!(r.h, 50);
    }

    #[test]
    fn capture_error_display_is_readable() {
        let e = CaptureError::Timeout;
        assert!(e.to_string().contains("timed out"));
    }
}

// Re-export Duration so callers can pick their retry interval without
// pulling in `std::time` everywhere.
pub use std::time::Duration as FrameInterval;