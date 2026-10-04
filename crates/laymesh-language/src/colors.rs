//! Static color recognition and edits. Never execute user functions or load assets.
use crate::{LanguageService, index::tokens};
use laymesh_core::{
    color::Color,
    parser::{self, ExprKind},
};
use serde_json::{Value as J, json};
fn constructor(s: &str) -> Option<Color> {
    laymesh_core::color::constant_expression(&parser::expression(s, "/color.lay").ok()?)
}
fn information(from: usize, to: usize, color: Color, format: &str, quote: &str) -> J {
    json!({"from":from,"to":to,"rgba":color.rgba,"space":color.space,"channels":color.channels,"alpha":color.alpha,"mapped":color.mapped,"format":format,"quote":quote})
}
fn color_key(s: &str) -> bool {
    s.ends_with("color")
        || [
            "fill",
            "background",
            "colors",
            "cmap",
            "stops",
            "under",
            "over",
        ]
        .contains(&s)
}
impl LanguageService {
    pub fn document_colors(&self, uri: &str) -> J {
        let source = self.documents.get(uri).map(String::as_str).unwrap_or("");
        let ts = tokens(source);
        let mut out = vec![];
        let mut stack: Vec<String> = vec![];
        let mut color_lists: Vec<bool> = vec![];
        let shadowed = ts
            .windows(2)
            .filter(|w| w[0].text == "function")
            .map(|w| w[1].text.as_str())
            .collect::<Vec<_>>();
        let mut i = 0;
        while i < ts.len() {
            let t = &ts[i];
            if t.text == "(" {
                stack.push(if i > 0 {
                    ts[i - 1].text.clone()
                } else {
                    String::new()
                })
            }
            if t.text == ")" {
                stack.pop();
            }
            let parameter = if i >= 2 && ts[i - 1].text == "=" {
                Some(ts[i - 2].text.as_str())
            } else {
                None
            };
            let color_context = parameter.is_some_and(color_key)
                || stack.is_empty() && parameter.is_some()
                || color_lists.last().copied().unwrap_or(false);
            if t.text == "[" {
                color_lists.push(color_context);
            }
            if t.text == "]" {
                color_lists.pop();
            }
            if t.kind == "string" && color_context {
                if let Ok(e) = parser::expression(&t.text, uri) {
                    if let ExprKind::String(s, _, false) = e.kind {
                        if let Ok(c) = Color::parse(&s) {
                            if s != "none" {
                                let quote = if t.text.starts_with('\'') { "'" } else { "\"" };
                                out.push(information(t.from, t.to, c, "string", quote))
                            }
                        }
                    }
                }
            }
            if t.kind == "name"
                && ["rgb", "hsv", "oklch"].contains(&t.text.as_str())
                && ts.get(i + 1).is_some_and(|t| t.text == "(")
                && color_context
                && !shadowed.contains(&t.text.as_str())
            {
                let mut depth = 0;
                for end in i + 1..ts.len() {
                    if ts[end].text == "(" {
                        depth += 1
                    }
                    if ts[end].text == ")" {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(c) = constructor(&source[t.from..ts[end].to]) {
                                out.push(information(t.from, ts[end].to, c, "constructor", ""))
                            }
                            break;
                        }
                    }
                }
            }
            i += 1;
        }
        // LCSS has unquoted hex and space-separated functions; inspect declaration values.
        let regions = if uri.ends_with(".lcss") {
            vec![(0, source.len())]
        } else {
            ts.windows(2)
                .filter(|w| w[0].text == "style" && w[1].text == "{")
                .filter_map(|w| {
                    let start = w[1].to;
                    let mut depth = 1;
                    for t in ts.iter().filter(|t| t.from >= start) {
                        if t.text == "{" {
                            depth += 1
                        }
                        if t.text == "}" {
                            depth -= 1;
                            if depth == 0 {
                                return Some((start, t.from));
                            }
                        }
                    }
                    None
                })
                .collect()
        };
        let re = regex::Regex::new(r#"(?i)#[0-9a-f]+\b|(?:rgb|hsv|oklch)\([^()]*\)"#).unwrap();
        for (a, b) in regions {
            let mut css = source[a..b].to_string();
            let comments = regex::Regex::new(r"(?s)/\*.*?\*/").unwrap();
            for m in comments.find_iter(&source[a..b]) {
                css.replace_range(m.start()..m.end(), &" ".repeat(m.len()));
            }
            for m in re.find_iter(&css) {
                let before = &css[..m.start()];
                let start = before.rfind([';', '{', '}']).map(|v| v + 1).unwrap_or(0);
                let declaration = &before[start..];
                let Some((key, _)) = declaration.split_once(':') else {
                    continue;
                };
                let key = key.trim().replace('-', "_");
                if !color_key(&key)
                    && !key.starts_with("__")
                    && !matches!(key.as_str(), "start_head" | "end_head")
                {
                    continue;
                }
                if let Ok(c) = Color::parse(m.as_str()) {
                    out.push(information(a + m.start(), a + m.end(), c, "css", ""));
                }
            }
        }
        out.sort_by_key(|v| v["from"].as_u64());
        // Embedded LCSS head values are also valid DSL strings. Keep the
        // outer quoted range so its one swatch preserves quotes on writeback.
        let mut distinct: Vec<J> = vec![];
        for c in out {
            if distinct
                .last()
                .is_some_and(|prev| prev["to"].as_u64() > c["from"].as_u64())
            {
                continue;
            }
            distinct.push(c);
        }
        json!(distinct)
    }
    pub fn color_presentations(&self, uri: &str, from: usize, to: usize, rgba: [f64; 4]) -> J {
        let Some(source) = self.documents.get(uri) else {
            return json!([]);
        };
        let Some(old) = source.get(from..to) else {
            return json!([]);
        };
        let color = Color::new(
            "rgb",
            [rgba[0] * 255., rgba[1] * 255., rgba[2] * 255.],
            rgba[3],
        );
        let Ok(color) = color else { return json!([]) };
        let info = self
            .document_colors(uri)
            .as_array()
            .and_then(|vs| vs.iter().find(|v| v["from"] == from && v["to"] == to))
            .cloned();
        let format =
            info.as_ref()
                .and_then(|v| v["format"].as_str())
                .unwrap_or(if uri.ends_with(".lcss") {
                    "css"
                } else if old.starts_with(['\"', '\'']) {
                    "string"
                } else {
                    "constructor"
                });
        let quote = info
            .as_ref()
            .and_then(|v| v["quote"].as_str())
            .unwrap_or("\"");
        let mut out = vec![];
        for space in ["hex", "rgb", "hsv", "oklch"] {
            let c = color.channels_in(space);
            let number = |v: f64| {
                format!("{:.6}", v)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            };
            let body = if space == "hex" {
                color.hex()
            } else if format == "constructor" {
                format!(
                    "{}({}, {}, {}, alpha={})",
                    space,
                    number(c[0]),
                    number(c[1]),
                    number(c[2]),
                    number(color.alpha)
                )
            } else {
                format!(
                    "{}({} {} {} / {})",
                    space,
                    number(c[0]),
                    number(c[1]),
                    number(c[2]),
                    number(color.alpha)
                )
            };
            let value = if format == "string" || format == "constructor" && space == "hex" {
                format!("{quote}{body}{quote}")
            } else {
                body.clone()
            };
            out.push(json!({"label":body,"from":from,"to":to,"text":value,"space":space}));
        }
        json!(out)
    }
}
