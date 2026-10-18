//! File metadata: pixel dimensions and embedded resolution (DPI).
//!
//! DPI is the main clue that artwork is meant for print or OOH: a 7200×3600
//! image tagged 300dpi is 609.6×304.8mm, i.e. a 48-sheet supplied at 10%.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct FileMeta {
    pub width_px: u32,
    pub height_px: u32,
    pub format: String,
    /// Resolution embedded in the file, if any (dots per inch).
    pub dpi: Option<f64>,
    /// Where the DPI came from: "pHYs", "JFIF", "EXIF".
    pub dpi_source: Option<String>,
}

impl FileMeta {
    /// DPI that actually tells us something. 72/96 and their @2x/@3x
    /// multiples (144, 192, 216, 288) are what Figma, Sketch and browsers
    /// stamp on screen exports, so they carry no print intent.
    pub fn meaningful_dpi(&self) -> Option<f64> {
        const SCREEN: [f64; 6] = [72.0, 96.0, 144.0, 192.0, 216.0, 288.0];
        self.dpi
            .filter(|d| *d >= 150.0 && SCREEN.iter().all(|s| (d - s).abs() > 0.5))
    }

    pub fn physical_mm(&self) -> Option<(f64, f64)> {
        self.meaningful_dpi().map(|d| {
            (
                self.width_px as f64 / d * 25.4,
                self.height_px as f64 / d * 25.4,
            )
        })
    }
}

pub fn read_dpi(bytes: &[u8]) -> Option<(f64, &'static str)> {
    png_phys(bytes)
        .map(|d| (d, "pHYs"))
        .or_else(|| exif_dpi(bytes).map(|d| (d, "EXIF")))
        .or_else(|| jfif_dpi(bytes).map(|d| (d, "JFIF")))
}

fn png_phys(b: &[u8]) -> Option<f64> {
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut i = 8;
    while i + 8 <= b.len() {
        let len = u32::from_be_bytes(b[i..i + 4].try_into().ok()?) as usize;
        let kind = &b[i + 4..i + 8];
        let data = b.get(i + 8..i + 8 + len)?;
        if kind == b"pHYs" && len == 9 {
            let ppu_x = u32::from_be_bytes(data[0..4].try_into().ok()?) as f64;
            if data[8] == 1 && ppu_x > 0.0 {
                return Some(ppu_x * 0.0254); // pixels per metre -> dpi
            }
            return None;
        }
        if kind == b"IDAT" || kind == b"IEND" {
            return None;
        }
        i += 12 + len;
    }
    None
}

fn jfif_dpi(b: &[u8]) -> Option<f64> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= b.len() && b[i] == 0xFF {
        let marker = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if marker == 0xE0 {
            let seg = b.get(i + 4..i + 2 + len)?;
            if seg.starts_with(b"JFIF\0") && seg.len() >= 12 {
                let units = seg[7];
                let x = u16::from_be_bytes([seg[8], seg[9]]) as f64;
                return match units {
                    1 if x > 0.0 => Some(x),
                    2 if x > 0.0 => Some(x * 2.54),
                    _ => None,
                };
            }
        }
        if marker == 0xDA {
            return None; // start of scan, no more headers
        }
        i += 2 + len;
    }
    None
}

fn exif_dpi(b: &[u8]) -> Option<f64> {
    use exif::{In, Tag, Value};
    let mut cur = std::io::Cursor::new(b);
    let ex = exif::Reader::new().read_from_container(&mut cur).ok()?;
    let x = match &ex.get_field(Tag::XResolution, In::PRIMARY)?.value {
        Value::Rational(v) if !v.is_empty() => v[0].to_f64(),
        _ => return None,
    };
    let unit = ex
        .get_field(Tag::ResolutionUnit, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(2);
    match unit {
        3 => Some(x * 2.54),
        _ => Some(x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_screen_default_dpi() {
        let m = FileMeta {
            width_px: 1080,
            height_px: 1920,
            format: "png".into(),
            dpi: Some(72.0),
            dpi_source: None,
        };
        assert!(m.meaningful_dpi().is_none());
        let m = FileMeta {
            dpi: Some(300.0),
            ..m
        };
        let (w, _) = m.physical_mm().unwrap();
        assert!((w - 91.44).abs() < 0.01);
    }
}
