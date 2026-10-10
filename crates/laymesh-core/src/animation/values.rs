use super::css;
use cssparser::{Parser, Token};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quantity {
    n: f64,
    p: f64,
    dim: u8,
}
impl Quantity {
    fn scalar(n: f64) -> Self {
        Self { n, p: 0., dim: 0 }
    }
    fn add(self, b: Self, sign: f64) -> std::result::Result<Self, String> {
        let dim = if self.dim == b.dim {
            self.dim
        } else if matches!((self.dim, b.dim), (1, 4) | (4, 1)) {
            1
        } else {
            return Err("Incompatible calc() dimensions".into());
        };
        Ok(Self {
            n: self.n + b.n * sign,
            p: self.p + b.p * sign,
            dim,
        })
    }
    fn mul(self, b: Self, divide: bool) -> std::result::Result<Self, String> {
        if divide {
            if b.dim != 0 || b.n == 0. {
                return Err("calc() divisor must be a nonzero number".into());
            }
            return Ok(Self {
                n: self.n / b.n,
                p: self.p / b.n,
                dim: self.dim,
            });
        }
        if self.dim == 0 {
            Ok(Self {
                n: b.n * self.n,
                p: b.p * self.n,
                dim: b.dim,
            })
        } else if b.dim == 0 {
            b.mul(self, false)
        } else {
            Err("calc() multiplication requires a number".into())
        }
    }
}
fn sum(p: &mut Parser<'_>) -> std::result::Result<Quantity, String> {
    let mut v = product(p)?;
    loop {
        let state = p.state();
        match p.next().cloned() {
            Ok(Token::Delim('+')) => v = v.add(product(p)?, 1.)?,
            Ok(Token::Delim('-')) => v = v.add(product(p)?, -1.)?,
            _ => {
                p.reset(&state);
                break;
            }
        }
    }
    Ok(v)
}
fn product(p: &mut Parser<'_>) -> std::result::Result<Quantity, String> {
    let mut v = atom(p)?;
    loop {
        let state = p.state();
        match p.next().cloned() {
            Ok(Token::Delim('*')) => v = v.mul(atom(p)?, false)?,
            Ok(Token::Delim('/')) => v = v.mul(atom(p)?, true)?,
            _ => {
                p.reset(&state);
                break;
            }
        }
    }
    Ok(v)
}
fn atom(p: &mut Parser<'_>) -> std::result::Result<Quantity, String> {
    match p
        .next()
        .cloned()
        .map_err(|_| "Expected CSS numeric value")?
    {
        Token::Number { value, .. } => Ok(Quantity::scalar(value as f64)),
        Token::Percentage { unit_value, .. } => Ok(Quantity {
            n: 0.,
            p: unit_value as f64,
            dim: 4,
        }),
        Token::Dimension { value, unit, .. } => {
            let (f, dim) = match unit.to_ascii_lowercase().as_str() {
                "px" => (25.4 / 96., 1),
                "mm" => (1., 1),
                "cm" => (10., 1),
                "in" => (25.4, 1),
                "pt" => (25.4 / 72., 1),
                "pc" => (25.4 / 6., 1),
                "q" => (0.25, 1),
                "s" => (1., 2),
                "ms" => (0.001, 2),
                "deg" => (std::f64::consts::PI / 180., 3),
                "rad" => (1., 3),
                "grad" => (std::f64::consts::PI / 200., 3),
                "turn" => (std::f64::consts::TAU, 3),
                _ => return Err(format!("Unsupported CSS unit: {unit}")),
            };
            Ok(Quantity {
                n: value as f64 * f,
                p: 0.,
                dim,
            })
        }
        Token::Function(ref s) if s.eq_ignore_ascii_case("calc") => nested_quantity(p),
        Token::ParenthesisBlock => nested_quantity(p),
        Token::Delim('-') => Ok(atom(p)?.mul(Quantity::scalar(-1.), false)?),
        Token::Delim('+') => atom(p),
        _ => Err("Expected number, dimension, or calc()".into()),
    }
}
fn nested_quantity(p: &mut Parser<'_>) -> std::result::Result<Quantity, String> {
    p.parse_nested_block(|p| {
        sum(p).map_err(|s| cssparser::ParseError {
            kind: cssparser::ParseErrorKind::Custom(s),
        })
    })
    .map_err(|e| format!("Invalid calc(): {e:?}"))
}
fn quantity(s: &str) -> std::result::Result<Quantity, String> {
    let mut p = Parser::new(s);
    let q = sum(&mut p)?;
    p.expect_exhausted()
        .map_err(|_| format!("Unexpected CSS numeric suffix: {s}"))?;
    if !q.n.is_finite() || !q.p.is_finite() {
        return Err("CSS value must be finite".into());
    }
    Ok(q)
}
pub fn number(s: &str) -> std::result::Result<f64, String> {
    let q = quantity(s)?;
    if q.dim != 0 {
        return Err("Expected unitless number".into());
    }
    Ok(q.n)
}
pub fn time(s: &str) -> std::result::Result<f64, String> {
    let q = quantity(s)?;
    if q.dim != 2 {
        return Err("Time requires s or ms".into());
    }
    Ok(q.n)
}
pub fn length(s: &str, reference: f64) -> std::result::Result<f64, String> {
    let q = quantity(s)?;
    if !matches!(q.dim, 1 | 4) && !(q.dim == 0 && q.n == 0.) {
        return Err("CSS length requires a unit".into());
    }
    Ok(q.n + q.p * reference)
}
pub fn angle(s: &str) -> std::result::Result<f64, String> {
    let q = quantity(s)?;
    if q.dim != 3 && !(q.dim == 0 && q.n == 0.) {
        return Err("Angle requires deg/rad/grad/turn".into());
    }
    Ok(q.n)
}
pub fn words(s: &str) -> Vec<String> {
    css::split(&s.replace(['\n', '\r', '\t'], " "), ' ')
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Bezier([f64; 4]),
    Steps(u32, String),
}
impl Default for Easing {
    fn default() -> Self {
        Self::Bezier([0.25, 0.1, 0.25, 1.])
    }
}
impl Easing {
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        let b = match s.trim() {
            "linear" => Some([0., 0., 1., 1.]),
            "ease" => Some([0.25, 0.1, 0.25, 1.]),
            "ease-in" => Some([0.42, 0., 1., 1.]),
            "ease-out" => Some([0., 0., 0.58, 1.]),
            "ease-in-out" => Some([0.42, 0., 0.58, 1.]),
            "step-start" => return Ok(Self::Steps(1, "start".into())),
            "step-end" => return Ok(Self::Steps(1, "end".into())),
            _ => None,
        };
        if let Some(b) = b {
            return Ok(Self::Bezier(b));
        }
        if let Some(s) = s
            .strip_prefix("cubic-bezier(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let v = css::split(s, ',')
                .iter()
                .map(|v| number(v))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if v.len() == 4 && (0.0..=1.).contains(&v[0]) && (0.0..=1.).contains(&v[2]) {
                return Ok(Self::Bezier([v[0], v[1], v[2], v[3]]));
            }
        }
        if let Some(s) = s.strip_prefix("steps(").and_then(|s| s.strip_suffix(')')) {
            let v = css::split(s, ',');
            let n = number(&v[0])?;
            let pos = v.get(1).map(String::as_str).unwrap_or("end");
            if v.len() <= 2
                && n.fract() == 0.
                && n >= 1.
                && n <= 1_000_000.
                && matches!(
                    pos,
                    "start" | "end" | "jump-start" | "jump-end" | "jump-none" | "jump-both"
                )
                && !(pos == "jump-none" && n < 2.)
            {
                return Ok(Self::Steps(n as u32, pos.into()));
            }
        }
        Err(format!("Invalid animation timing function: {s}"))
    }
    pub fn at(&self, x: f64, before: bool) -> f64 {
        match self {
            Self::Bezier([x1, y1, x2, y2]) => {
                if x <= 0. {
                    return 0.;
                }
                if x >= 1. {
                    return 1.;
                }
                let f = |t: f64, a: f64, b: f64| {
                    3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * b + t * t * t
                };
                let (mut lo, mut hi) = (0., 1.);
                for _ in 0..45 {
                    let m = (lo + hi) / 2.;
                    if f(m, *x1, *x2) < x { lo = m } else { hi = m }
                }
                f((lo + hi) / 2., *y1, *y2)
            }
            Self::Steps(n, pos) => {
                let n = *n as f64;
                let mut current = (x * n).floor();
                if matches!(pos.as_str(), "start" | "jump-start" | "jump-both") {
                    current += 1.;
                }
                if before && (x * n).fract().abs() < 1e-10 {
                    current -= 1.;
                }
                let jumps = n + if pos == "jump-both" {
                    1.
                } else if pos == "jump-none" {
                    -1.
                } else {
                    0.
                };
                current.clamp(0., jumps) / jumps
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub kind: String,
    pub args: Vec<f64>,
}
pub type Matrix = [f64; 16];
pub fn identity() -> Matrix {
    [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ]
}
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut c = [0.; 16];
    for r in 0..4 {
        for col in 0..4 {
            for k in 0..4 {
                c[r * 4 + col] += a[r * 4 + k] * b[k * 4 + col];
            }
        }
    }
    c
}
pub fn translate(x: f64, y: f64, z: f64) -> Matrix {
    let mut m = identity();
    m[3] = x;
    m[7] = y;
    m[11] = z;
    m
}
pub fn transform(s: &str, size: [f64; 2]) -> std::result::Result<Vec<Transform>, String> {
    if s.trim() == "none" {
        return Ok(vec![]);
    }
    let mut p = Parser::new(s);
    let mut out = vec![];
    while !p.is_exhausted() {
        let Token::Function(name) = p
            .next()
            .cloned()
            .map_err(|_| "Expected transform function")?
        else {
            return Err("Expected transform function".into());
        };
        let body = p
            .parse_nested_block(|q| {
                let start = q.position();
                while q.next_including_whitespace_and_comments().is_ok() {}
                Ok::<_, cssparser::ParseError<()>>(q.slice_from(start).to_string())
            })
            .map_err(|e| format!("Invalid transform: {e:?}"))?;
        let name = name.to_ascii_lowercase();
        let parts = css::split(&body, ',');
        let parts = if parts.len() == 1 {
            words(&body)
        } else {
            parts
        };
        let num = |i: usize, default: f64| parts.get(i).map(|s| number(s)).unwrap_or(Ok(default));
        let len =
            |i: usize, reference: f64| parts.get(i).map(|s| length(s, reference)).unwrap_or(Ok(0.));
        let ang = |i: usize| parts.get(i).map(|s| angle(s)).unwrap_or(Ok(0.));
        let (kind, args, min, max) = match name.as_str() {
            "translate" => (
                "translate",
                vec![len(0, size[0])?, len(1, size[1])?, 0.],
                1,
                2,
            ),
            "translatex" => ("translate", vec![len(0, size[0])?, 0., 0.], 1, 1),
            "translatey" => ("translate", vec![0., len(0, size[1])?, 0.], 1, 1),
            "translatez" => ("translate", vec![0., 0., len(0, 0.)?], 1, 1),
            "translate3d" => (
                "translate",
                vec![len(0, size[0])?, len(1, size[1])?, len(2, 0.)?],
                3,
                3,
            ),
            "scale" => ("scale", vec![num(0, 1.)?, num(1, num(0, 1.)?)?, 1.], 1, 2),
            "scalex" => ("scale", vec![num(0, 1.)?, 1., 1.], 1, 1),
            "scaley" => ("scale", vec![1., num(0, 1.)?, 1.], 1, 1),
            "scalez" => ("scale", vec![1., 1., num(0, 1.)?], 1, 1),
            "scale3d" => ("scale", vec![num(0, 1.)?, num(1, 1.)?, num(2, 1.)?], 3, 3),
            "rotate" | "rotatez" => ("rotate", vec![0., 0., 1., ang(0)?], 1, 1),
            "rotatex" => ("rotate", vec![1., 0., 0., ang(0)?], 1, 1),
            "rotatey" => ("rotate", vec![0., 1., 0., ang(0)?], 1, 1),
            "rotate3d" => (
                "rotate",
                vec![num(0, 0.)?, num(1, 0.)?, num(2, 0.)?, ang(3)?],
                4,
                4,
            ),
            "skew" => ("skew", vec![ang(0)?, ang(1)?], 1, 2),
            "skewx" => ("skew", vec![ang(0)?, 0.], 1, 1),
            "skewy" => ("skew", vec![0., ang(0)?], 1, 1),
            "perspective" => {
                let v = len(0, 0.)?;
                if v <= 0. {
                    return Err("perspective() requires positive length".into());
                }
                ("perspective", vec![-1. / v.max(25.4 / 96.)], 1, 1)
            }
            "matrix" => (
                "matrix",
                (0..6)
                    .map(|i| num(i, 0.))
                    .collect::<std::result::Result<Vec<_>, _>>()?,
                6,
                6,
            ),
            "matrix3d" => (
                "matrix3d",
                (0..16)
                    .map(|i| num(i, 0.))
                    .collect::<std::result::Result<Vec<_>, _>>()?,
                16,
                16,
            ),
            _ => return Err(format!("Unsupported transform function: {name}")),
        };
        if parts.len() < min || parts.len() > max {
            return Err(format!("Invalid arguments for {name}"));
        }
        out.push(Transform {
            kind: kind.into(),
            args,
        });
    }
    Ok(out)
}
impl Transform {
    fn neutral(&self) -> Self {
        let mut t = self.clone();
        for x in &mut t.args {
            *x = 0.;
        }
        match t.kind.as_str() {
            "scale" => t.args.fill(1.),
            "rotate" => {
                t.args[..3].copy_from_slice(&self.args[..3]);
            }
            "matrix" => {
                t.args[0] = 1.;
                t.args[3] = 1.;
            }
            "matrix3d" => {
                for i in [0, 5, 10, 15] {
                    t.args[i] = 1.;
                }
            }
            _ => {}
        }
        t
    }
    fn matrix(&self) -> Matrix {
        let a = &self.args;
        let mut m = identity();
        match self.kind.as_str() {
            "translate" => return translate(a[0], a[1], a[2]),
            "scale" => {
                m[0] = a[0];
                m[5] = a[1];
                m[10] = a[2];
            }
            "rotate" => {
                let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                if n > 1e-15 {
                    let (x, y, z) = (a[0] / n, a[1] / n, a[2] / n);
                    let (c, s) = (a[3].cos(), a[3].sin());
                    let t = 1. - c;
                    m[0] = t * x * x + c;
                    m[1] = t * x * y - s * z;
                    m[2] = t * x * z + s * y;
                    m[4] = t * x * y + s * z;
                    m[5] = t * y * y + c;
                    m[6] = t * y * z - s * x;
                    m[8] = t * x * z - s * y;
                    m[9] = t * y * z + s * x;
                    m[10] = t * z * z + c;
                }
            }
            "skew" => {
                m[1] = a[0].tan();
                m[4] = a[1].tan();
            }
            "perspective" => m[14] = a[0],
            "matrix" => {
                m[0] = a[0];
                m[4] = a[1];
                m[1] = a[2];
                m[5] = a[3];
                m[3] = a[4] * 25.4 / 96.;
                m[7] = a[5] * 25.4 / 96.;
            }
            "matrix3d" => {
                for r in 0..4 {
                    for c in 0..4 {
                        m[r * 4 + c] = a[c * 4 + r];
                    }
                }
                for i in [3, 7, 11] {
                    m[i] *= 25.4 / 96.;
                }
                for i in [12, 13, 14] {
                    m[i] *= 96. / 25.4;
                }
            }
            _ => {}
        }
        m
    }
}
pub fn matrix(ts: &[Transform]) -> Matrix {
    ts.iter().fold(identity(), |m, t| multiply(m, t.matrix()))
}
pub fn interpolate(a: &[Transform], b: &[Transform], p: f64) -> Matrix {
    let mut result = identity();
    for i in 0..a.len().max(b.len()) {
        let aa = a.get(i).cloned().unwrap_or_else(|| b[i].neutral());
        let bb = b.get(i).cloned().unwrap_or_else(|| a[i].neutral());
        if aa.kind != bb.kind
            || (aa.kind == "rotate" && aa.args[..3] != bb.args[..3])
            || matches!(aa.kind.as_str(), "matrix" | "matrix3d")
        {
            return multiply(
                result,
                super::matrix::interpolate(
                    matrix(a.get(i..).unwrap_or(&[])),
                    matrix(b.get(i..).unwrap_or(&[])),
                    p,
                ),
            );
        }
        let t = Transform {
            kind: aa.kind,
            args: aa
                .args
                .iter()
                .zip(bb.args)
                .map(|(a, b)| a + (b - a) * p)
                .collect(),
        };
        result = multiply(result, t.matrix());
    }
    result
}
pub fn origin(s: &str, size: [f64; 2]) -> std::result::Result<[f64; 3], String> {
    let mut v = words(s);
    if v.len() == 1 {
        if matches!(v[0].as_str(), "top" | "bottom") {
            v.insert(0, "center".into())
        } else {
            v.push("center".into());
        }
    }
    if v.len() < 2 || v.len() > 3 {
        return Err("Invalid transform-origin".into());
    }
    if matches!(v[0].as_str(), "top" | "bottom") || matches!(v[1].as_str(), "left" | "right") {
        v.swap(0, 1);
    }
    let val = |s: &str, n: f64| match s {
        "center" => Ok(n / 2.),
        "left" | "top" => Ok(0.),
        "right" | "bottom" => Ok(n),
        _ => length(s, n),
    };
    Ok([
        val(&v[0], size[0])?,
        val(&v[1], size[1])?,
        v.get(2).map(|s| length(s, 0.)).unwrap_or(Ok(0.))?,
    ])
}
