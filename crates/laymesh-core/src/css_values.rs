//! CSS appearance values. All lengths are resolved in the project's layout context.
use crate::{
    animation::{
        css::{Declaration, split},
        values::{self, Context},
    },
    color::Color,
};
use serde_json::{Value as Json, json};

pub fn color(s: &str, current: &str) -> Result<String, String> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("rebeccapurple") {
        return Ok("#663399".into());
    }
    if s.eq_ignore_ascii_case("currentcolor") {
        return Ok(current.into());
    }
    if matches!(s.to_ascii_lowercase().as_str(), "transparent" | "none") {
        return Ok("none".into());
    }
    if let Ok(c) = Color::parse(s) {
        return Ok(c.css());
    }
    if s.contains('(')
        && let Ok(c) = csscolor::parse_color(s)
    {
        let c = crate::gradient_color::rgba(c);
        return Ok(format!(
            "rgba({},{},{},{})",
            c[0] * 255.,
            c[1] * 255.,
            c[2] * 255.,
            c[3]
        ));
    }
    if s.starts_with('#') {
        return Err("Invalid hexadecimal CSS color".into());
    }
    if !s.contains('(') {
        if let Ok(c) = s.to_ascii_lowercase().parse::<svgtypes::Color>() {
            return Ok(if c.alpha == 255 {
                format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue)
            } else {
                format!(
                    "rgba({},{},{},{})",
                    c.red,
                    c.green,
                    c.blue,
                    c.alpha as f64 / 255.
                )
            });
        }
    }
    let (name, body) = s.split_once('(').ok_or("Invalid CSS color")?;
    let body = body.strip_suffix(')').ok_or("Unclosed CSS color")?;
    let tokens = values::words(&split(&split(body, ',').join(" "), '/').join(" "));
    if !matches!(tokens.len(), 3 | 4) {
        return Err("Color requires three channels and optional alpha".into());
    }
    let alpha = if tokens.len() == 4 {
        values::number_percentage(&tokens[3], 1.)?.clamp(0., 1.)
    } else {
        1.
    };
    let rgb = match name.to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => [0, 1, 2]
            .map(|i| values::number_percentage(&tokens[i], 255.).map(|v| v.clamp(0., 255.)))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?,
        "hsl" | "hsla" => {
            let h = values::angle(&tokens[0])
                .map(f64::to_degrees)
                .or_else(|_| values::number(&tokens[0]))?
                .rem_euclid(360.);
            if !tokens[1..3].iter().all(|v| v.contains('%')) {
                return Err("HSL saturation and lightness require percentages".into());
            }
            let sat = values::number_percentage(&tokens[1], 1.)?.clamp(0., 1.);
            let l = values::number_percentage(&tokens[2], 1.)?.clamp(0., 1.);
            let c = (1. - (2. * l - 1.).abs()) * sat;
            let x = c * (1. - ((h / 60.).rem_euclid(2.) - 1.).abs());
            let m = l - c / 2.;
            let a = match (h / 60.) as usize {
                0 => [c, x, 0.],
                1 => [x, c, 0.],
                2 => [0., c, x],
                3 => [0., x, c],
                4 => [x, 0., c],
                _ => [c, 0., x],
            };
            a.map(|v| (v + m) * 255.).to_vec()
        }
        _ => return Err(format!("Unsupported color function {name}")),
    };
    Ok(Color::new("rgb", [rgb[0], rgb[1], rgb[2]], alpha)?.css())
}

pub fn gradient(s: &str, ctx: Context, current: &str) -> Result<Json, String> {
    crate::gradients::parse(s, ctx, current)
}

