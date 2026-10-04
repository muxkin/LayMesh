//! Shared source colors and deterministic sRGB conversion, independent of rendering.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Color {
    pub space: String,
    pub channels: [f64; 3],
    pub alpha: f64,
    pub rgba: [f64; 4],
    pub mapped: bool,
}
fn linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encoded(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn oklab(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(linear);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}
fn from_lch([l, c, h]: [f64; 3]) -> [f64; 3] {
    let a = c * h.to_radians().cos();
    let b = c * h.to_radians().sin();
    let x = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let y = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let z = (l - 0.0894841775 * a - 1.291485548 * b).powi(3);
    [
        4.0767416621 * x - 3.3077115913 * y + 0.2309699292 * z,
        -1.2684380046 * x + 2.6097574011 * y - 0.3413193965 * z,
        -0.0041960863 * x - 0.7034186147 * y + 1.707614701 * z,
    ]
    .map(encoded)
}
impl Color {
    pub fn new(space: &str, mut channels: [f64; 3], alpha: f64) -> Result<Self, String> {
        if channels.iter().chain([&alpha]).any(|v| !v.is_finite()) || !(0.0..=1.0).contains(&alpha)
        {
            return Err("颜色通道需要有限数，透明度必须在 0–1 之间".into());
        }
        let mut mapped = false;
        let rgb = match space {
            "rgb" => {
                if channels.iter().any(|v| !(0.0..=255.0).contains(v)) {
                    return Err("RGB 通道必须在 0–255 之间".into());
                }
                channels.map(|v| v / 255.)
            }
            "hsv" => {
                if channels[1..].iter().any(|v| !(0.0..=1.0).contains(v)) {
                    return Err("HSV 饱和度和明度必须在 0–1 之间".into());
                }
                channels[0] = channels[0].rem_euclid(360.);
                let [h, s, v] = channels;
                let c = v * s;
                let x = c * (1. - ((h / 60.).rem_euclid(2.) - 1.).abs());
                let m = v - c;
                let rgb = match (h / 60.) as usize {
                    0 => [c, x, 0.],
                    1 => [x, c, 0.],
                    2 => [0., c, x],
                    3 => [0., x, c],
                    4 => [x, 0., c],
                    _ => [c, 0., x],
                };
                rgb.map(|n| n + m)
            }
            "oklch" => {
                if !(0.0..=1.0).contains(&channels[0]) || channels[1] < 0. {
                    return Err("OKLCH 明度必须在 0–1 之间，色度必须非负".into());
                }
                channels[2] = channels[2].rem_euclid(360.);
                let raw = from_lch(channels);
                mapped = raw
                    .iter()
                    .any(|v| !v.is_finite() || *v < -1e-7 || *v > 1. + 1e-7);
                if !mapped {
                    raw
                } else {
                    // Fixed constant-lightness/hue chroma reduction to the sRGB gamut.
                    let mut lo = 0.;
                    // Every in-gamut sRGB chroma is below 1. This also bounds
                    // the search when a finite source chroma is enormous.
                    let mut hi = channels[1].min(1.);
                    for _ in 0..48 {
                        let c = (lo + hi) / 2.;
                        let rgb = from_lch([channels[0], c, channels[2]]);
                        if rgb.iter().all(|v| (-1e-9..=1. + 1e-9).contains(v)) {
                            lo = c
                        } else {
                            hi = c
                        }
                    }
                    from_lch([channels[0], lo, channels[2]])
                }
            }
            _ => return Err("支持 RGB、HSV 和 OKLCH；CMYK 暂不支持".into()),
        };
        Ok(Self {
            space: space.into(),
            channels,
            alpha,
            rgba: [
                rgb[0].clamp(0., 1.),
                rgb[1].clamp(0., 1.),
                rgb[2].clamp(0., 1.),
                alpha,
            ],
            mapped,
        })
    }
    pub fn parse(source: &str) -> Result<Self, String> {
        let s = source.trim();
        if s == "none" {
            return Self::new("rgb", [0.; 3], 0.);
        }
        if let Some(h) = s.strip_prefix('#') {
            if !matches!(h.len(), 3 | 4 | 6 | 8) || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("十六进制颜色需要 3、4、6 或 8 位".into());
            }
            let h = if h.len() < 5 {
                h.chars().flat_map(|c| [c, c]).collect::<String>()
            } else {
                h.into()
            };
            let c = |i| u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64;
            let mut color = Self::new(
                "rgb",
                [c(0), c(2), c(4)],
                if h.len() == 8 { c(6) / 255. } else { 1. },
            )?;
            color.space = "hex".into();
            return Ok(color);
        }
        let (space, rest) = s
            .split_once('(')
            .ok_or("颜色须为十六进制或 rgb()/hsv()/oklch()")?;
        let rest = rest.strip_suffix(')').ok_or("未闭合颜色函数")?;
        let tokens = rest
            .split(|c: char| c.is_whitespace() || c == ',' || c == '/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if !matches!(tokens.len(), 3 | 4) {
            return Err("颜色函数需要三个通道和可选透明度".into());
        }
        let number = |s: &str| -> Result<f64, String> {
            if let Some(v) = s.strip_suffix("deg") {
                v.trim().parse().map_err(|_| "无效色相".into())
            } else if let Some(v) = s.strip_suffix("rad") {
                v.trim()
                    .parse::<f64>()
                    .map(f64::to_degrees)
                    .map_err(|_| "无效色相".into())
            } else {
                s.parse().map_err(|_| "无效颜色通道".into())
            }
        };
        let c = [number(tokens[0])?, number(tokens[1])?, number(tokens[2])?];
        // Only hue channels may carry angle units.
        for (i, t) in tokens.iter().enumerate() {
            if (t.ends_with("deg") || t.ends_with("rad"))
                && !((space == "hsv" && i == 0) || (space == "oklch" && i == 2))
            {
                return Err("角度单位只用于色相".into());
            }
        }
        Self::new(
            if space == "rgba" { "rgb" } else { space },
            c,
            if tokens.len() == 4 {
                number(tokens[3])?
            } else {
                1.
            },
        )
    }
    pub fn css(&self) -> String {
        let [r, g, b, a] = self.rgba;
        if a == 1. {
            format!(
                "#{:02x}{:02x}{:02x}",
                (r * 255.).round() as u8,
                (g * 255.).round() as u8,
                (b * 255.).round() as u8
            )
        } else {
            format!("rgba({},{},{},{})", r * 255., g * 255., b * 255., a)
        }
    }
    pub fn channels_in(&self, space: &str) -> [f64; 3] {
        if self.space == space {
            return self.channels;
        }
        let [r, g, b, _] = self.rgba;
        match space {
            "hsv" => {
                let max = r.max(g).max(b);
                let min = r.min(g).min(b);
                let d = max - min;
                let h = if d < 1e-12 {
                    0.
                } else if max == r {
                    60. * ((g - b) / d).rem_euclid(6.)
                } else if max == g {
                    60. * ((b - r) / d + 2.)
                } else {
                    60. * ((r - g) / d + 4.)
                };
                [h, if max == 0. { 0. } else { d / max }, max]
            }
            "oklch" => {
                let [l, a, b] = oklab([r, g, b]);
                [l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.)]
            }
            _ => [r * 255., g * 255., b * 255.],
        }
    }
    pub fn hex(&self) -> String {
        let [r, g, b, a] = self.rgba;
        format!(
            "#{:02x}{:02x}{:02x}{}",
            (r * 255.).round() as u8,
            (g * 255.).round() as u8,
            (b * 255.).round() as u8,
            if a < 1. {
                format!("{:02x}", (a * 255.).round() as u8)
            } else {
                String::new()
            }
        )
    }
}
pub fn css(s: &str) -> String {
    Color::parse(s)
        .map(|c| if s == "none" { "none".into() } else { c.css() })
        .unwrap_or_else(|_| s.into())
}
pub fn visible(s: &str) -> bool {
    s != "none" && Color::parse(s).map_or(true, |c| c.alpha > 0.)
}

