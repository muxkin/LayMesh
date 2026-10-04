//! Normalization and custom color interpolation; presets use the shared Matplotlib registry.
use super::*;
pub fn color_domain(s: &Args) -> [f64; 2] {
    let b = nums(s, "boundaries");
    if string(s, "norm", "linear") == "boundary" && b.len() > 1 {
        [b[0], *b.last().unwrap()]
    } else {
        [num(s, "vmin", 0.), num(s, "vmax", 1.)]
    }
}
pub fn color_position(s: &Args, v: f64) -> f64 {
    let [lo, hi] = color_domain(s);
    let norm = string(s, "norm", "linear");
    let c = num(s, "center", 0.);
    if norm == "centered" {
        if v <= c {
            (v - lo) / (c - lo) / 2.
        } else {
            0.5 + (v - c) / (hi - c) / 2.
        }
    } else {
        interpolate(v, [lo, hi], [0., 1.], &norm, num(s, "constant", 1.))
    }
}
pub fn color_value(s: &Args, t: f64) -> f64 {
    let [lo, hi] = color_domain(s);
    let norm = string(s, "norm", "linear");
    let c = num(s, "center", 0.);
    if norm == "centered" {
        if t <= 0.5 {
            lo + t * 2. * (c - lo)
        } else {
            c + (t - 0.5) * 2. * (hi - c)
        }
    } else {
        let c = num(s, "constant", 1.);
        inverse(
            transform(lo, &norm, c) + t * (transform(hi, &norm, c) - transform(lo, &norm, c)),
            &norm,
            c,
        )
    }
}
fn rgb(s: &str) -> [f64; 3] {
    if let Ok(c) = crate::color::Color::parse(s) {
        return [c.rgba[0] * 255., c.rgba[1] * 255., c.rgba[2] * 255.];
    }
    if let Ok(c) = s.parse::<svgtypes::Color>() {
        return [c.red as f64, c.green as f64, c.blue as f64];
    }
    let s = s.trim_start_matches('#');
    let s = if s.len() == 3 {
        s.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        s.into()
    };
    if s.len() != 6 {
        return [0., 0., 0.];
    }
    std::array::from_fn(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0) as f64)
}
pub fn scale_color(s: &Args, v: f64) -> String {
    if !v.is_finite() {
        return "none".into();
    }
    let [lo, hi] = color_domain(s);
    if v < lo && s.contains_key("under") {
        return string(s, "under", "");
    }
    if v > hi && s.contains_key("over") {
        return string(s, "over", "");
    }
    let norm = string(s, "norm", "linear");
    let mut t = color_position(s, v).clamp(0., 1.);
    let raw = array(s, "cmap");
    let colors: Vec<_> = raw.iter().map(V::as_str).collect();
    if norm == "boundary" {
        let b = nums(s, "boundaries");
        let i = b
            .partition_point(|b| *b <= v)
            .saturating_sub(1)
            .min(b.len().saturating_sub(2));
        t = (i as f64 + 0.5) / (b.len() - 1) as f64;
        if !colors.is_empty() {
            return colors[((t * colors.len() as f64) as usize).min(colors.len() - 1)].clone();
        }
    }
    if !colors.is_empty() {
        let p = t * (colors.len() - 1) as f64;
        let i = p.floor() as usize;
        let (a, b) = (rgb(&colors[i]), rgb(&colors[(i + 1).min(colors.len() - 1)]));
        let c = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * (p - i as f64));
        let alpha = |s: &str| crate::color::Color::parse(s).map(|c| c.alpha).unwrap_or(1.);
        let aa = alpha(&colors[i]);
        let ab = alpha(&colors[(i + 1).min(colors.len() - 1)]);
        return crate::color::Color::new("rgb", c, aa + (ab - aa) * (p - i as f64))
            .unwrap()
            .css();
    }
    let cm = match s.get("cmap") {
        Some(V::Cmap(cm)) => Some(cm.clone()),
        Some(V::Text(name, _)) => crate::colormap::Colormap::get(name),
        None => crate::colormap::Colormap::get("viridis"),
        _ => None,
    };
    cm.and_then(|cm| cm.sample(t).ok())
        .map(|c| c.css())
        .unwrap_or_else(|| "none".into())
}
