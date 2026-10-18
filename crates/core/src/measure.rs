//! Measures the legal copy inside a box: text colour, worst-case background,
//! contrast ratio, and type size in artwork pixels.

use image::RgbImage;
use serde::Serialize;

use crate::color::{contrast, Rgb};
use crate::detect::{Polarity, Rect};

#[derive(Clone, Debug, Serialize)]
pub struct Measurement {
    pub polarity: Polarity,
    /// Text colour (from the solid core of the glyphs, not anti-aliased edges).
    pub fg: Rgb,
    /// Typical background behind the text.
    pub bg_typical: Rgb,
    /// Background at the point of lowest contrast.
    pub bg_worst: Rgb,
    pub ratio_typical: f64,
    pub ratio_worst: f64,
    /// Background changes noticeably across the copy (gradient / photo).
    pub bg_varied: bool,
    /// Where the worst contrast was found.
    pub worst_at: Rect,
    /// Estimated font size (em) in artwork pixels.
    pub font_px: f64,
    pub x_height_px: f64,
    pub lines: usize,
    pub bold: Option<bool>,
    /// 0–1. Low when the box has little ink or text/background barely separate.
    pub confidence: f64,
}

/// x-height / em. Real fonts range ~0.45 (Garamond) to ~0.55 (Inter,
/// Verdana). We use the top of that range (plus anti-aliasing) so the size
/// estimate errs small, which applies the stricter normal-text thresholds.
const X_HEIGHT_RATIO: f64 = 0.56;
const CAP_HEIGHT_RATIO: f64 = 0.72;

fn luma(p: &image::Rgb<u8>) -> u8 {
    (0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64).round() as u8
}

fn otsu(hist: &[u32; 256]) -> u8 {
    let total: u64 = hist.iter().map(|&c| c as u64).sum();
    let sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum();
    let (mut sum_b, mut w_b, mut best, mut t) = (0.0, 0u64, 0.0, 0u8);
    for i in 0..256 {
        w_b += hist[i] as u64;
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += i as f64 * hist[i] as f64;
        let m_b = sum_b / w_b as f64;
        let m_f = (sum - sum_b) / w_f as f64;
        let between = w_b as f64 * w_f as f64 * (m_b - m_f).powi(2);
        if between > best {
            best = between;
            t = i as u8;
        }
    }
    t
}

fn median_rgb(px: &mut [Rgb]) -> Rgb {
    let mid = px.len() / 2;
    let ch = |px: &mut [Rgb], f: fn(&Rgb) -> u8| {
        let mut v: Vec<u8> = px.iter().map(f).collect();
        v.sort_unstable();
        v[mid]
    };
    Rgb(ch(px, |c| c.0), ch(px, |c| c.1), ch(px, |c| c.2))
}

