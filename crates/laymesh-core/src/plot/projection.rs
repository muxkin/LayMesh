use super::*;
use std::f64::consts::{PI, TAU};
fn tick_reach(e: &Engine, aa: &Args, small: bool) -> (f64, f64) {
    let length = length_arg(
        e,
        aa,
        if small {
            "minor_tick_length"
        } else {
            "tick_length"
        },
        if small { 0.6 } else { 1.2 },
    );
    match string(aa, "tick_direction", "out").as_str() {
        "in" => (-length, 0.),
        "inout" => (-length / 2., length / 2.),
        _ => (0., length),
    }
}
fn radial(m: &Json, v: f64) -> f64 {
    let a = &m["radial"];
    let d = [
        a["domain"][0].as_f64().unwrap(),
        a["domain"][1].as_f64().unwrap(),
    ];
    let mut r = [jnum(m, "innerRadius", 0.), jnum(m, "outerRadius", 0.)];
    if a["reverse"] == true {
        r.reverse();
    }
    interpolate(
        v.abs().max(if a["scale"] == "log" { d[0] } else { 0. }),
        d,
        r,
        jstr(a, "scale", "linear"),
        jnum(a, "constant", 1.),
    )
}
pub(super) fn polar_point(m: &Json, theta: f64, r: f64) -> [f64; 2] {
    let factor = if m["angleUnit"] == "rad" {
        1.
    } else {
        PI / 180.
    };
    let a = jnum(m, "thetaZero", 0.)
        + jnum(m, "direction", 1.) * (theta * factor + if r < 0. { PI } else { 0. });
    let radius = radial(m, r);
    [
        m["center"][0].as_f64().unwrap() + radius * a.cos(),
        m["center"][1].as_f64().unwrap() - radius * a.sin(),
    ]
}
fn radar_point(m: &Json, i: usize, v: f64) -> [f64; 2] {
    let ranges = m["ranges"].as_array().unwrap();
    let range = &ranges[i % ranges.len()];
    let lo = range[0].as_f64().unwrap();
    let hi = range[1].as_f64().unwrap();
    let radius = (v - lo) / (hi - lo) * jnum(m, "outerRadius", 0.);
    let a = jnum(m, "thetaZero", PI / 2.)
        + jnum(m, "direction", -1.) * i as f64 * TAU / ranges.len() as f64;
    [
        m["center"][0].as_f64().unwrap() + radius * a.cos(),
        m["center"][1].as_f64().unwrap() - radius * a.sin(),
    ]
}
fn polar_visible(m: &Json, p: [f64; 2]) -> bool {
    let period = if m["angleUnit"] == "rad" { TAU } else { 360. };
    let lo = m["theta"][0].as_f64().unwrap();
    let hi = m["theta"][1].as_f64().unwrap();
    let theta = lo + (p[0] + if p[1] < 0. { period / 2. } else { 0. } - lo).rem_euclid(period);
    let domain = &m["radial"]["domain"];
    p.iter().all(|v| v.is_finite())
        && p[1].abs() >= domain[0].as_f64().unwrap()
        && p[1].abs() <= domain[1].as_f64().unwrap()
        && theta <= hi + 1e-9
}
pub(super) fn projected_points(p: &Primitive, layer: &Layer, m: &Json) -> Vec<[f64; 2]> {
    project_raw(
        &p.points,
        p.closed,
        &layer.kind,
        &string(&layer.args, "wrap", "shortest"),
        &string(&layer.args, "interpolation", "polar"),
        m,
        0.0008,
    )
}
pub(super) fn unwrap_angle_points(source: &[[f64; 2]], period: f64) -> Vec<[f64; 2]> {
    let mut points = source.to_vec();
    for i in 1..points.len() {
        let mut delta = (points[i][0] - points[i - 1][0]).rem_euclid(period);
        if delta > period / 2. {
            delta -= period;
        }
        points[i][0] = points[i - 1][0] + delta;
    }
    points
}
fn project_raw(
    source: &[[f64; 2]],
    closed: bool,
    kind: &str,
    wrap: &str,
    interpolation: &str,
    m: &Json,
    tolerance: f64,
) -> Vec<[f64; 2]> {
    if m["kind"] == "radar" {
        return source
            .iter()
            .map(|q| radar_point(m, q[0] as usize, q[1]))
            .collect();
    }
    if source.len() < 2 {
        return source.iter().map(|q| polar_point(m, q[0], q[1])).collect();
    }
    let chord = interpolation == "chord";
    let shortest =
        wrap == "shortest" && !matches!(kind, "bar" | "hist" | "heatmap" | "contour" | "contourf");
    let period = if m["angleUnit"] == "rad" { TAU } else { 360. };
    let mut points = source.to_vec();
    if shortest {
        points = unwrap_angle_points(&points, period);
    }
    if closed {
        let mut end = points[0];
        if shortest {
            let delta =
                (end[0] - points.last().unwrap()[0] + period / 2.).rem_euclid(period) - period / 2.;
            end[0] = points.last().unwrap()[0] + delta;
        }
        points.push(end);
    }
    let mut out = vec![polar_point(m, points[0][0], points[0][1])];
    for pair in points.windows(2) {
        if chord {
            out.push(polar_point(m, pair[1][0], pair[1][1]));
            continue;
        }
        if pair[0][1] * pair[1][1] < 0. {
            let f = -pair[0][1] / (pair[1][1] - pair[0][1]);
            let zero = [pair[0][0] + f * (pair[1][0] - pair[0][0]), 0.];
            subdivide(m, pair[0], zero, 0, &mut out, tolerance);
            subdivide(m, zero, pair[1], 0, &mut out, tolerance);
        } else {
            subdivide(m, pair[0], pair[1], 0, &mut out, tolerance);
        }
    }
    out
}
fn subdivide(
    m: &Json,
    a: [f64; 2],
    b: [f64; 2],
    depth: u8,
    out: &mut Vec<[f64; 2]>,
    tolerance: f64,
) {
    let mid = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
    let (pa, pb, pm) = (
        polar_point(m, a[0], a[1]),
        polar_point(m, b[0], b[1]),
        polar_point(m, mid[0], mid[1]),
    );
    let err = (pm[0] - (pa[0] + pb[0]) / 2.).hypot(pm[1] - (pa[1] + pb[1]) / 2.);
    let factor = if m["angleUnit"] == "rad" {
        1.
    } else {
        PI / 180.
    };
    if depth < 28 && (err > tolerance || (b[0] - a[0]).abs() * factor > PI / 12.) {
        subdivide(m, a, mid, depth + 1, out, tolerance);
        subdivide(m, mid, b, depth + 1, out, tolerance);
    } else {
        out.push(pb);
    }
}
fn ring(m: &Json, radius: f64, start: f64, end: f64) -> Vec<[f64; 2]> {
    ring_tolerance(m, radius, start, end, 0.0008)
}
fn ring_tolerance(m: &Json, radius: f64, start: f64, end: f64, tolerance: f64) -> Vec<[f64; 2]> {
    let a0 = jnum(m, "thetaZero", 0.) + jnum(m, "direction", 1.) * start;
    let a1 = jnum(m, "thetaZero", 0.) + jnum(m, "direction", 1.) * end;
    let step = 4.
        * (tolerance / (2. * radius.max(tolerance)))
            .clamp(0., 1.)
            .sqrt()
            .asin();
    let n = ((a1 - a0).abs() / step).ceil().max(2.) as usize;
    linspace(a0, a1, n + 1)
        .iter()
        .map(|a| {
            [
                m["center"][0].as_f64().unwrap() + radius * a.cos(),
                m["center"][1].as_f64().unwrap() - radius * a.sin(),
            ]
        })
        .collect()
}
fn clip_path(m: &Json) -> String {
    clip_tolerance(m, 0.0008).to_svg()
}
fn clip_tolerance(m: &Json, tolerance: f64) -> BezPath {
    let outer = jnum(m, "outerRadius", 0.);
    let inner = jnum(m, "innerRadius", 0.);
    let fac = if m["angleUnit"] == "rad" {
        1.
    } else {
        PI / 180.
    };
    let start = m["theta"][0].as_f64().unwrap() * fac;
    let end = m["theta"][1].as_f64().unwrap() * fac;
    let full = (end - start - TAU).abs() < 1e-8;
    let mut p = BezPath::new();
    let pts = if m["kind"] == "radar" && m["radarFrame"] == "polygon" {
        let mut pts = vec![];
        for (i, range) in m["ranges"].as_array().unwrap().iter().enumerate() {
            pts.push(radar_point(m, i, range[1].as_f64().unwrap()));
        }
        pts
    } else {
        ring_tolerance(m, outer, start, end, tolerance)
    };
    for (i, q) in pts.iter().enumerate() {
        if i == 0 {
            p.move_to((q[0], q[1]));
        } else {
            p.line_to((q[0], q[1]));
        }
    }
    if inner > 0. {
        let innerpts = ring_tolerance(m, inner, end, start, tolerance);
        if full {
            p.close_path();
            p.move_to((innerpts[0][0], innerpts[0][1]));
        }
        for q in innerpts {
            p.line_to((q[0], q[1]));
        }
    } else if !full {
        p.line_to((
            m["center"][0].as_f64().unwrap(),
            m["center"][1].as_f64().unwrap(),
        ));
    }
    p.close_path();
    p
}
pub(super) fn resolved_polar_path(recipe: &Json, tolerance: f64) -> BezPath {
    let m = &recipe["m"];
    if recipe["mode"] == "boundary" {
        return clip_tolerance(m, tolerance);
    }
    let mut result = BezPath::new();
    let mut append = |points: Vec<[f64; 2]>, closed: bool| {
        for (i, q) in points.iter().enumerate() {
            if i == 0 {
                result.move_to((q[0], q[1]));
            } else {
                result.line_to((q[0], q[1]));
            }
        }
        if closed {
            result.close_path();
        }
    };
    if recipe["mode"] == "ring" {
        append(
            ring_tolerance(
                m,
                jnum(recipe, "radius", 0.),
                jnum(recipe, "start", 0.),
                jnum(recipe, "end", 0.),
                tolerance,
            ),
            recipe["closed"] == true,
        );
    } else {
        for raw in recipe["rings"].as_array().into_iter().flatten() {
            let points: Vec<[f64; 2]> = serde_json::from_value(raw.clone()).unwrap_or_default();
            let closed = recipe["closed"] == true;
            let kind = jstr(recipe, "layer", "line");
            let interpolation = jstr(recipe, "interpolation", "polar");
            let wrap = jstr(recipe, "wrap", "shortest");
            if !closed
                && m["kind"] != "radar"
                && interpolation != "chord"
                && points.windows(2).any(|p| p[0][1] * p[1][1] < 0.)
            {
                let points = if wrap == "shortest" {
                    unwrap_angle_points(&points, if m["angleUnit"] == "rad" { TAU } else { 360. })
                } else {
                    points
                };
                let mut run = vec![points[0]];
                for pair in points.windows(2) {
                    if pair[0][1] * pair[1][1] < 0. {
                        let f = -pair[0][1] / (pair[1][1] - pair[0][1]);
                        let zero = [pair[0][0] + f * (pair[1][0] - pair[0][0]), 0.];
                        run.push(zero);
                        append(
                            project_raw(&run, false, kind, "raw", interpolation, m, tolerance),
                            false,
                        );
                        run = vec![zero];
                    }
                    run.push(pair[1]);
                }
                append(
                    project_raw(&run, false, kind, "raw", interpolation, m, tolerance),
                    false,
                );
            } else {
                append(
                    project_raw(&points, closed, kind, wrap, interpolation, m, tolerance),
                    closed,
                );
            }
        }
    }
    result
}
fn projected_recipe(p: &Primitive, layer: &Layer, m: &Json) -> Json {
    let rings: Vec<_> = std::iter::once(&p.points).chain(p.holes.iter()).collect();
    json!({"kind":"polar","mode":"projected","m":m,"rings":rings,"closed":p.closed,"layer":layer.kind,"wrap":string(&layer.args,"wrap","shortest"),"interpolation":string(&layer.args,"interpolation","polar")})
}
fn ring_recipe(m: &Json, radius: f64, start: f64, end: f64, closed: bool) -> Json {
    json!({"kind":"polar","mode":"ring","m":m,"radius":radius,"start":start,"end":end,"closed":closed})
}
fn boundary_recipe(m: &Json) -> Json {
    json!({"kind":"polar","mode":"boundary","m":m})
}
pub fn polar_layout(
    e: &mut Engine,
    o: &Object,
    a: &Args,
    style: &Args,
    layers: &[Layer],
    mut area: [f64; 4],
    size: [f64; 2],
    fixed: bool,
) -> Result<Json> {
    let (w, h) = (size[0], size[1]);
    let kind = string(a, "projection", "polar");
    let radar = kind == "radar";
    let categories: Vec<_> = array(a, "categories").iter().map(V::as_str).collect();
    if radar
        && (categories.len() < 3
            || categories
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != categories.len())
    {
        return Err(e.error("E_PLOT", "雷达图需要至少三个唯一类别", o.loc));
    }
    if a.contains_key("x") || a.contains_key("y") {
        return Err(e.error("E_PLOT", "极坐标使用 theta/r 轴", o.loc));
    }
    let ra = part_style(e, a, style, "axis", Some("r"), &object_args(a, "r"))?;
    let ta = part_style(e, a, style, "axis", Some("theta"), &object_args(a, "theta"))?;
    if !array(&ra, "breaks").is_empty() || !array(&ta, "breaks").is_empty() {
        return Err(e.error("E_PLOT", "极坐标和雷达图不支持断轴", o.loc));
    }
    if string(&ta, "scale", "linear") != "linear" || yes(&ta, "reverse", false) {
        return Err(e.error(
            "E_PLOT",
            "角轴使用线性刻度；方向由 theta_direction 控制",
            o.loc,
        ));
    }
    if radar && (string(&ra, "scale", "linear") != "linear" || yes(&ra, "reverse", false)) {
        return Err(e.error("E_PLOT", "雷达图使用线性、非反向径向尺度", o.loc));
    }
    let period = if string(a, "angle_unit", "deg") == "rad" {
        TAU
    } else {
        360.
    };
    let tdom = nums(&ta, "range");
    let theta = if tdom.len() == 2 {
        [tdom[0], tdom[1]]
    } else {
        [0., period]
    };
    if theta[1] <= theta[0] || theta[1] - theta[0] > period * (1. + 1e-12) {
        return Err(e.error("E_PLOT", "角范围必须递增且不能超过一周", o.loc));
    }
    let values: Vec<_> = layers
        .iter()
        .flat_map(|l| layer_values(l, 1))
        .map(|v| if radar { v } else { v.abs() })
        .collect();
    let (low, high) =
        bounds(&values).ok_or_else(|| e.error("E_DATA", "没有有效半径数据", o.loc))?;
    let scale = string(&ra, "scale", "linear");
    if scale == "log" && low <= 0. {
        return Err(e.error("E_PLOT", "对数半径的数据必须非零", o.loc));
    }
    let explicit = nums(&ra, "range");
    let ranges: Vec<Vec<f64>> = array(a, "ranges")
        .iter()
        .map(|v| {
            v.list()
                .iter()
                .map(|v| v.number().unwrap_or(f64::NAN))
                .collect()
        })
        .collect();
    if !ranges.is_empty() && !explicit.is_empty() {
        return Err(e.error("E_PLOT", "ranges 与共同 r.range 二选一", o.loc));
    }
    let domain = if !ranges.is_empty() {
        [0., 1.]
    } else if explicit.len() == 2 {
        [explicit[0], explicit[1]]
    } else if scale == "log" {
        if low == high {
            [low / 10f64.sqrt(), high * 10f64.sqrt()]
        } else {
            [low, high]
        }
    } else {
        [
            if radar { low.min(0.) } else { 0. },
            high.max(if radar { 0. } else { 1e-6 }),
        ]
    };
    if domain[0] >= domain[1] || !radar && (domain[0] < 0. || scale == "log" && domain[0] <= 0.) {
        return Err(e.error("E_PLOT", "径向范围须递增非负，对数范围须为正", o.loc));
    }
    let ranges = if radar {
        if ranges.is_empty() {
            vec![domain.to_vec(); categories.len()]
        } else {
            ranges
        }
    } else {
        vec![]
    };
    if radar
        && (ranges.len() != categories.len() || ranges.iter().any(|v| v.len() != 2 || v[0] >= v[1]))
    {
        return Err(e.error("E_PLOT", "ranges 须与 categories 等长且递增", o.loc));
    }
    if radar {
        for layer in layers {
            let ys = nums(&layer.args, "y");
            if ys.len() != categories.len() {
                return Err(e.error("E_PLOT", "values 须与 categories 等长", o.loc));
            }
            for (i, v) in ys.iter().enumerate() {
                if v.is_finite() && (*v < ranges[i][0] || *v > ranges[i][1]) {
                    return Err(e.error("E_PLOT", "雷达指标超出范围", o.loc));
                }
            }
        }
    }
    let fs = length_arg(e, style, "font_size", 8. * PT);
    let ts = length_arg(e, style, "tick_font_size", fs);
    let tick_style = |axis: &Args, name: &str| -> Result<Args> {
        let mut st = style.clone();
        st.insert("color".into(), axis_text_color(axis, style));
        for (key, value) in axis {
            if key.starts_with("font_") {
                st.insert(key.clone(), value.clone());
            }
        }
        st = part_style(e, a, &st, "tick-label", Some(name), &Args::new())?;
        for (from, to) in [
            ("tick_font_family", "font_family"),
            ("tick_font_weight", "font_weight"),
            ("tick_font_style", "font_style"),
            ("tick_color", "color"),
        ] {
            if let Some(v) = axis.get(from) {
                st.insert(to.into(), v.clone());
            }
        }
        Ok(st)
    };
    let rstyle = tick_style(&ra, "r")?;
    let tstyle = tick_style(&ta, "theta")?;
    let rts = length_arg(
        e,
        &ra,
        "tick_font_size",
        length_arg(e, &rstyle, "font_size", ts),
    );
    let tts = length_arg(
        e,
        &ta,
        "tick_font_size",
        length_arg(e, &tstyle, "font_size", ts),
    );
    if !fixed && !a.contains_key("__projection_iteration") {
        area = [10., 10., w - 20., h - 20.];
    }
    let outer = area[2].min(area[3]) / 2.;
    let inner = length_arg(e, a, "inner_radius", 0.);
    if inner < 0. || inner >= outer {
        return Err(e.error("E_LAYOUT", "inner_radius 必须非负且小于绘图区外半径", o.loc));
    }
    let zero = angle(a, "theta_zero", if radar { 90. } else { 0. }).to_radians();
    let direction = if string(a, "theta_direction", if radar { "cw" } else { "ccw" }) == "cw" {
        -1.
    } else {
        1.
    };
    let m = json!({"kind":kind,"center":[area[0]+area[2]/2.,area[1]+area[3]/2.],"outerRadius":outer,"innerRadius":inner,"angleUnit":string(a,"angle_unit","deg"),"thetaZero":zero,"direction":direction,"rLabelAngle":angle(a,"r_label_angle",22.5).to_radians(),"theta":theta,"radial":{"scale":scale,"domain":domain,"constant":num(&ra,"constant",1.),"reverse":yes(&ra,"reverse",false)},"categories":categories,"ranges":ranges,"radarFrame":string(a,"radar_frame","polygon")});
    let mut children = vec![];
    let mut grid = vec![];
    let mut data = vec![];
    let mut decorations = vec![];
    let gc = string(style, "grid_color", "#dddddd");
    let color = string(style, "line_color", &string(style, "color", "#222222"));
    let sw = length_arg(e, style, "line_width", 0.6 * PT);
    let gw = length_arg(e, style, "grid_line_width", 0.3 * PT);
    let mut radial_axis = ra.clone();
    radial_axis.insert(
        "range".into(),
        V::List(domain.into_iter().map(V::num).collect()),
    );
    let axis = Axis::new(e, radial_axis, "r", "left", 0., &[], o.loc)?;
    let rticks = axis.ticks();
    let center = [
        m["center"][0].as_f64().unwrap(),
        m["center"][1].as_f64().unwrap(),
    ];
    let point = |theta: f64, radius: f64| {
        let bearing = zero + direction * theta;
        [
            center[0] + radius * bearing.cos(),
            center[1] - radius * bearing.sin(),
        ]
    };

    let rcustom = array(&ra, "tick_text");
    for (ri, v) in rticks.iter().enumerate() {
        if *v < domain[0] || *v > domain[1] {
            return Err(e.error("E_PLOT", "径向刻度须位于范围内", o.loc));
        }
        let radius = radial(&m, *v);
        if string(&ra, "grid", "none") != "none" {
            let pts = if radar && m["radarFrame"] == "polygon" {
                (0..categories.len())
                    .map(|i| {
                        let t = (v - domain[0]) / (domain[1] - domain[0]);
                        radar_point(&m, i, ranges[i][0] + t * (ranges[i][1] - ranges[i][0]))
                    })
                    .collect()
            } else {
                ring(&m, radius, theta[0] / period * TAU, theta[1] / period * TAU)
            };
            let mut grid_node = path(&pts, radar, w, h, json!("none"), &gc, gw);
            if !radar || m["radarFrame"] != "polygon" {
                grid_node["geometryRecipe"] = ring_recipe(
                    &m,
                    radius,
                    theta[0] / period * TAU,
                    theta[1] / period * TAU,
                    radar,
                );
            }
            grid.push(grid_node);
        }
        if yes(&ra, "tick_labels", true) && !radar {
            let mut n = if let Some(v) = rcustom.get(ri) {
                label_at(e, v, &rstyle, rts, o, &ra, "tick_text")?
            } else {
                number_label(e, *v, &ra, domain, &rstyle, rts, o)?
            };
            let angle = jnum(&m, "rLabelAngle", 0.);
            let q = point(angle, radius);
            let nh = jnum(&n, "height", 0.);
            let offset = pair(&ra, "tick_offset", [0., 0.], &e.unit, e.dpi);
            n["rotation"] = json!(super::angle(&ra, "tick_rotation", 0.));
            place(
                &mut n,
                q[0] + offset[0],
                q[1] - nh / 2. - 1.2 + offset[1],
                [0.5, 0.5],
            );
            decorations.push(decoration_bounds(&n, &format!("r.tick:{v}")));
            children.push(n);
        }
    }
    if !radar {
        let ray = jnum(&m, "rLabelAngle", 0.);
        let bearing = zero + direction * ray;
        let rc = string(&ra, "color", &color);
        let rw = length_arg(e, &ra, "line_width", sw);
        if yes(&ra, "spine", true) {
            children.push(line(point(ray, inner), point(ray, outer), w, h, &rc, rw));
        }
        for (small, values) in [(false, rticks.clone()), (true, axis.minor_ticks(&rticks))] {
            for v in values {
                let radius = radial(&m, v);
                let q = point(ray, radius);
                let (inside, outside) = tick_reach(e, &ra, small);
                children.push(line(
                    [q[0] + bearing.sin() * inside, q[1] + bearing.cos() * inside],
                    [
                        q[0] + bearing.sin() * outside,
                        q[1] + bearing.cos() * outside,
                    ],
                    w,
                    h,
                    &rc,
                    rw,
                ));
                if small && string(&ra, "grid", "none") == "both" {
                    let mut n = path(
                        &ring(&m, radius, theta[0] / period * TAU, theta[1] / period * TAU),
                        false,
                        w,
                        h,
                        json!("none"),
                        &gc,
                        gw,
                    );
                    n["opacity"] = json!(0.5);
                    n["geometryRecipe"] = ring_recipe(
                        &m,
                        radius,
                        theta[0] / period * TAU,
                        theta[1] / period * TAU,
                        false,
                    );
                    grid.push(n);
                }
            }
        }
    }
    let tticks = if radar {
        (0..categories.len()).map(|i| i as f64).collect::<Vec<_>>()
    } else if ta.contains_key("ticks") {
        nums(&ta, "ticks")
    } else if (theta[1] - theta[0] - period).abs() < 1e-8 {
        (0..8).map(|i| theta[0] + i as f64 * period / 8.).collect()
    } else {
        scales::ticks(theta, 6)
    };
    let catlabels = array(a, "category_labels");
    for (i, t) in tticks.iter().enumerate() {
        if !radar && (*t < theta[0] || *t > theta[1]) {
            return Err(e.error("E_PLOT", "角刻度须位于角范围内", o.loc));
        }
        let q = if radar {
            radar_point(&m, i, ranges[i][1])
        } else {
            polar_point(
                &m,
                *t,
                if yes(&ra, "reverse", false) {
                    domain[0]
                } else {
                    domain[1]
                },
            )
        };
        let center = [
            m["center"][0].as_f64().unwrap(),
            m["center"][1].as_f64().unwrap(),
        ];
        if string(&ta, "grid", "none") != "none" || radar {
            grid.push(line(center, q, w, h, &gc, gw));
        }
        if radar {
            if yes(&ra, "spine", true) {
                children.push(line(center, q, w, h, &string(&ra, "color", &color), sw));
            }
            if yes(&ra, "tick_labels", true) {
                for (ri, v) in rticks.iter().enumerate() {
                    let frac = (v - domain[0]) / (domain[1] - domain[0]);
                    if frac == 0. {
                        continue;
                    }
                    let value = ranges[i][0] + frac * (ranges[i][1] - ranges[i][0]);
                    let mut n = if let Some(v) = rcustom.get(ri) {
                        label_at(e, v, &rstyle, rts, o, &ra, "tick_text")?
                    } else {
                        number_label(e, value, &ra, [ranges[i][0], ranges[i][1]], &rstyle, rts, o)?
                    };
                    let q = point(
                        i as f64 * TAU / categories.len() as f64,
                        (frac * outer - 1.2 - jnum(&n, "height", 0.) / 2.).max(0.),
                    );
                    let offset = pair(&ra, "tick_offset", [0., 0.], &e.unit, e.dpi);
                    place(&mut n, q[0] + 2. + offset[0], q[1] + offset[1], [0.5, 0.5]);
                    n["rotation"] = json!(super::angle(&ra, "tick_rotation", 0.));
                    decorations.push(decoration_bounds(
                        &n,
                        &format!("{}.tick:{value}", categories[i]),
                    ));
                    children.push(n);
                }
            }
        } else {
            let (inside, outside) = tick_reach(e, &ta, false);
            children.push(line(
                point(t / period * TAU, outer + inside),
                point(t / period * TAU, outer + outside),
                w,
                h,
                &string(&ta, "color", &color),
                sw,
            ));
        }
        if yes(&ta, "tick_labels", true)
            && (radar
                || (theta[1] - theta[0] - period).abs() > 1e-8
                || (*t - theta[1]).abs() > 1e-8)
        {
            let value = if radar {
                catlabels.get(i).cloned().unwrap_or(V::text(&categories[i]))
            } else {
                let custom = array(&ta, "tick_text");
                custom.get(i).cloned().unwrap_or(V::text(
                    if string(a, "angle_unit", "deg") == "deg" {
                        format!("{}°", format_number(*t, &ta))
                    } else {
                        format_number(*t, &ta)
                    },
                ))
            };
            let mut n = label_at(e, &value, &tstyle, tts, o, &ta, "tick_text")?;
            let dx = (q[0] - center[0]) / outer;
            let dy = (q[1] - center[1]) / outer;
            let extra = if radar {
                3. + dx.abs() * jnum(&n, "width", 0.) / 2.
            } else {
                tick_reach(e, &ta, false).1.max(0.)
                    + 1.2
                    + dx.abs() * jnum(&n, "width", 0.) / 2.
                    + dy.abs() * jnum(&n, "height", 0.) / 2.
            };
            let offset = pair(&ta, "tick_offset", [0., 0.], &e.unit, e.dpi);
            n["rotation"] = json!(super::angle(&ta, "tick_rotation", 0.));
            place(
                &mut n,
                q[0] + extra * dx + offset[0],
                q[1] + extra * dy + offset[1],
                [0.5, 0.5],
            );
            decorations.push(decoration_bounds(
                &n,
                &format!(
                    "{}.label",
                    if radar {
                        categories[i].clone()
                    } else {
                        format!("theta:{t}")
                    }
                ),
            ));
            children.push(n);
        }
    }
    if !radar {
        let mut spec = ta.clone();
        spec.insert(
            "range".into(),
            V::List(theta.into_iter().map(V::num).collect()),
        );
        let axis = Axis::new(e, spec, "theta", "bottom", 0., &[], o.loc)?;
        for v in axis.minor_ticks(&tticks) {
            let (inside, outside) = tick_reach(e, &ta, true);
            let theta = v / period * TAU;
            children.push(line(
                point(theta, outer + inside),
                point(theta, outer + outside),
                w,
                h,
                &string(&ta, "color", &color),
                sw,
            ));
            if string(&ta, "grid", "none") == "both" {
                let mut n = line(point(theta, inner), point(theta, outer), w, h, &gc, gw);
                n["opacity"] = json!(0.5);
                grid.push(n);
            }
        }
    }
    for layer in layers {
        let layer_start = data.len();
        for p in &layer.primitives {
            if p.marker.is_some() {
                let q = p.points[0];
                if radar || polar_visible(&m, q) {
                    data.push(marker_node(
                        p,
                        if radar {
                            radar_point(&m, q[0] as usize, q[1])
                        } else {
                            polar_point(&m, q[0], q[1])
                        },
                        w,
                        h,
                    ));
                }
                continue;
            }
            let pts = projected_points(p, layer, &m);
            if pts.iter().flatten().any(|v| !v.is_finite()) {
                return Err(e.error("E_PLOT", "投影坐标映射溢出", o.loc));
            }
            let mut n = path(&pts, p.closed, w, h, p.fill.clone(), &p.stroke, p.width);
            if !p.holes.is_empty() {
                let mut d = n["d"].as_str().unwrap().to_string();
                for hole in &p.holes {
                    let mut hp = p.clone();
                    hp.points = hole.clone();
                    let mapped = projected_points(&hp, layer, &m);
                    d.push_str(
                        path(&mapped, true, w, h, json!("none"), "none", 0.)["d"]
                            .as_str()
                            .unwrap(),
                    );
                }
                n["d"] = json!(d);
            }
            n["geometryRecipe"] = projected_recipe(p, layer, &m);
            n["d"] = json!(resolved_polar_path(&n["geometryRecipe"], 0.0008).to_svg());
            n["opacity"] = json!(p.opacity);
            let mut stroke = e.stroke(&layer.args, !p.closed, p.width);
            stroke["color"] = json!(p.stroke);
            n["strokeStyle"] = stroke;
            data.push(n);
            let mut rings = vec![pts];
            rings.extend(p.holes.iter().map(|hole| {
                let mut hp = p.clone();
                hp.points = hole.clone();
                projected_points(&hp, layer, &m)
            }));
            let mut hatches = hatch(e, p, &rings, &layer.args, w, h)?;
            for node in &mut hatches {
                node["geometryRecipe"] = json!({"kind":"polar_hatch","region":projected_recipe(p,layer,&m),"width":w,"height":h,"spacing":length_arg(e,&layer.args,"hatch_spacing",1.5)*std::f64::consts::SQRT_2,"cross":string(&layer.args,"hatch","slash")=="cross"});
            }
            data.extend(hatches);
            if layer.kind == "errorbar" {
                let cap = length_arg(e, &layer.args, "cap_size", 3. * PT) / 2.;
                for q in &p.points {
                    if polar_visible(&m, *q) {
                        let point = polar_point(&m, q[0], q[1]);
                        let ang = zero + direction * q[0] / period * TAU;
                        let dir = if p.index == 1 {
                            [ang.cos(), -ang.sin()]
                        } else {
                            [ang.sin(), ang.cos()]
                        };
                        let mut cap_node = line(
                            [point[0] - cap * dir[0], point[1] - cap * dir[1]],
                            [point[0] + cap * dir[0], point[1] + cap * dir[1]],
                            w,
                            h,
                            &p.stroke,
                            p.width,
                        );
                        cap_node["opacity"] = json!(p.opacity);
                        data.push(cap_node);
                    }
                }
            }
        }
        if let Some(masks) = &layer.masks {
            let children = data.split_off(layer_start);
            if !masks.is_empty() && !children.is_empty() {
                let mut d = String::new();
                for [x0, x1, y0, y1] in masks {
                    let mut p = layer.primitives.first().cloned().unwrap();
                    p.points = vec![[*x0, *y0], [*x1, *y0], [*x1, *y1], [*x0, *y1]];
                    p.closed = true;
                    let points = projected_points(&p, layer, &m);
                    d.push_str(
                        path(&points, true, w, h, json!("none"), "none", 0.)["d"]
                            .as_str()
                            .unwrap(),
                    );
                }
                let mut masked = group(w, h, children);
                let rings: Vec<_> = masks
                    .iter()
                    .map(|[x0, x1, y0, y1]| vec![[*x0, *y0], [*x1, *y0], [*x1, *y1], [*x0, *y1]])
                    .collect();
                masked["clipPath"] = json!({"d":d,"fillRule":"nonzero","geometryRecipe":{"kind":"polar","mode":"projected","m":m,"rings":rings,"closed":true,"layer":layer.kind,"wrap":"raw","interpolation":"polar"}});
                data.push(masked);
            }
        }
    }
    let cp = clip_path(&m);
    for (id, nodes) in [("plot-grid", grid), ("plot-data", batch_markers(data))] {
        let mut g = group(w, h, nodes);
        g["id"] = json!(id);
        g["clipPath"] = json!({"d":cp,"fillRule":"evenodd","geometryRecipe":boundary_recipe(&m)});
        children.insert(if id == "plot-grid" { 0 } else { 1 }, g);
    }
    let area_theme = part_style(e, a, style, "area", None, &Args::new())?;
    if let Some(fill) = area_theme.get("background") {
        let mut bg = base("path", w, h);
        bg["d"] = json!(cp);
        bg["geometryRecipe"] = boundary_recipe(&m);
        bg["fill"] = fill.json();
        bg["fillRule"] = json!("evenodd");
        bg["id"] = json!("plot-area-background");
        children.insert(0, bg);
    }
    if yes(&ta, "spine", true) {
        let mut node = base("path", w, h);
        node["d"] = json!(cp);
        node["geometryRecipe"] = boundary_recipe(&m);
        node["fill"] = json!("none");
        node["strokeStyle"] =
            json!({"color":color,"width":sw,"dash":[],"cap":"butt","join":"round"});
        children.push(node);
    }
    let ls = length_arg(e, style, "label_font_size", fs);
    for (aa, name) in [(&ta, "theta"), (&ra, "r")] {
        if let Some(value) = aa.get("label") {
            let mut parent = style.clone();
            parent.insert("color".into(), axis_text_color(aa, style));
            parent.insert("font_size".into(), V::mm(ls));
            let st = part_style(e, a, &parent, "axis-label", Some(name), &Args::new())?;
            let mut n = label_at(
                e,
                value,
                &st,
                length_arg(e, &st, "font_size", ls),
                o,
                aa,
                "label",
            )?;
            let offset = pair(aa, "label_offset", [0., 0.], &e.unit, e.dpi);
            let left = decorations
                .iter()
                .map(|b| jnum(b, "left", center[0] - outer))
                .fold(center[0] - outer, f64::min);
            let bottom = decorations
                .iter()
                .map(|b| jnum(b, "bottom", center[1] + outer))
                .fold(center[1] + outer, f64::max);
            let height = jnum(&n, "height", 0.);
            let q = if name == "theta" {
                [center[0], bottom + 1.2 + height / 2.]
            } else {
                [left - 1.2 - height / 2., center[1]]
            };
            n["rotation"] = json!(if name == "r" { -90. } else { 0. });
            place(&mut n, q[0] + offset[0], q[1] + offset[1], [0.5, 0.5]);
            decorations.push(decoration_bounds(&n, &format!("{name}.label")));
            children.push(n);
        }
    }
    if string(&ra, "notation", "plain") == "offset" {
        for i in 0..if radar { categories.len() } else { 1 } {
            let dom = if radar {
                [ranges[i][0], ranges[i][1]]
            } else {
                domain
            };
            let exp = exponent(&ra, dom);
            if exp != 0 {
                let mut n = formula(e, format!("\\times 10^{{{exp}}}"), style, ts, o)?;
                let q = if radar {
                    radar_point(&m, i, ranges[i][1])
                } else {
                    [area[0] + area[2], area[1]]
                };
                let offset = pair(&ra, "exponent_offset", [0., 0.], &e.unit, e.dpi);
                place(&mut n, q[0] + offset[0], q[1] - 3. + offset[1], [0.5, 1.]);
                decorations.push(decoration_bounds(
                    &n,
                    &format!("{}.exponent", if radar { &categories[i] } else { "r" }),
                ));
                children.push(n);
            }
        }
    }
    add_decorations(
        e,
        o,
        style,
        layers,
        area,
        size,
        &mut children,
        &mut decorations,
    )?;
    if !fixed {
        let mut extra = [0f64; 4];
        for b in &decorations {
            if b["name"] == "legend" {
                continue;
            }
            for (i, v) in [
                1. - jnum(b, "left", 0.),
                1. - jnum(b, "top", 0.),
                jnum(b, "right", 0.) + 1. - w,
                jnum(b, "bottom", 0.) + 1. - h,
            ]
            .into_iter()
            .enumerate()
            {
                extra[i] = extra[i].max(v);
            }
        }
        if extra.iter().any(|v| *v > 1e-6) {
            let iteration = num(a, "__projection_iteration", 0.);
            if iteration >= 19. {
                return Err(e.error("E_LAYOUT", "极坐标自动留白未收敛，请指定 plot_area", o.loc));
            }
            let mut next = a.clone();
            next.insert("__projection_iteration".into(), V::num(iteration + 1.));
            let area = [
                area[0] + extra[0],
                area[1] + extra[1],
                area[2] - extra[0] - extra[2],
                area[3] - extra[1] - extra[3],
            ];
            return polar_layout(e, o, &next, style, layers, area, size, false);
        }
    }
    warn_layout(e, o, &decorations, size, fixed, a.get("__placement_origin"));
    let mut n = group(w, h, children);
    n["plotBounds"] = json!({"x":area[0],"y":area[1],"width":area[2],"height":area[3]});
    n["plotProjection"] = m;
    n["plotAxes"] =
        json!({"theta":{"scale":"linear","domain":theta},"r":{"scale":scale,"domain":domain}});
    n["plotDecorations"] = json!(decorations);
    Ok(n)
}
pub fn projected_anchor(
    e: &Engine,
    n: &Json,
    method: &str,
    pos: &[V],
    a: &Args,
    l: Loc,
) -> Result<[f64; 2]> {
    let m = &n["plotProjection"];
    let radar = m["kind"] == "radar";
    if method == "axis" {
        let name = string(a, "name", &pos.first().map(V::as_str).unwrap_or_default());
        let anchor = string(a, "anchor", "center");
        let f = match anchor.as_str() {
            "start" => 0.,
            "center" => 0.5,
            "end" => 1.,
            _ => return Err(e.error("E_PLOT", "轴锚点须为 start/center/end", l)),
        };
        if radar {
            let cats = m["categories"].as_array().unwrap();
            let i = cats
                .iter()
                .position(|v| v == &name)
                .ok_or_else(|| e.error("E_PLOT", "雷达类别不存在", l))?;
            let lo = m["ranges"][i][0].as_f64().unwrap();
            let hi = m["ranges"][i][1].as_f64().unwrap();
            return Ok(radar_point(m, i, lo + f * (hi - lo)));
        }
        let lo = m["radial"]["domain"][0].as_f64().unwrap();
        let hi = m["radial"]["domain"][1].as_f64().unwrap();
        if name == "theta" {
            return Ok(polar_point(
                m,
                m["theta"][0].as_f64().unwrap()
                    + f * (m["theta"][1].as_f64().unwrap() - m["theta"][0].as_f64().unwrap()),
                if m["radial"]["reverse"] == true {
                    lo
                } else {
                    hi
                },
            ));
        }
        if name == "r" {
            let fac = if m["angleUnit"] == "rad" {
                1.
            } else {
                180. / PI
            };
            let val = inverse(
                transform(
                    lo,
                    jstr(&m["radial"], "scale", "linear"),
                    jnum(&m["radial"], "constant", 1.),
                ) + f
                    * (transform(
                        hi,
                        jstr(&m["radial"], "scale", "linear"),
                        jnum(&m["radial"], "constant", 1.),
                    ) - transform(
                        lo,
                        jstr(&m["radial"], "scale", "linear"),
                        jnum(&m["radial"], "constant", 1.),
                    )),
                jstr(&m["radial"], "scale", "linear"),
                jnum(&m["radial"], "constant", 1.),
            );
            return Ok(polar_point(m, jnum(m, "rLabelAngle", 0.) * fac, val));
        }
        return Err(e.error("E_PLOT", "极坐标轴名称须为 theta 或 r", l));
    }
    if radar {
        let category = string(
            a,
            "category",
            &pos.first().map(V::as_str).unwrap_or_default(),
        );
        let i = m["categories"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v == &category)
            .ok_or_else(|| e.error("E_PLOT", "雷达类别不存在", l))?;
        let value = a
            .get("value")
            .or_else(|| pos.get(1))
            .and_then(V::number)
            .ok_or_else(|| e.error("E_PLOT", "数据锚点需要有限数值", l))?;
        if value < m["ranges"][i][0].as_f64().unwrap()
            || value > m["ranges"][i][1].as_f64().unwrap()
        {
            return Err(e.error("E_PLOT", "数据锚点超出指标范围", l));
        }
        Ok(radar_point(m, i, value))
    } else {
        let theta = a
            .get("theta")
            .or_else(|| pos.first())
            .and_then(V::number)
            .unwrap_or(f64::NAN);
        let r = a
            .get("r")
            .or_else(|| pos.get(1))
            .and_then(V::number)
            .unwrap_or(f64::NAN);
        if !polar_visible(m, [theta, r]) {
            return Err(e.error("E_PLOT", "极坐标数据锚点超出可见范围或数据无效", l));
        }
        Ok(polar_point(m, theta, r))
    }
}
