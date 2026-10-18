//! Renders the memo from approved fragments.

use std::collections::HashMap;

use minijinja::Environment;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_TEMPLATES: &str = include_str!("../templates.toml");

#[derive(Deserialize)]
struct Fragment {
    text: String,
}

#[derive(Deserialize)]
pub struct Templates {
    pub ruleset: String,
    fragments: HashMap<String, Fragment>,
}

impl Templates {
    pub fn parse(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| format!("templates: {e}"))
    }
    pub fn default_set() -> Self {
        Self::parse(DEFAULT_TEMPLATES).expect("built-in templates parse")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Memo {
    pub summary: String,
    pub paragraphs: Vec<String>,
    pub footer: String,
    /// Fragment ids used, in order: the audit trail of which approved wording appeared.
    pub fragment_ids: Vec<String>,
    /// Plain text, ready to paste.
    pub text: String,
}

pub fn render(
    t: &Templates,
    summary: (&str, Value),
    body: &[(&str, Value)],
    footer: Value,
) -> Result<Memo, String> {
    let env = Environment::new();
    let one = |id: &str, v: &Value| -> Result<String, String> {
        let f = t
            .fragments
            .get(id)
            .ok_or_else(|| format!("templates: missing fragment `{id}`"))?;
        env.render_str(&f.text, v)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("fragment `{id}`: {e}"))
    };
    let s = one(summary.0, &summary.1)?;
    // body fragments flow into paragraphs: context / measurement / verdict / notes
    let mut paragraphs: Vec<String> = Vec::new();
    let mut cur = String::new();
    let breaks_before = [
        "size_screen",
        "size_physical",
        "result_aaa",
        "result_aa",
        "result_fail",
        "near_threshold",
        "low_confidence",
        "tiny_text",
        "images_of_text",
        "specialist",
        "alt_text",
    ];
    for (id, v) in body {
        let text = one(id, v)?;
        if breaks_before.contains(id) && !cur.is_empty() {
            paragraphs.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(&text);
    }
    if !cur.is_empty() {
        paragraphs.push(cur);
    }
    let footer = one("footer", &footer)?;
    let mut ids = vec![summary.0.to_string()];
    ids.extend(body.iter().map(|(id, _)| id.to_string()));
    ids.push("footer".into());
    let text = std::iter::once(s.clone())
        .chain(paragraphs.iter().cloned())
        .chain(std::iter::once(footer.clone()))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(Memo {
        summary: s,
        paragraphs,
        footer,
        fragment_ids: ids,
        text,
    })
}
