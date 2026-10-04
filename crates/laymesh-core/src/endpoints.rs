//! Endpoint decorations retain the logical path and generate shared vector ink.
use crate::geometry::{compound_outline_transformed, difference, intersection, union};
use crate::{Loc, Result, engine::Engine, model::*};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveArclen, PathSeg, Point, Shape, Vec2};
use serde_json::{Value as J, json};
#[derive(Clone, Debug)]
pub struct InkLayer {
    pub path: BezPath,
    pub color: String,
    pub opacity: f64,
}
pub fn head_args(v: &V) -> Option<Args> {
    match v {
        V::Map(m) if m.get("__head").is_some_and(|v| matches!(v, V::Bool(true))) => Some(m.clone()),
        V::Object(o) if o.borrow().kind == "head" => Some(o.borrow().args.clone()),
        _ => None,
    }
}
pub fn css_head(s: &str, file: &str, l: Loc) -> Result<V> {
    use crate::parser::{ExprKind, expression};
    fn literal(e: &crate::parser::Expr) -> Option<V> {
        match &e.kind {
            ExprKind::Number(n, u) => Some(V::Number(*n, u.clone())),
            ExprKind::String(s, _, _) => Some(V::text(s)),
            ExprKind::List(xs) => {
                Some(V::List(xs.iter().map(literal).collect::<Option<Vec<_>>>()?))
            }
            ExprKind::Call(..) => {
                crate::color::constant_expression(e).map(|c| V::Color(std::rc::Rc::new(c)))
            }
            _ => None,
        }
    }
    let expr = expression(s, file)?;
    if let ExprKind::Call(parts, args) = expr.kind {
        if parts == ["head"] {
            let mut out = Args::new();
            out.insert("__head".into(), V::Bool(true));
            for (k, v) in args {
                let k = k.ok_or_else(|| {
                    crate::Diagnostic::new("E_LCSS", "head 样式使用命名参数", file, l)
                })?;
                if ![
                    "shape",
                    "size",
                    "fill",
                    "border_color",
                    "border_width",
                    "opacity",
                ]
                .contains(&k.as_str())
                {
                    return Err(crate::Diagnostic::new(
                        "E_LCSS",
                        format!("未知头部参数 {k}"),
                        file,
                        l,
                    ));
                }
                out.insert(
                    k,
                    literal(&v).ok_or_else(|| {
                        crate::Diagnostic::new("E_LCSS", "head 样式参数需要常量", file, l)
                    })?,
                );
            }
            return Ok(V::Map(out));
        }
    }
    Err(crate::Diagnostic::new(
        "E_LCSS",
        "端点头部需要 head(...)",
        file,
        l,
    ))
}
pub fn line_vector(e: &Engine, a: &Args, l: Loc) -> Result<(f64, f64)> {
    let polar = a.contains_key("length") || a.contains_key("angle");
    if polar {
        if a.contains_key("dx") || a.contains_key("dy") {
            return Err(e.error("E_ARG", "length/angle 与 dx/dy 互斥", l));
        }
        let length = a
            .get("length")
            .ok_or_else(|| e.error("E_ARG", "缺少 length", l))?;
        let length = e.len(length, l)?;
        if !length.is_finite() || length < 0. {
            return Err(e.error("E_LAYOUT", "length 必须为非负有限长度", l));
        }
        if length == 0. && !a.contains_key("angle") {
            return Err(e.error("E_ANCHOR_DIRECTION", "零长度线需要显式 angle", l));
        }
        let angle = match a.get("angle").map(V::value) {
            None => 0.,
            Some(V::Number(v, u)) if u.is_empty() || u == "deg" => *v,
            Some(V::Number(v, u)) if u == "rad" => v.to_degrees(),
            _ => return Err(e.error("E_UNIT", "angle 需要角度", l)),
        };
        if !angle.is_finite() {
            return Err(e.error("E_UNIT", "angle 必须为有限角度", l));
        }
        let (s, c) = angle.to_radians().sin_cos();
        Ok((length * c, length * s))
    } else {
        let dx = e.len(
            a.get("dx")
                .ok_or_else(|| e.error("E_ARG", "需要 dx/dy 或 length/angle", l))?,
            l,
        )?;
        let dy = e.len(
            a.get("dy").ok_or_else(|| e.error("E_ARG", "缺少 dy", l))?,
            l,
        )?;
        if !dx.is_finite() || !dy.is_finite() {
            return Err(e.error("E_LAYOUT", "位移需要有限长度", l));
        }
        if dx == 0. && dy == 0. {
            return Err(e.error(
                "E_ANCHOR_DIRECTION",
                "零长度线使用 length=0 和显式 angle",
                l,
            ));
        }
        Ok((dx, dy))
    }
}
pub fn configure(e: &Engine, n: &mut J, a: &Args, l: Loc) -> Result<()> {
    if !["start_head", "end_head", "start_cap", "end_cap"]
        .iter()
        .any(|k| a.contains_key(*k))
        && n["zeroAngle"].is_null()
    {
        return Ok(());
    }
    let sw = jnum(&n["strokeStyle"], "width", 0.);
    let len = (sw * 4.).max(1.5);
    let width = (len * 0.9).max(sw);
    let mut config = json!({"start_cap":string(a,"start_cap",jstr(&n["strokeStyle"],"cap","butt")),"end_cap":string(a,"end_cap",jstr(&n["strokeStyle"],"cap","butt"))});
    for key in ["start_cap", "end_cap"] {
        if !matches!(jstr(&config, key, ""), "butt" | "round" | "square") {
            return Err(e.error("E_STROKE", format!("无效 {key}"), l));
        }
    }
    for key in ["start_head", "end_head"] {
        if let Some(v) = a.get(key) {
            let args = head_args(v).ok_or_else(|| e.error("E_STROKE", "需要 head(...) 配置", l))?;
            let h = &args;
            let shape = string(h, "shape", "triangle");
            if !["triangle", "open", "stealth", "dot", "diamond", "bar"].contains(&shape.as_str()) {
                return Err(e.error("E_STROKE", "未知头部形状", l));
            }
            let size = if let Some(v) = h.get("size") {
                let v = v.list();
                if v.len() != 2 {
                    return Err(e.error("E_STROKE", "head.size 需要两个正长度", l));
                }
                [e.len(&v[0], l)?, e.len(&v[1], l)?]
            } else {
                [len, width]
            };
            if size.iter().any(|v| !v.is_finite() || *v <= 0.) {
                return Err(e.error("E_STROKE", "head.size 需要两个正长度", l));
            }
            let line_color = jstr(&n["strokeStyle"], "color", "#000000");
            let open = shape == "open";
            let fill = string(h, "fill", if open { "none" } else { line_color });
            let border = string(h, "border_color", if open { line_color } else { "none" });
            for c in [&fill, &border] {
                crate::color::Color::parse(c).map_err(|m| e.error("E_COLOR", m, l))?;
            }
            let bw = h
                .get("border_width")
                .map(|v| e.len(v, l))
                .transpose()?
                .unwrap_or(if open { sw } else { 0. });
            if !bw.is_finite() || bw < 0. || !(0.0..=1.0).contains(&num(h, "opacity", 1.)) {
                return Err(e.error("E_STROKE", "头部描边需要非负宽度，透明度在 0–1 之间", l));
            }
            config[key] = json!({"shape":shape,"size":size,"fill":crate::color::css(&fill),"border_color":crate::color::css(&border),"border_width":bw,"opacity":num(h,"opacity",1.)});
        }
    }
    n["endpointRecipe"] = config;
    layers(n, Affine::IDENTITY).map_err(|m| e.error("E_STROKE", m, l))?;
    Ok(())
}
/// A head-only line needs an intrinsic frame just like other vector materials.
/// Measure nominal geometry independently of paint visibility; the logical
/// endpoints remain coincident inside that frame.
pub fn zero_layout(n: &mut J) {
    let mut nominal = n.clone();
    nominal["strokeStyle"]["color"] = json!("#000000");
    nominal["strokeStyle"]["opacity"] = json!(1.);
    for key in ["start_head", "end_head"] {
        let h = &mut nominal["endpointRecipe"][key];
        if h.is_object() {
            h["fill"] = json!("#000000");
            h["opacity"] = json!(1.);
            if h["border_color"] != "none" {
                h["border_color"] = json!("#000000");
            }
        }
    }
    let region = layers(&nominal, Affine::IDENTITY)
        .expect("validated endpoint recipe")
        .iter()
        .fold(BezPath::new(), |p, q| union(&p, &q.path));
    let b = if region.elements().is_empty() {
        // A butt cap draws no ink but still reserves the nominal pen footprint.
        let r = jnum(&n["strokeStyle"], "width", 0.) / 2.;
        kurbo::Rect::new(-r, -r, r, r)
    } else {
        region.bounding_box()
    };
    let p = Point::new(-b.x0, -b.y0);
    let mut logical = BezPath::new();
    logical.move_to(p);
    logical.line_to(p);
    n["d"] = json!(logical.to_svg());
    n["endpoints"] = json!([[p.x, p.y], [p.x, p.y]]);
    n["width"] = json!(b.width());
    n["height"] = json!(b.height());
}
fn segments(path: &BezPath) -> Vec<(Vec<PathSeg>, bool)> {
    let mut routes = vec![];
    let mut begin = Point::ORIGIN;
    let mut p = begin;
    let mut current = vec![];
    for el in path.iter() {
        match el {
            kurbo::PathEl::MoveTo(q) => {
                if !current.is_empty() {
                    routes.push((std::mem::take(&mut current), false));
                }
                p = q;
                begin = q
            }
            kurbo::PathEl::LineTo(q) => {
                current.push(PathSeg::Line(kurbo::Line::new(p, q)));
                p = q
            }
            kurbo::PathEl::QuadTo(c, q) => {
                current.push(PathSeg::Quad(kurbo::QuadBez::new(p, c, q)));
                p = q
            }
            kurbo::PathEl::CurveTo(a, b, q) => {
                current.push(PathSeg::Cubic(kurbo::CubicBez::new(p, a, b, q)));
                p = q
            }
            kurbo::PathEl::ClosePath => {
                if p.distance(begin) > 1e-12 {
                    current.push(PathSeg::Line(kurbo::Line::new(p, begin)))
                }
                routes.push((std::mem::take(&mut current), true));
                p = begin
            }
        }
    }
    if !current.is_empty() {
        routes.push((current, false))
    }
    routes
}
fn point_at(s: &[PathSeg], lengths: &[f64], mut distance: f64) -> Point {
    for (seg, len) in s.iter().zip(lengths) {
        if distance <= *len {
            return seg.eval(if *len > 0. {
                seg.inv_arclen(distance.clamp(0., *len), 1e-7)
            } else {
                0.
            });
        }
        distance -= len
    }
    s.last().unwrap().end()
}
fn subset(s: &[PathSeg], lengths: &[f64], lo: f64, hi: f64) -> BezPath {
    let mut path = BezPath::new();
    let mut offset = 0.;
    for (seg, len) in s.iter().zip(lengths) {
        let a = (lo - offset).max(0.);
        let b = (hi - offset).min(*len);
        if b > a + 1e-10 {
            let q = seg.subsegment(seg.inv_arclen(a, 1e-7)..seg.inv_arclen(b, 1e-7));
            if path.elements().is_empty() {
                path.move_to(q.start())
            }
            match q {
                PathSeg::Line(q) => path.line_to(q.p1),
                PathSeg::Quad(q) => path.quad_to(q.p1, q.p2),
                PathSeg::Cubic(q) => path.curve_to(q.p1, q.p2, q.p3),
            }
        }
        offset += len
    }
    path
}
fn direction(s: &[PathSeg], start: bool) -> Option<Vec2> {
    let iter: Box<dyn Iterator<Item = &PathSeg>> = if start {
        Box::new(s.iter())
    } else {
        Box::new(s.iter().rev())
    };
    for seg in iter {
        // The first noncoincident control vector is the exact one-sided
        // endpoint limit, including repeated controls and zero derivatives.
        let controls = match *seg {
            PathSeg::Line(q) => vec![q.p1 - q.p0],
            PathSeg::Quad(q) if start => vec![q.p1 - q.p0, q.p2 - q.p0],
            PathSeg::Quad(q) => vec![q.p2 - q.p1, q.p2 - q.p0],
            PathSeg::Cubic(q) if start => vec![q.p1 - q.p0, q.p2 - q.p0, q.p3 - q.p0],
            PathSeg::Cubic(q) => vec![q.p3 - q.p2, q.p3 - q.p1, q.p3 - q.p0],
        };
        for v in controls {
            if v.hypot() > 1e-14 {
                return Some(v / v.hypot());
            }
        }
    }
    None
}
fn head_path(h: &J, tolerance: f64) -> BezPath {
    let len = h["size"][0].as_f64().unwrap();
    let w = h["size"][1].as_f64().unwrap() / 2.;
    let mut path = BezPath::new();
    match jstr(h, "shape", "triangle") {
        "triangle" => {
            path.move_to((0., 0.));
            path.line_to((-len, -w));
            path.line_to((-len, w));
            path.close_path()
        }
        "stealth" => {
            path.move_to((0., 0.));
            path.line_to((-len, -w));
            path.line_to((-len * 0.7, 0.));
            path.line_to((-len, w));
            path.close_path()
        }
        "open" => {
            path.move_to((-len, -w));
            path.line_to((0., 0.));
            path.line_to((-len, w))
        }
        "dot" => path = kurbo::Ellipse::new((0., 0.), (len / 2., w), 0.).to_path(tolerance),
        "diamond" => {
            path.move_to((len / 2., 0.));
            path.line_to((0., w));
            path.line_to((-len / 2., 0.));
            path.line_to((0., -w));
            path.close_path()
        }
        "bar" => path = kurbo::Rect::new(-len / 2., -w, len / 2., w).to_path(0.00001),
        _ => {}
    }
    path
}
fn head(h: &J, p: Point, v: Vec2, tr: Affine) -> (BezPath, Vec<InkLayer>) {
    let path = head_path(h, physical_tolerance(tr));
    let local = Affine::new([v.x, v.y, -v.y, v.x, p.x, p.y]);
    let mut layers = vec![];
    let mut region = BezPath::new();
    let op = jnum(h, "opacity", 1.);
    let fill = jstr(h, "fill", "none");
    if crate::color::visible(fill) && op > 0. {
        let q = tr * local * path.clone();
        region = union(&region, &q);
        layers.push(InkLayer {
            path: q,
            color: fill.into(),
            opacity: op,
        })
    }
    let color = jstr(h, "border_color", "none");
    let bw = jnum(h, "border_width", 0.);
    if crate::color::visible(color) && bw > 0. && op > 0. {
        let q = compound_outline_transformed(
            &path,
            &json!({"width":bw,"cap":"butt","join":"miter"}),
            tr * local,
        );
        region = union(&region, &q);
        layers.push(InkLayer {
            path: q,
            color: color.into(),
            opacity: op,
        })
    }
    // Closed hollow heads still exclude the shaft from their interior. Open
    // V heads use only their actual stroke as the junction silhouette.
    if jstr(h, "shape", "") != "open" && !region.is_empty() {
        region = union(&region, &(tr * local * path));
    }
    (region, layers)
}
fn terminal_span(
    s: &[PathSeg],
    lens: &[f64],
    total: f64,
    region: &BezPath,
    start: bool,
    tolerance: f64,
) -> f64 {
    // Identify the contiguous source interval adjoining the endpoint. This
    // changes clipping ownership, never the logical path or its stroke phase.
    let inside = |d: f64| region.winding(point_at(s, lens, if start { d } else { total - d })) != 0;
    if total <= 1e-12 || region.is_empty() {
        return 0.;
    }
    let mut crossings = vec![0., total];
    let edges = region
        .segments()
        .filter_map(|seg| match seg {
            PathSeg::Line(line) => Some(line),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut offset = 0.;
    for (seg, len) in s.iter().zip(lens) {
        for edge in &edges {
            for hit in seg.intersect_line(*edge) {
                let d = offset
                    + seg
                        .subsegment(0.0..hit.segment_t.clamp(0., 1.))
                        .arclen(1e-7);
                crossings.push(if start { d } else { total - d });
            }
        }
        offset += len;
    }
    crossings.sort_by(f64::total_cmp);
    crossings.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    for pair in crossings.windows(2) {
        // Overlay coordinates can leave a microscopic gap at the exact tip.
        // Resolve it using the same final-space error budget as the outline.
        if pair[1] - pair[0] <= tolerance {
            continue;
        }
        if !inside((pair[0] + pair[1]) / 2.) {
            return pair[0];
        }
    }
    total
}
/// Convex carrier swept backwards in the head frame. The six presets have
/// convex carriers; stealth and open use their outer triangle, retaining the
/// original concave/open head paint independently of the clipping carrier.
fn hull(mut pts: Vec<Point>) -> BezPath {
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    pts.dedup_by(|a, b| a.distance(*b) < 1e-12);
    let chain = |iter: Vec<Point>| {
        let mut out = Vec::<Point>::new();
        for p in iter {
            while out.len() >= 2
                && (out[out.len() - 1] - out[out.len() - 2]).cross(p - out[out.len() - 1]) <= 0.
            {
                out.pop();
            }
            out.push(p);
        }
        out
    };
    let mut polygon = chain(pts.clone());
    polygon.pop();
    let mut upper = chain(pts.into_iter().rev().collect());
    upper.pop();
    polygon.extend(upper);
    let mut path = BezPath::new();
    if let Some(first) = polygon.first() {
        path.move_to(*first);
        for p in &polygon[1..] {
            path.line_to(*p);
        }
        path.close_path();
    }
    path
}
struct Clip {
    start: bool,
    span: f64,
    envelope: BezPath,
}
fn clipping(
    h: &J,
    p: Point,
    v: Vec2,
    tr: Affine,
    source: &BezPath,
    mask: &BezPath,
) -> (BezPath, BezPath) {
    let local = Affine::new([v.x, v.y, -v.y, v.x, p.x, p.y]);
    let frame = tr * local;
    let mut carrier = h.clone();
    if matches!(jstr(h, "shape", ""), "open" | "stealth") {
        carrier["shape"] = json!("triangle");
    }
    let path = head_path(&carrier, physical_tolerance(tr));
    let mut pts = crate::geometry::points(&path, physical_tolerance(tr))
        .into_iter()
        .flatten()
        .map(|q| Point::new(q[0], q[1]))
        .collect::<Vec<_>>();
    if frame.determinant().abs() > 1e-30 {
        pts.extend(
            crate::geometry::points(&(frame.inverse() * mask.clone()), physical_tolerance(tr))
                .into_iter()
                .flatten()
                .map(|q| Point::new(q[0], q[1])),
        );
    }
    let b = hull(pts.clone()).bounding_box();
    let source_bounds = (local.inverse() * source.clone()).bounding_box();
    let reach = (b.x0 - source_bounds.x0).max(0.) + b.width() + 1.;
    pts.extend(pts.clone().into_iter().map(|q| q - Vec2::new(reach, 0.)));
    let envelope = hull(pts);
    let window = intersection(
        &envelope,
        &kurbo::Rect::new(b.x0, b.y0 - 1., b.x1 + 1., b.y1 + 1.).to_path(0.00001),
    );
    (frame * envelope, local * window)
}
fn stroke_interval(
    s: &[PathSeg],
    lens: &[f64],
    lo: f64,
    hi: f64,
    stroke: &J,
    caps: [&str; 2],
    closed: bool,
    tr: Affine,
) -> BezPath {
    if hi <= lo + 1e-12 {
        return BezPath::new();
    }
    let total = lens.iter().sum::<f64>();
    let mut route = subset(s, lens, lo, hi);
    if closed {
        route.close_path();
    }
    let mut style = stroke.clone();
    style["dashOffset"] = json!(jnum(stroke, "dashOffset", 0.) + lo);
    let dash = style["dash"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(J::as_f64)
        .collect::<Vec<_>>();
    let dashed = if dash.is_empty() {
        route
    } else {
        kurbo::dash(route.iter(), jnum(&style, "dashOffset", 0.), &dash).collect::<BezPath>()
    };
    style["dash"] = json!([]);
    let mut body = BezPath::new();
    for (ds, dc) in segments(&dashed) {
        if ds.is_empty() {
            continue;
        }
        let mut part = BezPath::new();
        part.move_to(ds[0].start());
        for seg in &ds {
            match *seg {
                PathSeg::Line(q) => part.line_to(q.p1),
                PathSeg::Quad(q) => part.quad_to(q.p1, q.p2),
                PathSeg::Cubic(q) => part.curve_to(q.p1, q.p2, q.p3),
            }
        }
        if dc || (closed && dash.is_empty()) {
            part.close_path();
        }
        style["startCap"] = json!(if ds[0].start().distance(point_at(s, lens, lo)) < 1e-7 {
            if lo <= 1e-12 { caps[0] } else { "butt" }
        } else {
            jstr(stroke, "cap", "butt")
        });
        style["endCap"] = json!(
            if ds.last().unwrap().end().distance(point_at(s, lens, hi)) < 1e-7 {
                if hi >= total - 1e-12 { caps[1] } else { "butt" }
            } else {
                jstr(stroke, "cap", "butt")
            }
        );
        body = union(&body, &compound_outline_transformed(&part, &style, tr));
    }
    body
}
fn physical_tolerance(tr: Affine) -> f64 {
    let c = tr.as_coeffs();
    0.0001 / c[..4].iter().map(|v| v * v).sum::<f64>().sqrt().max(1.)
}
pub fn layers(n: &J, transform: Affine) -> std::result::Result<Vec<InkLayer>, String> {
    let sx = if jnum(n, "intrinsicWidth", 0.) > 0. {
        jnum(n, "width", 0.) / jnum(n, "intrinsicWidth", 1.)
    } else {
        1.
    };
    let sy = if jnum(n, "intrinsicHeight", 0.) > 0. {
        jnum(n, "height", 0.) / jnum(n, "intrinsicHeight", 1.)
    } else {
        1.
    };
    let tr = transform * Affine::scale_non_uniform(sx, sy);
    let source = if tr.determinant().abs() > 1e-30 {
        tr.inverse() * crate::geometry_recipe::resolved_path(n, tr)
    } else {
        BezPath::from_svg(jstr(n, "d", "")).unwrap_or_default()
    };
    let config = &n["endpointRecipe"];
    let stroke = &n["strokeStyle"];
    let color = crate::color::css(jstr(stroke, "color", "none"));
    let opacity = jnum(stroke, "opacity", 1.);
    let sw = jnum(stroke, "width", 0.);
    let mut out = vec![];
    if crate::color::visible(jstr(n, "fill", "none")) {
        out.push(InkLayer {
            path: tr * source.clone(),
            color: crate::color::css(jstr(n, "fill", "none")),
            opacity: 1.,
        })
    }
    for (s, closed) in segments(&source) {
        if s.is_empty() {
            continue;
        }
        if closed && (config["start_head"].is_object() || config["end_head"].is_object()) {
            return Err("闭合子路径不支持头部，请拆分为开放路径".into());
        }
        let lens = s.iter().map(|q| q.arclen(1e-7)).collect::<Vec<_>>();
        let total = lens.iter().sum::<f64>();
        let mut heads = vec![];
        let mut clips = vec![];
        let mut masks = BezPath::new();
        let mut active = [false; 2];
        for (index, (start, key)) in [(true, "start_head"), (false, "end_head")]
            .into_iter()
            .enumerate()
        {
            let h = &config[key];
            if !h.is_object() {
                continue;
            }
            let angle = n["zeroAngle"].as_f64();
            let v = direction(&s, start)
                .or_else(|| angle.map(|a| Vec2::new(a.to_radians().cos(), a.to_radians().sin())))
                .ok_or("端点没有可定义的方向")?;
            let p = if start {
                s[0].start()
            } else {
                s.last().unwrap().end()
            };
            let facing = if start { -v } else { v };
            let (mask, paint) = head(h, p, facing, tr);
            if !paint.is_empty() {
                active[index] = true;
                let (envelope, window) = clipping(h, p, facing, tr, &source, &mask);
                let span = terminal_span(&s, &lens, total, &window, start, physical_tolerance(tr));
                clips.push(Clip {
                    start,
                    span,
                    envelope,
                });
                masks = union(&masks, &mask);
            }
            heads.extend(paint);
        }
        let caps = [
            if active[0] {
                "butt"
            } else {
                jstr(config, "start_cap", jstr(stroke, "cap", "butt"))
            },
            if active[1] {
                "butt"
            } else {
                jstr(config, "end_cap", jstr(stroke, "cap", "butt"))
            },
        ];
        let mut body = BezPath::new();
        if total > 1e-12 {
            // Always generate the complete stroke first. Split only its source
            // ownership for local clipping; preserve the source dash origin.
            let full = stroke_interval(&s, &lens, 0., total, stroke, caps, closed, tr);
            let mut cuts = vec![0., total];
            for c in &clips {
                cuts.push(if c.start { c.span } else { total - c.span });
            }
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
            let mut raw = BezPath::new();
            for pair in cuts.windows(2) {
                let (lo, hi) = (pair[0], pair[1]);
                let mut part = stroke_interval(&s, &lens, lo, hi, stroke, caps, closed, tr);
                raw = union(&raw, &part);
                let mid = (lo + hi) / 2.;
                for c in &clips {
                    if if c.start {
                        mid < c.span
                    } else {
                        mid > total - c.span
                    } {
                        part = intersection(&part, &c.envelope);
                    }
                }
                body = union(&body, &part);
            }
            // Keep original joins when an ownership boundary coincides with a
            // source node. Apply its endpoint clip before restoring that join.
            let mut residual = difference(&full, &raw);
            for cut in cuts.iter().skip(1).take(cuts.len().saturating_sub(2)) {
                let neighborhood = tr
                    * kurbo::Circle::new(
                        point_at(&s, &lens, *cut),
                        sw * jnum(stroke, "miterLimit", 4.).max(1.) + 0.001,
                    )
                    .to_path(physical_tolerance(tr));
                let mut join = intersection(&residual, &neighborhood);
                residual = difference(&residual, &neighborhood);
                for c in &clips {
                    if if c.start {
                        *cut <= c.span + 1e-10
                    } else {
                        *cut >= total - c.span - 1e-10
                    } {
                        join = intersection(&join, &c.envelope);
                    }
                }
                body = union(&body, &join);
            }
            body = union(&body, &residual);
            // The complete head silhouette owns the overlap, including the
            // empty interior of closed hollow heads. Open heads mask only arms.
            body = difference(&body, &masks);
        } else if total <= 1e-12
            && !config["start_head"].is_object()
            && !config["end_head"].is_object()
            && sw > 0.
        {
            let p = s[0].start();
            for key in ["start_cap", "end_cap"] {
                let cap = jstr(config, key, "butt");
                let q = match cap {
                    "round" => kurbo::Circle::new(p, sw / 2.).to_path(physical_tolerance(tr)),
                    "square" => {
                        let angle = jnum(n, "zeroAngle", 0.).to_radians();
                        Affine::translate(p.to_vec2())
                            * Affine::rotate(angle)
                            * kurbo::Rect::new(-sw / 2., -sw / 2., sw / 2., sw / 2.)
                                .to_path(0.00001)
                    }
                    _ => BezPath::new(),
                };
                body = union(&body, &(tr * q))
            }
        }
        if crate::color::visible(&color) && opacity > 0. && !body.elements().is_empty() {
            out.push(InkLayer {
                path: body,
                color: color.clone(),
                opacity,
            })
        }
        out.extend(heads);
    }
    // Union equal paints before compositing; preserve ordering for distinct paints.
    let mut merged: Vec<InkLayer> = vec![];
    for layer in out {
        if let Some(existing) = merged
            .last_mut()
            .filter(|p| p.color == layer.color && p.opacity == layer.opacity)
        {
            existing.path = union(&existing.path, &layer.path)
        } else {
            merged.push(layer)
        }
    }
    // Remove shaft ink under independent head paints, including translucent ones.
    for i in 0..merged.len() {
        let mask = merged
            .iter()
            .skip(i + 1)
            .filter(|q| q.color != merged[i].color || q.opacity != merged[i].opacity)
            .fold(BezPath::new(), |p, q| union(&p, &q.path));
        merged[i].path = difference(&merged[i].path, &mask)
    }
    Ok(merged)
}