pub fn gradient_box(p: &Json, bbox: [f64; 4]) -> std::borrow::Cow<'_, Json> {
    if p["cssAngle"].is_null() && p["cssRadial"].is_null() {
        return std::borrow::Cow::Borrowed(p);
    }
    let mut p = p.clone();
    let b = p["paintBox"]
        .as_array()
        .map(|v| std::array::from_fn(|i| v[i].as_f64().unwrap_or(bbox[i])))
        .unwrap_or(bbox);
    let (w, h) = (b[2].abs().max(1e-12), b[3].abs().max(1e-12));
    if let Some(angle) = p["cssAngle"].as_f64() {
        let (mut dx, mut dy) = (angle.sin(), -angle.cos());
        if p["cssCorner"] == true {
            dx = dx.signum() * h;
            dy = dy.signum() * w;
            let norm = dx.hypot(dy);
            dx /= norm;
            dy /= norm;
        }
        let half = (dx.abs() * w + dy.abs() * h) / 2.;
        p["start"] = json!([0.5 - dx * half / w, 0.5 - dy * half / h]);
        p["end"] = json!([0.5 + dx * half / w, 0.5 + dy * half / h]);
    } else {
        let c = p["center"].as_array().unwrap();
        let (x, y) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
        let (rx, ry) = (x.abs().max((1. - x).abs()), y.abs().max((1. - y).abs()));
        if p["cssRadial"] == "circle" {
            p["radius"] = json!((rx * w).hypot(ry * h) / w.max(h));
            p["paintBox"] = json!(b);
        } else {
            // Default CSS ellipse reaches the farthest corner with its side aspect ratio.
            p["center"] = json!([x / rx, y / ry]);
            p["radius"] = json!(2f64.sqrt());
            // Existing normalized radial paints already apply the box transform.
            p["cssEllipseBox"] = json!([b[0], b[1], w * rx, h * ry]);
            p["paintBox"] = Json::Null;
        }
    }
    std::borrow::Cow::Owned(p)
}

