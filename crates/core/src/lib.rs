//! smallpint: is the legal copy on this ad accessible enough to sign off?
//!
//! `analyze()` takes the raw file bytes and does everything: works out the
//! format, finds the legal copy, measures it, walks the rules and writes the
//! memo. Every input except the bytes is optional.

pub mod color;
pub mod context;
pub mod detect;
pub mod measure;
pub mod memo;
pub mod meta;
pub mod rules;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

pub use color::Rgb;
pub use context::{Candidate, DetectedContext, MediumHint};
pub use detect::{Polarity, Rect, TextLine};
pub use measure::Measurement;
pub use memo::Memo;
pub use meta::FileMeta;
pub use rules::{Category, Level, Market, Step, Verdict};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Options {
    /// User picked a format from the alternatives ("d6", "48_sheet" …).
    pub format_id: Option<String>,
    /// User answered the one question for an unrecognised size.
    pub medium_hint: Option<MediumHint>,
    /// User drew a single legal copy box (shorthand for `legal_rects: [r]`).
    pub legal_rect: Option<Rect>,
    /// User's choice of which areas are legal copy. Detected blocks that
    /// overlap one of these are marked legal; the rest of these rects are
    /// added as hand-drawn blocks. `Some(vec![])` means none are legal.
    pub legal_rects: Option<Vec<Rect>>,
    pub market: Market,
    pub category: Option<Category>,
    /// ISO date for the footer (the engine has no clock in WASM).
    pub assessed_at: Option<String>,
    /// Alternative templates.toml source.
    pub templates: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LegalCopy {
    pub rect: Rect,
    pub auto_detected: bool,
}

/// One block of text on the artwork, measured on its own.
#[derive(Clone, Debug, Serialize)]
pub struct Block {
    pub rect: Rect,
    /// Counted as legal copy (auto-picked or chosen by the user).
    pub legal: bool,
    /// Drawn by hand rather than detected.
    pub manual: bool,
    /// "bottom-left", "top", …
    pub position: String,
    pub font_pt: Option<f64>,
    pub ratio: Option<f64>,
    pub level: Option<Level>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub meta: FileMeta,
    pub context: DetectedContext,
    /// The legal block the verdict is for (the weakest one).
    pub legal: Option<LegalCopy>,
    /// Every block of text found, measured individually.
    pub blocks: Vec<Block>,
    /// Every text line found, for drawing on the artwork.
    pub text_lines: Vec<TextLine>,
    pub measurement: Option<Measurement>,
    pub verdict: Option<Verdict>,
    pub path: Vec<Step>,
    pub memo: Memo,
    pub ruleset: String,
    pub hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't read the image: {0}")]
    Decode(String),
    #[error("{0}")]
    Templates(String),
}

pub fn read_meta(bytes: &[u8]) -> Result<(FileMeta, image::RgbImage), Error> {
    let fmt = image::guess_format(bytes).map_err(|e| Error::Decode(e.to_string()))?;
    let img = image::load_from_memory_with_format(bytes, fmt)
        .map_err(|e| Error::Decode(e.to_string()))?
        .to_rgb8();
    let dpi = meta::read_dpi(bytes);
    let meta = FileMeta {
        width_px: img.width(),
        height_px: img.height(),
        format: format!("{fmt:?}").to_lowercase(),
        dpi: dpi.map(|d| d.0),
        dpi_source: dpi.map(|d| d.1.to_string()),
    };
    Ok((meta, img))
}

pub fn analyze(bytes: &[u8], opts: &Options) -> Result<Report, Error> {
    let templates = match &opts.templates {
        Some(src) => memo::Templates::parse(src).map_err(Error::Templates)?,
        None => memo::Templates::default_set(),
    };
    let (meta, img) = read_meta(bytes)?;
    let hash = {
        let d = Sha256::digest(bytes);
        d.iter()
            .take(6)
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };

    // 1. context: detect, then apply any user correction
    let mut context = context::detect(&meta);
    let user_pick = opts
        .format_id
        .as_deref()
        .and_then(|id| context::from_format(&meta, id))
        .or_else(|| opts.medium_hint.map(|h| context::from_hint(&meta, h)));
    if let Some(pick) = user_pick {
        let prev = std::mem::replace(&mut context.chosen, pick);
        context
            .alternatives
            .retain(|a| a.format_id != context.chosen.format_id);
        if prev.confidence > context::Confidence::Low {
            context.alternatives.insert(0, prev);
        }
        context.needs_question = false;
    }

    // 2. find every block of text, decide which are legal copy
    let mut text_lines = detect::find_text_lines(&img);
    let found = detect::blocks(&text_lines);
    text_lines.retain(|l| !l.weak);
    let chosen: Option<Vec<Rect>> = opts
        .legal_rects
        .clone()
        .or_else(|| opts.legal_rect.map(|r| vec![r]));
    let overlaps =
        |a: Rect, b: Rect| a.intersection(b) as f64 > 0.5 * a.area().min(b.area()) as f64;
    // (rect, polarity hint, legal, manual)
    let mut cands: Vec<(Rect, Option<Polarity>, bool, bool)> = Vec::new();
    match &chosen {
        None => {
            let auto = detect::likely_legal(&found);
            for (i, b) in found.iter().enumerate() {
                cands.push((b.rect, Some(b.polarity), auto.contains(&i), false));
            }
        }
        Some(rs) => {
            for b in &found {
                cands.push((
                    b.rect,
                    Some(b.polarity),
                    rs.iter().any(|&r| overlaps(r, b.rect)),
                    false,
                ));
            }
            for &r in rs {
                if !found.iter().any(|b| overlaps(r, b.rect)) {
                    cands.push((r, None, true, true));
                }
            }
        }
    }
    cands.sort_by_key(|c| (c.0.y, c.0.x));

    fn ctx_inputs<'a>(
        meta: &FileMeta,
        ctx: &'a Candidate,
        opts: &Options,
        m: &'a Measurement,
        manual: bool,
    ) -> rules::Inputs<'a> {
        rules::Inputs {
            width: meta.width_px,
            height: meta.height_px,
            ctx,
            m,
            manual_area: manual,
            market: opts.market,
            category: opts.category,
        }
    }
    let mut blocks = Vec::new();
    let mut weakest: Option<(usize, Measurement, f64)> = None;
    for (rect, pol, legal, manual) in &cands {
        let m = measure::measure(&img, *rect, *pol);
        let v = m.as_ref().map(|m| {
            rules::evaluate(&ctx_inputs(&meta, &context.chosen, opts, m, *manual)).verdict
        });
        if let (true, Some(m), Some(v)) = (*legal, &m, &v) {
            let margin = v.ratio / v.required_aa;
            if weakest.as_ref().is_none_or(|w| margin < w.2) {
                weakest = Some((blocks.len(), m.clone(), margin));
            }
        }
        blocks.push(Block {
            rect: *rect,
            legal: *legal,
            manual: *manual,
            position: position(*rect, meta.width_px, meta.height_px),
            font_pt: v.as_ref().map(|v| v.font_pt),
            ratio: v.as_ref().map(|v| v.ratio),
            level: v.as_ref().map(|v| v.level),
        });
    }
    let auto_detected = chosen.is_none();
    let legal = weakest.as_ref().map(|w| LegalCopy {
        rect: blocks[w.0].rect,
        auto_detected,
    });

    let footer = json!({
        "date": opts.assessed_at.clone().unwrap_or_else(|| "—".into()),
        "ruleset": templates.ruleset, "version": VERSION, "hash": hash,
    });

    // 3. the verdict is for the weakest legal block
    let Some((wi, m, _)) = weakest else {
        let memo = memo::render(&templates, ("summary_no_copy", json!({})), &[], footer)
            .map_err(Error::Templates)?;
        return Ok(Report {
            meta,
            context,
            legal,
            blocks,
            text_lines,
            measurement: None,
            verdict: None,
            path: vec![],
            memo,
            ruleset: templates.ruleset.clone(),
            hash,
        });
    };
    let mut out = rules::evaluate(&ctx_inputs(
        &meta,
        &context.chosen,
        opts,
        &m,
        blocks[wi].manual,
    ));
    // say which text was assessed, and what the other legal blocks measured
    let others: Vec<serde_json::Value> = blocks
        .iter()
        .enumerate()
        .filter(|(i, b)| *i != wi && b.legal && b.ratio.is_some())
        .map(|(_, b)| {
            json!({
                "position": b.position,
                "pt": format!("{:.1}", b.font_pt.unwrap_or(0.0)),
                "ratio": format!("{:.2}", (b.ratio.unwrap_or(0.0) * 100.0).floor() / 100.0),
                "level": level_label(b.level),
            })
        })
        .collect();
    let at = out
        .fragments
        .iter()
        .position(|(id, _)| *id == "context")
        .map_or(0, |i| i + 1);
    let loc = if others.is_empty() {
        ("legal_location", json!({ "position": blocks[wi].position }))
    } else {
        (
            "legal_blocks",
            json!({ "count": others.len() + 1, "position": blocks[wi].position, "others": others }),
        )
    };
    out.fragments.insert(at, loc);
    let summary = match out.verdict.level {
        Level::Aaa => "summary_aaa",
        Level::Aa => "summary_aa",
        Level::Fail => "summary_fail",
    };
    let memo = memo::render(&templates, (summary, json!({})), &out.fragments, footer)
        .map_err(Error::Templates)?;

    Ok(Report {
        meta,
        context,
        legal,
        blocks,
        text_lines,
        measurement: Some(m),
        verdict: Some(out.verdict),
        path: out.path,
        memo,
        ruleset: templates.ruleset.clone(),
        hash,
    })
}

fn level_label(l: Option<Level>) -> &'static str {
    match l {
        Some(Level::Aaa) => "meets AAA",
        Some(Level::Aa) => "meets AA",
        Some(Level::Fail) => "below AA",
        None => "not measurable",
    }
}

/// Plain-English location of a rect on the artwork, for the memo.
fn position(r: Rect, w: u32, h: u32) -> String {
    let cx = (r.x as f64 + r.w as f64 / 2.0) / w as f64;
    let cy = (r.y as f64 + r.h as f64 / 2.0) / h as f64;
    let v = if cy < 0.34 {
        "top"
    } else if cy > 0.66 {
        "bottom"
    } else {
        "middle"
    };
    // a block spanning most of the width is just "bottom", not "bottom-centre"
    let wide = r.w as f64 / w as f64 > 0.6;
    let hz = if wide {
        ""
    } else if cx < 0.34 {
        "-left"
    } else if cx > 0.66 {
        "-right"
    } else {
        ""
    };
    if v == "middle" && hz.is_empty() {
        "centre".into()
    } else {
        format!("{v}{hz}")
    }
}
