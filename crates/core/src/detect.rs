//! Finds lines of text without OCR, then picks the legal copy.
//!
//! Text is a row of similar-height blobs sharing a baseline. We find blobs
//! that are locally darker (or lighter) than their surroundings, chain them
//! into lines, and call the smallest consistent text block the legal copy.
//! The user can always redraw the box; this just makes the default good.

use image::{imageops, GrayImage, RgbImage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn union(self, o: Rect) -> Rect {
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        }
    }
    pub fn area(self) -> u64 {
        self.w as u64 * self.h as u64
    }
    pub fn intersection(self, o: Rect) -> u64 {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        if x1 > x0 && y1 > y0 {
            (x1 - x0) as u64 * (y1 - y0) as u64
        } else {
            0
        }
    }
    pub fn pad(self, p: u32, max_w: u32, max_h: u32) -> Rect {
        let x = self.x.saturating_sub(p);
        let y = self.y.saturating_sub(p);
        let x1 = (self.x + self.w + p).min(max_w);
        let y1 = (self.y + self.h + p).min(max_h);
        Rect {
            x,
            y,
            w: x1 - x,
            h: y1 - y,
        }
    }
    fn scaled(self, s: f64) -> Rect {
        Rect {
            x: (self.x as f64 * s).floor() as u32,
            y: (self.y as f64 * s).floor() as u32,
            w: (self.w as f64 * s).ceil() as u32,
            h: (self.h as f64 * s).ceil() as u32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    DarkOnLight,
    LightOnDark,
}

#[derive(Clone, Debug, Serialize)]
pub struct TextLine {
    pub rect: Rect,
    /// Median blob height in px (≈ x-height for lowercase text).
    pub glyph_px: f64,
    pub glyphs: usize,
    pub polarity: Polarity,
    /// Short fragment (e.g. the last word of a wrapped line). Never chosen as
    /// legal copy alone, but joins a legal block it sits against.
    pub weak: bool,
}

#[derive(Clone, Copy, Debug)]
struct Blob {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    n: u32,
}

impl Blob {
    fn w(&self) -> u32 {
        self.x1 - self.x0 + 1
    }
    fn h(&self) -> u32 {
        self.y1 - self.y0 + 1
    }
    fn cy(&self) -> f64 {
        (self.y0 + self.y1) as f64 / 2.0
    }
}

/// Max side we analyse at. Big print files are downsampled for *finding*
/// text; measurement always happens at full resolution.
const WORK_MAX: u32 = 3000;

pub fn find_text_lines(img: &RgbImage) -> Vec<TextLine> {
    let gray = imageops::grayscale(img);
    let (w, h) = gray.dimensions();
    let long = w.max(h);
    let (work, s) = if long > WORK_MAX {
        let s = WORK_MAX as f64 / long as f64;
        let nw = ((w as f64 * s).round() as u32).max(1);
        let nh = ((h as f64 * s).round() as u32).max(1);
        (
            imageops::resize(&gray, nw, nh, imageops::FilterType::Triangle),
            s,
        )
    } else {
        (gray, 1.0)
    };

    let mut lines = Vec::new();
    for pol in [Polarity::DarkOnLight, Polarity::LightOnDark] {
        let mask = local_threshold(&work, pol);
        let blobs = components(&mask, work.width(), work.height());
        lines.extend(chain(&blobs, work.height(), pol));
    }
    let mut lines = dedupe(lines);
    if s != 1.0 {
        for l in &mut lines {
            l.rect = l.rect.scaled(1.0 / s).pad(0, w, h);
            l.glyph_px /= s;
        }
    }
    lines.sort_by_key(|l| (l.rect.y, l.rect.x));
    lines
}

/// Pixel is ink if it differs from the local mean by more than `k`.
fn local_threshold(g: &GrayImage, pol: Polarity) -> Vec<bool> {
    let (w, h) = (g.width() as usize, g.height() as usize);
    let r = ((w.min(h) / 60).clamp(8, 40)) as isize;
    // integral image
    let mut ii = vec![0u64; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0u64;
        for x in 0..w {
            row += g.as_raw()[y * w + x] as u64;
            ii[(y + 1) * (w + 1) + x + 1] = ii[y * (w + 1) + x + 1] + row;
        }
    }
    let mut out = vec![false; w * h];
    for y in 0..h {
        let y0 = (y as isize - r).max(0) as usize;
        let y1 = (y as isize + r + 1).min(h as isize) as usize;
        for x in 0..w {
            let x0 = (x as isize - r).max(0) as usize;
            let x1 = (x as isize + r + 1).min(w as isize) as usize;
            let sum = ii[y1 * (w + 1) + x1] + ii[y0 * (w + 1) + x0]
                - ii[y0 * (w + 1) + x1]
                - ii[y1 * (w + 1) + x0];
            let mean = sum as f64 / ((x1 - x0) * (y1 - y0)) as f64;
            let v = g.as_raw()[y * w + x] as f64;
            let k = 14.0f64.max(mean * 0.08);
            out[y * w + x] = match pol {
                Polarity::DarkOnLight => v < mean - k,
                Polarity::LightOnDark => v > mean + k,
            };
        }
    }
    out
}

fn components(mask: &[bool], w: u32, h: u32) -> Vec<Blob> {
    let (wu, hu) = (w as usize, h as usize);
    let mut seen = vec![false; mask.len()];
    let mut blobs = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut b = Blob {
            x0: u32::MAX,
            y0: u32::MAX,
            x1: 0,
            y1: 0,
            n: 0,
        };
        while let Some(i) = stack.pop() {
            let (x, y) = (i % wu, i / wu);
            b.x0 = b.x0.min(x as u32);
            b.y0 = b.y0.min(y as u32);
            b.x1 = b.x1.max(x as u32);
            b.y1 = b.y1.max(y as u32);
            b.n += 1;
            let mut push = |j: usize| {
                if mask[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                push(i - 1)
            }
            if x + 1 < wu {
                push(i + 1)
            }
            if y > 0 {
                push(i - wu)
            }
            if y + 1 < hu {
                push(i + wu)
            }
        }
        blobs.push(b);
    }
    blobs
}

fn is_glyph(b: &Blob, img_h: u32) -> bool {
    let (w, h) = (b.w() as f64, b.h() as f64);
    let fill = b.n as f64 / (w * h);
    h >= 4.0
        && h <= (img_h as f64 / 6.0)
        && w <= h * 5.0
        && (0.08..=0.95).contains(&fill)
        && b.n >= 6
}

fn chain(all: &[Blob], img_h: u32, pol: Polarity) -> Vec<TextLine> {
    let mut g: Vec<Blob> = all.iter().copied().filter(|b| is_glyph(b, img_h)).collect();
    g.sort_by_key(|b| b.x0);
    let n = g.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], i: usize) -> usize {
        let mut i = i;
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..n {
        let a = g[i];
        for j in i + 1..n {
            let b = g[j];
            let mh = a.h().max(b.h()) as f64;
            let gap = b.x0 as f64 - a.x1 as f64;
            if gap > mh * 1.3 {
                break; // sorted by x0: everything after is further away
            }
            let hr = a.h().max(b.h()) as f64 / a.h().min(b.h()) as f64;
            // share a baseline-ish band and a similar size
            let same_band = (a.y1 as f64 - b.y1 as f64).abs() < mh * 0.45
                || (a.cy() - b.cy()).abs() < mh * 0.35;
            if gap >= -mh * 0.3 && hr < 2.0 && same_band {
                let (ra, rb) = (find(&mut parent, i), find(&mut parent, j));
                parent[ra] = rb;
            }
        }
    }
    let mut groups: std::collections::HashMap<usize, Vec<Blob>> = Default::default();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(g[i]);
    }
    let mut out = Vec::new();
    for (_, bs) in groups {
        if bs.len() < 2 {
            continue;
        }
        let mut hs: Vec<f64> = bs.iter().map(|b| b.h() as f64).collect();
        hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = hs[hs.len() / 2];
        // baseline consistency: most glyph bottoms within a band
        let mut bottoms: Vec<f64> = bs.iter().map(|b| b.y1 as f64).collect();
        bottoms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let bmed = bottoms[bottoms.len() / 2];
        let aligned = bottoms
            .iter()
            .filter(|&&y| (y - bmed).abs() <= med * 0.3 + 1.0)
            .count();
        if (aligned as f64) < bs.len() as f64 * 0.6 {
            continue;
        }
        let rect = bs
            .iter()
            .map(|b| Rect {
                x: b.x0,
                y: b.y0,
                w: b.w(),
                h: b.h(),
            })
            .reduce(Rect::union)
            .unwrap();
        let strong = bs.len() >= 4 && rect.w as f64 >= med * 4.0;
        if !strong && (rect.w as f64) < med * 1.5 {
            continue;
        }
        out.push(TextLine {
            rect,
            glyph_px: med,
            glyphs: bs.len(),
            polarity: pol,
            weak: !strong,
        });
    }
    out
}

