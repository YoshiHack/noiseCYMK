//! Solid effect: drives devices to a single static color.

use super::{Effect, Rgb};

pub fn step(effect: &Effect, _now_ms: u64) -> Option<Rgb> {
    match effect {
        Effect::Solid { color } => Some(*color),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_returns_input() {
        let e = Effect::Solid { color: Rgb(10, 20, 30) };
        assert_eq!(step(&e, 0), Some(Rgb(10, 20, 30)));
        assert_eq!(step(&e, 999_999), Some(Rgb(10, 20, 30)));
    }

    #[test]
    fn non_solid_returns_none() {
        assert_eq!(step(&Effect::ScreenSync, 0), None);
    }
}