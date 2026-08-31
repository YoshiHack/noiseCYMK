//! Screen-capture surface.

pub mod sampler;

#[cfg(target_os = "windows")]
pub mod dxgi;

/// A captured frame: BGRA, 4 bytes per pixel.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    // No DXGI tests on Linux; sampler tests live in sampler.rs.
}