fn dedupe(mut lines: Vec<TextLine>) -> Vec<TextLine> {
    lines.sort_by(|a, b| a.weak.cmp(&b.weak).then(b.glyphs.cmp(&a.glyphs)));
    let mut keep: Vec<TextLine> = Vec::new();
    for l in lines {
        let overlaps = keep.iter().any(|k| {
            let i = k.rect.intersection(l.rect) as f64;
            i > 0.5 * l.rect.area().min(k.rect.area()) as f64
        });
        if !overlaps {
            keep.push(l);
        }
    }
    keep
}

/// Legal copy is (almost always) the smallest consistent text on the artwork.
/// Returns the block rect, its polarity, and the indices of lines in it.
pub fn pick_legal(lines: &[TextLine]) -> Option<(Rect, Polarity, Vec<usize>)> {
    let min = lines
        .iter()
        .filter(|l| !l.weak)
        .map(|l| l.glyph_px)
        .fold(f64::INFINITY, f64::min);
    if !min.is_finite() {
        return None;
    }
    let small: Vec<usize> = (0..lines.len())
        .filter(|&i| !lines[i].weak && lines[i].glyph_px <= min * 1.35 + 1.0)
        .collect();

    // group vertically adjacent small lines into blocks
    let mut blocks: Vec<Vec<usize>> = Vec::new();
    for &i in &small {
        let l = &lines[i];
        let near = blocks.iter_mut().find(|blk| {
            blk.iter().any(|&j| {
                let o = &lines[j];
                let vgap = (l.rect.y as i64 - (o.rect.y + o.rect.h) as i64)
                    .max(o.rect.y as i64 - (l.rect.y + l.rect.h) as i64);
                let hover = l.rect.x < o.rect.x + o.rect.w + (o.glyph_px * 6.0) as u32
                    && o.rect.x < l.rect.x + l.rect.w + (l.glyph_px * 6.0) as u32;
                vgap < (l.glyph_px * 2.5) as i64 && hover
            })
        });
        match near {
            Some(b) => b.push(i),
            None => blocks.push(vec![i]),
        }
    }
    let mut best = blocks
        .into_iter()
        .max_by_key(|b| b.iter().map(|&i| lines[i].glyphs).sum::<usize>())?;
    // absorb short fragments (wrapped last words) that touch the block
    loop {
        let r = best.iter().map(|&i| lines[i].rect).reduce(Rect::union)?;
        let g = best.iter().map(|&i| lines[i].glyph_px).sum::<f64>() / best.len() as f64;
        let add: Vec<usize> = (0..lines.len())
            .filter(|i| !best.contains(i) && lines[*i].weak)
            .filter(|&i| {
                let l = &lines[i];
                let similar = (l.glyph_px / g - 1.0).abs() < 0.4;
                let vgap = (l.rect.y as i64 - (r.y + r.h) as i64)
                    .max(r.y as i64 - (l.rect.y + l.rect.h) as i64);
                let inside_x = l.rect.x + 2 >= r.x && l.rect.x <= r.x + r.w;
                similar && vgap < (g * 2.5) as i64 && inside_x
            })
            .collect();
        if add.is_empty() {
            break;
        }
        best.extend(add);
    }
    let rect = best.iter().map(|&i| lines[i].rect).reduce(Rect::union)?;
    // majority polarity
    let dark = best
        .iter()
        .filter(|&&i| lines[i].polarity == Polarity::DarkOnLight)
        .count();
    let pol = if dark * 2 >= best.len() {
        Polarity::DarkOnLight
    } else {
        Polarity::LightOnDark
    };
    Some((rect, pol, best))
}
