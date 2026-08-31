//! DXGI Desktop Duplication wrapper (Windows only).
//!
//! Captures the primary monitor at the GPU frame rate. This is the same
//! path used by OBS, NVIDIA ShadowPlay, and Discord's screen share —
//! it can see fullscreen DX12 games where GDI cannot.
//!
//! **Status:** scaffold only. The full implementation lives in Phase 2 of
//! PLAN.md. For now, `DxgiCapture::new()` returns an error pointing the
//! caller to the Microsoft Desktop Duplication sample.
//!
//! Stubbed on non-Windows so the rest of the project still compiles and
// tests cleanly on Linux/macOS.

#[cfg(target_os = "windows")]
mod imp {
    use super::super::CapturedFrame;
    use anyhow::{anyhow, Result};

    pub struct DxgiCapture {
        _priv: (),
    }

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            Err(anyhow!(
                "DXGI Desktop Duplication: real implementation lives here; \
                 see PLAN.md Phase 2 and the Microsoft Desktop Duplication \
                 API sample at \
                 https://learn.microsoft.com/windows/win32/direct3ddxgi/desktop-duplication-api"
            ))
        }

        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            Err(anyhow!("DXGI capture not yet implemented"))
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::super::CapturedFrame;
    use anyhow::{anyhow, Result};

    pub struct DxgiCapture;

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }

        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            Err(anyhow!("DXGI is Windows-only; rebuild on Windows"))
        }
    }
}

pub use imp::DxgiCapture;

pub fn is_available() -> bool {
    cfg!(target_os = "windows")
}