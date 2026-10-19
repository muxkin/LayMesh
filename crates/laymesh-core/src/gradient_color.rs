//! Source colors remain floating point and unmapped until after interpolation.
use crate::animation::values;
use csscolor::{ColorSpaceTag as Space, DynamicColor, HueDirection, Interpolator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SourceColor {
    #[serde(flatten)]
    pub value: DynamicColor,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub linear_p3: bool,
}
impl SourceColor {
    fn converted(self, cs: Space) -> DynamicColor {
        if !self.linear_p3 {
            return self.value.convert(cs);
        }
        let [r, g, b, a] = self.value.components;
        let mut result = DynamicColor {
            cs: Space::XyzD65,
            flags: csscolor::Flags::default(),
            components: [
                0.48657095 * r + 0.2656677 * g + 0.19821729 * b,
                0.22897457 * r + 0.69173855 * g + 0.07928691 * b,
                0.04511338 * g + 1.0439444 * b,
                a,
            ],
        }
        .convert(cs);
        let missing = self.value.flags.missing();
        let analogous = matches!(
            cs,
            Space::Srgb
                | Space::LinearSrgb
                | Space::DisplayP3
                | Space::A98Rgb
                | Space::ProphotoRgb
                | Space::Rec2020
                | Space::XyzD50
                | Space::XyzD65
        );
        let missing = if analogous {
            missing
        } else {
            missing & csscolor::Missing::single(3)
        };
        for i in 0..4 {
            if missing.contains(i) {
                result.components[i] = 0.;
            }
        }
        result.flags = csscolor::Flags::from_missing(missing);
        result
    }
    fn linear_components(self) -> DynamicColor {
        if self.linear_p3 {
            return DynamicColor {
                cs: Space::LinearSrgb,
                ..self.value
            };
        }
        let mut c = self.value.convert(Space::DisplayP3);
        for v in &mut c.components[..3] {
            let x = v.abs();
            *v = v.signum()
                * if x <= 0.04045 {
                    x / 12.92
                } else {
                    ((x + 0.055) / 1.055).powf(2.4)
                };
        }
        c.cs = Space::LinearSrgb;
        c
    }
}

pub struct ColorInterpolator {
    value: Interpolator,
    linear_p3: bool,
}
impl ColorInterpolator {
    pub fn eval(&self, t: f32) -> DynamicColor {
        let c = self.value.eval(t);
        if self.linear_p3 {
            SourceColor {
                value: c,
                linear_p3: true,
            }
            .converted(Space::Srgb)
        } else {
            c
        }
    }
}
pub fn source_rgba(c: SourceColor) -> [f64; 4] {
    rgba(c.converted(Space::Srgb))
}

pub fn space(name: &str) -> Result<Space, String> {
    Ok(match name {
        "srgb" => Space::Srgb,
        "srgb-linear" | "display-p3-linear" => Space::LinearSrgb,
        "display-p3" => Space::DisplayP3,
        "a98-rgb" => Space::A98Rgb,
        "prophoto-rgb" => Space::ProphotoRgb,
        "rec2020" => Space::Rec2020,
        "xyz" | "xyz-d65" => Space::XyzD65,
        "xyz-d50" => Space::XyzD50,
        "lab" => Space::Lab,
        "lch" => Space::Lch,
        "oklab" => Space::Oklab,
        "oklch" => Space::Oklch,
        "hsl" => Space::Hsl,
        "hwb" => Space::Hwb,
        _ => return Err(format!("Unsupported gradient color space: {name}")),
    })
}
pub fn hue(name: &str) -> Result<HueDirection, String> {
    Ok(match name {
        "shorter" => HueDirection::Shorter,
        "longer" => HueDirection::Longer,
        "increasing" => HueDirection::Increasing,
        "decreasing" => HueDirection::Decreasing,
        _ => return Err(format!("Invalid hue interpolation: {name}")),
    })
}
pub fn legacy(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    !s.contains('(')
        || ["rgb(", "rgba(", "hsl(", "hsla(", "hwb("]
            .iter()
            .any(|p| s.starts_with(p))
}
// CSS numeric expressions keep floating source channels just like literal
// colors. Resolving calc() through the legacy paint serializer would round
// channels to bytes before interpolation.
fn numeric_color(s: &str) -> Result<DynamicColor, String> {
    use crate::animation::css::split;
    let (name, body) = s.split_once('(').ok_or("Expected a color function")?;
    let body = body.strip_suffix(')').ok_or("Unclosed color")?;
    let name = name.to_ascii_lowercase();
    let commas = split(body, ',');
    let mut words = if commas.len() > 1 {
        if !matches!(name.as_str(), "rgb" | "rgba" | "hsl" | "hsla")
            || !matches!(commas.len(), 3 | 4)
            || split(body, '/').len() > 1
            || commas.iter().any(|c| values::words(c).len() != 1)
        {
            return Err("Invalid comma color syntax".into());
        }
        commas
    } else {
        let halves = split(body, '/');
        if halves.len() > 2 {
            return Err("Duplicate alpha separator".into());
        }
        let mut words = values::words(&halves[0]);
        if words.len() != if name == "color" { 4 } else { 3 } {
            return Err("Expected three color channels".into());
        }
        if halves.len() == 2 {
            let alpha = values::words(&halves[1]);
            if alpha.len() != 1 {
                return Err("Expected one alpha channel".into());
            }
            words.extend(alpha);
        }
        words
    };
    let cs = match name.as_str() {
        "rgb" | "rgba" => Space::Srgb,
        "hsl" | "hsla" => Space::Hsl,
        "hwb" => Space::Hwb,
        "lab" => Space::Lab,
        "lch" => Space::Lch,
        "oklab" => Space::Oklab,
        "oklch" => Space::Oklch,
        "color" => {
            if words.is_empty() {
                return Err("Missing color space".into());
            }
            let name = words.remove(0);
            if !matches!(
                name.as_str(),
                "srgb"
                    | "srgb-linear"
                    | "display-p3"
                    | "a98-rgb"
                    | "prophoto-rgb"
                    | "rec2020"
                    | "xyz"
                    | "xyz-d50"
                    | "xyz-d65"
            ) {
                return Err("Expected predefined rectangular color space".into());
            }
            space(&name)?
        }
        _ => return Err("Unsupported numeric color".into()),
    };
    if !matches!(words.len(), 3 | 4) {
        return Err("Expected three color channels and optional alpha".into());
    }
    let mut flags = csscolor::Missing::default();
    let mut c = [0., 0., 0., 1.];
    for (i, v) in words.iter().enumerate() {
        if v == "none" {
            flags.insert(i);
            c[i] = 0.;
            continue;
        }
        let (factor, hue) = if i == 3 {
            (1., false)
        } else {
            match cs {
                Space::Srgb if name != "color" => (255., false),
                Space::Hsl | Space::Hwb => (if i == 0 { 360. } else { 100. }, i == 0),
                Space::Lab => (if i == 0 { 100. } else { 125. }, false),
                Space::Lch => (if i == 0 { 100. } else { 150. }, i == 2),
                Space::Oklab => (if i == 0 { 1. } else { 0.4 }, false),
                Space::Oklch => (if i == 0 { 1. } else { 0.4 }, i == 2),
                _ => (1., false),
            }
        };
        let mut n = if hue {
            values::angle(v)
                .map(f64::to_degrees)
                .or_else(|_| values::number(v))?
        } else {
            values::number_percentage(v, factor)?
        };
        if cs == Space::Srgb && name != "color" && i < 3 {
            n = n.clamp(0., 255.) / 255.;
        }
        if i == 3 {
            n = n.clamp(0., 1.);
        }
        c[i] = n as f32;
    }
    Ok(DynamicColor {
        cs,
        flags: csscolor::Flags::from_missing(flags),
        components: c,
    })
}
pub fn parse(s: &str, current: &str) -> Result<SourceColor, String> {
    let s = if s.eq_ignore_ascii_case("currentcolor") {
        current
    } else {
        s
    };
    if s.eq_ignore_ascii_case("none") {
        return Err("none is a missing component, not a gradient color".into());
    }
    let normalized = s.to_ascii_lowercase();
    let s = normalized.as_str();
    // Preserve linear P3 channels, including missing components, in their
    // source space. A premature matrix conversion loses component semantics.
    if let Some(body) = s.strip_prefix("color(").and_then(|s| s.strip_suffix(')')) {
        let words = values::words(body);
        if words.first().is_some_and(|w| w == "display-p3-linear") {
            let encoded = s.replacen("display-p3-linear", "display-p3", 1);
            let value = csscolor::parse_color(&encoded)
                .or_else(|_| {
                    numeric_color(&encoded).map_err(|_| csscolor::ParseError::UnknownColorSyntax)
                })
                .map_err(|e| format!("Invalid linear P3 color: {e}"))?;
            return Ok(SourceColor {
                value,
                linear_p3: true,
            });
        }
    }
    csscolor::parse_color(s)
        .or_else(|_| numeric_color(s).map_err(|_| csscolor::ParseError::UnknownColorSyntax))
        .or_else(|_| {
            // Retain LayMesh's pre-existing HSV extension without letting the
            // legacy permissive parser accept malformed CSS color functions.
            if !s.starts_with("hsv(") {
                return Err(csscolor::ParseError::UnknownColorSyntax);
            }
            crate::css_values::color(s, current)
                .ok()
                .and_then(|v| csscolor::parse_color(&v).ok())
                .ok_or(csscolor::ParseError::UnknownColorSyntax)
        })
        .map(|value| SourceColor {
            value,
            linear_p3: false,
        })
        .map_err(|e| format!("Invalid gradient color {s}: {e}"))
}
pub fn interpolator(
    a: SourceColor,
    b: SourceColor,
    cs: &str,
    direction: &str,
) -> ColorInterpolator {
    let linear_p3 = cs == "display-p3-linear";
    let target = space(cs).expect("validated color space");
    let a = if linear_p3 {
        a.linear_components()
    } else {
        a.converted(target)
    };
    let b = if linear_p3 {
        b.linear_components()
    } else {
        b.converted(target)
    };
    ColorInterpolator {
        value: a.interpolate(b, target, hue(direction).expect("validated hue")),
        linear_p3,
    }
}
/// CSS Color 4 local-MINDE gamut mapping. No mapping is applied to source
/// colors before interpolation. The output surface is explicitly sRGB.
pub fn rgba(c: DynamicColor) -> [f64; 4] {
    let rgb = c.convert(Space::Srgb).components;
    let in_gamut = |v: [f32; 4]| v[..3].iter().all(|x| (-1e-6..=1.000001).contains(x));
    let mapped = if in_gamut(rgb) {
        rgb
    } else {
        let mut lch = c.convert(Space::Oklch);
        if lch.components[0] >= 1. {
            [1., 1., 1., rgb[3]]
        } else if lch.components[0] <= 0. {
            [0., 0., 0., rgb[3]]
        } else {
            let (mut lo, mut hi) = (0., lch.components[1]);
            let mut out = rgb;
            let mut min_in_gamut = true;
            for _ in 0..32 {
                if hi - lo < 0.0001 {
                    break;
                }
                let chroma = (lo + hi) / 2.;
                lch.components[1] = chroma;
                let test = lch.convert(Space::Srgb);
                out = test.components.map(|v| v.clamp(0., 1.));
                if min_in_gamut && in_gamut(test.components) {
                    lo = chroma;
                    continue;
                }
                let mut clipped = test;
                clipped.components = out;
                let x = clipped.convert(Space::Oklab).components;
                let y = lch.convert(Space::Oklab).components;
                let delta =
                    ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt();
                if delta < 0.02 {
                    if 0.02 - delta < 0.0001 {
                        break;
                    }
                    min_in_gamut = false;
                    lo = chroma;
                } else {
                    hi = chroma;
                }
            }
            out
        }
    };
    mapped.map(|v| v.clamp(0., 1.) as f64)
}
