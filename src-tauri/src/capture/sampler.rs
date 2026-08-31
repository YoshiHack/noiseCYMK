//! Color-zone screen sampler.
//!
//! Cross-platform: takes a frame buffer (BGRA, 4 bytes per pixel) plus a
//! list of rectangular zones and produces an average color for each zone.
//! Uses gamma-correct linear-light averaging — the #1 ambient-light
//! bug is naive mean-of-sRGB which over-darkens midtones.

use crate::effects::ColorZone;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub fn average_zones(bgra: &[u8], stride: u32, height: u32, zones: &[Rect]) -> Vec<ColorZone> {
    zones
        .iter()
        .map(|r| average_zone(bgra, stride, height, *r))
        .collect()
}

fn average_zone(bgra: &[u8], stride: u32, height: u32, r: Rect) -> ColorZone {
    let x_end = (r.x + r.w).min(stride / 4);
    let y_end = (r.y + r.h).min(height);

    // Linear-light accumulators.
    let mut sum_r = 0f64;
    let mut sum_g = 0f64;
    let mut sum_b = 0f64;
    let mut n = 0u64;

    for y in r.y..y_end {
        let row_start = (y as usize) * (stride as usize);
        for x in r.x..x_end {
            let i = row_start + (x as usize) * 4;
            if i + 2 >= bgra.len() {
                break;
            }
            // BGRA byte order (DXGI default).
            let b = bgra[i] as f64;
            let g = bgra[i + 1] as f64;
            let rv = bgra[i + 2] as f64;

            // sRGB → linear.
            sum_r += srgb_to_linear(rv);
            sum_g += srgb_to_linear(g);
            sum_b += srgb_to_linear(b);
            n += 1;
        }
    }

    if n == 0 {
        return ColorZone { r: 0, g: 0, b: 0 };
    }

    let avg_r = sum_r / n as f64;
    let avg_g = sum_g / n as f64;
    let avg_b = sum_b / n as f64;

    // Linear → sRGB.
    ColorZone {
        r: linear_to_srgb(avg_r),
        g: linear_to_srgb(avg_g),
        b: linear_to_srgb(avg_b),
    }
}

fn srgb_to_linear(c: f64) -> f64 {
    let c = c / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let out = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (out * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_solid(width: u32, height: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let stride = width * 4;
        let mut buf = vec![0u8; (stride * height) as usize];
        for chunk in buf.chunks_mut(4) {
            chunk[0] = b;
            chunk[1] = g;
            chunk[2] = r;
            chunk[3] = 255;
        }
        buf
    }

    #[test]
    fn solid_red_averages_to_red() {
        let buf = fill_solid(100, 100, 255, 0, 0);
        let z = average_zones(
            &buf,
            100 * 4,
            100,
            &[Rect { x: 0, y: 0, w: 100, h: 100 }],
        );
        assert_eq!(z.len(), 1);
        // Gamma-corrected round-trip should be ~exact for solid color.
        assert!(z[0].r >= 250, "expected red ~255, got {}", z[0].r);
        assert!(z[0].g <= 5);
        assert!(z[0].b <= 5);
    }

    #[test]
    fn checkerboard_averages_to_gray() {
        // 50/50 black/white checkerboard.
        let mut buf = vec![0u8; 4 * 100 * 100];
        for y in 0..100 {
            for x in 0..100 {
                let i = (y * 100 + x) * 4;
                let on = (x + y) % 2 == 0;
                let v = if on { 255 } else { 0 };
                buf[i] = v;     // B
                buf[i + 1] = v; // G
                buf[i + 2] = v; // R
                buf[i + 3] = 255;
            }
        }
        let z = average_zones(
            &buf,
            100 * 4,
            100,
            &[Rect { x: 0, y: 0, w: 100, h: 100 }],
        );
        // Linear-light mean of 0 and 1 is 0.5, which gamma-converts back to
        // ~188, not 127 — that's the entire point of gamma-correct sampling.
        assert!(z[0].r > 170 && z[0].r < 210, "got {}", z[0].r);
    }

    #[test]
    fn empty_zone_is_black() {
        let z = average_zones(&[], 0, 0, &[Rect { x: 0, y: 0, w: 0, h: 0 }]);
        assert_eq!(z[0].r, 0);
        assert_eq!(z[0].g, 0);
        assert_eq!(z[0].b, 0);
    }
}