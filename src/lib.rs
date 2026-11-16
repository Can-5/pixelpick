//! PixelPick: color science for Orbit.
//!
//! Pure functions on sRGB values: parsing, formatting, conversion, palettes, matching.
//! Screen sampling (eyedropper) is a platform capability and belongs to the Orbit host —
//! this crate never claims to read pixels it cannot see.
//!
//! All math is in 8-bit sRGB space; matching uses squared Euclidean distance which is
//! fast and monotonic — fine for "nearest named color".

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PixelPickError {
    #[error("expected #RGB or #RRGGBB hex string, got {0:?}")]
    BadHex(String),
    #[error("HSL component out of range (h: 0..360, s/l: 0..100)")]
    BadHsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// "#rrggbb", lowercase.
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Parse "#rgb", "#rrggbb" (case-insensitive, optional leading '#').
pub fn parse_color(input: &str) -> Result<Rgb, PixelPickError> {
    let s = input.trim().trim_start_matches('#');
    match s.len() {
        3 => {
            let mut chars = s.chars();
            let r = expand_nibble(chars.next().unwrap())?;
            let g = expand_nibble(chars.next().unwrap())?;
            let b = expand_nibble(chars.next().unwrap())?;
            Ok(Rgb::new(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| PixelPickError::BadHex(input.into()))?;
            let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| PixelPickError::BadHex(input.into()))?;
            let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| PixelPickError::BadHex(input.into()))?;
            Ok(Rgb::new(r, g, b))
        }
        _ => Err(PixelPickError::BadHex(input.into())),
    }
}

fn expand_nibble(c: char) -> Result<u8, PixelPickError> {
    let v = c.to_digit(16).ok_or_else(|| PixelPickError::BadHex(c.to_string()))? as u8;
    Ok(v * 17)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hsl {
    /// 0..360
    pub h: f64,
    /// 0..100
    pub s: f64,
    /// 0..100
    pub l: f64,
}

pub fn rgb_to_hsl(rgb: Rgb) -> Hsl {
    let r = rgb.r as f64 / 255.0;
    let g = rgb.g as f64 / 255.0;
    let b = rgb.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return Hsl { h: 0.0, s: 0.0, l: l * 100.0 };
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    Hsl { h: h.rem_euclid(360.0), s: s * 100.0, l: l * 100.0 }
}

pub fn hsl_to_rgb(hsl: Hsl) -> Result<Rgb, PixelPickError> {
    if !(0.0..=360.0).contains(&hsl.h)
        || !(0.0..=100.0).contains(&hsl.s)
        || !(0.0..=100.0).contains(&hsl.l)
        || hsl.h.is_nan()
        || hsl.s.is_nan()
        || hsl.l.is_nan()
    {
        return Err(PixelPickError::BadHsl);
    }
    let h = hsl.h.rem_euclid(360.0); // 360° ≡ 0°
    let s = hsl.s / 100.0;
    let l = hsl.l / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Ok(Rgb::new(
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ))
}

/// A named palette, e.g. from a wallpaper or a brand kit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<Rgb>,
}

impl Palette {
    pub fn new(name: impl Into<String>, colors: Vec<Rgb>) -> Self {
        Self { name: name.into(), colors }
    }

    /// Nearest palette color by squared sRGB distance. `None` for an empty palette.
    pub fn nearest(&self, target: Rgb) -> Option<Rgb> {
        self.colors
            .iter()
            .copied()
            .min_by_key(|c| squared_distance(*c, target))
    }

    /// Number of colors in the palette.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// True when the palette holds no colors.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// The `n` nearest palette colors to `target` with their Euclidean distances,
    /// sorted by distance (ties broken by channel values for determinism).
    /// Returns every color when `n >= self.len()`. Empty palette or `n == 0` yields `[]`.
    pub fn nearest_n(&self, target: Rgb, n: usize) -> Vec<(Rgb, f64)> {
        let mut scored: Vec<(Rgb, f64)> = self
            .colors
            .iter()
            .map(|c| (*c, (squared_distance(*c, target) as f64).sqrt()))
            .collect();
        scored.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.r.cmp(&b.0.r))
                .then(a.0.g.cmp(&b.0.g))
                .then(a.0.b.cmp(&b.0.b))
        });
        scored.truncate(n);
        scored
    }
}

