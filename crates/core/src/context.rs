//! Works out what the artwork *is* from the file alone: placement, medium
//! and — the bit that matters for WCAG — how big one image pixel will be
//! when a person actually sees it.
//!
//! Signals, strongest first:
//!   1. Exact pixel match to a standard ad unit (incl. @2x/@3x exports)
//!   2. Embedded DPI → physical size → print/OOH format (incl. 10%/25% supply scales)
//!   3. Aspect ratio match to a format family (9:16, 4:5, 16:9 …)
//!   4. Nothing matched → ask one question (Phone / Web / Billboard / Print)

use serde::{Deserialize, Serialize};

use crate::meta::FileMeta;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Medium {
    Mobile,
    Display,
    Video,
    Dooh,
    Ooh,
    Print,
}

impl Medium {
    /// WCAG is written for web content. Off-screen media use it as a benchmark.
    pub fn wcag_applies(self) -> bool {
        matches!(self, Medium::Mobile | Medium::Display | Medium::Video)
    }
    pub fn label(self) -> &'static str {
        match self {
            Medium::Mobile => "mobile",
            Medium::Display => "web display",
            Medium::Video => "online video",
            Medium::Dooh => "digital out-of-home",
            Medium::Ooh => "out-of-home",
            Medium::Print => "print",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediumHint {
    Phone,
    Web,
    Billboard,
    Print,
}

#[derive(Clone, Copy, Debug)]
enum Match {
    /// Exact pixel size; @2x and @3x exports also match.
    Exact(u32, u32),
    /// Any size with this aspect ratio. `typical` = the usual export size.
    Ratio { w: u32, h: u32, typical: (u32, u32) },
    /// Physical size in mm; needs DPI in the file. Rotatable = can be landscape.
    Size {
        w_mm: f64,
        h_mm: f64,
        rotatable: bool,
    },
}

#[derive(Clone, Copy, Debug)]
enum Render {
    /// Shown on a screen this many CSS px wide.
    Css(f64),
    /// Exists at this physical size, read from this distance.
    Physical { w_mm: f64, viewing_m: f64 },
}

#[derive(Debug)]
pub struct Format {
    pub id: &'static str,
    pub name: &'static str,
    pub medium: Medium,
    m: Match,
    r: Render,
}

const fn f(id: &'static str, name: &'static str, medium: Medium, m: Match, r: Render) -> Format {
    Format {
        id,
        name,
        medium,
        m,
        r,
    }
}

use Match::*;
use Medium::*;
use Render::*;

/// Phone viewport most social placements render into (iPhone 13–16 class).
pub const PHONE_CSS: f64 = 390.0;
/// Distance the WCAG point sizes implicitly assume: a screen or page held/sat at ~40cm.
pub const READING_M: f64 = 0.4;

/// Order matters within a tie: earlier = preferred default.
pub static CATALOGUE: &[Format] = &[
    // --- Social / mobile (aspect families) ---
    f(
        "story",
        "Story / Reels / TikTok (9:16)",
        Mobile,
        Ratio {
            w: 9,
            h: 16,
            typical: (1080, 1920),
        },
        Css(PHONE_CSS),
    ),
    f(
        "feed_portrait",
        "Social feed, portrait (4:5)",
        Mobile,
        Ratio {
            w: 4,
            h: 5,
            typical: (1080, 1350),
        },
        Css(PHONE_CSS),
    ),
    f(
        "feed_square",
        "Social feed, square (1:1)",
        Mobile,
        Ratio {
            w: 1,
            h: 1,
            typical: (1080, 1080),
        },
        Css(PHONE_CSS),
    ),
    f(
        "feed_landscape",
        "Social link ad (1.91:1)",
        Mobile,
        Ratio {
            w: 191,
            h: 100,
            typical: (1200, 628),
        },
        Css(PHONE_CSS),
    ),
    // --- IAB display (exact sizes) ---
    f(
        "mpu",
        "MPU / medium rectangle 300×250",
        Display,
        Exact(300, 250),
        Css(300.0),
    ),
    f(
        "large_rect",
        "Large rectangle 336×280",
        Display,
        Exact(336, 280),
        Css(336.0),
    ),
    f(
        "leaderboard",
        "Leaderboard 728×90",
        Display,
        Exact(728, 90),
        Css(728.0),
    ),
    f(
        "super_leaderboard",
        "Super leaderboard 970×90",
        Display,
        Exact(970, 90),
        Css(970.0),
    ),
    f(
        "billboard_display",
        "Billboard display 970×250",
        Display,
        Exact(970, 250),
        Css(970.0),
    ),
    f(
        "skyscraper",
        "Wide skyscraper 160×600",
        Display,
        Exact(160, 600),
        Css(160.0),
    ),
    f(
        "skyscraper_narrow",
        "Skyscraper 120×600",
        Display,
        Exact(120, 600),
        Css(120.0),
    ),
    f(
        "half_page",
        "Half page 300×600",
        Display,
        Exact(300, 600),
        Css(300.0),
    ),
    f(
        "portrait_display",
        "Portrait 300×1050",
        Display,
        Exact(300, 1050),
        Css(300.0),
    ),
    f(
        "banner",
        "Banner 468×60",
        Display,
        Exact(468, 60),
        Css(468.0),
    ),
    f(
        "square_display",
        "Square 250×250",
        Display,
        Exact(250, 250),
        Css(250.0),
    ),
    f(
        "small_square",
        "Small square 200×200",
        Display,
        Exact(200, 200),
        Css(200.0),
    ),
    f(
        "mobile_banner",
        "Mobile banner 320×50",
        Display,
        Exact(320, 50),
        Css(320.0),
    ),
    f(
        "mobile_banner_lg",
        "Large mobile banner 320×100",
        Display,
        Exact(320, 100),
        Css(320.0),
    ),
    f(
        "mobile_banner_sm",
        "Mobile banner 300×50",
        Display,
        Exact(300, 50),
        Css(300.0),
    ),
    f(
        "interstitial",
        "Mobile interstitial 320×480",
        Display,
        Exact(320, 480),
        Css(320.0),
    ),
    // --- Video / landscape screens ---
    f(
        "video_16x9",
        "Online video / web hero (16:9)",
        Video,
        Ratio {
            w: 16,
            h: 9,
            typical: (1920, 1080),
        },
        Css(854.0),
    ),
    // --- Digital out-of-home (matched by ratio, rendered physically) ---
    f(
        "d6",
        "Digital 6-sheet (D6) portrait screen",
        Dooh,
        Ratio {
            w: 9,
            h: 16,
            typical: (1080, 1920),
        },
        Physical {
            w_mm: 1012.0,
            viewing_m: 2.0,
        },
    ),
    f(
        "dooh_landscape",
        "Landscape DOOH screen (rail / mall)",
        Dooh,
        Ratio {
            w: 16,
            h: 9,
            typical: (1920, 1080),
        },
        Physical {
            w_mm: 1660.0,
            viewing_m: 3.0,
        },
    ),
    f(
        "d48",
        "Digital 48-sheet roadside screen",
        Dooh,
        Ratio {
            w: 2,
            h: 1,
            typical: (1920, 960),
        },
        Physical {
            w_mm: 6096.0,
            viewing_m: 15.0,
        },
    ),
    // --- Print (needs DPI) ---
    f(
        "a5",
        "A5 print",
        Print,
        Size {
            w_mm: 148.0,
            h_mm: 210.0,
            rotatable: true,
        },
        Physical {
            w_mm: 148.0,
            viewing_m: 0.4,
        },
    ),
    f(
        "a4",
        "A4 print",
        Print,
        Size {
            w_mm: 210.0,
            h_mm: 297.0,
            rotatable: true,
        },
        Physical {
            w_mm: 210.0,
            viewing_m: 0.4,
        },
    ),
    f(
        "letter",
        "US Letter print",
        Print,
        Size {
            w_mm: 215.9,
            h_mm: 279.4,
            rotatable: true,
        },
        Physical {
            w_mm: 215.9,
            viewing_m: 0.4,
        },
    ),
    f(
        "a3",
        "A3 print / press DPS",
        Print,
        Size {
            w_mm: 297.0,
            h_mm: 420.0,
            rotatable: true,
        },
        Physical {
            w_mm: 297.0,
            viewing_m: 0.6,
        },
    ),
    f(
        "a2",
        "A2 poster",
        Print,
        Size {
            w_mm: 420.0,
            h_mm: 594.0,
            rotatable: true,
        },
        Physical {
            w_mm: 420.0,
            viewing_m: 1.0,
        },
    ),
    f(
        "a1",
        "A1 poster",
        Print,
        Size {
            w_mm: 594.0,
            h_mm: 841.0,
            rotatable: true,
        },
        Physical {
            w_mm: 594.0,
            viewing_m: 1.5,
        },
    ),
    // --- Classic OOH (needs DPI; often supplied at 10% / 25%) ---
    f(
        "4_sheet",
        "4-sheet poster",
        Ooh,
        Size {
            w_mm: 1016.0,
            h_mm: 1524.0,
            rotatable: false,
        },
        Physical {
            w_mm: 1016.0,
            viewing_m: 2.0,
        },
    ),
    f(
        "6_sheet",
        "6-sheet (bus shelter / Adshel)",
        Ooh,
        Size {
            w_mm: 1200.0,
            h_mm: 1800.0,
            rotatable: false,
        },
        Physical {
            w_mm: 1200.0,
            viewing_m: 2.0,
        },
    ),
    // 48 before 12: same 2:1 shape, and a 48 at 10% is a 12 at 20%. When the
    // file can't tell them apart, default to the stricter (further) reading.
    f(
        "48_sheet",
        "48-sheet billboard",
        Ooh,
        Size {
            w_mm: 6096.0,
            h_mm: 3048.0,
            rotatable: false,
        },
        Physical {
            w_mm: 6096.0,
            viewing_m: 15.0,
        },
    ),
    f(
        "12_sheet",
        "12-sheet poster",
        Ooh,
        Size {
            w_mm: 3048.0,
            h_mm: 1524.0,
            rotatable: false,
        },
        Physical {
            w_mm: 3048.0,
            viewing_m: 6.0,
        },
    ),
    f(
        "96_sheet",
        "96-sheet billboard",
        Ooh,
        Size {
            w_mm: 12192.0,
            h_mm: 3048.0,
            rotatable: false,
        },
        Physical {
            w_mm: 12192.0,
            viewing_m: 20.0,
        },
    ),
    f(
        "us_bulletin",
        "US bulletin 14×48ft",
        Ooh,
        Size {
            w_mm: 14630.0,
            h_mm: 4267.0,
            rotatable: false,
        },
        Physical {
            w_mm: 14630.0,
            viewing_m: 60.0,
        },
    ),
];

/// Scales OOH artwork is commonly supplied at.
const SUPPLY_SCALES: &[f64] = &[1.0, 0.5, 0.25, 0.2, 0.1, 0.05];

pub fn format_by_id(id: &str) -> Option<&'static Format> {
    CATALOGUE.iter().find(|f| f.id == id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
    /// Chosen by the user.
    Confirmed,
}

/// How one artwork pixel maps to what a person sees.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scale {
    Screen {
        css_px_per_px: f64,
        css_width: f64,
    },
    Physical {
        mm_per_px: f64,
        viewing_m: f64,
        width_mm: f64,
    },
}

impl Scale {
    /// Font size in pt as WCAG means it. For physical media this is the
    /// size that subtends the same visual angle at 40cm reading distance.
    pub fn font_pt(&self, font_px: f64) -> f64 {
        match *self {
            Scale::Screen { css_px_per_px, .. } => font_px * css_px_per_px * 0.75,
            Scale::Physical {
                mm_per_px,
                viewing_m,
                ..
            } => {
                let mm = font_px * mm_per_px;
                let at_reading = mm * READING_M / viewing_m.max(READING_M);
                at_reading / 0.352_777_8
            }
        }
    }