pub fn measure(img: &RgbImage, rect: Rect, hint: Option<Polarity>) -> Option<Measurement> {
    let r = rect.pad(2, img.width(), img.height());
    if r.w < 4 || r.h < 4 {
        return None;
    }
    let (w, h) = (r.w as usize, r.h as usize);
    let px = |x: usize, y: usize| img.get_pixel(r.x + x as u32, r.y + y as u32);

    let mut hist = [0u32; 256];
    for y in 0..h {
        for x in 0..w {
            hist[luma(px(x, y)) as usize] += 1;
        }
    }
    let t = otsu(&hist);
    let dark_n: u64 = hist[..=t as usize].iter().map(|&c| c as u64).sum();
    let polarity = hint.unwrap_or(if dark_n * 2 <= (w * h) as u64 {
        Polarity::DarkOnLight
    } else {
        Polarity::LightOnDark
    });
    let is_ink = |l: u8| match polarity {
        Polarity::DarkOnLight => l <= t,
        Polarity::LightOnDark => l > t,
    };
    let ink: Vec<bool> = (0..w * h).map(|i| is_ink(luma(px(i % w, i / w)))).collect();
    let ink_n = ink.iter().filter(|&&b| b).count();
    if ink_n < 12 || ink_n == w * h {
        return None;
    }

    // --- text colour: most extreme 30% of ink pixels (skips AA fringes)
    let mut ink_px: Vec<(u8, Rgb)> = (0..w * h)
        .filter(|&i| ink[i])
        .map(|i| {
            let p = px(i % w, i / w);
            (luma(p), Rgb(p[0], p[1], p[2]))
        })
        .collect();
    ink_px.sort_by_key(|(l, _)| *l);
    if polarity == Polarity::LightOnDark {
        ink_px.reverse();
    }
    let take = (ink_px.len() * 3 / 10).max(1);
    let mut core: Vec<Rgb> = ink_px[..take].iter().map(|(_, c)| *c).collect();
    let fg = median_rgb(&mut core);

    // --- background: non-ink pixels at least 2px away from any ink
    let mut near = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if ink[y * w + x] {
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                            near[ny as usize * w + nx as usize] = true;
                        }
                    }
                }
            }
        }
    }

    // --- type size from the row profile
    let (x_height, font_px, lines) = type_size(&ink, w, h)?;

    // --- tiled worst-case background
    let tile = (font_px * 2.0).clamp(12.0, 200.0) as usize;
    let mut all_bg: Vec<Rgb> = Vec::new();
    let mut tiles: Vec<(Rgb, Rect)> = Vec::new();
    for ty in (0..h).step_by(tile) {
        for tx in (0..w).step_by(tile) {
            let mut v = Vec::new();
            let mut has_ink = false;
            for y in ty..(ty + tile).min(h) {
                for x in tx..(tx + tile).min(w) {
                    let i = y * w + x;
                    has_ink |= ink[i];
                    if !near[i] {
                        let p = px(x, y);
                        v.push(Rgb(p[0], p[1], p[2]));
                    }
                }
            }
            all_bg.extend_from_slice(&v);
            // only tiles that actually sit behind text count for worst case
            if has_ink && v.len() >= 12 {
                let c = median_rgb(&mut v);
                let rr = Rect {
                    x: r.x + tx as u32,
                    y: r.y + ty as u32,
                    w: tile.min(w - tx) as u32,
                    h: tile.min(h - ty) as u32,
                };
                tiles.push((c, rr));
            }
        }
    }
    if all_bg.len() < 12 {
        // box drawn tight to the glyphs: fall back to the non-ink pixels
        all_bg = (0..w * h)
            .filter(|&i| !ink[i])
            .map(|i| {
                let p = px(i % w, i / w);
                Rgb(p[0], p[1], p[2])
            })
            .collect();
    }
    let bg_typical = median_rgb(&mut all_bg);
    if tiles.is_empty() {
        tiles.push((bg_typical, r));
    }
    let ratio_typical = contrast(fg, bg_typical);
    let (bg_worst, worst_at, ratio_worst) = tiles
        .iter()
        .map(|(c, rr)| (*c, *rr, contrast(fg, *c)))
        .fold((bg_typical, r, f64::INFINITY), |acc, t| {
            if t.2 < acc.2 {
                t
            } else {
                acc
            }
        });
    let ratio_worst = ratio_worst.min(ratio_typical);
    let lums: Vec<f64> = tiles.iter().map(|(c, _)| c.luminance()).collect();
    let (lmin, lmax) = lums
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), &l| (a.min(l), b.max(l)));
    let bg_varied = (lmax + 0.05) / (lmin + 0.05) > 1.2;

    // --- weight: only trust it when glyphs are big enough to resolve strokes
    let stroke = median_run(&ink, w, h);
    let bold = if x_height >= 12.0 {
        Some(stroke / x_height > 0.24)
    } else {
        None
    };

    let coverage = ink_n as f64 / (w * h) as f64;
    let mut confidence: f64 = 1.0;
    if coverage < 0.03 || coverage > 0.6 {
        confidence -= 0.35;
    }
    if ratio_typical < 1.3 {
        confidence -= 0.4; // text and background barely separate
    }
    if x_height < 5.0 {
        confidence -= 0.25; // too few pixels to measure size well
    }

    Some(Measurement {
        polarity,
        fg,
        bg_typical,
        bg_worst,
        ratio_typical,
        ratio_worst,
        bg_varied,
        worst_at,
        font_px,
        x_height_px: x_height,
        lines,
        bold,
        confidence: confidence.clamp(0.0, 1.0),
    })
}

/// Returns (x-height px, font size px, line count) — median across lines.
fn type_size(ink: &[bool], w: usize, h: usize) -> Option<(f64, f64, usize)> {
    let rows: Vec<usize> = (0..h)
        .map(|y| ink[y * w..(y + 1) * w].iter().filter(|&&b| b).count())
        .collect();
    let floor = ((w as f64 * 0.01) as usize).max(1);
    let mut runs = Vec::new();
    let mut start = None;
    for y in 0..=h {
        let on = y < h && rows[y] >= floor;
        match (on, start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                if y - s >= 3 {
                    runs.push((s, y));
                }
                start = None;
            }
            _ => {}
        }
    }
    if runs.is_empty() {
        return None;
    }
    let mut xs = Vec::new();
    let mut fonts = Vec::new();
    for (s, e) in &runs {
        let seg = &rows[*s..*e];
        let peak = *seg.iter().max().unwrap();
        let dense: Vec<usize> = (0..seg.len()).filter(|&i| seg[i] * 2 >= peak).collect();
        let band = (dense.last().unwrap() - dense.first().unwrap() + 1) as f64;
        let extent = seg.len() as f64;
        let font = if band / extent > 0.85 {
            extent / CAP_HEIGHT_RATIO // caps / figures: no x-height band
        } else {
            band / X_HEIGHT_RATIO
        };
        xs.push(band);
        fonts.push(font);
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    fonts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some((xs[xs.len() / 2], fonts[fonts.len() / 2], runs.len()))
}

fn median_run(ink: &[bool], w: usize, h: usize) -> f64 {
    let mut runs = Vec::new();
    for y in 0..h {
        let mut n = 0;
        for x in 0..=w {
            if x < w && ink[y * w + x] {
                n += 1;
            } else if n > 0 {
                runs.push(n);
                n = 0;
            }
        }
    }
    if runs.is_empty() {
        return 0.0;
    }
    runs.sort_unstable();
    runs[runs.len() / 2] as f64
}