// Static recognition for editor colors and constant LCSS endpoint values.
fn constant(e: &crate::parser::Expr) -> Option<f64> {
    use crate::parser::ExprKind;
    match &e.kind {
        ExprKind::Number(v, u) if u.is_empty() || u == "deg" => Some(*v),
        ExprKind::Number(v, u) if u == "rad" => Some(v.to_degrees()),
        ExprKind::Unary(op, v) if op == "-" => Some(-constant(v)?),
        ExprKind::Unary(op, v) if op == "+" => constant(v),
        _ => None,
    }
}
pub fn constant_expression(e: &crate::parser::Expr) -> Option<Color> {
    use crate::parser::ExprKind;
    let ExprKind::Call(parts, args) = &e.kind else {
        return None;
    };
    let name = parts.first()?.as_str();
    if parts.len() != 1 || !["rgb", "hsv", "oklch"].contains(&name) {
        return None;
    }
    let names = match name {
        "rgb" => ["r", "g", "b"],
        "hsv" => ["h", "s", "v"],
        _ => ["l", "c", "h"],
    };
    let mut c = [None; 3];
    let mut alpha = 1.;
    let mut has_alpha = false;
    let mut positional = 0;
    for (k, v) in args {
        let key = k
            .as_deref()
            .unwrap_or_else(|| names.get(positional).copied().unwrap_or(""));
        let hue = (name == "hsv" && key == "h") || (name == "oklch" && key == "h");
        fn angle(e: &crate::parser::Expr) -> bool {
            match &e.kind {
                ExprKind::Number(_, u) => !u.is_empty(),
                ExprKind::Unary(_, v) => angle(v),
                _ => false,
            }
        }
        if !hue && angle(&v) {
            return None;
        }
        let value = constant(v)?;
        if let Some(k) = k {
            if k == "alpha" {
                if has_alpha {
                    return None;
                }
                has_alpha = true;
                alpha = value
            } else {
                let i = names.iter().position(|n| *n == k)?;
                if c[i].is_some() {
                    return None;
                }
                c[i] = Some(value)
            }
        } else {
            if positional >= 3 || c[positional].is_some() {
                return None;
            }
            c[positional] = Some(value);
            positional += 1
        }
    }
    Color::new(name, [c[0]?, c[1]?, c[2]?], alpha).ok()
}
