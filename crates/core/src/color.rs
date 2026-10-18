//! WCAG 2.x relative luminance and contrast ratio.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn luminance(self) -> f64 {
        0.2126 * channel(self.0) + 0.7152 * channel(self.1) + 0.0722 * channel(self.2)
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// Linear blend toward `target` by `t` (0..=1), in sRGB space.
    pub fn mix(self, target: Rgb, t: f64) -> Rgb {
        let m = |a: u8, b: u8| {
            (a as f64 + (b as f64 - a as f64) * t)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Rgb(
            m(self.0, target.0),
            m(self.1, target.1),
            m(self.2, target.2),
        )
    }
}

/// sRGB channel to linear light. Uses the 0.04045 knee from IEC 61966-2-1
/// (WCAG 2.x text says 0.03928; the difference is invisible at 8 bits).
pub fn channel(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn contrast_luminance(l1: f64, l2: f64) -> f64 {
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    contrast_luminance(a.luminance(), b.luminance())
}

/// Smallest change to `fg` (toward black or white, whichever increases
/// contrast against `bg`) that reaches `target` ratio. None if impossible.
pub fn nearest_passing(fg: Rgb, bg: Rgb, target: f64) -> Option<Rgb> {
    if contrast(fg, bg) >= target {
        return Some(fg);
    }
    let toward = if fg.luminance() < bg.luminance() {
        Rgb(0, 0, 0)
    } else {
        Rgb(255, 255, 255)
    };
    if contrast(toward, bg) < target {
        return None;
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if contrast(fg.mix(toward, mid), bg) >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    // rounding in mix() can land a hair under; nudge until it passes
    let mut t = hi;
    loop {
        let c = fg.mix(toward, t);
        if contrast(c, bg) >= target || t >= 1.0 {
            return Some(c);
        }
        t = (t + 0.005).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        assert!((contrast(Rgb(0, 0, 0), Rgb(255, 255, 255)) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn known_pairs() {
        // #767676 on white is the classic "just passes AA" grey: 4.54:1
        let r = contrast(Rgb(0x76, 0x76, 0x76), Rgb(255, 255, 255));
        assert!((r - 4.54).abs() < 0.01, "{r}");
        // #777777 just fails
        assert!(contrast(Rgb(0x77, 0x77, 0x77), Rgb(255, 255, 255)) < 4.5);
    }

    #[test]
    fn nearest_passing_reaches_target() {
        let fg = Rgb(0xAA, 0xAA, 0xAA);
        let bg = Rgb(255, 255, 255);
        let fixed = nearest_passing(fg, bg, 4.5).unwrap();
        assert!(contrast(fixed, bg) >= 4.5);
        assert!(fixed.0 < fg.0);
    }
}
