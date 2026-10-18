//! The internal decision tree. Takes measurements + detected context and
//! returns a verdict, the path taken, and which memo fragments to use.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::color::{nearest_passing, Rgb};
use crate::context::{Candidate, Scale};
use crate::measure::Measurement;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    #[serde(rename = "AAA")]
    Aaa,
    #[serde(rename = "AA")]
    Aa,
    #[serde(rename = "Fail")]
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    #[default]
    Uk,
    Us,
    Eu,
    Other,
}

impl Market {
    fn label(self) -> Option<&'static str> {
        match self {
            Market::Uk => Some("UK"),
            Market::Us => Some("US"),
            Market::Eu => Some("EU"),
            Market::Other => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Financial,
    Gambling,
    Alcohol,
    Healthcare,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Category::Financial => "financial services",
            Category::Gambling => "gambling",
            Category::Alcohol => "alcohol",
            Category::Healthcare => "healthcare",
        }
    }
}

/// Below this (reading-equivalent pt) we flag prominence regardless of contrast.
/// Not a WCAG number: an internal house rule, versioned with the ruleset.
pub const TINY_PT: f64 = 6.0;
const LOW_CONFIDENCE: f64 = 0.7;

#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub level: Level,
    pub ratio: f64,
    pub required_aa: f64,
    pub required_aaa: f64,
    pub large_text: bool,
    pub font_pt: f64,
    pub suggested_fg: Option<Rgb>,
    pub needs_review: bool,
}

pub struct Inputs<'a> {
    pub width: u32,
    pub height: u32,
    pub ctx: &'a Candidate,
    pub m: &'a Measurement,
    pub manual_area: bool,
    pub market: Market,
    pub category: Option<Category>,
}

