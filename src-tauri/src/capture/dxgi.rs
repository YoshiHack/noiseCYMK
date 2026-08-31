//! DXGI Desktop Duplication wrapper (Windows only).
//!
//! Captures the primary monitor at the GPU frame rate. This is the same
//! path used by OBS, NVIDIA ShadowPlay, and Discord's screen share —
//! it can see fullscreen DX12 games where GDI cannot.
//!
//! Stubbed on non-Windows; `is_available()` always returns false there.

#[cfg(target_os = "windows")]
mod imp {
    use super::super::CapturedFrame;
    use anyhow::{Context, Result};
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    pub struct DxgiCapture {
        // Real implementation would hold IDXGIOutputDuplication, etc.
        // For now, this stub compiles and returns "not yet implemented".
        _priv: (),
    }

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            // Real work: enumerate adapters, find primary output, create
            // duplication, grab a frame. See
            // https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-duplication-api
            // for the canonical example.
            Err(anyhow::anyhow!(
                "DXGI capture: real implementation lives here; \
                 see PLAN.md Phase 2 for the reference walkthrough"
            ))
            .map(|_| Self { _priv: () })
        }

        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            Err(anyhow::anyhow!("not yet implemented"))
        }

        pub fn format_supported(_f: DXGI_FORMAT_B8G8R8A8_UNORM) -> bool {
            true
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use anyhow::Result;
    use super::super::CapturedFrame;

    pub struct DxgiCapture;

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }

        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            Err(anyhow::anyhow!("DXGI is Windows-only; rebuild on Windows"))
        }
    }
}

pub use imp::DxgiCapture;

pub fn is_available() -> bool {
    cfg!(target_os = "windows")
}