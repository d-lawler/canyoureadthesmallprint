//! Regression test over generated artwork. Run `python3 fixtures/make.py` first.

use serde_json::Value;
use smallpint_core::{analyze, Options};

#[test]
fn fixtures_match_truth() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let Ok(truth) = std::fs::read_to_string(format!("{dir}/truth.json")) else {
        eprintln!("no fixtures; run python3 fixtures/make.py");
        return;
    };
    let truth: Value = serde_json::from_str(&truth).unwrap();
    for (name, t) in truth.as_object().unwrap() {
        let bytes = std::fs::read(format!("{dir}/{name}")).unwrap();
        let r = analyze(&bytes, &Options::default()).unwrap();
        if let Some(fmt) = t["format"].as_str() {
            assert_eq!(r.context.chosen.format_id, fmt, "{name}: format");
        } else {
            assert!(r.context.needs_question, "{name}: should ask");
        }
        let m = r
            .measurement
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: no legal copy found"));
        let truth_px = t["font_px"].as_f64().unwrap();
        let err = (m.font_px / truth_px - 1.0).abs();
        assert!(
            err < 0.12,
            "{name}: font {:.1}px vs {truth_px}px",
            m.font_px
        );
        if let Some(level) = t.get("level").and_then(|l| l.as_str()) {
            let got = serde_json::to_value(r.verdict.unwrap().level).unwrap();
            assert_eq!(got, level, "{name}: level");
        }
    }
}
