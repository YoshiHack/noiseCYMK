//! Screen-capture surface.
//!
//! Provides:
//! - `sampler` — gamma-correct BGRA → ColorZone averaging (cross-platform)
//! - `xcap` — xcap-based capture for Linux/macOS development
//! - `screenshots_win` — screenshots-crate-based capture for Windows
//! - `ScreenCapture` — cross-platform enum that picks the right backend
//!
//! On Linux/macOS we use `xcap`; on Windows we use `screenshots`.
//! Both crates expose a similar interface and avoid dragging the full
//! `windows` crate into our build, which keeps the project portable
//! and avoids version conflicts with Tauri's own windows-rs usage.

pub mod sampler;

#[cfg(not(target_os = "windows"))]
pub mod xcap;

#[cfg(target_os = "windows")]
pub mod screenshots_win;

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
#[allow(clippy::large_enum_variant)]
pub enum ScreenCapture {
    #[cfg(not(target_os = "windows"))]
    Xcap(xcap::XCap),
    #[cfg(target_os = "windows")]
    Screenshots(screenshots_win::ScreenshotsCapture),
}

impl ScreenCapture {
    /// Build a screen capture for the primary monitor.
    pub fn new() -> Result<Self> {
        #[cfg(not(target_os = "windows"))]
        {
            Ok(ScreenCapture::Xcap(xcap::XCap::new()?))
        }
        #[cfg(target_os = "windows")]
        {
            Ok(ScreenCapture::Screenshots(
                screenshots_win::ScreenshotsCapture::new()?,
            ))
        }
    }

    /// Block for up to ~one frame interval and return a captured BGRA
    /// frame, or a typed error on timeout / platform failure.
    pub fn grab_frame(&mut self) -> Result<CapturedFrame, CaptureError> {
        match self {
            #[cfg(not(target_os = "windows"))]
            ScreenCapture::Xcap(c) => match c.grab_frame() {
                Ok(f) => Ok(f),
                Err(xcap::XCapError::Timeout) => Err(CaptureError::Timeout),
                Err(other) => Err(CaptureError::Other(other.to_string())),
            },
            #[cfg(target_os = "windows")]
            ScreenCapture::Screenshots(c) => match c.grab_frame() {
                Ok(f) => Ok(f),
                Err(e) => Err(CaptureError::Other(e.to_string())),
            },
            #[allow(unreachable_patterns)]
            _ => Err(CaptureError::Unsupported),
        }
    }

    /// Convenience: grab a frame, average the supplied zones, return the
    /// gamma-corrected `ColorZone`s. Used by the screen-sync effect.
    pub fn grab_zones(&mut self, zones: &[Rect]) -> Result<Vec<ColorZone>, CaptureError> {
        let frame = self.grab_frame()?;
        Ok(sampler::average_zones(
            &frame.data,
            frame.stride,
            frame.height,
            zones,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_is_construction() {
        let r = Rect {
            x: 10,
            y: 20,
            w: 100,
            h: 50,
        };
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