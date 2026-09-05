//! Screen-capture surface.
//!
//! Provides:
//! - `sampler` — gamma-correct BGRA → ColorZone averaging (cross-platform)
//! - `dxgi` — DXGI Desktop Duplication wrapper (Windows only, feature-gated)
//! - `xcap` — xcap-based capture for Linux/macOS development
//! - `ScreenCapture` — cross-platform enum that picks the right backend
//!
//! The `dxgi` module is gated behind the `windows-capture` cargo feature
//! so the lib + tests can build on Windows CI without dragging the full
//! `windows` crate into the test binary. The main app binary turns the
//! feature on via `--features windows-capture`.

pub mod sampler;

#[cfg(all(target_os = "windows", feature = "windows-capture"))]
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
    #[error("DXGI capture not compiled in — rebuild with `--features windows-capture`")]
    DxgiNotCompiled,
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
/// effect. Picks the right backend at runtime based on cfg + feature flags.
#[allow(clippy::large_enum_variant)]
pub enum ScreenCapture {
    #[cfg(all(target_os = "windows", feature = "windows-capture"))]
    Dxgi(dxgi::DxgiCapture),
    #[cfg(not(target_os = "windows"))]
    Xcap(xcap::XCap),
}

impl ScreenCapture {
    /// Build a screen capture for the primary monitor. Returns
    /// `Err(CaptureError::Unsupported)` on Windows when the
    /// `windows-capture` feature is off (this should only happen in
    /// tests on Windows; the app binary turns the feature on).
    pub fn new() -> Result<Self> {
        #[cfg(all(target_os = "windows", feature = "windows-capture"))]
        {
            Ok(ScreenCapture::Dxgi(dxgi::DxgiCapture::new()?))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(ScreenCapture::Xcap(xcap::XCap::new()?))
        }
        #[cfg(all(target_os = "windows", not(feature = "windows-capture")))]
        {
            // Tests-only path: the lib was built without windows-capture,
            // so we can't construct a DXGI capture. Caller gets a clear
            // error rather than a confusing linker failure.
            Err(anyhow::anyhow!(
                "screen capture disabled: rebuild with `--features windows-capture`"
            ))
        }
    }

    /// Block for up to ~one frame interval and return a captured BGRA
    /// frame, or a typed error on timeout / platform failure.
    pub fn grab_frame(&mut self) -> Result<CapturedFrame, CaptureError> {
        match self {
            #[cfg(all(target_os = "windows", feature = "windows-capture"))]
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
            #[allow(unreachable_patterns)]
            _ => Err(CaptureError::DxgiNotCompiled),
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

    #[test]
    fn dxgi_not_compiled_error_is_actionable() {
        let e = CaptureError::DxgiNotCompiled;
        assert!(e.to_string().contains("windows-capture"));
    }
}

// Re-export Duration so callers can pick their retry interval without
// pulling in `std::time` everywhere.
pub use std::time::Duration as FrameInterval;