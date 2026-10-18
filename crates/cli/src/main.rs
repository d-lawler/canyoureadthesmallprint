//! smallpint <artwork> [--json] [--format ID] [--hint phone|web|billboard|print]
//!           [--rect x,y,w,h] [--market uk|us|eu|other] [--category financial|…]
//!           [--opts '<Options JSON>']

use smallpint_core::{analyze, Options};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args
        .iter()
        .find(|a| !a.starts_with("--") && !is_value(&args, a))
    else {
        eprintln!("usage: smallpint <artwork> [--json] [--format ID] [--hint phone|web|billboard|print] [--rect x,y,w,h] [--market uk|us|eu] [--category financial|gambling|alcohol|healthcare]");
        return ExitCode::from(2);
    };
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let mut o = serde_json::Map::new();
    if let Some(v) = flag("--format") {
        o.insert("format_id".into(), v.into());
    }
    if let Some(v) = flag("--hint") {
        o.insert("medium_hint".into(), v.into());
    }
    if let Some(v) = flag("--market") {
        o.insert("market".into(), v.into());
    }
    if let Some(v) = flag("--category") {
        o.insert("category".into(), v.into());
    }
    if let Some(v) = flag("--rect") {
        let n: Vec<u32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if n.len() == 4 {
            o.insert(
                "legal_rect".into(),
                serde_json::json!({"x":n[0],"y":n[1],"w":n[2],"h":n[3]}),
            );
        }
    }
    // full Options as JSON, e.g. --opts '{"legal_rects":[{"x":0,"y":0,"w":10,"h":10}]}'
    if let Some(v) = flag("--opts") {
        match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&v) {
            Ok(m) => o.extend(m),
            Err(e) => {
                eprintln!("--opts: {e}");
                return ExitCode::from(2);
            }
        }
    }
    o.insert("assessed_at".into(), today().into());
    let opts: Options = match serde_json::from_value(o.into()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("bad option: {e}");
            return ExitCode::from(2);
        }
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    let r = match analyze(&bytes, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(1);
        }
    };
    if args.iter().any(|a| a == "--json") {
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
    } else {
        let level = r
            .verdict
            .as_ref()
            .map(|v| format!("{:?}", v.level).to_uppercase())
            .unwrap_or("—".into());
        println!(
            "{path}\n  {}  ·  {:?} confidence",
            r.context.chosen.name, r.context.chosen.confidence
        );
        if !r.context.alternatives.is_empty() {
            let alts: Vec<&str> = r
                .context
                .alternatives
                .iter()
                .map(|a| a.name.as_str())
                .collect();
            println!("  or: {}", alts.join(" · "));
        }
        if let Some(l) = &r.legal {
            println!(
                "  legal copy at {:?} ({})",
                l.rect,
                if l.auto_detected { "auto" } else { "manual" }
            );
        }
        println!("  → {level}\n");
        for s in &r.path {
            println!("  {} {}", s.question, s.answer);
        }
        println!("\n{}\n", r.memo.text);
    }
    match r.verdict.map(|v| v.level) {
        Some(smallpint_core::Level::Fail) | None => ExitCode::from(1),
        _ => ExitCode::SUCCESS,
    }
}

fn is_value(args: &[String], a: &String) -> bool {
    args.iter()
        .position(|x| std::ptr::eq(x, a))
        .map(|i| i > 0 && args[i - 1].starts_with("--") && args[i - 1] != "--json")
        .unwrap_or(false)
}

fn today() -> String {
    // days since epoch → civil date (no chrono dependency)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
