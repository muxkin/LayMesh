//! CSS Images 3/4 gradients, resolved only after the paint box is known.
use crate::gradient_color::SourceColor;
use crate::{
    animation::{
        css::split,
        values::{self, Context},
    },
    gradient_color as colors,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Units {
    pub dpi: f64,
    pub font: f64,
    pub root_font: f64,
    pub viewport: [f64; 2],
}
impl From<Context> for Units {
    fn from(c: Context) -> Self {
        Self {
            dpi: c.dpi,
            font: c.font,
            root_font: c.root_font,
            viewport: c.viewport,
        }
    }
}
impl Units {
    pub fn context(&self, reference: f64) -> Context {
        Context {
            dpi: self.dpi,
            font: self.font,
            root_font: self.root_font,
            viewport: self.viewport,
            reference,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub anchor: f64,
    pub offset: String,
    pub sign: f64,
}
impl Coordinate {
    pub fn resolve(&self, units: &Units, reference: f64) -> f64 {
        self.anchor * reference
            + self.sign
                * values::length_context(&self.offset, units.context(reference))
                    .expect("validated position")
    }
}
fn coordinate(word: &str, horizontal: bool, ctx: Context) -> Result<Coordinate, String> {
    let anchor = match word {
        "center" => Some(0.5),
        "left" if horizontal => Some(0.),
        "right" if horizontal => Some(1.),
        "top" if !horizontal => Some(0.),
        "bottom" if !horizontal => Some(1.),
        _ => None,
    };
    if let Some(anchor) = anchor {
        Ok(Coordinate {
            anchor,
            offset: "0".into(),
            sign: 1.,
        })
    } else {
        values::length_context(
            word,
            Context {
                reference: 1.,
                ..ctx
            },
        )?;
        Ok(Coordinate {
            anchor: 0.,
            offset: word.into(),
            sign: 1.,
        })
    }
}
pub fn position(words: &[String], ctx: Context) -> Result<[Coordinate; 2], String> {
    let normalized: Vec<String> = words.iter().map(|s| s.to_ascii_lowercase()).collect();
    let words = normalized.as_slice();
    let center = coordinate("center", true, ctx)?;
    if words.is_empty() {
        return Ok([center.clone(), center]);
    }
    if words.len() <= 2 {
        let mut w = words.to_vec();
        if w.len() == 1 {
            w.push("center".into());
        }
        if matches!(w[0].as_str(), "top" | "bottom") || matches!(w[1].as_str(), "left" | "right") {
            if w.iter().any(|v| {
                values::length_context(
                    v,
                    Context {
                        reference: 1.,
                        ..ctx
                    },
                )
                .is_ok()
            }) {
                return Err("Numeric positions cannot swap axes".into());
            }
            w.swap(0, 1);
        }
        return Ok([
            coordinate(&w[0], true, ctx)?,
            coordinate(&w[1], false, ctx)?,
        ]);
    }
    if words.len() > 4 {
        return Err("Position needs one to four components".into());
    }
    let mut out: [Option<Coordinate>; 2] = [None, None];
    let mut i = 0;
    while i < words.len() {
        let word = words[i].as_str();
        let axis = match word {
            "left" | "right" => 0,
            "top" | "bottom" => 1,
            "center" => {
                if out[0].is_none() {
                    0
                } else {
                    1
                }
            }
            _ => return Err("Expected position edge".into()),
        };
        if out[axis].is_some() {
            return Err("Duplicate position axis".into());
        }
        let mut c = coordinate(word, axis == 0, ctx)?;
        i += 1;
        if i < words.len()
            && values::length_context(
                &words[i],
                Context {
                    reference: 1.,
                    ..ctx
                },
            )
            .is_ok()
        {
            if word == "center" {
                return Err("Center cannot take an edge offset".into());
            }
            c.offset = words[i].clone();
            c.sign = if matches!(word, "right" | "bottom") {
                -1.
            } else {
                1.
            };
            i += 1;
        }
        out[axis] = Some(c);
    }
    Ok([
        out[0].take().ok_or("Missing horizontal position")?,
        out[1].take().ok_or("Missing vertical position")?,
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorStop {
    pub color: Option<SourceColor>,
    pub position: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gradient {
    pub kind: String,
    pub repeating: bool,
    pub angle: f64,
    pub corner: Option<[f64; 2]>,
    pub center: [Coordinate; 2],
    pub circle: bool,
    pub size: Vec<String>,
    pub color_space: String,
    pub hue: String,
    pub stops: Vec<ColorStop>,
    pub units: Units,
}
#[derive(Clone, Debug)]
pub enum Geometry {
    Linear { start: [f64; 2], end: [f64; 2] },
    Radial { center: [f64; 2], radii: [f64; 2] },
    Conic { center: [f64; 2], angle: f64 },
}
#[derive(Clone, Debug)]
pub struct Resolved {
    pub geometry: Geometry,
    pub stops: Vec<(f64, [f64; 4])>,
    pub repeating: bool,
    pub range: [f64; 2],
}

fn stop_position(s: &str, angular: bool, ctx: Context, reference: f64) -> Result<f64, String> {
    if angular {
        values::angle_percentage(s)
    } else {
        values::length_context(s, Context { reference, ..ctx }).map(|v| v / reference.max(1e-12))
    }
}
pub fn parse(s: &str, ctx: Context, current: &str) -> Result<Json, String> {
    let (name, body) = s
        .trim()
        .split_once('(')
        .ok_or("Expected gradient function")?;
    let body = body.strip_suffix(')').ok_or("Unclosed gradient")?;
    let lower = name.to_ascii_lowercase();
    let (repeating, name) = lower
        .strip_prefix("repeating-")
        .map(|s| (true, s))
        .unwrap_or((false, lower.as_str()));
    let kind = match name {
        "linear-gradient" => "linear",
        "radial-gradient" => "radial",
        "conic-gradient" => "conic",
        _ => return Err("Expected linear, radial or conic gradient".into()),
    };
    let mut args = split(body, ',');
    if args.is_empty() {
        return Err("Gradient needs at least two colors".into());
    }
    let first = values::words(&args[0]);
    let has_prelude = first
        .first()
        .is_some_and(|s| colors::parse(s, current).is_err());
    let mut prelude = if has_prelude {
        values::words(&args.remove(0))
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect()
    } else {
        vec![]
    };
    let mut cs = None;
    let mut hue = "shorter".to_string();
    if let Some(i) = prelude.iter().position(|v| v == "in") {
        let value = prelude
            .get(i + 1)
            .ok_or("Missing gradient color space")?
            .clone();
        colors::space(&value)?;
        let mut end = i + 2;
        if prelude.get(end + 1).is_some_and(|v| v == "hue") {
            hue = prelude[end].clone();
            colors::hue(&hue)?;
            if !matches!(value.as_str(), "hsl" | "hwb" | "lch" | "oklch") {
                return Err("Hue interpolation requires a polar color space".into());
            }
            end += 2;
        }
        prelude.drain(i..end);
        cs = Some(value);
    }
    let mut g = Gradient {
        kind: kind.into(),
        repeating,
        angle: if kind == "linear" { PI } else { 0. },
        corner: None,
        center: position(&[], ctx)?,
        circle: false,
        size: vec!["farthest-corner".into()],
        color_space: String::new(),
        hue,
        stops: vec![],
        units: ctx.into(),
    };
    if kind == "linear" {
        if prelude.first().is_some_and(|s| s == "to") {
            let mut d: [f64; 2] = [0., 0.];
            for w in &prelude[1..] {
                let (i, v) = match w.as_str() {
                    "top" => (1, -1.),
                    "bottom" => (1, 1.),
                    "left" => (0, -1.),
                    "right" => (0, 1.),
                    _ => return Err("Invalid gradient direction".into()),
                };
                if d[i] != 0. {
                    return Err("Duplicate gradient direction".into());
                }
                d[i] = v;
            }
            if prelude.len() < 2 || prelude.len() > 3 {
                return Err("Invalid gradient direction".into());
            }
            g.angle = d[0].atan2(-d[1]);
            if d[0] != 0. && d[1] != 0. {
                g.corner = Some(d);
            }
        } else if prelude.len() == 1 {
            g.angle = values::angle(&prelude[0])?;
        } else if !prelude.is_empty() {
            return Err("Invalid linear gradient prelude".into());
        }
    } else {
        if let Some(i) = prelude.iter().position(|s| s == "at") {
            if i + 1 == prelude.len() {
                return Err("Missing gradient center".into());
            }
            g.center = position(&prelude[i + 1..], ctx)?;
            prelude.truncate(i);
        }
        if kind == "conic" {
            if !prelude.is_empty() {
                if prelude.len() != 2 || prelude[0] != "from" {
                    return Err("Conic gradient expects from <angle>".into());
                }
                g.angle = values::angle(&prelude[1])?;
            }
        } else {
            let mut shape = None;
            if let Some(i) = prelude.iter().position(|s| s == "circle" || s == "ellipse") {
                shape = Some(prelude.remove(i) == "circle");
            }
            if !prelude.is_empty() {
                if prelude.len() == 1
                    && matches!(
                        prelude[0].as_str(),
                        "closest-side" | "farthest-side" | "closest-corner" | "farthest-corner"
                    )
                {
                } else {
                    if prelude.len() > 2 {
                        return Err("Invalid radial size".into());
                    }
                    for w in &prelude {
                        if values::length_context(
                            w,
                            Context {
                                reference: 1.,
                                ..ctx
                            },
                        )? < 0.
                            && !["calc(", "min(", "max(", "clamp("]
                                .iter()
                                .any(|f| w.starts_with(f))
                        {
                            return Err("Radial size cannot be negative".into());
                        }
                    }
                    let circle = shape.unwrap_or(prelude.len() == 1);
                    if circle && (prelude.len() != 1 || prelude[0].contains('%'))
                        || !circle && prelude.len() != 2
                    {
                        return Err(
                            "Circle needs a length; ellipse needs two length-percentages".into(),
                        );
                    }
                    g.circle = circle;
                }
                g.size = prelude;
            }
            if let Some(circle) = shape {
                g.circle = circle;
            }
        }
    }
    let mut all_legacy = true;
    for arg in args {
        let words = values::words(&arg);
        if words.is_empty() || words.len() > 3 {
            return Err("Invalid gradient stop".into());
        }
        match colors::parse(&words[0], current) {
            Ok(color) => {
                all_legacy &= colors::legacy(&words[0]);
                if words.len() == 1 {
                    g.stops.push(ColorStop {
                        color: Some(color),
                        position: None,
                    });
                } else {
                    for w in &words[1..] {
                        stop_position(w, kind == "conic", ctx, 1.)?;
                        g.stops.push(ColorStop {
                            color: Some(color),
                            position: Some(w.clone()),
                        });
                    }
                }
            }
            Err(e) => {
                if words.len() != 1 || g.stops.last().is_none_or(|s| s.color.is_none()) {
                    return Err(e);
                }
                stop_position(&words[0], kind == "conic", ctx, 1.)?;
                g.stops.push(ColorStop {
                    color: None,
                    position: Some(words[0].clone()),
                });
            }
        }
    }
    if g.stops.iter().filter(|s| s.color.is_some()).count() < 1
        || g.stops.last().is_none_or(|s| s.color.is_none())
    {
        return Err("Gradient needs first and last colors".into());
    }
    if g.stops.len() == 1 {
        g.stops.push(g.stops[0].clone());
    }
    g.color_space = cs.unwrap_or_else(|| if all_legacy { "srgb" } else { "oklab" }.into());
    Ok(json!({"kind":format!("{}Gradient",g.kind),"gradient":g}))
}

impl Gradient {
    pub fn resolve(&self, b: [f64; 4]) -> Resolved {
        let [x, y, w, h] = b;
        let (w, h) = (w.abs().max(1e-12), h.abs().max(1e-12));
        let c = [
            x + self.center[0].resolve(&self.units, w),
            y + self.center[1].resolve(&self.units, h),
        ];
        let mut zero_height = false;
        let (geometry, reference) = match self.kind.as_str() {
            "linear" => {
                let (mut dx, mut dy) = (self.angle.sin(), -self.angle.cos());
                if let Some(d) = self.corner {
                    dx = d[0] * h;
                    dy = d[1] * w;
                    let len = dx.hypot(dy);
                    dx /= len;
                    dy /= len;
                }
                let len = dx.abs() * w + dy.abs() * h;
                (
                    Geometry::Linear {
                        start: [x + w / 2. - dx * len / 2., y + h / 2. - dy * len / 2.],
                        end: [x + w / 2. + dx * len / 2., y + h / 2. + dy * len / 2.],
                    },
                    len,
                )
            }
            "conic" => (
                Geometry::Conic {
                    center: c,
                    angle: self.angle,
                },
                1.,
            ),
            _ => {
                let (cx, cy) = (c[0] - x, c[1] - y);
                let near = self.size[0].starts_with("closest");
                let choose = |a: f64, b: f64| if near { a.min(b) } else { a.max(b) };
                let mut rx = choose(cx.abs(), (w - cx).abs());
                let mut ry = choose(cy.abs(), (h - cy).abs());
                if self.size[0].ends_with("side") || self.size[0].ends_with("corner") {
                    if self.circle {
                        let r = if self.size[0].ends_with("corner") {
                            rx.hypot(ry)
                        } else {
                            choose(rx, ry)
                        };
                        rx = r;
                        ry = r;
                    } else if self.size[0].ends_with("corner") {
                        rx *= 2f64.sqrt();
                        ry *= 2f64.sqrt();
                    }
                } else {
                    rx = values::length_context(&self.size[0], self.units.context(w))
                        .expect("validated size");
                    ry = if self.circle {
                        rx
                    } else {
                        values::length_context(&self.size[1], self.units.context(h))
                            .expect("validated size")
                    };
                }
                if !self.circle && rx <= 0. {
                    rx = 1e-7;
                    ry = 1e12;
                } else if !self.circle && ry <= 0. {
                    zero_height = true;
                    ry = 1e-7;
                }
                (
                    Geometry::Radial {
                        center: c,
                        radii: [rx.max(1e-7), ry.max(1e-7)],
                    },
                    rx.max(1e-7),
                )
            }
        };
        let mut positions: Vec<Option<f64>> = self
            .stops
            .iter()
            .map(|s| {
                s.position.as_ref().map(|p| {
                    stop_position(
                        p,
                        self.kind == "conic",
                        self.units.context(reference),
                        reference,
                    )
                    .expect("validated stop")
                })
            })
            .collect();
        positions[0].get_or_insert(0.);
        positions.last_mut().unwrap().get_or_insert(1.);
        let mut previous = f64::NEG_INFINITY;
        for p in positions.iter_mut().flatten() {
            *p = p.max(previous);
            previous = *p;
        }
        let indices: Vec<usize> = self
            .stops
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.color.map(|_| i))
            .collect();
        // Hints are already positioned anchors in the stop list. Keep them
        // fixed when distributing a run of omitted color positions; otherwise
        // e.g. red, 80%, blue, lime would cease to be monotonic.
        let mut i = 0;
        while i < positions.len() - 1 {
            let j = (i + 1..positions.len())
                .find(|&j| positions[j].is_some())
                .unwrap();
            let a = positions[i].unwrap();
            let z = positions[j].unwrap();
            for k in i + 1..j {
                positions[k] = Some(a + (z - a) * (k - i) as f64 / (j - i) as f64);
            }
            i = j;
        }
        let positions: Vec<f64> = positions.into_iter().map(Option::unwrap).collect();
        let range = [positions[0], *positions.last().unwrap()];
        let mut stops = vec![];
        for pair in indices.windows(2) {
            let (i, j) = (pair[0], pair[1]);
            let (a, b) = (positions[i], positions[j]);
            let interp = colors::interpolator(
                self.stops[i].color.unwrap(),
                self.stops[j].color.unwrap(),
                &self.color_space,
                &self.hue,
            );
            let hint = if j > i + 1 && b > a {
                Some(((positions[i + 1] - a) / (b - a)).clamp(0., 1.))
            } else {
                None
            };
            let evaluate = |t: f64| {
                let t = if let Some(h) = hint {
                    if h <= 0. {
                        if t <= 0. { 0. } else { 1. }
                    } else if h >= 1. {
                        if t >= 1. { 1. } else { 0. }
                    } else {
                        t.powf(0.5f64.ln() / h.ln())
                    }
                } else {
                    t
                };
                colors::rgba(interp.eval(t as f32))
            };
            let ca = evaluate(0.);
            let cb = evaluate(1.);
            stops.push((a, ca));
            if hint == Some(0.) {
                stops.push((a, cb));
                stops.push((b, cb));
                continue;
            }
            if hint == Some(1.) {
                stops.push((b, ca));
                stops.push((b, cb));
                continue;
            }
            if b > a {
                sample_interval(&evaluate, 0., ca, 1., cb, 0, &mut |t, c| {
                    stops.push((a + t * (b - a), c))
                });
            } else {
                stops.push((b, cb));
            }
        }
        // The final duplicate at each boundary is intentional: adjacent
        // intervals can have different missing-component substitutions.
        let period = range[1] - range[0];
        if self.repeating && period <= 1e-12 {
            let evenly: Vec<_> = indices
                .iter()
                .enumerate()
                .map(|(i, &j)| (i as f64, colors::source_rgba(self.stops[j].color.unwrap())))
                .collect();
            let c = average(&evenly);
            return Resolved {
                geometry,
                stops: vec![(0., c), (1., c)],
                repeating: false,
                range: [0., 1.],
            };
        }
        if zero_height {
            let c = if self.repeating {
                average(&stops)
            } else {
                stops.last().unwrap().1
            };
            return Resolved {
                geometry,
                stops: vec![(0., c), (1., c)],
                repeating: false,
                range: [0., 1.],
            };
        }
        if self.kind == "conic" {
            // Sweep shaders only address the visible turn.
            stops = visible_stops(&stops, self.repeating, range, 0., 1.);
            return Resolved {
                geometry,
                stops,
                repeating: false,
                range: [0., 1.],
            };
        }
        Resolved {
            geometry,
            stops,
            repeating: self.repeating,
            range,
        }
    }
}
// Approximate the *unpremultiplied* channels consumed by Skia/SVG, but measure
// error after premultiplication. Three probes prevent midpoint-only aliasing
// of hue rotations. Hard edges never enter this subdivision.
fn sample_interval(
    f: &impl Fn(f64) -> [f64; 4],
    a: f64,
    ca: [f64; 4],
    b: f64,
    cb: [f64; 4],
    depth: u32,
    emit: &mut impl FnMut(f64, [f64; 4]),
) {
    let mut error: f64 = 0.;
    for u in [0.25, 0.5, 0.75] {
        let actual = f(a + (b - a) * u);
        let approx: [f64; 4] = std::array::from_fn(|i| ca[i] + (cb[i] - ca[i]) * u);
        for i in 0..4 {
            let alpha = if i == 3 { 1. } else { actual[3] };
            let aa = if i == 3 { 1. } else { approx[3] };
            error = error.max((actual[i] * alpha - approx[i] * aa).abs());
        }
    }
    if error > 0.00075 && depth < 18 {
        let m = (a + b) / 2.;
        let cm = f(m);
        sample_interval(f, a, ca, m, cm, depth + 1, emit);
        sample_interval(f, m, cm, b, cb, depth + 1, emit);
    } else {
        emit(b, cb);
    }
}
fn at(stops: &[(f64, [f64; 4])], t: f64) -> [f64; 4] {
    let j = stops.partition_point(|s| s.0 <= t);
    if j == 0 {
        return stops[0].1;
    }
    if j == stops.len() {
        return stops[j - 1].1;
    }
    let (a, ca) = stops[j - 1];
    let (b, cb) = stops[j];
    let u = (t - a) / (b - a);
    std::array::from_fn(|i| ca[i] + (cb[i] - ca[i]) * u)
}
fn average(stops: &[(f64, [f64; 4])]) -> [f64; 4] {
    let period = stops.last().unwrap().0 - stops[0].0;
    if period <= 0. {
        return stops.last().unwrap().1;
    }
    let mut avg = [0.; 4];
    for pair in stops.windows(2) {
        let w = (pair[1].0 - pair[0].0) / period;
        for (i, v) in avg.iter_mut().enumerate() {
            let a = pair[0].1;
            let b = pair[1].1;
            *v += w * if i == 3 {
                (a[3] + b[3]) / 2.
            } else {
                (a[i] * a[3] + b[i] * b[3]) / 2.
            };
        }
    }
    if avg[3] > 0. {
        for i in 0..3 {
            avg[i] /= avg[3];
        }
    }
    avg
}
pub fn visible_stops(
    stops: &[(f64, [f64; 4])],
    repeat: bool,
    range: [f64; 2],
    lo: f64,
    hi: f64,
) -> Vec<(f64, [f64; 4])> {
    let period = range[1] - range[0];
    if repeat && period <= 1e-12 {
        return vec![(lo, stops.last().unwrap().1), (hi, stops.last().unwrap().1)];
    }
    let mut out = vec![];
    if repeat {
        let first = ((lo - range[1]) / period).floor() as i64;
        let last = ((hi - range[0]) / period).ceil() as i64;
        // Subpixel repeats converge to their integrated color, rather than
        // allocating an unbounded number of angular stops.
        if last as i128 - first as i128 > 16384 {
            let avg = average(stops);
            return vec![(lo, avg), (hi, avg)];
        }
        for n in first..=last {
            for &(t, c) in stops {
                out.push((t + n as f64 * period, c));
            }
        }
    } else {
        out.extend_from_slice(stops);
    }
    let ca = at(&out, lo);
    let cb = at(&out, hi);
    out.retain(|s| s.0 >= lo && s.0 <= hi);
    if out.first().is_none_or(|s| s.0 > lo) {
        out.insert(0, (lo, ca));
    }
    if out.last().is_none_or(|s| s.0 < hi) {
        out.push((hi, cb));
    }
    out
}

pub fn from_json(p: &Json) -> Result<Gradient, String> {
    serde_json::from_value(p["gradient"].clone()).map_err(|e| format!("Invalid gradient: {e}"))
}
