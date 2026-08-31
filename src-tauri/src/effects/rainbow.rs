//! Rainbow effect — hue rotation over time, identical color to all
//! devices. The "rainbow across devices" variant lives in the scheduler
//! since it depends on having multiple devices.

use super::{Effect, Rgb};

pub fn step(effect: &Effect, now_ms: u64) -> Option<Rgb> {
    match effect {
        Effect::Rainbow { period_ms } => {
            let hue = ((now_ms % *period_ms as u64) as f64 / *period_ms as f64) * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
            Some(Rgb(r, g, b))
        }
        _ => None,
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let hh = h / 60.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_rotates_through_hue() {
        let e = Effect::Rainbow { period_ms: 3600 };
        let at0 = step(&e, 0).unwrap();
        let at1 = step(&e, 900).unwrap(); // ~90° hue
        // 0° hue → red-ish; 90° → green-ish. The red channel should drop.
        assert!(at1.0 < at0.0);
    }

    #[test]
    fn non_rainbow_returns_none() {
        assert_eq!(
            step(&Effect::Solid { color: Rgb(1, 2, 3) }, 0),
            None
        );
    }
}