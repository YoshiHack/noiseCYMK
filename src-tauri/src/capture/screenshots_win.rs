//! `screenshots`-based screen capture for Windows.
//!
//! We use the `screenshots` crate instead of rolling our own DXGI code
//! because (a) it handles all the version-specific DXGI API quirks,
//! (b) it avoids dragging the full `windows` crate into our build
//! (which conflicts with Tauri's own windows-rs usage), and (c) it
//! gives us the same uniform "list monitors / capture_image → buffer"
//! interface as `xcap` does on Linux/macOS.
//!
//! On Linux/macOS, see `xcap.rs` for the cross-platform dev path.

use super::CapturedFrame;
use anyhow::{anyhow, Context, Result};
use screenshots::Screen;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScreenError {
    #[error("no screens found")]
    NoScreens,
    #[error("capture failed: {0}")]
    Other(String),
}

/// Wrapper around the `screenshots` crate's primary-screen capture.
/// Cheap to construct (no GPU resources held); each `grab_frame`
/// re-encodes the screen and is therefore not free — call at your
/// target FPS, not faster.
pub struct ScreenshotsCapture {
    screen: Screen,
}

impl ScreenshotsCapture {
    pub fn new() -> Result<Self> {
        let screens = Screen::all().context("screenshots: enumerate screens")?;
        let screen = screens
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("screenshots: no screens found"))?;
        Ok(Self { screen })
    }

    /// Grab a single BGRA frame from the primary monitor.
    pub fn grab_frame(&mut self) -> Result<CapturedFrame, ScreenError> {
        let image = self
            .screen
            .capture()
            .map_err(|e| ScreenError::Other(format!("screenshots capture: {e}")))?;
        let width = image.width() as u32;
        let height = image.height() as u32;
        let rgba = image.into_raw();
        // `screenshots` returns RGBA on Windows; the sampler expects BGRA.
        // Byte-swap R↔B in 4-byte chunks.
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