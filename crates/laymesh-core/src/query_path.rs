//! Source-order geometry, independent of rendering tessellation.
use kurbo::{
    Affine, Arc, BezPath, CubicBez, Line, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathEl,
    PathSeg, Point, QuadBez, Rect, Vec2,
};
use std::f64::consts::TAU;

pub const EPS: f64 = 0.00001;
#[derive(Clone, Debug)]
pub enum Curve {
    Poly(PathSeg),
    Arc(Arc),
}
#[derive(Clone, Debug)]
pub struct Segment {
    pub curve: Curve,
    pub transform: Affine,
}
#[derive(Clone, Debug)]
pub struct Route {
    pub zero_direction: Option<Vec2>,
    pub segments: Vec<Segment>,
    pub closed: bool,
}
#[derive(Clone, Debug)]
pub struct Paths {
    /// Stable selector identity, never inferred from coincident geometry.
    pub identity: String,
    pub routes: Vec<Route>,
    pub frame: Affine,
    pub local: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub route: usize,
    pub segment: usize,
    pub t: f64,
}
#[derive(Clone, Debug)]
pub struct PathPoint {
    pub paths: std::rc::Rc<Paths>,
    pub position: Position,
    pub side: Option<String>,
}

pub fn vector(tr: Affine, v: Vec2) -> Vec2 {
    tr * v.to_point() - tr * Point::ORIGIN
}
fn coefficients(p: PathSeg) -> Vec<Vec2> {
    match p {
        PathSeg::Line(p) => vec![p.p0.to_vec2(), p.p1 - p.p0],
        PathSeg::Quad(p) => vec![
            p.p0.to_vec2(),
            2. * (p.p1 - p.p0),
            p.p2.to_vec2() - 2. * p.p1.to_vec2() + p.p0.to_vec2(),
        ],
        PathSeg::Cubic(p) => vec![
            p.p0.to_vec2(),
            3. * (p.p1 - p.p0),
            3. * (p.p2.to_vec2() - 2. * p.p1.to_vec2() + p.p0.to_vec2()),
            p.p3.to_vec2() - 3. * p.p2.to_vec2() + 3. * p.p1.to_vec2() - p.p0.to_vec2(),
        ],
    }
}
fn eval(c: &[f64], t: f64) -> f64 {
    c.iter().rev().fold(0., |n, x| n * t + x)
}
pub fn roots(c: &[f64]) -> Vec<f64> {
    let scale = c.iter().fold(0f64, |v, x| v.max(x.abs()));
    if scale == 0. {
        return vec![];
    }
    let mut c = c.iter().map(|v| v / scale).collect::<Vec<_>>();
    while c.len() > 1 && c.last().unwrap().abs() < 1e-14 {
        c.pop();
    }
    if c.len() == 1 {
        return vec![];
    }
    if c.len() == 2 {
        let t = -c[0] / c[1];
        return if (-1e-12..=1. + 1e-12).contains(&t) {
            vec![t.clamp(0., 1.)]
        } else {
            vec![]
        };
    }
    let dc = c
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, x)| *x * i as f64)
        .collect::<Vec<_>>();
    let mut splits = vec![0.];
    splits.extend(roots(&dc));
    splits.push(1.);
    splits.sort_by(f64::total_cmp);
    splits.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let mut out = vec![];
    for &t in &splits {
        if eval(&c, t).abs() < 1e-11 {
            out.push(t);
        }
    }
    for w in splits.windows(2) {
        let (mut lo, mut hi) = (w[0], w[1]);
        let mut a = eval(&c, lo);
        let b = eval(&c, hi);
        if a * b >= 0. {
            continue;
        }
        for _ in 0..60 {
            let mid = (lo + hi) / 2.;
            let v = eval(&c, mid);
            if a * v <= 0. {
                hi = mid;
            } else {
                lo = mid;
                a = v;
            }
        }
        out.push((lo + hi) / 2.);
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    out
}
fn trig_roots(a: f64, b: f64, c: f64, start: f64, sweep: f64) -> Vec<f64> {
    let r = a.hypot(b);
    if r < 1e-14 || sweep.abs() < 1e-14 {
        return vec![];
    }
    let z = -c / r;
    if z.abs() > 1. + 1e-12 {
        return vec![];
    }
    let phi = b.atan2(a);
    let delta = z.clamp(-1., 1.).acos();
    let mut out = vec![];
    let lo = start.min(start + sweep);
    let hi = start.max(start + sweep);
    for angle in [phi + delta, phi - delta] {
        for k in ((lo - angle) / TAU).ceil() as i64..=((hi - angle) / TAU).floor() as i64 {
            out.push(((angle + k as f64 * TAU - start) / sweep).clamp(0., 1.));
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    out
}
impl Segment {
    pub fn point(&self, t: f64) -> Point {
        self.transform
            * match self.curve {
                Curve::Poly(p) => p.eval(t),
                Curve::Arc(a) => {
                    let v = a.start_angle + a.sweep_angle * t;
                    let (s, c) = v.sin_cos();
                    let (sr, cr) = a.x_rotation.sin_cos();
                    Point::new(
                        a.center.x + a.radii.x * c * cr - a.radii.y * s * sr,
                        a.center.y + a.radii.x * c * sr + a.radii.y * s * cr,
                    )
                }
            }
    }
    pub fn deriv(&self, t: f64) -> Vec2 {
        vector(
            self.transform,
            match self.curve {
                Curve::Poly(PathSeg::Line(p)) => p.deriv().eval(t).to_vec2(),
                Curve::Poly(PathSeg::Quad(p)) => p.deriv().eval(t).to_vec2(),
                Curve::Poly(PathSeg::Cubic(p)) => p.deriv().eval(t).to_vec2(),
                Curve::Arc(a) => {
                    let v = a.start_angle + a.sweep_angle * t;
                    let (s, c) = v.sin_cos();
                    let (sr, cr) = a.x_rotation.sin_cos();
                    Vec2::new(
                        -a.radii.x * s * cr - a.radii.y * c * sr,
                        -a.radii.x * s * sr + a.radii.y * c * cr,
                    ) * a.sweep_angle
                }
            },
        )
    }
    pub fn second(&self, t: f64) -> Vec2 {
        vector(
            self.transform,
            match self.curve {
                Curve::Poly(PathSeg::Line(_)) => Vec2::ZERO,
                Curve::Poly(PathSeg::Quad(p)) => p.deriv().deriv().eval(t).to_vec2(),
                Curve::Poly(PathSeg::Cubic(p)) => p.deriv().deriv().eval(t).to_vec2(),
                Curve::Arc(a) => {
                    let v = a.start_angle + a.sweep_angle * t;
                    let (s, c) = v.sin_cos();
                    let (sr, cr) = a.x_rotation.sin_cos();
                    Vec2::new(
                        -a.radii.x * c * cr + a.radii.y * s * sr,
                        -a.radii.x * c * sr - a.radii.y * s * cr,
                    ) * a.sweep_angle.powi(2)
                }
            },
        )
    }
    pub fn measured(&self, paths: &Paths) -> Self {
        let mut s = self.clone();
        if paths.local {
            s.transform = paths.frame.inverse() * s.transform;
        }
        s
    }
    pub fn controls(&self) -> Vec<Point> {
        match self.curve {
            Curve::Poly(PathSeg::Quad(p)) => vec![self.transform * p.p1],
            Curve::Poly(PathSeg::Cubic(p)) => vec![self.transform * p.p1, self.transform * p.p2],
            _ => vec![],
        }
    }
    pub fn length(&self, to: f64) -> f64 {
        if to <= 0. {
            return 0.;
        }
        if let Curve::Poly(p) = self.curve {
            return (self.transform * p).subsegment(0.0..to).arclen(EPS / 10.);
        }
        fn integrate(
            s: &Segment,
            a: f64,
            b: f64,
            fa: f64,
            fm: f64,
            fb: f64,
            whole: f64,
            tol: f64,
            depth: u32,
        ) -> f64 {
            let m = (a + b) / 2.;
            let l = s.deriv((a + m) / 2.).hypot();
            let r = s.deriv((m + b) / 2.).hypot();
            let left = (m - a) / 6. * (fa + 4. * l + fm);
            let right = (b - m) / 6. * (fm + 4. * r + fb);
            if depth == 0 || (left + right - whole).abs() < 15. * tol {
                return left + right + (left + right - whole) / 15.;
            }
            integrate(s, a, m, fa, l, fm, left, tol / 2., depth - 1)
                + integrate(s, m, b, fm, r, fb, right, tol / 2., depth - 1)
        }
        let fa = self.deriv(0.).hypot();
        let fm = self.deriv(to / 2.).hypot();
        let fb = self.deriv(to).hypot();
        integrate(
            self,
            0.,
            to,
            fa,
            fm,
            fb,
            to / 6. * (fa + 4. * fm + fb),
            EPS / 10.,
            24,
        )
    }
    pub fn parameter_at_length(&self, length: f64) -> f64 {
        let total = self.length(1.);
        if length <= 0. {
            return 0.;
        }
        if length >= total {
            return 1.;
        }
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..48 {
            let m = (lo + hi) / 2.;
            if self.length(m) < length {
                lo = m;
            } else {
                hi = m;
            }
        }
        (lo + hi) / 2.
    }
    pub fn extrema(&self, axis: usize) -> Vec<f64> {
        let candidates = match self.curve {
            Curve::Poly(p) => {
                let cs = coefficients(self.transform * p);
                roots(
                    &cs.iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, c)| {
                            if axis == 0 {
                                c.x * i as f64
                            } else {
                                c.y * i as f64
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            }
            Curve::Arc(a) => {
                let (u, v) = self.arc_vectors(a);
                let (aa, bb) = if axis == 0 { (v.x, -u.x) } else { (v.y, -u.y) };
                trig_roots(aa, bb, 0., a.start_angle, a.sweep_angle)
            }
        };
        candidates
            .into_iter()
            .filter(|t| *t > 1e-9 && *t < 1. - 1e-9)
            .filter(|t| {
                let d = |u| {
                    let p = self.deriv(u);
                    if axis == 0 { p.x } else { p.y }
                };
                d((t - 1e-6).max(0.)) * d((t + 1e-6).min(1.)) < 0.
            })
            .collect()
    }
    fn arc_vectors(&self, a: Arc) -> (Vec2, Vec2) {
        let (s, c) = a.x_rotation.sin_cos();
        (
            vector(self.transform, Vec2::new(a.radii.x * c, a.radii.x * s)),
            vector(self.transform, Vec2::new(-a.radii.y * s, a.radii.y * c)),
        )
    }
    pub fn inflections(&self) -> Vec<f64> {
        let Curve::Poly(p) = self.curve else {
            return vec![];
        };
        let cs = coefficients(self.transform * p);
        if cs.len() < 4 {
            return vec![];
        }
        let a = cs[1];
        let b = 2. * cs[2];
        let c = 3. * cs[3];
        roots(&[a.cross(b), 2. * a.cross(c), b.cross(c)])
            .into_iter()
            .filter(|t| *t > 1e-9 && *t < 1. - 1e-9)
            .filter(|t| {
                let curvature = |u| self.deriv(u).cross(self.second(u));
                self.deriv(*t).hypot() > 1e-12 && curvature(t - 1e-6) * curvature(t + 1e-6) < 0.
            })
            .collect()
    }
    pub fn nearest(&self, p: Point) -> std::result::Result<Vec<f64>, String> {
        let mut candidates = vec![0., 1.];
        match self.curve {
            Curve::Poly(q) => {
                let mut c = coefficients(self.transform * q);
                c[0] -= p.to_vec2();
                let mut poly = vec![0.; c.len() * 2 - 2];
                for (i, a) in c.iter().enumerate() {
                    for (j, b) in c.iter().enumerate().skip(1) {
                        poly[i + j - 1] += a.dot(*b) * j as f64;
                    }
                }
                if poly.iter().all(|v| v.abs() < 1e-24) {
                    return Err("最近点构成连续区间，请明确选取路径位置".into());
                }
                candidates.extend(roots(&poly));
            }
            Curve::Arc(a) => {
                let center = self.transform * a.center;
                let (u, v) = self.arc_vectors(a);
                if center.distance(p) < 1e-10
                    && (u.hypot() - v.hypot()).abs() < 1e-10
                    && u.dot(v).abs() < 1e-10
                {
                    return Err("最近点构成连续区间，请明确选取路径位置".into());
                }
                // tan(theta/2) turns the analytic stationarity equation into a
                // quartic. Bounded angular intervals avoid the pole at pi.
                let count = (a.sweep_angle.abs() / std::f64::consts::FRAC_PI_2)
                    .ceil()
                    .max(1.) as usize;
                for i in 0..count {
                    let lo = i as f64 / count as f64;
                    let hi = (i + 1) as f64 / count as f64;
                    let middle = a.start_angle + a.sweep_angle * (lo + hi) / 2.;
                    let (sin, cos) = middle.sin_cos();
                    let rotated_u = u * cos + v * sin;
                    let rotated_v = v * cos - u * sin;
                    let (u, v) = (rotated_u, rotated_v);
                    let d = center - p;
                    let du = d.dot(u);
                    let dv = d.dot(v);
                    let uv = u.dot(v);
                    let delta = v.hypot2() - u.hypot2();
                    let polynomial = [
                        dv + uv,
                        -2. * du + 2. * delta,
                        -6. * uv,
                        -2. * du - 2. * delta,
                        -dv + uv,
                    ];
                    let half = a.sweep_angle * (hi - lo) / 2.;
                    let left = (-half / 2.).tan().min((half / 2.).tan());
                    let right = (-half / 2.).tan().max((half / 2.).tan());
                    let mut shifted = vec![0.; 5];
                    for (j, c) in polynomial.iter().enumerate() {
                        let mut choose = 1.;
                        for (k, out) in shifted.iter_mut().enumerate().take(j + 1) {
                            *out += c
                                * choose
                                * left.powi((j - k) as i32)
                                * (right - left).powi(k as i32);
                            if k < j {
                                choose *= (j - k) as f64 / (k + 1) as f64;
                            }
                        }
                    }
                    for root in roots(&shifted) {
                        let theta = middle + 2. * (left + root * (right - left)).atan();
                        candidates.push(((theta - a.start_angle) / a.sweep_angle).clamp(0., 1.));
                    }
                }
            }
        }
        let best = candidates
            .iter()
            .map(|t| self.point(*t).distance(p))
            .fold(f64::INFINITY, f64::min);
        candidates.retain(|t| self.point(*t).distance(p) <= best + EPS);
        candidates.sort_by(f64::total_cmp);
        candidates.dedup_by(|a, b| (*a - *b).abs() < 1e-8);
        Ok(candidates)
    }
    pub fn intersections(
        &self,
        origin: Point,
        direction: Vec2,
    ) -> std::result::Result<Vec<f64>, String> {
        let candidates = match self.curve {
            Curve::Poly(p) => {
                let mut c = coefficients(self.transform * p);
                c[0] -= origin.to_vec2();
                let poly = c.iter().map(|p| p.cross(direction)).collect::<Vec<_>>();
                if poly.iter().all(|v| v.abs() < 1e-12) {
                    let projected = c.iter().map(|v| v.dot(direction)).collect::<Vec<_>>();
                    let derivative = projected
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, v)| i as f64 * v)
                        .collect::<Vec<_>>();
                    let mut samples = roots(&derivative);
                    samples.extend([0., 1.]);
                    if samples.iter().all(|t| eval(&projected, *t) < -EPS) {
                        return Ok(vec![]);
                    }
                    return Err("射线与路径存在连续重合区间".into());
                }
                roots(&poly)
            }
            Curve::Arc(a) => {
                let (u, v) = self.arc_vectors(a);
                let c = (self.transform * a.center - origin).cross(direction);
                trig_roots(
                    u.cross(direction),
                    v.cross(direction),
                    c,
                    a.start_angle,
                    a.sweep_angle,
                )
            }
        };
        Ok(candidates
            .into_iter()
            .filter(|t| (self.point(*t) - origin).dot(direction) >= -EPS)
            .collect())
    }
    pub fn bounds(&self) -> Rect {
        let mut b = Rect::from_points(self.point(0.), self.point(1.));
        for axis in 0..2 {
            for t in self.extrema(axis) {
                b = b.union_pt(self.point(t));
            }
        }
        b
    }
}
impl Paths {
    pub fn metric(&self, p: Point) -> Point {
        if self.local {
            self.frame.inverse() * p
        } else {
            p
        }
    }
    pub fn ordered(&self, mut positions: Vec<Position>) -> Vec<Position> {
        // Collapse a shared node, never coincident occurrences at a self-intersection.
        for p in &mut positions {
            let r = &self.routes[p.route];
            if p.t > 1. - 1e-9 && p.segment + 1 < r.segments.len() {
                p.segment += 1;
                p.t = 0.;
            }
            if r.closed && p.segment + 1 == r.segments.len() && p.t > 1. - 1e-9 {
                p.segment = 0;
                p.t = 0.;
            }
        }
        positions.sort_by(|a, b| {
            a.route
                .cmp(&b.route)
                .then(a.segment.cmp(&b.segment))
                .then(a.t.total_cmp(&b.t))
        });
        positions.dedup_by(|a, b| {
            a.route == b.route && a.segment == b.segment && (a.t - b.t).abs() < 1e-8
        });
        positions
    }
    pub fn bounds(&self) -> Option<Rect> {
        self.routes
            .iter()
            .flat_map(|r| &r.segments)
            .map(|s| s.measured(self).bounds())
            .reduce(|a, b| a.union(b))
    }
}
impl PathPoint {
    pub fn point(&self) -> Point {
        self.paths.routes[self.position.route].segments[self.position.segment]
            .point(self.position.t)
    }
    pub fn tangent(&self) -> std::result::Result<Vec2, String> {
        let p = self.position;
        let route = &self.paths.routes[p.route];
        let current = &route.segments[p.segment];
        if current.length(1.) == 0. {
            if let Some(v) = self.paths.routes[self.position.route].zero_direction {
                return if v.hypot() > 1e-14 {
                    Ok(v / v.hypot())
                } else {
                    Err("此路径位置没有可定义的方向".into())
                };
            }
        }
        let unit = |s: &Segment, t: f64, sign: f64| {
            let mut v = s.deriv(t);
            if v.hypot() < 1e-12 {
                v = s.second(t) * sign;
            }
            if v.hypot() < 1e-12 {
                if let Curve::Poly(PathSeg::Cubic(q)) = s.curve {
                    v = vector(s.transform, ((q.p3 - q.p0) + (q.p1 - q.p2) * 3.) * 6.);
                }
            }
            if v.hypot() < 1e-12 {
                None
            } else {
                Some(v / v.hypot())
            }
        };
        let incoming = if p.t > 1e-9 {
            unit(current, p.t, -1.)
        } else if p.segment > 0 {
            unit(&route.segments[p.segment - 1], 1., -1.)
        } else if route.closed {
            unit(route.segments.last().unwrap(), 1., -1.)
        } else {
            None
        };
        let outgoing = if p.t < 1. - 1e-9 {
            unit(current, p.t, 1.)
        } else if p.segment + 1 < route.segments.len() {
            unit(&route.segments[p.segment + 1], 0., 1.)
        } else if route.closed {
            unit(&route.segments[0], 0., 1.)
        } else {
            None
        };
        match self.side.as_deref() {
            Some("incoming") => incoming,
            Some("outgoing") => outgoing,
            _ => match (incoming, outgoing) {
                (Some(a), Some(b)) if (a - b).hypot() > 1e-6 => {
                    return Err(
                        "尖角或尖点的方向不唯一，请使用 with_side(\"incoming\"|\"outgoing\")"
                            .into(),
                    );
                }
                (Some(a), _) => Some(a),
                (_, Some(b)) => Some(b),
                _ => None,
            },
        }
        .ok_or_else(|| "此路径位置没有可定义的方向".into())
    }
}
pub fn from_bez(path: &BezPath, transform: Affine) -> Vec<Route> {
    let mut out = vec![];
    let mut route = Route {
        zero_direction: None,
        segments: vec![],
        closed: false,
    };
    let mut point = Point::ORIGIN;
    let mut start = point;
    for el in path.iter() {
        match el {
            PathEl::MoveTo(p) => {
                if !route.segments.is_empty() {
                    out.push(route);
                    route = Route {
                        zero_direction: None,
                        segments: vec![],
                        closed: false,
                    };
                }
                point = p;
                start = p;
            }
            PathEl::LineTo(p) => {
                route.segments.push(Segment {
                    curve: Curve::Poly(PathSeg::Line(Line::new(point, p))),
                    transform,
                });
                point = p;
            }
            PathEl::QuadTo(c, p) => {
                route.segments.push(Segment {
                    curve: Curve::Poly(PathSeg::Quad(QuadBez::new(point, c, p))),
                    transform,
                });
                point = p;
            }
            PathEl::CurveTo(a, b, p) => {
                route.segments.push(Segment {
                    curve: Curve::Poly(PathSeg::Cubic(CubicBez::new(point, a, b, p))),
                    transform,
                });
                point = p;
            }
            PathEl::ClosePath => {
                if point.distance(start) > 1e-12 {
                    route.segments.push(Segment {
                        curve: Curve::Poly(PathSeg::Line(Line::new(point, start))),
                        transform,
                    });
                }
                route.closed = true;
                point = start;
            }
        }
    }
    if !route.segments.is_empty() {
        out.push(route);
    }
    out
}
