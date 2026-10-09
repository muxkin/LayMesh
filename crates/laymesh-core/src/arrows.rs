//! Filled arrow materials. The source centerline is retained independently of
//! the adaptively tessellated, single closed silhouette.
use crate::{
    Loc, Result,
    engine::Engine,
    model::*,
    query_path::{Curve, Route, Segment},
};
use kurbo::{Affine, Arc, BezPath, CubicBez, Line, PathSeg, Point, QuadBez, Shape, Vec2};
use serde_json::{Value as Json, json};
use std::f64::consts::{PI, TAU};

pub fn is_arrow(name: &str) -> bool {
    matches!(
        name,
        "arrow" | "arrow.arc" | "arrow.bent" | "arrow.uturn" | "arrow.chevron" | "arrow.path"
    )
}
fn normal(v: Vec2) -> Vec2 {
    Vec2::new(-v.y, v.x)
}
fn angle(e: &Engine, a: &Args, key: &str, default: f64, l: Loc) -> Result<f64> {
    let x = match a.get(key).map(V::value) {
        None => default,
        Some(V::Number(n, u)) if u.is_empty() || u == "deg" => n.to_radians(),
        Some(V::Number(n, u)) if u == "rad" => *n,
        _ => return Err(e.error("E_UNIT", format!("{key} 需要角度"), l)),
    };
    if !x.is_finite() {
        return Err(e.error("E_ARROW", format!("{key} 必须有限"), l));
    }
    Ok(x)
}
fn positive(e: &Engine, v: &V, l: Loc) -> Result<f64> {
    let x = e.len(v, l)?;
    if !x.is_finite() || x <= 0. {
        return Err(e.error("E_ARROW", "箭头尺寸需要正的有限长度", l));
    }
    Ok(x)
}
fn pair(e: &Engine, a: &Args, key: &str, default: [f64; 2], l: Loc) -> Result<[f64; 2]> {
    let Some(v) = a.get(key) else {
        return Ok(default);
    };
    let v = v.list();
    if v.len() != 2 {
        return Err(e.error("E_ARROW", format!("{key} 需要两个正长度"), l));
    }
    Ok([positive(e, &v[0], l)?, positive(e, &v[1], l)?])
}
fn widths(e: &Engine, a: &Args, l: Loc) -> Result<Vec<[f64; 2]>> {
    let Some(v) = a.get("shaft_width") else {
        return Ok(vec![[0., 3.], [1., 3.]]);
    };
    let out = match v.value() {
        V::Number(..) => {
            let w = positive(e, v, l)?;
            vec![[0., w], [1., w]]
        }
        V::List(v) if v.len() == 2 && v.iter().all(|v| matches!(v.value(), V::Number(..))) => {
            vec![[0., positive(e, &v[0], l)?], [1., positive(e, &v[1], l)?]]
        }
        V::List(v) if (2..=1000).contains(&v.len()) => {
            let mut out = vec![];
            for v in v {
                let v = v.list();
                if v.len() != 2 {
                    return Err(e.error("E_ARROW", "宽度控制点需要 (位置,宽度)", l));
                }
                out.push([e.scalar(&v[0], l)?, positive(e, &v[1], l)?]);
            }
            out
        }
        _ => return Err(e.error("E_ARROW", "shaft_width 需要长度、首尾长度对或控制点列表", l)),
    };
    if out[0][0] != 0.
        || out.last().unwrap()[0] != 1.
        || out.iter().any(|p| !p[0].is_finite())
        || out.windows(2).any(|p| p[0][0] >= p[1][0])
    {
        return Err(e.error("E_ARROW", "宽度位置必须从 0 到 1 严格递增", l));
    }
    Ok(out)
}
pub(crate) fn validate(e: &Engine, name: &str, a: &Args, l: Loc) -> Result<()> {
    if !is_arrow(name) {
        return Ok(());
    }
    widths(e, a, l)?;
    for k in ["head_size", "start_head_size", "end_head_size"] {
        pair(e, a, k, [1., 1.], l)?;
    }
    let heads = string(a, "heads", "end");
    if !["start", "end", "both"].contains(&heads.as_str())
        || name == "arrow.chevron" && heads == "both"
    {
        return Err(e.error(
            "E_ARROW",
            "heads 需要 start/end/both；燕尾箭头只能有一个头部",
            l,
        ));
    }
    if matches!(name, "arrow" | "arrow.chevron") && crate::endpoints::has_line_geometry(a) {
        let (x, y) = crate::endpoints::line_vector(e, a, l)?;
        if x.hypot(y) == 0. {
            return Err(e.error("E_ARROW", "形状箭头长度必须大于零", l));
        }
    }
    if name == "arrow.arc" {
        angle(e, a, "start_angle", 0., l)?;
        if a.contains_key("sweep_angle") {
            let s = angle(e, a, "sweep_angle", 0., l)?;
            if s.abs() <= 1e-12 || s.abs() >= TAU {
                return Err(e.error("E_ARROW", "扫角必须满足 0 < |sweep_angle| < 360deg", l));
            }
        }
        if let Some(r) = a.get("radius") {
            let r = e.len(r, l)?;
            if !r.is_finite() || r == 0. || a.contains_key("sweep_angle") && r < 0. {
                return Err(e.error(
                    "E_ARROW",
                    "radius 必须非零；与 sweep_angle 同时指定时使用正半径",
                    l,
                ));
            }
        }
    }
    if matches!(name, "arrow.bent" | "arrow.uturn") {
        pair(e, a, "span", [30., 20.], l)?;
    }
    for k in ["corner_radius", "notch_depth"] {
        if let Some(v) = a.get(k) {
            let x = e.len(v, l)?;
            if !x.is_finite() || x < 0. {
                return Err(e.error("E_ARROW", format!("{k} 需要非负有限长度"), l));
            }
        }
    }
    if name == "arrow.path" {
        let o = a
            .get("path")
            .and_then(V::object)
            .ok_or_else(|| e.error("E_ARROW", "arrow.path 需要开放路径素材 path", l))?;
        if !matches!(
            o.borrow().kind.as_str(),
            "line" | "arc" | "path" | "polyline"
        ) {
            return Err(e.error("E_ARROW", "path 需要 line/arc/path/polyline 素材", l));
        }
    }
    Ok(())
}
fn line(a: Point, b: Point) -> Segment {
    Segment {
        curve: Curve::Poly(PathSeg::Line(Line::new(a, b))),
        transform: Affine::IDENTITY,
    }
}
fn route(segments: Vec<Segment>) -> Route {
    Route {
        segments,
        closed: false,
        zero_direction: None,
    }
}
fn arc(center: Point, r: f64, start: f64, sweep: f64) -> Route {
    route(vec![Segment {
        curve: Curve::Arc(Arc::new(center, (r, r), start, sweep, 0.)),
        transform: Affine::IDENTITY,
    }])
}
fn body_sweep(radius: f64, sweep: f64, heads: [f64; 2]) -> std::result::Result<f64, String> {
    let remaining = sweep.abs() - heads.iter().map(|h| h.atan2(radius)).sum::<f64>();
    if !radius.is_finite() || radius <= 0. || remaining <= 1e-12 {
        return Err("总扫角不足以容纳头部和正长度的圆弧箭身".into());
    }
    Ok(sweep.signum() * remaining)
}
fn arc_chord(radius: f64, sweep: f64, heads: [f64; 2]) -> f64 {
    let a = radius.hypot(heads[0]);
    let b = radius.hypot(heads[1]);
    // Half-angle form avoids cancellation for small sweeps.
    (a - b).hypot(2. * a.sqrt() * b.sqrt() * (sweep / 2.).sin())
}
fn arc_radius(distance: f64, sweep: f64, heads: [f64; 2]) -> std::result::Result<f64, String> {
    let unit = distance.max(heads[0]).max(heads[1]);
    let d = distance / unit;
    let hs = heads.map(|h| h / unit);
    let [a, b] = hs.map(|h| h * h);
    // The chord can have two positive-radius solutions for major arcs with
    // unequal heads. Bracket the increasing branch beyond its exact minimum.
    let mut lo = if sweep.cos() > 0. {
        (0.5 * ((a - b).abs() / sweep.sin().abs() - a - b))
            .max(0.)
            .sqrt()
    } else {
        0.
    };
    if arc_chord(lo, sweep, hs) > d + 1e-12 {
        return Err("端点距离、扫角与头部尺寸无可用圆弧解".into());
    }
    let mut hi = lo.max(1.);
    while arc_chord(hi, sweep, hs) < d {
        hi *= 2.;
        if !hi.is_finite() {
            return Err("圆弧半径超出可计算范围".into());
        }
    }
    for _ in 0..100 {
        let mid = lo + (hi - lo) / 2.;
        if arc_chord(mid, sweep, hs) < d {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let radius = hi * unit;
    body_sweep(radius, sweep, heads)?;
    if (arc_chord(radius, sweep, heads) - distance).abs() > 1e-8 * distance.max(1.) {
        return Err("端点距离、扫角与头部尺寸无可用圆弧解".into());
    }
    Ok(radius)
}
fn append_heads(r: &mut Route, heads: [f64; 2]) -> std::result::Result<(), String> {
    let first = &r.segments[0];
    let last = r.segments.last().unwrap();
    let (a, b) = (first.point(0.), last.point(1.));
    let (ta, tb) = (tangent(first, 0.)?, tangent(last, 1.)?);
    if heads[0] > 0. {
        r.segments.insert(0, line(a - ta * heads[0], a));
    }
    if heads[1] > 0. {
        r.segments.push(line(b, b + tb * heads[1]));
    }
    Ok(())
}
fn transform_route(r: &mut Route, tr: Affine) {
    for s in &mut r.segments {
        s.transform = tr * s.transform;
    }
}
fn centerline(e: &Engine, o: &Object, a: &Args, heads: [f64; 2]) -> Result<Route> {
    let l = o.loc;
    let connection = a
        .get("__arrow_vector")
        .map(|v| v.list())
        .map(|v| Vec2::new(v[0].number().unwrap(), v[1].number().unwrap()));
    let mut r = match o.kind.as_str() {
        "arrow" | "arrow.chevron" => {
            let v = if let Some(v) = connection {
                v
            } else {
                if !crate::endpoints::has_line_geometry(a) {
                    return Err(e.error(
                        "E_ARROW",
                        "箭头未定义几何；请在 add 指定 start/end 或定义 length/angle、dx/dy",
                        l,
                    ));
                }
                let (x, y) = crate::endpoints::line_vector(e, a, l)?;
                Vec2::new(x, y)
            };
            route(vec![line(Point::ORIGIN, v.to_point())])
        }
        "arrow.arc" => {
            let radius = a.get("radius").map(|v| e.len(v, l)).transpose()?;
            let sweep = if a.contains_key("sweep_angle") {
                Some(angle(e, a, "sweep_angle", 0., l)?)
            } else {
                None
            };
            let (radius, sweep, start) = if let Some(v) = connection {
                let d = v.hypot();
                let (r, s) = match (radius, sweep) {
                    (Some(r), Some(s)) => (r, s),
                    (None, Some(s)) => (
                        arc_radius(d, s, heads).map_err(|m| e.error("E_ARROW", m, l))?,
                        s,
                    ),
                    (Some(r), None) => {
                        let (a, b) = (r.hypot(heads[0]), r.hypot(heads[1]));
                        if d > a + b + 1e-9 || d < (a - b).abs() - 1e-9 {
                            return Err(e.error(
                                "E_ARROW",
                                "端点距离超出此半径和头部尺寸的可达范围",
                                l,
                            ));
                        }
                        let half_sin = ((d - (a - b).abs()).max(0.) * (d + (a - b).abs())).sqrt()
                            / (2. * a.sqrt() * b.sqrt());
                        (r.abs(), r.signum() * 2. * half_sin.min(1.).asin())
                    }
                    (None, None) => {
                        return Err(e.error("E_ARROW", "弧形连接需要 sweep_angle 或 radius", l));
                    }
                };
                if (arc_chord(r, s, heads) - d).abs() > 1e-8 * d.max(1.) {
                    return Err(e.error("E_ARROW", "半径、总扫角、头部尺寸与端点距离不一致", l));
                }
                (r, s, 0.)
            } else {
                let r = radius.filter(|r| *r > 0.).ok_or_else(|| {
                    e.error(
                        "E_ARROW",
                        "本地圆弧需要正 radius；也可在 add 指定 start/end",
                        l,
                    )
                })?;
                let s = sweep.ok_or_else(|| e.error("E_ARROW", "本地圆弧需要 sweep_angle", l))?;
                (r, s, angle(e, a, "start_angle", 0., l)?)
            };
            let body = body_sweep(radius, sweep, heads).map_err(|m| e.error("E_ARROW", m, l))?;
            arc(
                Point::new(radius, radius),
                radius,
                start + sweep.signum() * heads[0].atan2(radius),
                body,
            )
        }
        "arrow.bent" | "arrow.uturn" => {
            let [w, h] = pair(e, a, "span", [30., 20.], l)?;
            let rad = length(a, "corner_radius", w.min(h) / 4., &e.unit, e.dpi);
            if rad > h || rad > if o.kind == "arrow.uturn" { w / 2. } else { w } {
                return Err(e.error("E_ARROW", "corner_radius 超过折弯可用空间", l));
            }
            let mut ps = vec![Point::new(0., h), Point::ORIGIN, Point::new(w, 0.)];
            if o.kind == "arrow.uturn" {
                ps.push(Point::new(w, h));
            }
            let mut segs = vec![];
            let mut p = ps[0];
            for i in 1..ps.len() - 1 {
                let t = (ps[i] - ps[i - 1]).normalize();
                let u = (ps[i + 1] - ps[i]).normalize();
                let entry = ps[i] - t * rad;
                let exit = ps[i] + u * rad;
                if p.distance(entry) > 1e-12 {
                    segs.push(line(p, entry));
                }
                if rad > 0. {
                    let center = entry + u * rad;
                    let start = entry - center;
                    segs.extend(
                        arc(
                            center,
                            rad,
                            start.y.atan2(start.x),
                            t.cross(u).signum() * PI / 2.,
                        )
                        .segments,
                    );
                }
                p = exit;
            }
            if p.distance(*ps.last().unwrap()) > 1e-12 {
                segs.push(line(p, *ps.last().unwrap()));
            }
            route(segs)
        }
        "arrow.path" => {
            let mat = a["path"].object().unwrap();
            let mat = mat.borrow();
            let n = e.shape(&mat, &mat.args, [f64::NAN; 2])?;
            let mut paths = crate::geometry_query::material_paths(&n, e, l)?;
            if paths.routes.len() != 1 || paths.routes[0].closed {
                return Err(e.error("E_ARROW", "arrow.path 需要单条开放路径", l));
            }
            paths.routes.remove(0)
        }
        _ => unreachable!(),
    };
    r.segments.retain(|s| s.length(1.) > 1e-12);
    if r.segments.is_empty() {
        return Err(e.error("E_ARROW", "箭头中心线不能为零长度", l));
    }
    if o.kind == "arrow.path" {
        if let Some(v) = connection {
            let from = r.segments[0].point(0.);
            let d = r.segments.last().unwrap().point(1.) - from;
            if d.hypot() < 1e-12 {
                return Err(e.error("E_ARROW", "双端连接的路径首尾不能重合", l));
            }
            let t0 = tangent(&r.segments[0], 0.).map_err(|m| e.error("E_ARROW", m, l))?;
            let t1 =
                tangent(r.segments.last().unwrap(), 1.).map_err(|m| e.error("E_ARROW", m, l))?;
            let h = t0 * heads[0] + t1 * heads[1];
            let n = d.normalize();
            let unit = v.hypot().max(h.hypot());
            let h = h / unit;
            let target = v.hypot() / unit;
            let perpendicular = h.cross(n).abs();
            let discriminant = (target - perpendicular) * (target + perpendicular);
            if discriminant < -1e-12 {
                return Err(e.error("E_ARROW", "端点距离无法容纳路径两端的头部", l));
            }
            let root = discriminant.max(0.).sqrt();
            let along = h.dot(n);
            let chord = if along > 0. {
                ((target - h.hypot()) * (target + h.hypot())) / (root + along)
            } else {
                root - along
            } * unit;
            let scale = chord / d.hypot();
            if !scale.is_finite() || scale <= 0. {
                return Err(e.error("E_ARROW", "连接无正长度箭身解；请调整端点或头部尺寸", l));
            }
            let complete = d * scale + h * unit;
            let rotation = Affine::rotate(v.y.atan2(v.x) - complete.y.atan2(complete.x));
            let tr = rotation
                * Affine::translate(t0 * heads[0])
                * Affine::scale(scale)
                * Affine::translate(-from.to_vec2());
            transform_route(&mut r, tr);
        }
        append_heads(&mut r, heads).map_err(|m| e.error("E_ARROW", m, l))?;
    } else if o.kind == "arrow.arc" {
        append_heads(&mut r, heads).map_err(|m| e.error("E_ARROW", m, l))?;
        if let Some(v) = connection {
            let from = r.segments[0].point(0.);
            let d = r.segments.last().unwrap().point(1.) - from;
            let tr = Affine::rotate(v.y.atan2(v.x) - d.y.atan2(d.x))
                * Affine::translate(-from.to_vec2());
            transform_route(&mut r, tr);
        }
    } else if !matches!(o.kind.as_str(), "arrow" | "arrow.chevron") {
        if let Some(v) = connection {
            let from = r.segments[0].point(0.);
            let to = r.segments.last().unwrap().point(1.);
            let d = to - from;
            if d.hypot() < 1e-12 {
                return Err(e.error("E_ARROW", "双端连接的路径首尾不能重合", l));
            }
            let tr = Affine::rotate(v.y.atan2(v.x) - d.y.atan2(d.x))
                * Affine::scale(v.hypot() / d.hypot())
                * Affine::translate(-from.to_vec2());
            transform_route(&mut r, tr);
        }
    }
    Ok(r)
}
fn point_json(p: Point) -> Json {
    json!([p.x, p.y])
}
fn serialize(r: &Route) -> Json {
    json!(r.segments.iter().map(|s| {
        let mut v=match s.curve {
            Curve::Poly(PathSeg::Line(p))=>json!({"points":[point_json(p.p0),point_json(p.p1)]}),
            Curve::Poly(PathSeg::Quad(p))=>json!({"points":[point_json(p.p0),point_json(p.p1),point_json(p.p2)]}),
            Curve::Poly(PathSeg::Cubic(p))=>json!({"points":[point_json(p.p0),point_json(p.p1),point_json(p.p2),point_json(p.p3)]}),
            Curve::Arc(a)=>json!({"center":[a.center.x,a.center.y],"radii":[a.radii.x,a.radii.y],"start":a.start_angle,"sweep":a.sweep_angle,"rotation":a.x_rotation}),
        };
        v["transform"]=json!(s.transform.as_coeffs());v
    }).collect::<Vec<_>>())
}
pub(crate) fn recipe_route(recipe: &Json, tr: Affine) -> Route {
    route(
        recipe["centerline"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let p = |v: &Json| Point::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
                let curve = if let Some(ps) = s["points"].as_array() {
                    Curve::Poly(match ps.len() {
                        2 => PathSeg::Line(Line::new(p(&ps[0]), p(&ps[1]))),
                        3 => PathSeg::Quad(QuadBez::new(p(&ps[0]), p(&ps[1]), p(&ps[2]))),
                        _ => PathSeg::Cubic(CubicBez::new(
                            p(&ps[0]),
                            p(&ps[1]),
                            p(&ps[2]),
                            p(&ps[3]),
                        )),
                    })
                } else {
                    Curve::Arc(Arc::new(
                        p(&s["center"]),
                        p(&s["radii"]).to_vec2(),
                        jnum(s, "start", 0.),
                        jnum(s, "sweep", 0.),
                        jnum(s, "rotation", 0.),
                    ))
                };
                let coeff = std::array::from_fn(|i| s["transform"][i].as_f64().unwrap());
                Segment {
                    curve,
                    transform: tr * Affine::new(coeff),
                }
            })
            .collect(),
    )
}
fn tangent(s: &Segment, t: f64) -> std::result::Result<Vec2, String> {
    let mut d = s.deriv(t);
    if d.hypot() < 1e-12 {
        d = if t == 0. {
            s.point(1e-6) - s.point(0.)
        } else if t == 1. {
            s.point(1.) - s.point(1. - 1e-6)
        } else {
            Vec2::ZERO
        };
    }
    if d.hypot() < 1e-14 {
        return Err("中心线包含方向不确定的尖点".into());
    }
    Ok(d.normalize())
}
fn width_at(stops: &[[f64; 2]], u: f64) -> f64 {
    let u = u.clamp(0., 1.);
    let i = stops
        .partition_point(|p| p[0] < u)
        .max(1)
        .min(stops.len() - 1);
    let (a, b) = (stops[i - 1], stops[i]);
    a[1] + (b[1] - a[1]) * (u - a[0]) / (b[0] - a[0])
}
fn cut_station(
    r: &Route,
    prefix: &[f64],
    base: Point,
    t: Vec2,
    start: bool,
) -> std::result::Result<f64, String> {
    let mut hits = vec![];
    for (i, s) in r.segments.iter().enumerate() {
        for dir in [normal(t), -normal(t)] {
            for u in s.intersections(base, dir)? {
                hits.push(prefix[i] + s.length(u));
            }
        }
    }
    hits.sort_by(f64::total_cmp);
    if start { hits.first() } else { hits.last() }
        .copied()
        .ok_or_else(|| "头部尺寸无法容纳于中心线；请缩短头部或调整路径".into())
}
fn split_sample(
    f: &impl Fn(f64) -> std::result::Result<Point, String>,
    lo: f64,
    hi: f64,
    a: Point,
    b: Point,
    tol: f64,
    depth: u32,
    out: &mut Vec<Point>,
) -> std::result::Result<(), String> {
    let m = (lo + hi) / 2.;
    let p = f(m)?;
    let q = f((lo + m) / 2.)?;
    let z = f((m + hi) / 2.)?;
    let err = p
        .distance(a.lerp(b, 0.5))
        .max(q.distance(a.lerp(b, 0.25)))
        .max(z.distance(a.lerp(b, 0.75)));
    if err <= tol {
        out.push(b);
    } else {
        if depth >= 24 || out.len() > 200_000 {
            return Err("箭头轮廓超过几何细分上限".into());
        }
        split_sample(f, lo, m, a, p, tol, depth + 1, out)?;
        split_sample(f, m, hi, p, b, tol, depth + 1, out)?;
    }
    Ok(())
}
fn clip_front(ps: &mut Vec<Point>, base: Point, t: Vec2) -> std::result::Result<(), String> {
    for i in 0..ps.len() - 1 {
        let a = (ps[i] - base).dot(t);
        let b = (ps[i + 1] - base).dot(t);
        if a <= 0. && b >= 0. && b - a > 1e-14 {
            let p = ps[i].lerp(ps[i + 1], -a / (b - a));
            ps.drain(..=i);
            ps.insert(0, p);
            return Ok(());
        }
    }
    Err("箭身与头部无法相接；请减小宽度或头部尺寸".into())
}
fn simple_polygon(ps: &[Point]) -> bool {
    // Sweep bounding boxes instead of comparing all sampled segments.
    let n = ps.len();
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|&a, &b| {
        ps[a]
            .x
            .min(ps[(a + 1) % n].x)
            .total_cmp(&ps[b].x.min(ps[(b + 1) % n].x))
    });
    let mut active = Vec::<usize>::new();
    for i in order {
        let (a, b) = (ps[i], ps[(i + 1) % n]);
        active.retain(|&j| ps[j].x.max(ps[(j + 1) % n].x) >= a.x.min(b.x) - 1e-10);
        for &j in &active {
            if (i + 1) % n == j || (j + 1) % n == i {
                continue;
            }
            let (c, d) = (ps[j], ps[(j + 1) % n]);
            if a.y.min(b.y) > c.y.max(d.y) + 1e-10 || c.y.min(d.y) > a.y.max(b.y) + 1e-10 {
                continue;
            }
            let v = b - a;
            let w = d - c;
            let den = v.cross(w);
            if den.abs() > 1e-14 {
                let t = (c - a).cross(w) / den;
                let u = (c - a).cross(v) / den;
                if (-1e-9..=1. + 1e-9).contains(&t) && (-1e-9..=1. + 1e-9).contains(&u) {
                    return false;
                }
            } else if (c - a).cross(v).abs() < 1e-12 && v.hypot2() > 0. {
                let x = (c - a).dot(v) / v.hypot2();
                let y = (d - a).dot(v) / v.hypot2();
                if x.min(y) < 1. - 1e-9 && x.max(y) > 1e-9 {
                    return false;
                }
            }
        }
        active.push(i);
    }
    true
}
fn silhouette(recipe: &Json, tol: f64, check: bool) -> std::result::Result<BezPath, String> {
    let r = recipe_route(recipe, Affine::IDENTITY);
    let mut prefix = vec![0.];
    for s in &r.segments {
        prefix.push(prefix.last().unwrap() + s.length(1.));
    }
    let total = *prefix.last().unwrap();
    let first = &r.segments[0];
    let last = r.segments.last().unwrap();
    let (p0, p1) = (first.point(0.), last.point(1.));
    let (t0, t1) = (tangent(first, 0.)?, tangent(last, 1.)?);
    let hs = |key: &str| {
        [
            recipe[key][0].as_f64().unwrap(),
            recipe[key][1].as_f64().unwrap(),
        ]
    };
    let (h0, h1) = (hs("startHead"), hs("endHead"));
    let appended = recipe["appendedHeads"] == true;
    let begin = usize::from(appended && h0[0] > 0.);
    let end = r.segments.len() - usize::from(appended && h1[0] > 0.);
    let (b0, b1) = if appended {
        (r.segments[begin].point(0.), r.segments[end - 1].point(1.))
    } else {
        (p0 + t0 * h0[0], p1 - t1 * h1[0])
    };
    let s0 = if appended {
        prefix[begin]
    } else if h0[0] > 0. {
        cut_station(&r, &prefix, b0, t0, true)?
    } else {
        0.
    };
    let s1 = if appended {
        prefix[end]
    } else if h1[0] > 0. {
        cut_station(&r, &prefix, b1, t1, false)?
    } else {
        total
    };
    if s1 - s0 <= 1e-9 {
        return Err("路径太短，无法容纳箭身及头部".into());
    }
    let stops: Vec<[f64; 2]> = recipe["widths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
        .collect();
    let mut sides = vec![];
    for sign in [1., -1.] {
        let mut ps: Vec<Point> = vec![];
        for (i, s) in r.segments.iter().enumerate().take(end).skip(begin) {
            let eval = |t: f64| -> std::result::Result<Point, String> {
                let w = width_at(&stops, (prefix[i] + s.length(t) - s0) / (s1 - s0));
                Ok(s.point(t) + normal(tangent(s, t)?) * (sign * w / 2.))
            };
            let first = eval(0.)?;
            let mut joined = false;
            if let Some(&previous) = ps.last() {
                if i > 0 && previous.distance(first) > tol {
                    let a = tangent(&r.segments[i - 1], 1.)?;
                    let b = tangent(s, 0.)?;
                    let det = a.cross(b);
                    if det.abs() > 1e-10 {
                        let q = previous + a * ((first - previous).cross(b) / det);
                        let w = width_at(&stops, (prefix[i] - s0) / (s1 - s0));
                        if q.distance(s.point(0.)) <= 2. * w {
                            ps.pop();
                            ps.push(q);
                            joined = true;
                        }
                    } else if a.dot(b) < 0. {
                        return Err("中心线不能包含折返尖点".into());
                    }
                }
            }
            if !joined {
                ps.push(first);
            }
            let mut splits = vec![0., 1.];
            for stop in &stops {
                let d = s0 + stop[0] * (s1 - s0) - prefix[i];
                if d > 0. && d < prefix[i + 1] - prefix[i] {
                    splits.push(s.parameter_at_length(d));
                }
            }
            splits.sort_by(f64::total_cmp);
            splits.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            for w in splits.windows(2) {
                split_sample(&eval, w[0], w[1], eval(w[0])?, eval(w[1])?, tol, 0, &mut ps)?;
            }
        }
        if !appended && h0[0] > 0. {
            clip_front(&mut ps, b0, t0)?;
        }
        if !appended && h1[0] > 0. {
            ps.reverse();
            clip_front(&mut ps, b1, -t1)?;
            ps.reverse();
        }
        sides.push(ps);
    }
    let mut ps = sides.remove(0);
    if h1[0] > 0. {
        ps.extend([
            b1 + normal(t1) * h1[1] / 2.,
            p1,
            b1 - normal(t1) * h1[1] / 2.,
        ]);
    }
    let mut other = sides.remove(0);
    other.reverse();
    ps.extend(other);
    if h0[0] > 0. {
        ps.extend([
            b0 - normal(t0) * h0[1] / 2.,
            p0,
            b0 + normal(t0) * h0[1] / 2.,
        ]);
    }
    let notch = jnum(recipe, "notch", 0.);
    if notch > 0. {
        if notch >= s1 - s0 {
            return Err("notch_depth 超过可用箭身长度".into());
        }
        if h0[0] == 0. {
            ps.push(p0 + t0 * notch);
        } else {
            // Insert the notch between the two end-side tail corners.
            let at = ps
                .iter()
                .position(|p| p.distance(p1 + normal(t1) * stops.last().unwrap()[1] / 2.) < 1e-8)
                .ok_or("无法定位燕尾端点")?;
            ps.insert(at + 1, p1 - t1 * notch);
        }
    }
    ps.dedup_by(|a, b| a.distance(*b) < 1e-10);
    if ps.len() > 1 && ps[0].distance(*ps.last().unwrap()) < 1e-10 {
        ps.pop();
    }
    if check && !simple_polygon(&ps) {
        return Err("箭头轮廓自交；请减小宽度、头部或调整曲率".into());
    }
    let mut p = BezPath::new();
    p.move_to(ps[0]);
    for q in &ps[1..] {
        p.line_to(*q);
    }
    p.close_path();
    Ok(p)
}
pub(crate) fn resolved(recipe: &Json, tol: f64) -> BezPath {
    // Definition-time topology validation is invariant under later transforms.
    // Accuracy is recomputed from the retained analytic centerline.
    silhouette(recipe, tol, false).expect("validated arrow geometry")
}
/// Validate the same final-space subdivision budget before any effect, ink
/// query or renderer can request a magnified silhouette. Ordinary nodes only
/// pay for a traversal; topology was already checked in the constructor.
pub(crate) fn validate_transform(n: &Json, parent: Affine, e: &Engine, l: Loc) -> Result<()> {
    let mut tr = parent * crate::geometry::node_transform(n);
    if n["kind"] == "group" {
        tr = tr
            * Affine::scale_non_uniform(
                jnum(n, "width", 1.) / jnum(n, "contentWidth", 1.).max(1e-12),
                jnum(n, "height", 1.) / jnum(n, "contentHeight", 1.).max(1e-12),
            );
        for child in n["children"].as_array().into_iter().flatten() {
            validate_transform(child, tr, e, l)?;
        }
    } else if n["geometryRecipe"]["kind"] == "arrow" {
        tr = tr
            * Affine::scale_non_uniform(
                jnum(n, "width", 1.) / jnum(n, "intrinsicWidth", 1.),
                jnum(n, "height", 1.) / jnum(n, "intrinsicHeight", 1.),
            );
        let [a, b, c, d, _, _] = tr.as_coeffs();
        let scale = ((a + d).hypot(b - c) + (a - d).hypot(b + c)) / 2.;
        if !scale.is_finite() {
            return Err(e.error("E_ARROW", "箭头变换必须有限", l));
        }
        if scale > 1. + 1e-12 {
            silhouette(&n["geometryRecipe"], 0.0001 / scale, false)
                .map_err(|message| e.error("E_LIMIT", message, l))?;
        }
    }
    Ok(())
}
pub(crate) fn shape(e: &Engine, o: &Object, a: &Args, size: [f64; 2]) -> Result<Json> {
    validate(e, &o.kind, a, o.loc)?;
    let ws = widths(e, a, o.loc)?;
    let heads = string(a, "heads", "end");
    let mut h0 = pair(e, a, "head_size", [ws[0][1] * 2.; 2], o.loc)?;
    let mut h1 = pair(e, a, "head_size", [ws.last().unwrap()[1] * 2.; 2], o.loc)?;
    h0 = pair(e, a, "start_head_size", h0, o.loc)?;
    h1 = pair(e, a, "end_head_size", h1, o.loc)?;
    if heads == "end" {
        h0 = [0.; 2];
    }
    if heads == "start" {
        h1 = [0.; 2];
    }
    if h0[0] > 0. && h0[1] < ws[0][1] || h1[0] > 0. && h1[1] < ws.last().unwrap()[1] {
        return Err(e.error("E_ARROW", "头部宽度不能小于对应颈部宽度", o.loc));
    }
    let r = centerline(e, o, a, [h0[0], h1[0]])?;
    let notch = if o.kind == "arrow.chevron" {
        length(
            a,
            "notch_depth",
            if heads == "start" {
                ws.last().unwrap()[1]
            } else {
                ws[0][1]
            },
            &e.unit,
            e.dpi,
        )
    } else {
        0.
    };
    let mut recipe = json!({"kind":"arrow","template":o.kind,"centerline":serialize(&r),"widths":ws,"startHead":h0,"endHead":h1,"notch":notch});
    if matches!(o.kind.as_str(), "arrow.arc" | "arrow.path") {
        recipe["appendedHeads"] = json!(true);
    }
    let p = silhouette(&recipe, 0.0001, true).map_err(|m| e.error("E_ARROW", m, o.loc))?;
    let b = p.bounding_box();
    let origin = Vec2::new(b.x0, b.y0);
    recipe["origin"] = json!([origin.x, origin.y]);
    let color = string(a, "border_color", "none");
    let width = length(
        a,
        "border_width",
        if color == "none" { 0. } else { 0.3 },
        "pt",
        e.dpi,
    );
    let fill = a.get("fill").map(V::json).unwrap_or(json!("#000000"));
    let mut n = crate::geometry::path_node(Affine::translate(-origin) * p, fill, &color, width);
    n["strokeStyle"] = e.stroke(a, false, width);
    n["geometryRecipe"] = recipe;
    n["intrinsicWidth"] = json!(b.width());
    n["intrinsicHeight"] = json!(b.height());
    n["width"] = json!(if size[0].is_finite() {
        size[0]
    } else {
        b.width()
    });
    n["height"] = json!(if size[1].is_finite() {
        size[1]
    } else {
        b.height()
    });
    let scale = Affine::scale_non_uniform(
        jnum(&n, "width", 0.) / b.width(),
        jnum(&n, "height", 0.) / b.height(),
    );
    n["endpoints"] = json!([
        point_json(scale * (r.segments[0].point(0.) - origin)),
        point_json(scale * (r.segments.last().unwrap().point(1.) - origin))
    ]);
    Ok(n)
}