    /// Inverse of font_pt: artwork pixels needed for a given pt.
    pub fn px_for_pt(&self, pt: f64) -> f64 {
        pt / self.font_pt(1.0)
    }

    /// Physical size of `font_px` in mm, for physical media.
    pub fn font_mm(&self, font_px: f64) -> Option<f64> {
        match *self {
            Scale::Physical { mm_per_px, .. } => Some(font_px * mm_per_px),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub format_id: String,
    pub name: String,
    pub medium: Medium,
    pub confidence: Confidence,
    pub scale: Scale,
    /// Plain-English reason, used in the memo and the "how we got here" panel.
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DetectedContext {
    /// The assumption the assessment runs on.
    pub chosen: Candidate,
    /// Other plausible readings, for the "Change" chip.
    pub alternatives: Vec<Candidate>,
    /// True when nothing matched and the UI should ask Phone/Web/Billboard/Print.
    pub needs_question: bool,
}

fn screen(img_w: u32, css: f64) -> Scale {
    Scale::Screen {
        css_px_per_px: css / img_w as f64,
        css_width: css,
    }
}

fn physical(img_w: u32, w_mm: f64, viewing_m: f64) -> Scale {
    Scale::Physical {
        mm_per_px: w_mm / img_w as f64,
        viewing_m,
        width_mm: w_mm,
    }
}

fn candidate(f: &Format, img_w: u32, confidence: Confidence, reason: String) -> Candidate {
    let scale = match f.r {
        Css(css) => screen(img_w, css),
        Physical { w_mm, viewing_m } => physical(img_w, w_mm, viewing_m),
    };
    Candidate {
        format_id: f.id.into(),
        name: f.name.into(),
        medium: f.medium,
        confidence,
        scale,
        reason,
    }
}

fn ratio_close(a: f64, b: f64, tol: f64) -> bool {
    (a / b - 1.0).abs() <= tol
}

fn fmt_mm(mm: f64) -> String {
    if mm >= 1000.0 {
        format!("{:.2}m", mm / 1000.0)
    } else {
        format!("{:.0}mm", mm)
    }
}

pub fn detect(meta: &FileMeta) -> DetectedContext {
    let (w, h) = (meta.width_px, meta.height_px);
    let ar = w as f64 / h as f64;
    let mut found: Vec<Candidate> = Vec::new();

    // 1. Exact pixel sizes, including @2x / @3x exports
    for fm in CATALOGUE {
        if let Exact(fw, fh) = fm.m {
            for k in 1..=3u32 {
                if w == fw * k && h == fh * k {
                    let at = if k == 1 {
                        String::new()
                    } else {
                        format!(" exported @{k}x")
                    };
                    found.push(candidate(
                        fm,
                        w,
                        Confidence::High,
                        format!("{w}×{h}px is the standard {}{at}.", fm.name),
                    ));
                }
            }
        }
    }

    // 2. Physical size from DPI
    if let (Some(dpi), Some((wmm, hmm))) = (meta.meaningful_dpi(), meta.physical_mm()) {
        let before = found.len();
        for fm in CATALOGUE {
            if let Size {
                w_mm,
                h_mm,
                rotatable,
            } = fm.m
            {
                let orients: &[(f64, f64)] = if rotatable {
                    &[(w_mm, h_mm), (h_mm, w_mm)]
                } else {
                    &[(w_mm, h_mm)]
                };
                for &(fw, fh) in orients {
                    if !ratio_close(ar, fw / fh, 0.015) {
                        continue;
                    }
                    let s = wmm / fw;
                    // print is supplied at 100%; only OOH comes scaled down
                    let scales: &[f64] = if fm.medium == Ooh {
                        SUPPLY_SCALES
                    } else {
                        &[1.0]
                    };
                    if let Some(&scale) = scales.iter().find(|&&sc| ratio_close(s, sc, 0.03)) {
                        let supplied = if scale == 1.0 {
                            "at full size".to_string()
                        } else {
                            format!("supplied at {:.0}% scale", scale * 100.0)
                        };
                        let mut c = candidate(
                            fm,
                            w,
                            Confidence::High,
                            format!(
                                "{dpi:.0}dpi gives {}×{}, matching a {} {supplied}.",
                                fmt_mm(wmm),
                                fmt_mm(hmm),
                                fm.name
                            ),
                        );
                        // true physical width is the format's, whatever the supply scale
                        if let Render::Physical { viewing_m, .. } = fm.r {
                            c.scale = physical(w, fw, viewing_m);
                        }
                        found.push(c);
                        break;
                    }
                }
            }
        }
        let diag = (wmm * wmm + hmm * hmm).sqrt();
        // Print-intent artwork with no standard size: use what the DPI says.
        // Skipped for standard screen exports (Photoshop stamps 300dpi on
        // everything) and for implausibly small sizes.
        let screen_typical = CATALOGUE
            .iter()
            .any(|fm| matches!(fm.m, Ratio { typical, .. } if typical == (w, h)));
        if found.len() == before && found.is_empty() && !screen_typical && diag >= 150.0 {
            let viewing = match diag {
                d if d < 500.0 => 0.4,
                d if d < 1200.0 => 1.0,
                d if d < 3000.0 => 2.0,
                _ => 10.0,
            };
            found.push(Candidate {
                format_id: "custom_print".into(),
                name: format!("Custom print, {}×{}", fmt_mm(wmm), fmt_mm(hmm)),
                medium: if diag < 1200.0 { Print } else { Ooh },
                confidence: Confidence::Medium,
                scale: physical(w, wmm, viewing),
                reason: format!(
                    "{dpi:.0}dpi gives {}×{}, which isn't a standard size; assumed a {viewing}m viewing distance.",
                    fmt_mm(wmm),
                    fmt_mm(hmm)
                ),
            });
        }
    }

    // 3. Aspect-ratio families
    for fm in CATALOGUE {
        if let Ratio {
            w: rw,
            h: rh,
            typical,
        } = fm.m
        {
            if ratio_close(ar, rw as f64 / rh as f64, 0.012) {
                let exact = (w, h) == typical;
                let conf = if exact {
                    Confidence::High
                } else {
                    Confidence::Medium
                };
                let why = if exact {
                    format!("{w}×{h}px is the standard {} size.", fm.name)
                } else {
                    format!("{w}×{h}px has the {}:{} shape of a {}.", rw, rh, fm.name)
                };
                found.push(candidate(fm, w, conf, why));
            }
        }
    }

    // Physical/print evidence outranks screen guesses only when DPI said so,
    // which it did by pushing first. Keep insertion order within confidence.
    found.sort_by(|a, b| b.confidence.cmp(&a.confidence));
    dedupe(&mut found);

    if found.is_empty() {
        let fallback = from_hint(meta, MediumHint::Phone);
        return DetectedContext {
            chosen: Candidate {
                confidence: Confidence::Low,
                reason: format!(
                    "{w}×{h}px doesn't match a standard format; assessed as a phone placement until confirmed."
                ),
                ..fallback
            },
            alternatives: [MediumHint::Web, MediumHint::Billboard, MediumHint::Print]
                .into_iter()
                .map(|hnt| from_hint(meta, hnt))
                .collect(),
            needs_question: true,
        };
    }

    let chosen = found.remove(0);
    DetectedContext {
        chosen,
        alternatives: found,
        needs_question: false,
    }
}

fn dedupe(v: &mut Vec<Candidate>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|c| seen.insert(c.format_id.clone()));
}

/// The one-question fallback: user tells us roughly where it's going.
pub fn from_hint(meta: &FileMeta, hint: MediumHint) -> Candidate {
    let (w, h) = (meta.width_px, meta.height_px);
    let (id, name, medium, scale, reason) = match hint {
        MediumHint::Phone => (
            "hint_phone",
            "Phone, full width".to_string(),
            Mobile,
            Scale::Screen {
                css_px_per_px: PHONE_CSS / w as f64,
                css_width: PHONE_CSS,
            },
            format!("Shown full-width on a {PHONE_CSS:.0}px-wide phone screen."),
        ),
        MediumHint::Web => {
            let css = if w > 1600 { w as f64 / 2.0 } else { w as f64 };
            (
                "hint_web",
                "Web page".to_string(),
                Display,
                Scale::Screen {
                    css_px_per_px: css / w as f64,
                    css_width: css,
                },
                if w > 1600 {
                    format!("Shown on the web as a @2x image, {css:.0} CSS px wide.")
                } else {
                    format!("Shown on the web at its natural size, {css:.0} CSS px wide.")
                },
            )
        }
        MediumHint::Billboard => {
            // assume the height of a standard 10ft (3048mm) poster panel
            let h_mm = 3048.0;
            let w_mm = h_mm * w as f64 / h as f64;
            (
                "hint_billboard",
                format!("Billboard, {}×{}", fmt_mm(w_mm), fmt_mm(h_mm)),
                Ooh,
                physical(w, w_mm, 15.0),
                "Assumed a 3m-tall roadside panel viewed from 15m.".to_string(),
            )
        }
        MediumHint::Print => {
            let long = 297.0;
            let w_mm = if w >= h {
                long
            } else {
                long * w as f64 / h as f64
            };
            (
                "hint_print",
                "Print, A4-sized".to_string(),
                Print,
                physical(w, w_mm, READING_M),
                "Assumed an A4-sized page read at 40cm.".to_string(),
            )
        }
    };
    Candidate {
        format_id: id.into(),
        name,
        medium,
        confidence: Confidence::Confirmed,
        scale,
        reason,
    }
}

/// User picked a specific format from the alternatives list.
pub fn from_format(meta: &FileMeta, id: &str) -> Option<Candidate> {
    let fm = format_by_id(id)?;
    let mut c = candidate(
        fm,
        meta.width_px,
        Confidence::Confirmed,
        format!("Confirmed as {}.", fm.name),
    );
    if let Size { .. } = fm.m {
        // physical format chosen without DPI: assume artwork fills it
        if let Render::Physical { w_mm, viewing_m } = fm.r {
            let fw = if meta.width_px >= meta.height_px {
                w_mm.max(phys_h(fm))
            } else {
                w_mm
            };
            c.scale = physical(meta.width_px, fw, viewing_m);
        }
    }
    Some(c)
}

fn phys_h(fm: &Format) -> f64 {
    match fm.m {
        Size {
            w_mm,
            h_mm,
            rotatable: true,
        } => w_mm.max(h_mm),
        Size { w_mm, .. } => w_mm,
        _ => 0.0,
    }
}

/// For the UI picker.
#[derive(Serialize)]
pub struct FormatInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub medium: Medium,
}

pub fn list_formats() -> Vec<FormatInfo> {
    CATALOGUE
        .iter()
        .map(|f| FormatInfo {
            id: f.id,
            name: f.name,
            medium: f.medium,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(w: u32, h: u32, dpi: Option<f64>) -> FileMeta {
        FileMeta {
            width_px: w,
            height_px: h,
            format: "png".into(),
            dpi,
            dpi_source: None,
        }
    }

    #[test]
    fn story_is_default_for_1080x1920_with_d6_alternative() {
        let c = detect(&meta(1080, 1920, Some(72.0)));
        assert_eq!(c.chosen.format_id, "story");
        assert_eq!(c.chosen.confidence, Confidence::High);
        assert!(c.alternatives.iter().any(|a| a.format_id == "d6"));
    }

    #[test]
    fn retina_mpu() {
        let c = detect(&meta(600, 500, Some(144.0)));
        assert_eq!(c.chosen.format_id, "mpu");
        assert!(c.chosen.reason.contains("@2x"));
        // 20 image px at @2x = 10 CSS px = 7.5pt
        assert!((c.chosen.scale.font_pt(20.0) - 7.5).abs() < 1e-6);
    }

    #[test]
    fn story_at_other_resolution() {
        let c = detect(&meta(720, 1280, None));
        assert_eq!(c.chosen.format_id, "story");
        assert_eq!(c.chosen.confidence, Confidence::Medium);
    }

    #[test]
    fn forty_eight_sheet_at_ten_percent() {
        // 609.6 x 304.8mm at 300dpi
        let c = detect(&meta(7200, 3600, Some(300.0)));
        assert_eq!(c.chosen.format_id, "48_sheet");
        assert!(c.chosen.reason.contains("10% scale"), "{}", c.chosen.reason);
        if let Scale::Physical { width_mm, .. } = c.chosen.scale {
            assert!((width_mm - 6096.0).abs() < 1.0);
        } else {
            panic!()
        }
    }

    #[test]
    fn a4_landscape() {
        // 297 x 210mm at 300dpi
        let c = detect(&meta(3508, 2480, Some(300.0)));
        assert_eq!(c.chosen.format_id, "a4");
        // 50px at 300dpi = 4.23mm = 12pt at reading distance
        assert!((c.chosen.scale.font_pt(50.0) - 12.0).abs() < 0.05);
    }

    #[test]
    fn unknown_size_asks() {
        let c = detect(&meta(1234, 777, None));
        assert!(c.needs_question);
        assert_eq!(c.alternatives.len(), 3);
    }

    #[test]
    fn billboard_text_size_is_distance_scaled() {
        // 48-sheet: 6096mm wide over 7200px; 100px tall glyphs = 84.7mm
        // at 15m that looks like 2.26mm at 40cm = 6.4pt
        let c = detect(&meta(7200, 3600, Some(300.0)));
        let pt = c.chosen.scale.font_pt(100.0);
        assert!((pt - 6.4).abs() < 0.1, "{pt}");
    }
}