pub struct Outcome {
    pub verdict: Verdict,
    pub path: Vec<Step>,
    pub fragments: Vec<(&'static str, Value)>,
}

fn r1(v: f64) -> String {
    format!("{:.1}", v)
}
fn r2(v: f64) -> String {
    // contrast ratios: truncate, never round up across a threshold
    format!("{:.2}", (v * 100.0).floor() / 100.0)
}
fn thr(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

pub fn evaluate(i: &Inputs) -> Outcome {
    let mut path = Vec::new();
    let mut fr: Vec<(&'static str, Value)> = Vec::new();
    let mut step = |q: &str, a: String| {
        path.push(Step {
            question: q.into(),
            answer: a,
        })
    };
    let m = i.m;
    let medium = i.ctx.medium;

    // context
    fr.push((
        "context",
        json!({
            "width": i.width, "height": i.height, "format": i.ctx.name,
            "medium": medium.label(), "market": i.market.label(), "reason": i.ctx.reason,
        }),
    ));
    step("What is this artwork?", i.ctx.name.clone());

    // scope
    let viewing = match i.ctx.scale {
        Scale::Physical { viewing_m, .. } => Some(viewing_m),
        _ => None,
    };
    let viewing_s = viewing.map(|v| {
        if v < 1.0 {
            format!("{:.0}cm", v * 100.0)
        } else {
            format!("{v:.0}m")
        }
    });
    if medium.wcag_applies() {
        fr.push(("wcag_applies", json!({ "medium": medium.label() })));
        step("Does WCAG apply?", "Yes, digital placement".into());
    } else {
        fr.push((
            "wcag_benchmark",
            json!({ "medium": medium.label(), "viewing": viewing_s }),
        ));
        step("Does WCAG apply?", "Used as a benchmark".into());
    }

    if i.manual_area {
        fr.push(("manual_area", json!({})));
    }

    // size
    let pt = i.ctx.scale.font_pt(m.font_px);
    match i.ctx.scale {
        Scale::Screen { css_px_per_px, .. } => fr.push((
            "size_screen",
            json!({ "pt": r1(pt), "css_px": r1(m.font_px * css_px_per_px), "lines": m.lines }),
        )),
        Scale::Physical { .. } => fr.push((
            "size_physical",
            json!({ "pt": r1(pt), "mm": r1(i.ctx.scale.font_mm(m.font_px).unwrap_or(0.0) * CAP_FROM_EM), "viewing": viewing_s, "lines": m.lines }),
        )),
    }
    let bold = m.bold.unwrap_or(false);
    let large = pt >= 18.0 || (bold && pt >= 14.0);
    let (aa, aaa) = if large { (3.0, 4.5) } else { (4.5, 7.0) };
    if large {
        let basis = if pt >= 18.0 {
            "18pt or larger"
        } else {
            "14pt or larger and bold"
        };
        fr.push(("size_large", json!({ "basis": basis })));
        step("How big is the text?", format!("{}pt: large text", r1(pt)));
    } else {
        fr.push(("size_normal", json!({})));
        step("How big is the text?", format!("{}pt: normal text", r1(pt)));
        if m.bold.is_none() && pt >= 14.0 {
            fr.push(("weight_unknown", json!({})));
        }
    }

    // background
    if m.bg_varied {
        fr.push(("bg_varied", json!({})));
        step(
            "What's behind it?",
            "Varied background, worst case used".into(),
        );
    } else {
        fr.push(("bg_uniform", json!({})));
        step("What's behind it?", "Consistent background".into());
    }

    // contrast verdict
    let ratio = m.ratio_worst;
    let vars = json!({
        "ratio": r2(ratio), "fg": m.fg.hex(), "bg": m.bg_worst.hex(), "aa": thr(aa), "aaa": thr(aaa),
    });
    let (level, passed_at) = if ratio >= aaa {
        (Level::Aaa, aaa)
    } else if ratio >= aa {
        (Level::Aa, aa)
    } else {
        (Level::Fail, aa)
    };
    let mut suggested = None;
    match level {
        Level::Aaa => fr.push(("result_aaa", vars)),
        Level::Aa => fr.push(("result_aa", vars)),
        Level::Fail => {
            fr.push(("result_fail", vars));
            let flip = if m.fg.luminance() < m.bg_worst.luminance() {
                Rgb(255, 255, 255)
            } else {
                Rgb(0, 0, 0)
            };
            // on a varied background a flipped colour would fail elsewhere
            let fix = nearest_passing(m.fg, m.bg_worst, aa).or_else(|| {
                if m.bg_varied {
                    None
                } else {
                    nearest_passing(flip, m.bg_worst, aa)
                }
            });
            match fix {
                Some(fix) => {
                    suggested = Some(fix);
                    fr.push(("fix_colour", json!({ "fix": fix.hex() })));
                }
                None => fr.push(("fix_colour_impossible", json!({}))),
            }
            if !large && ratio >= 3.0 {
                let px = i.ctx.scale.px_for_pt(18.0);
                fr.push((
                    "fix_size",
                    json!({ "pt": "18", "px": format!("{:.0}", px * CAP_FROM_EM) }),
                ));
            }
        }
    }
    step(
        "Is the contrast enough?",
        format!(
            "{}:1 → {}",
            r2(ratio),
            match level {
                Level::Aaa => "AAA",
                Level::Aa => "AA",
                Level::Fail => "below AA",
            }
        ),
    );

    let mut needs_review = false;
    if level != Level::Fail && ratio < passed_at * 1.1 {
        fr.push(("near_threshold", json!({})));
        needs_review = true;
    }
    if m.confidence < LOW_CONFIDENCE {
        fr.push((
            "low_confidence",
            json!({ "confidence": format!("{:.0}", m.confidence * 100.0) }),
        ));
        needs_review = true;
    }
    if pt < TINY_PT {
        fr.push(("tiny_text", json!({ "pt": r1(pt) })));
        needs_review = true;
    }
    if matches!(
        medium,
        crate::context::Medium::Display | crate::context::Medium::Mobile
    ) {
        fr.push(("images_of_text", json!({})));
    }
    if let Some(c) = i.category {
        fr.push(("specialist", json!({ "category": c.label() })));
        needs_review = true;
    }
    if medium.wcag_applies() {
        fr.push(("alt_text", json!({})));
    }

    Outcome {
        verdict: Verdict {
            level,
            ratio,
            required_aa: aa,
            required_aaa: aaa,
            large_text: large,
            font_pt: pt,
            suggested_fg: suggested,
            needs_review,
        },
        path,
        fragments: fr,
    }
}

/// Cap height as a share of the em: what someone measuring "how tall is the
/// text" on a proof would see.
const CAP_FROM_EM: f64 = 0.72;
