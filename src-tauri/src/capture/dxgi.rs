//! DXGI Desktop Duplication capture (Windows only).
//!
//! Captures the primary monitor at the GPU frame rate. This is the same
//! path used by OBS, NVIDIA ShadowPlay, and Discord's screen share —
//! it can see fullscreen DX12 games where GDI cannot.
//!
//! Implementation note: we use the `windows` crate to walk the DXGI
//! stack (factory → adapter → output → output1 → duplication). The
//! `AcquireNextFrame` / `ReleaseFrame` lifecycle is the standard
//! Desktop Duplication pattern from Microsoft's documentation.
//!
//! On non-Windows we keep a stub so the rest of the project still
//! compiles cleanly on Linux/macOS for development and CI.

#[cfg(target_os = "windows")]
mod imp {
    use super::super::CapturedFrame;
    use anyhow::{anyhow, Context, Result};
    use std::time::Duration;
    use windows::core::Interface;
    use windows::Win32::Graphics::Dxgi::{
        IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, IDXGISurface, DXGI_ERROR_WAIT_TIMEOUT,
        DXGI_OUTDUPL_FRAME_INFO,
    };
    use windows::Win32::Graphics::DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM;
    use windows::Win32::System::Com::{
        CreateDXGIDevice, IInspectable, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };

    /// A live DXGI Desktop Duplication capture of the primary monitor.
    ///
    /// `grab_frame` blocks for up to ~16ms (≈60 fps) waiting for a new
    /// frame; the underlying GPU buffer is mapped and copied into a
    /// BGRA `Vec<u8>` so callers don't need to worry about DXGI
    /// lifetimes.
    pub struct DxgiCapture {
        duplication: IDXGIOutputDuplication,
        width: u32,
        height: u32,
    }

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            unsafe {
                // COM must be initialized on this thread for DXGI.
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

                // Create a DXGIDevice from the implicit render-driver factory.
                // Using the modern CreateDXGIDevice entrypoint avoids the
                // legacy D3D10/11 dance.
                let dxgi_device: IDXGIDevice =
                    CreateDXGIDevice::<IInspectable>(None).context("CreateDXGIDevice")?;

                // Enumerate adapters → outputs → pick primary. If anything
                // fails we surface a clear error so callers can fall back to
                // GDI capture.
                let adapter = dxgi_device
                    .GetAdapter()
                    .context("DXGI device has no adapter")?;
                let outputs: Vec<IDXGIOutput1> = (0..)
                    .map_while(|i| adapter.EnumOutputs(i).ok())
                    .map(|o| o.cast::<IDXGIOutput1>().unwrap_or_else(|_| {
                        // unreachable for modern drivers but keep a sane fallback
                        panic!("output cast failed")
                    }))
                    .collect();

                if outputs.is_empty() {
                    return Err(anyhow!("no DXGI outputs found"));
                }

                // Pick the first output as primary; multi-monitor selection
                // is a future feature.
                let output = &outputs[0];
                let desc = output.GetDesc().context("GetDesc on output")?;
                let width = desc.DesktopCoordinates.right - desc.DesktopCoordinates.left;
                let height = desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top;

                let duplication: IDXGIOutputDuplication = output
                    .DuplicateOutput(&dxgi_device)
                    .context("DuplicateOutput — desktop duplication unsupported on this adapter")?;

                Ok(Self {
                    duplication,
                    width: width.max(0) as u32,
                    height: height.max(0) as u32,
                })
            }
        }

        /// Wait for and copy out the next frame. Returns a `CapturedFrame`
        /// in BGRA byte order, 4 bytes per pixel, padded to a 4-byte
        /// stride. Returns `Err` on timeout (caller should retry) and a
        /// hard error if the duplication is invalidated (session lost —
        /// caller should re-create the capture).
        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            unsafe {
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut resource = None;

                // Up to ~60ms = ~3 frames at 60fps; gives us a fast retry
                // when no new frame is ready.
                let hr = self
                    .duplication
                    .AcquireNextFrame(60, &mut info, &mut resource);

                if hr.is_err() {
                    // WAIT_TIMEOUT = DXGI still building the next frame.
                    // Other errors = session lost → bubble up.
                    if hr == DXGI_ERROR_WAIT_TIMEOUT {
                        return Err(anyhow!("DXGI frame timeout"));
                    }
                    return Err(anyhow!("AcquireNextFrame failed: {:?}", hr));
                }

                let resource = resource.ok_or_else(|| anyhow!("AcquireNextFrame returned null resource"))?;
                let surface: IDXGISurface = resource.cast::<IDXGISurface>()
                    .context("cast resource to IDXGISurface")?;

                let mut mapped = windows::Win32::Graphics::Dxgi::DXGI_MAPPED_RECT::default();
                surface.Map(&mut mapped, 1).context("Map surface")?;

                let stride = mapped.Pitch.max(0) as u32;
                let bytes = (stride * self.height) as usize;
                let mut data = vec![0u8; bytes];
                std::ptr::copy_nonoverlapping(mapped.pBits, data.as_mut_ptr(), bytes);

                surface.Unmap();
                let _ = self.duplication.ReleaseFrame();

                Ok(CapturedFrame {
                    width: self.width,
                    height: self.height,
                    stride,
                    data,
                })
            }
        }
    }

    // We drop Send/Sync guarantees here because the windows crate does not
    // blanket-impl them; single-threaded use is fine for an effects engine.
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::super::CapturedFrame;
    use anyhow::{anyhow, Result};

    /// Stub: DXGI is Windows-only. On other platforms the cross-platform
    /// `ScreenCapture` enum (in `capture/mod.rs`) uses `xcap` instead.
    pub struct DxgiCapture;

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            Err(anyhow!(
                "DXGI is Windows-only; use ScreenCapture::Xcap on this platform"
            ))
        }

        pub fn grab_frame(&mut self) -> Result<CapturedFrame> {
            Err(anyhow!("DXGI is Windows-only; use ScreenCapture::Xcap on this platform"))
        }
    }
}

pub use imp::DxgiCapture;

pub fn is_available() -> bool {
    cfg!(target_os = "windows")
}

/// Helper for tests / docs: how long should a frame wait before we treat
/// it as "no new frame". Used by `ScreenCapture::grab_frame` to map the
/// platform-specific timeout onto a uniform retry interval.
pub const FRAME_TIMEOUT: Duration = Duration::from_millis(60);