pub fn squared_distance(a: Rgb, b: Rgb) -> u32 {
    let dr = a.r as i32 - b.r as i32;
    let dg = a.g as i32 - b.g as i32;
    let db = a.b as i32 - b.b as i32;
    (dr * dr + dg * dg + db * db) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = parse_color("#6C8CFF").unwrap();
        assert_eq!(c.to_hex(), "#6c8cff");
    }

    #[test]
    fn short_hex_expands() {
        assert_eq!(parse_color("#f0a").unwrap(), Rgb::new(255, 0, 170));
        assert_eq!(parse_color("f00").unwrap(), Rgb::new(255, 0, 0));
    }

    #[test]
    fn bad_hex_rejected() {
        assert!(parse_color("#12").is_err());
        assert!(parse_color("#gggggg").is_err());
        assert!(parse_color("").is_err());
    }

    #[test]
    fn hsl_roundtrip_within_one_channel_step() {
        for (r, g, b) in [(108u8, 140u8, 255u8), (255, 0, 0), (0, 255, 0), (0, 0, 255), (17, 200, 90), (255, 255, 255), (0, 0, 0)] {
            let hsl = rgb_to_hsl(Rgb::new(r, g, b));
            let back = hsl_to_rgb(hsl).unwrap();
            assert!((back.r as i32 - r as i32).abs() <= 1, "r: {r} vs {}", back.r);
            assert!((back.g as i32 - g as i32).abs() <= 1);
            assert!((back.b as i32 - b as i32).abs() <= 1);
        }
    }

    #[test]
    fn pure_red_hsl() {
        let hsl = rgb_to_hsl(Rgb::new(255, 0, 0));
        assert!((hsl.h - 0.0).abs() < 1e-9 && (hsl.s - 100.0).abs() < 1e-9 && (hsl.l - 50.0).abs() < 1e-9);
    }

    #[test]
    fn hsl_rejects_out_of_range() {
        assert!(hsl_to_rgb(Hsl { h: 400.0, s: 50.0, l: 50.0 }).is_err());
        assert!(hsl_to_rgb(Hsl { h: f64::NAN, s: 50.0, l: 50.0 }).is_err());
    }

    #[test]
    fn palette_len_and_is_empty() {
        let empty = Palette::default();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        let p = Palette::new("x", vec![Rgb::new(1, 2, 3)]);
        assert_eq!(p.len(), 1);
        assert!(!p.is_empty());
    }

    #[test]
    fn nearest_n_returns_sorted_and_capped() {
        let p = Palette::new("n", vec![
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
            Rgb::new(200, 0, 0),
            Rgb::new(100, 100, 100),
        ]);
        let got = p.nearest_n(Rgb::new(90, 90, 90), 2);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, Rgb::new(100, 100, 100)); // closest
        assert!(got[0].1 <= got[1].1, "distances must be ascending");
        // n larger than palette returns everything
        assert_eq!(p.nearest_n(Rgb::new(0, 0, 0), 99).len(), 4);
        // edge cases
        assert_eq!(p.nearest_n(Rgb::new(0, 0, 0), 0).len(), 0);
        assert!(Palette::default().nearest_n(Rgb::new(0, 0, 0), 3).is_empty());
        // exact distance check: target == palette entry -> distance 0
        let exact = p.nearest_n(Rgb::new(255, 255, 255), 1);
        assert_eq!(exact[0].0, Rgb::new(255, 255, 255));
        assert_eq!(exact[0].1, 0.0);
    }

    #[test]
    fn palette_nearest_matches_itself() {
        let p = Palette::new("brand", vec![Rgb::new(0, 0, 0), Rgb::new(255, 255, 255), Rgb::new(108, 140, 255)]);
        assert_eq!(p.nearest(Rgb::new(108, 140, 255)).unwrap(), Rgb::new(108, 140, 255));
        assert_eq!(p.nearest(Rgb::new(250, 250, 250)).unwrap(), Rgb::new(255, 255, 255));
        assert_eq!(Palette::default().nearest(Rgb::new(1, 2, 3)), None);
    }
}
