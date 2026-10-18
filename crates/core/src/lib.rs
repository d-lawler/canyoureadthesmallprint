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
    /// User drew/adjusted the legal copy box.
    pub legal_rect: Option<Rect>,
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

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub meta: FileMeta,
    pub context: DetectedContext,
    pub legal: Option<LegalCopy>,
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

    // 2. find the legal copy
    let mut text_lines = detect::find_text_lines(&img);
    let (legal, hint) = match opts.legal_rect {
        Some(r) => (
            Some(LegalCopy {
                rect: r,
                auto_detected: false,
            }),
            None,
        ),
        None => match detect::pick_legal(&text_lines) {
            Some((r, pol, _)) => (
                Some(LegalCopy {
                    rect: r,
                    auto_detected: true,
                }),
                Some(pol),
            ),
            None => (None, None),
        },
    };

    // weak fragments were only needed for picking; don't draw them
    text_lines.retain(|l| !l.weak);

    let footer = json!({
        "date": opts.assessed_at.clone().unwrap_or_else(|| "—".into()),
        "ruleset": templates.ruleset, "version": VERSION, "hash": hash,
    });

    // 3. measure + 4. rules
    let measured = legal
        .as_ref()
        .and_then(|l| measure::measure(&img, l.rect, hint));
    let (Some(l), Some(m)) = (&legal, measured) else {
        let memo = memo::render(&templates, ("summary_no_copy", json!({})), &[], footer)
            .map_err(Error::Templates)?;
        return Ok(Report {
            meta,
            context,
            legal,
            text_lines,
            measurement: None,
            verdict: None,
            path: vec![],
            memo,
            ruleset: templates.ruleset.clone(),
            hash,
        });
    };
    let out = rules::evaluate(&rules::Inputs {
        width: meta.width_px,
        height: meta.height_px,
        ctx: &context.chosen,
        m: &m,
        manual_area: !l.auto_detected,
        market: opts.market,
        category: opts.category,
    });
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
        text_lines,
        measurement: Some(m),
        verdict: Some(out.verdict),
        path: out.path,
        memo,
        ruleset: templates.ruleset.clone(),
        hash,
    })
}