pub fn shadow(s: &str, ctx: Context, current: &str, kind: &str) -> Result<Vec<Json>, String> {
    if s.trim() == "none" {
        return Ok(vec![]);
    }
    split(s, ',').iter().map(|layer| {
        let mut lengths = vec![]; let mut ink = current.to_string(); let mut has_color = false; let mut inset = false;
        for word in values::words(layer) {
            if word == "inset" && kind == "box" && !inset { inset = true; }
            else if let Ok(n) = values::length_context(&word, Context { reference: 0., ..ctx }) {
                if word.contains('%') { return Err("Shadow lengths cannot be percentages".into()); }
                lengths.push(n);
            } else {
                if has_color { return Err("Duplicate shadow color".into()); }
                ink = color(&word, current)?; has_color = true;
            }
        }
        let max = if kind == "box" { 4 } else { 3 };
        if lengths.len() < 2 || lengths.len() > max { return Err("Shadow requires offset x/y and optional blur/spread".into()); }
        let blur = *lengths.get(2).unwrap_or(&0.);
        if blur < 0. { return Err("Shadow blur cannot be negative".into()); }
        Ok(json!({"kind":"cssShadow","color":ink,"opacity":1.,"offset":[lengths[0],lengths[1]],"blur":if kind=="drop" {blur} else {blur/2.},"spread":lengths.get(3).copied().unwrap_or(0.),"mode":if inset {"inner"} else {"outer"},"source":kind}))
    }).collect()
}
pub const SHADOW_PARTS: [&str; 5] = [
    "box_shadow_offset",
    "box_shadow_blur",
    "box_shadow_spread",
    "box_shadow_color",
    "box_shadow_position",
];
pub fn shadow_expansion(
    s: &str,
    ctx: Context,
    current: &str,
) -> Result<Vec<(String, String)>, String> {
    let layers = shadow(s, ctx, current, "box")?;
    let values: Vec<String> = if layers.is_empty() {
        vec![
            "none".into(),
            "0mm".into(),
            "0mm".into(),
            "currentColor".into(),
            "outset".into(),
        ]
    } else {
        (0..5)
            .map(|i| {
                layers
                    .iter()
                    .map(|e| match i {
                        0 => format!("{}mm {}mm", e["offset"][0], e["offset"][1]),
                        1 => format!("{}mm", e["blur"].as_f64().unwrap() * 2.),
                        2 => format!("{}mm", e["spread"]),
                        3 => e["color"].as_str().unwrap().into(),
                        _ => {
                            if e["mode"] == "inner" {
                                "inset".into()
                            } else {
                                "outset".into()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect()
    };
    let mut parts: Vec<_> = SHADOW_PARTS
        .into_iter()
        .map(str::to_string)
        .zip(values)
        .collect();
    parts.push(("__css_shadow_layers".into(), layers.len().to_string()));
    Ok(parts)
}
pub fn box_shadows(
    parts: &std::collections::BTreeMap<String, Declaration>,
    ctx: Context,
    current: &str,
) -> Result<Vec<Json>, String> {
    let Some(offset) = parts.get("box_shadow_offset") else {
        return Ok(vec![]);
    };
    let offsets = split(&offset.value, ',');
    if offsets.len() > 32 {
        return Err("At most 32 shadow layers are supported".into());
    }
    let mut out = vec![];
    let base = parts
        .get("__css_shadow_layers")
        .and_then(|d| d.value.parse::<usize>().ok())
        .unwrap_or(0);
    let count = if offsets.len() == 1 {
        base.max(1)
    } else {
        offsets.len()
    };
    for i in 0..count {
        let offset = &offsets[i % offsets.len()];
        if offset == "none" {
            continue;
        }
        let xy = values::words(offset);
        if xy.is_empty() || xy.len() > 2 {
            return Err("Shadow offset requires one or two lengths".into());
        }
        let length = |s: &str| {
            if s.contains('%') {
                return Err("Shadow lengths cannot be percentages".into());
            }
            values::length_context(
                s,
                Context {
                    reference: 0.,
                    ..ctx
                },
            )
        };
        let x = length(&xy[0])?;
        let y = length(xy.get(1).unwrap_or(&xy[0]))?;
        let nth = |key: &str, default: &str| {
            let v = split(
                parts.get(key).map(|d| d.value.as_str()).unwrap_or(default),
                ',',
            );
            v[i % v.len()].clone()
        };
        let blur = length(&nth("box_shadow_blur", "0"))?;
        let spread = length(&nth("box_shadow_spread", "0"))?;
        let ink = color(&nth("box_shadow_color", "currentColor"), current)?;
        let position = nth("box_shadow_position", "outset");
        if blur < 0. || !matches!(position.as_str(), "inset" | "outset") {
            return Err("Invalid shadow blur/position".into());
        }
        out.push(json!({"kind":"cssShadow","color":ink,"opacity":1.,"offset":[x,y],"blur":blur/2.,"spread":spread,"mode":if position=="inset" {"inner"} else {"outer"},"source":"box"}));
    }
    Ok(out)
}

/// Three-valued conditions: an unsupported condition remains unknown, even
/// under `not`, so an unsupported feature never activates a rule accidentally.
pub fn condition(query: &str, test: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
    let q = query.trim();
    if q.is_empty() {
        return Some(true);
    }
    if let Some(q) = q.strip_prefix("not ") {
        return condition(q, test).map(|v| !v);
    }
    let words = values::words(q);
    for op in ["or", "and"] {
        if words.iter().any(|v| v == op) {
            let chunks: Vec<_> = words
                .split(|v| v == op)
                .map(|v| condition(&v.join(" "), test))
                .collect();
            return if op == "or" {
                if chunks.contains(&Some(true)) {
                    Some(true)
                } else if chunks.iter().all(|v| *v == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            } else {
                if chunks.contains(&Some(false)) {
                    Some(false)
                } else if chunks.iter().all(|v| *v == Some(true)) {
                    Some(true)
                } else {
                    None
                }
            };
        }
    }
    if let Some(inner) = q.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        if let Some(v) = test(inner.trim()) {
            return Some(v);
        }
        if inner.contains(" and ") || inner.contains(" or ") || inner.starts_with('(') {
            return condition(inner, test);
        }
        return None;
    }
    test(q)
}
pub fn media(query: &str, ctx: Context, reduced: bool, print: bool) -> Option<bool> {
    let test = |s: &str| -> Option<bool> {
        match s {
            "screen" => return Some(!print),
            "print" => return Some(print),
            "all" => return Some(true),
            "prefers-reduced-motion: reduce" => return Some(reduced),
            "prefers-reduced-motion: no-preference" => return Some(!reduced),
            _ => {}
        }
        let (key, value) = s.split_once(':')?;
        let actual = match key
            .trim()
            .trim_start_matches("min-")
            .trim_start_matches("max-")
        {
            "width" => ctx.viewport[0],
            "height" => ctx.viewport[1],
            _ => return None,
        };
        let expected = values::length_context(value.trim(), ctx).ok()?;
        Some(if key.trim().starts_with("min-") {
            actual >= expected
        } else if key.trim().starts_with("max-") {
            actual <= expected
        } else {
            (actual - expected).abs() < 1e-6
        })
    };
    let outcomes: Vec<_> = split(query, ',')
        .iter()
        .map(|q| condition(q.strip_prefix("only ").unwrap_or(q), &test))
        .collect();
    if outcomes.contains(&Some(true)) {
        Some(true)
    } else if outcomes.iter().all(|v| *v == Some(false)) {
        Some(false)
    } else {
        None
    }
}
