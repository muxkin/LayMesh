//! Native scientific plot layout; all coordinates are physical millimetres.
use crate::{Loc, Result, engine::Engine, model::*};
use kurbo::BezPath;
use serde_json::{Value as Json, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
mod colors;
mod decorations;
mod layers;
mod projection;
mod scales;
use colors::*;
pub use decorations::decoration;
use decorations::*;
use layers::*;
use projection::*;
pub(crate) fn resolved_polar_path(recipe: &Json, tolerance: f64) -> BezPath {
    projection::resolved_polar_path(recipe, tolerance)
}
use scales::*;
const PALETTE: [&str; 7] = [
    "#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9", "#000000",
];
fn bounds(v: &[f64]) -> Option<(f64, f64)> {
    let lo = v
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    let hi = v
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if lo <= hi { Some((lo, hi)) } else { None }
}
fn linspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| lo + (hi - lo) * i as f64 / (n - 1).max(1) as f64)
        .collect()
}
fn diagnostic_origin(mut error: crate::Diagnostic, origin: Option<&V>) -> crate::Diagnostic {
    if let Some(V::Map(origin)) = origin {
        if let Some(loc) = origin
            .get("loc")
            .and_then(|value| serde_json::from_value::<Loc>(value.json()).ok())
        {
            error.loc = loc;
        }
        if let Some(file) = origin.get("file") {
            error.file = file.as_str();
        }
    }
    error
}
fn length_arg(e: &Engine, a: &Args, k: &str, d: f64) -> f64 {
    length(a, k, d, &e.unit, e.dpi)
}
fn part_style(
    e: &Engine,
    a: &Args,
    parent: &Args,
    part: &str,
    name: Option<&str>,
    explicit: &Args,
) -> Result<Args> {
    let mut selector = Args::new();
    if let Some(class) = a.get("class") {
        selector.insert("class".into(), class.clone());
    }
    let mut inherited = parent.clone();
    for (key, value) in a {
        if key.starts_with("__") {
            inherited
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }
    inherited.insert("__style_type".into(), V::text("plot"));
    if let Some(class) = a.get("class") {
        inherited.insert("class".into(), class.clone());
    }
    if let Some(file) = a.get("__style_file") {
        selector.insert("__style_file".into(), file.clone());
    }
    let mut out = inherited.clone();
    out.extend(e.styled_part("plot", &selector, &inherited, Some(part), name)?);
    out.extend(explicit.clone());
    Ok(out)
}
fn axis_text_color(axis: &Args, style: &Args) -> V {
    axis.get("__axis_text_color")
        .or_else(|| style.get("__css_text_color"))
        .or_else(|| axis.get("line_color"))
        .or_else(|| style.get("line_color"))
        .or_else(|| style.get("color"))
        .cloned()
        .unwrap_or(V::text("#222222"))
}
fn object_args(a: &Args, k: &str) -> Args {
    a.get(k)
        .and_then(V::object)
        .map(|o| {
            let o = o.borrow();
            let mut args = o.args.clone();
            fn resolve_font(v: &V, file: &str) -> V {
                match v {
                    V::Text(s, raw)
                        if s.contains('/')
                            || s.contains('\\')
                            || [".ttf", ".otf", ".ttc", ".otc"].iter().any(|ext| {
                                s.split('#')
                                    .next()
                                    .unwrap_or(s)
                                    .to_lowercase()
                                    .ends_with(ext)
                            }) =>
                    {
                        let (path, face) = s
                            .split_once('#')
                            .map(|(p, f)| (p, format!("#{f}")))
                            .unwrap_or((s.as_str(), String::new()));
                        V::Text(format!("{}{face}", resolve(file, path)), *raw)
                    }
                    V::List(v) => V::List(v.iter().map(|v| resolve_font(v, file)).collect()),
                    _ => v.clone(),
                }
            }
            if o.kind == "axis" {
                if let Some(value) = args
                    .get("color")
                    .or_else(|| args.get("line_color"))
                    .cloned()
                {
                    args.insert("__axis_text_color".into(), value);
                }
            }
            for key in ["font_family", "tick_font_family"] {
                if let Some(v) = args.get(key).cloned() {
                    args.insert(key.into(), resolve_font(&v, &o.file));
                }
            }
            args
        })
        .unwrap_or_default()
}
fn group(w: f64, h: f64, children: Vec<Json>) -> Json {
    let mut n = base("group", w, h);
    n["children"] = json!(children);
    n["contentWidth"] = json!(w);
    n["contentHeight"] = json!(h);
    n
}
fn path(
    points: &[[f64; 2]],
    closed: bool,
    w: f64,
    h: f64,
    fill: Json,
    color: &str,
    sw: f64,
) -> Json {
    let mut p = BezPath::new();
    for (i, q) in points.iter().enumerate() {
        if i == 0 {
            p.move_to((q[0], q[1]));
        } else {
            p.line_to((q[0], q[1]));
        }
    }
    if closed {
        p.close_path();
    }
    let mut n = crate::geometry::path_node(p, fill, color, sw);
    n["width"] = json!(w);
    n["height"] = json!(h);
    n["fillRule"] = json!("evenodd");
    n
}
fn line(a: [f64; 2], b: [f64; 2], w: f64, h: f64, color: &str, sw: f64) -> Json {
    path(&[a, b], false, w, h, json!("none"), color, sw)
}
fn label(e: &mut Engine, value: &V, style: &Args, size: f64, o: &Object) -> Result<Json> {
    if let Some(obj) = value.object() {
        let mut p = Args::new();
        if !obj.borrow().args.contains_key("font_size") {
            p.insert("font_size".into(), V::mm(size));
        }
        let mut parent = style.clone();
        // A text()/formula() value owns its typography; plain plot labels inherit plot color.
        parent.remove("color");
        parent.remove("line_color");
        return e.materialize(obj, &p, &parent);
    }
    let mut spec = args_json(style);
    spec["font_size"] = json!(size);
    if spec.get("line_height").is_none() {
        spec["line_height"] = json!(size * 1.25);
    }
    spec["content"] = value.json();
    if let Some(V::Map(location)) = style.get("__label_location") {
        if let V::Text(content, raw) = value {
            let mut content = json!({"value":content,"raw":raw});
            for (key, v) in location {
                content[key] = v.json();
                if key == "loc" {
                    if let V::Map(fields) = v {
                        for key in ["line", "column", "offset"] {
                            if let Some(V::Number(value, _)) = fields.get(key) {
                                content["loc"][key] = json!(*value as usize);
                            }
                        }
                    }
                }
            }
            spec["content"] = content;
        }
    }

    for key in ["font_family", "math_font"] {
        if let Some(font) = style.get(key) {
            e.fonts
                .load_requested(font, &e.host, &o.file, o.loc, &mut e.warnings);
        }
    }
    crate::text::layout_text(&spec, None, &mut e.fonts, &mut e.warnings, &o.file, o.loc)
}
fn label_at(
    e: &mut Engine,
    value: &V,
    style: &Args,
    size: f64,
    o: &Object,
    spec: &Args,
    key: &str,
) -> Result<Json> {
    let mut st = style.clone();
    if let Some(V::Map(locations)) = spec.get("__arg_locations") {
        if let Some(location) = locations.get(key) {
            st.insert("__label_location".into(), location.clone());
        }
    }
    label(e, value, &st, size, o)
}
fn formula(e: &mut Engine, source: String, style: &Args, size: f64, o: &Object) -> Result<Json> {
    let mut spec = e.text_spec(style, &o.file, o.loc)?;
    spec["font_size"] = json!(size);
    spec["source"] = json!(source);
    crate::text::formula(&spec, &mut e.fonts, &mut e.warnings, &o.file, o.loc)
}
fn place(n: &mut Json, x: f64, y: f64, anchor: [f64; 2]) {
    n["x"] = json!(x - anchor[0] * jnum(n, "width", 0.));
    n["y"] = json!(y - anchor[1] * jnum(n, "height", 0.));
}
fn decoration_bounds(n: &Json, name: &str) -> Json {
    let points =
        ["top_left", "top_right", "bottom_left", "bottom_right"].map(|a| node_anchor(n, a));
    json!({"name":name,"left":points.iter().map(|p|p[0]).fold(f64::INFINITY,f64::min),"top":points.iter().map(|p|p[1]).fold(f64::INFINITY,f64::min),"right":points.iter().map(|p|p[0]).fold(f64::NEG_INFINITY,f64::max),"bottom":points.iter().map(|p|p[1]).fold(f64::NEG_INFINITY,f64::max)})
}
fn marker_node(p: &Primitive, q: [f64; 2], w: f64, h: f64) -> Json {
    let mut n = base("points", w, h);
    n["positions"] = json!([q[0], q[1]]);
    n["marker"] = json!(p.marker.as_ref().unwrap());
    n["markerSize"] = json!(p.marker_size);
    n["markerFill"] = p.fill.clone();
    n["markerStroke"] = json!(p.stroke);
    n["markerStrokeWidth"] = json!(p.width);
    n["color"] = p.fill.clone();
    n["pointOpacity"] = json!(p.opacity);
    n
}
fn batch_markers(nodes: Vec<Json>) -> Vec<Json> {
    let mut out: Vec<Json> = vec![];
    for n in nodes {
        if n["kind"] == "points" {
            if let Some(last) = out.last_mut() {
                if last["kind"] == "points"
                    && last["marker"] == n["marker"]
                    && last["markerStroke"] == n["markerStroke"]
                    && last["markerStrokeWidth"] == n["markerStrokeWidth"]
                {
                    let count = last["positions"].as_array().unwrap().len() / 2;
                    for (arr, key) in [
                        ("markerSizes", "markerSize"),
                        ("pointOpacities", "pointOpacity"),
                        ("markerFills", "markerFill"),
                    ] {
                        if last.get(arr).is_none() {
                            last[arr] = json!(vec![last[key].clone(); count]);
                        }
                        last[arr].as_array_mut().unwrap().push(n[key].clone());
                    }
                    last["positions"]
                        .as_array_mut()
                        .unwrap()
                        .extend(n["positions"].as_array().unwrap().clone());
                    continue;
                }
            }
        }
        out.push(n);
    }
    out
}
fn raster_heatmap(layer: &Layer, x: &Axis, y: &Axis, w: f64, h: f64) -> Option<Json> {
    if layer.kind != "heatmap"
        || string(&layer.args, "mode", "raster") != "raster"
        || x.segments.len() != 1
        || y.segments.len() != 1
        || x.scale != "linear"
        || y.scale != "linear"
        || x.reverse
        || y.reverse
        || layer.args.contains_key("x_edges")
        || layer.args.contains_key("y_edges")
    {
        return None;
    }
    let z: Vec<Vec<f64>> = array(&layer.args, "z")
        .iter()
        .map(|r| {
            r.list()
                .iter()
                .map(|v| v.number().unwrap_or(f64::NAN))
                .collect()
        })
        .collect();
    let rows = z.len();
    let cols = z[0].len();
    let mut pixels = vec![0u8; rows * cols * 4];
    let lower = string(&layer.args, "origin", "lower") == "lower";
    for (r, row) in z.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            if !v.is_finite() {
                continue;
            }
            let color = scale_color(layer.scale.as_ref()?, *v);
            let color = crate::color::Color::parse(&color).ok()?;
            let idx = ((if lower { rows - 1 - r } else { r }) * cols + c) * 4;
            for k in 0..4 {
                pixels[idx + k] = (color.rgba[k] * 255.).round() as u8;
            }
        }
    }
    let mut png = std::io::Cursor::new(vec![]);
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_raw(
        cols as u32,
        rows as u32,
        pixels,
    )?)
    .write_to(&mut png, image::ImageFormat::Png)
    .ok()?;
    let extent = nums(&layer.args, "extent");
    let [x0, x1, y0, y1] = if extent.len() == 4 {
        [extent[0], extent[1], extent[2], extent[3]]
    } else {
        [0., cols as f64, 0., rows as f64]
    };
    let x0 = x.map_in(x0, &x.segments[0]);
    let x1 = x.map_in(x1, &x.segments[0]);
    let y0 = y.map_in(y0, &y.segments[0]);
    let y1 = y.map_in(y1, &y.segments[0]);
    let mut n = base("image", x1 - x0, y0 - y1);
    n["x"] = json!(x0);
    n["y"] = json!(y1);
    n["mime"] = json!("image/png");
    n["data"] = json!(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        png.into_inner()
    ));
    n["interpolation"] = json!("nearest");
    n["fit"] = json!("stretch");
    n["opacity"] = json!(num(&layer.args, "opacity", 1.));
    let mut g = group(w, h, vec![n]);
    g["clip"] = json!({"x":0,"y":0,"width":w,"height":h});
    Some(g)
}
/// Physical hatch lines share a plot-space phase and use even-odd ring intersections.
fn hatch(
    e: &Engine,
    p: &Primitive,
    rings: &[Vec<[f64; 2]>],
    a: &Args,
    w: f64,
    h: f64,
) -> Result<Vec<Json>> {
    if !p.closed || !a.contains_key("hatch") {
        return Ok(vec![]);
    }
    let spacing = length_arg(e, a, "hatch_spacing", 1.5) * std::f64::consts::SQRT_2;
    if !spacing.is_finite() || spacing <= 0. || (w + h) / spacing > 100000. {
        return Err(diagnostic_origin(
            e.error(
                "E_LIMIT",
                "纹理过密：每个方向最多 100000 条线，请增大 hatch_spacing",
                Loc::default(),
            ),
            a.get("__call_origin"),
        ));
    }
    let mut d = BezPath::new();
    for slope in if string(a, "hatch", "slash") == "cross" {
        vec![-1., 1.]
    } else {
        vec![-1.]
    } {
        let (min, max) = if slope == -1. { (0., w + h) } else { (-w, h) };
        let first = (min / spacing).floor() as i64;
        let last = (max / spacing).floor() as i64;
        for index in first..=last {
            let k = index as f64 * spacing;
            let mut hits = vec![];
            for ring in rings {
                for i in 0..ring.len() {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    let (fa, fb) = (a[1] - slope * a[0], b[1] - slope * b[0]);
                    if fa <= k && k < fb || fb <= k && k < fa {
                        hits.push(a[0] + (b[0] - a[0]) * (k - fa) / (fb - fa));
                    }
                }
            }
            hits.sort_by(f64::total_cmp);
            for pair in hits.chunks_exact(2) {
                let (lo, hi) = (pair[0].max(0.), pair[1].min(w));
                if lo < hi {
                    d.move_to((lo, slope * lo + k));
                    d.line_to((hi, slope * hi + k));
                }
            }
        }
    }
    if d.is_empty() {
        return Ok(vec![]);
    }
    let color = if p.stroke == "none" {
        string(a, "color", "#0072B2")
    } else {
        p.stroke.clone()
    };
    let mut n = crate::geometry::path_node(
        d,
        json!("none"),
        &color,
        length_arg(e, a, "hatch_width", 0.15),
    );
    n["width"] = json!(w);
    n["height"] = json!(h);
    n["opacity"] = json!(p.opacity);
    Ok(vec![n])
}
fn cartesian_layer(
    e: &Engine,
    layer: &Layer,
    x: &Axis,
    y: &Axis,
    w: f64,
    h: f64,
) -> Result<Vec<Json>> {
    if let Some(raster) = raster_heatmap(layer, x, y, w, h) {
        return Ok(vec![raster]);
    }
    let mut groups = vec![];
    for sx in &x.segments {
        for sy in &y.segments {
            let mut children = vec![];
            for p in &layer.primitives {
                let visible = |q: [f64; 2]| {
                    q[0] >= sx.domain[0]
                        && q[0] <= sx.domain[1]
                        && q[1] >= sy.domain[0]
                        && q[1] <= sy.domain[1]
                };
                let map = |q: [f64; 2]| {
                    let mut q = q;
                    for (dim, axis, s) in [(0, x, sx), (1, y, sy)] {
                        if q[dim] == f64::NEG_INFINITY {
                            q[dim] = s.domain[0];
                        }
                        if q[dim] == f64::INFINITY {
                            q[dim] = s.domain[1];
                        }
                        if axis.scale == "log" && q[dim] == 0. && layer.zero == Some(dim) {
                            q[dim] = axis.domain[0];
                        }
                    }
                    [x.map_in(q[0], sx), y.map_in(q[1], sy)]
                };
                if p.marker.is_some() {
                    if visible(p.points[0]) {
                        children.push(marker_node(p, map(p.points[0]), w, h));
                    }
                    continue;
                }
                let mut original = p.points.clone();
                if layer.kind == "ecdf" && !original.is_empty() {
                    if y.scale != "log" && x.domain[0] < original[0][0] {
                        original.insert(0, [x.domain[0], 0.]);
                    }
                    if x.domain[1] > original.last().unwrap()[0] {
                        original.push([x.domain[1], 1.]);
                    }
                }
                let pts: Vec<_> = original.iter().copied().map(map).collect();
                if pts.iter().flatten().any(|v| !v.is_finite()) {
                    continue;
                }
                let mut n = path(&pts, p.closed, w, h, p.fill.clone(), &p.stroke, p.width);
                if !p.holes.is_empty() {
                    let mut d = n["d"].as_str().unwrap().to_string();
                    for hole in &p.holes {
                        let mapped: Vec<_> = hole.iter().copied().map(map).collect();
                        d.push_str(
                            path(&mapped, true, w, h, json!("none"), "none", 0.)["d"]
                                .as_str()
                                .unwrap(),
                        );
                    }
                    n["d"] = json!(d);
                }
                n["opacity"] = json!(p.opacity);
                let mut s = e.stroke(&layer.args, !p.closed, p.width);
                s["color"] = json!(p.stroke);
                n["strokeStyle"] = s;
                if layer.kind == "errorbar" && p.index > 0 {
                    let mut d = n["d"].as_str().unwrap().to_string();
                    let cap = length_arg(e, &layer.args, "cap_size", 3. * PT) / 2.;
                    for (i, q) in p.points.iter().enumerate() {
                        if visible(*q) {
                            let v = pts[i];
                            let (a, b) = if p.index == 1 {
                                ([v[0], v[1] - cap], [v[0], v[1] + cap])
                            } else {
                                ([v[0] - cap, v[1]], [v[0] + cap, v[1]])
                            };
                            d.push_str(&format!("M{},{}L{},{}", a[0], a[1], b[0], b[1]));
                        }
                    }
                    n["d"] = json!(d);
                }
                children.push(n);
                let mut rings = vec![pts];
                rings.extend(p.holes.iter().map(|r| r.iter().copied().map(map).collect()));
                children.extend(hatch(e, p, &rings, &layer.args, w, h)?);
            }
            let children = batch_markers(children);
            let mut rectangles = vec![[
                sx.range[0].min(sx.range[1]),
                sx.range[0].max(sx.range[1]),
                sy.range[0].min(sy.range[1]),
                sy.range[0].max(sy.range[1]),
            ]];
            if let Some(masks) = &layer.masks {
                let viewport = rectangles[0];
                rectangles = masks
                    .iter()
                    .filter_map(|m| {
                        let xx = [x.map_in(m[0], sx), x.map_in(m[1], sx)];
                        let yy = [y.map_in(m[2], sy), y.map_in(m[3], sy)];
                        let r = [
                            viewport[0].max(xx[0].min(xx[1])),
                            viewport[1].min(xx[0].max(xx[1])),
                            viewport[2].max(yy[0].min(yy[1])),
                            viewport[3].min(yy[0].max(yy[1])),
                        ];
                        (r[1] > r[0] && r[3] > r[2]).then_some(r)
                    })
                    .collect();
            }
            for r in rectangles {
                let mut g = group(w, h, children.clone());
                g["clip"] = json!({"x":r[0],"y":r[2],"width":r[1]-r[0],"height":r[3]-r[2]});
                g["statistics"] = layer.statistics.clone();
                groups.push(g);
            }
        }
    }
    Ok(groups)
}

pub fn layout(e: &mut Engine, o: &Object, a: &Args) -> Result<Json> {
    let natural = pair(&o.args, "size", [80., 60.], &e.unit, e.dpi);
    let size = pair(a, "size", natural, &e.unit, e.dpi);
    let (w, h) = (size[0], size[1]);
    if w <= 0. || h <= 0. {
        return Err(e.error("E_LAYOUT", "图表尺寸必须大于零", o.loc));
    }
    let mut style = object_args(a, "style");
    if let Some(color) = a.get("color") {
        style.insert("__css_text_color".into(), color.clone());
    }
    for (k, v) in a {
        if k.starts_with("font_")
            || k.starts_with("__")
            || matches!(k.as_str(), "color" | "line_height")
        {
            style.insert(k.clone(), v.clone());
        }
    }
    style
        .entry("color".into())
        .or_insert_with(|| V::text("#222222"));
    let grid_theme = part_style(e, a, &style, "grid", None, &Args::new())?;
    for (from, to) in [
        ("line_color", "grid_color"),
        ("line_width", "grid_line_width"),
        ("line_dash", "grid_dash"),
    ] {
        if let Some(v) = grid_theme.get(from) {
            style.insert(to.into(), v.clone());
        }
    }
    let fs = length_arg(e, &style, "font_size", 8. * PT);
    let ts = length_arg(e, &style, "tick_font_size", fs);
    let ls = length_arg(e, &style, "label_font_size", fs);
    let projection = string(a, "projection", "cartesian");
    let mut layers = vec![];
    for (kind, args) in &o.layers {
        if matches!(kind.as_str(), "legend" | "colorbar" | "add_axis") {
            continue;
        }
        let mut themed = e.styled(kind, args, a)?;
        let marker = part_style(e, a, &style, "marker", None, &Args::new())?;
        for (from, to) in [
            ("fill", "marker_fill"),
            ("border_color", "marker_border_color"),
            ("border_width", "marker_border_width"),
        ] {
            if !themed.contains_key(to) {
                if let Some(v) = marker.get(from) {
                    themed.insert(to.into(), v.clone());
                }
            }
        }
        let warning_count = e.warnings.len();
        layers.push(build(
            e,
            kind,
            &themed,
            layers.len(),
            &style,
            &projection,
            a,
            o.loc,
        )?);
        e.warnings.truncate(warning_count);
    }
    if layers.is_empty() {
        return Err(e.error("E_PLOT", "不能放置空图表", o.loc));
    }
    let mut margins = [10., 4., 4., 10.];
    let mut fixed = false;
    if let Some(v) = a.get("margins") {
        let v = v.list();
        if v.len() != 4 {
            return Err(e.error("E_PLOT", "margins 需要四个长度", o.loc));
        }
        for i in 0..4 {
            margins[i] = e.len(&v[i], o.loc)?;
        }
        fixed = true;
    }
    let mut area = [
        margins[0],
        margins[1],
        w - margins[0] - margins[2],
        h - margins[1] - margins[3],
    ];
    if let Some(obj) = a.get("plot_area").and_then(V::object) {
        let b = &obj.borrow().args;
        let pos = pair(b, "offset", [0., 0.], &e.unit, e.dpi);
        let sz = pair(b, "size", [0., 0.], &e.unit, e.dpi);
        area = [pos[0], pos[1], sz[0], sz[1]];
        fixed = true;
        if a.contains_key("margins") {
            return Err(e.error("E_ARG", "margins 与 plot_area 互斥", o.loc));
        }
    }
    if projection != "cartesian" {
        return polar_layout(e, o, a, &style, &layers, area, size, fixed);
    }
    let mut axes = vec![];
    for (name, side) in [("x", "bottom"), ("y", "left")] {
        let dim = usize::from(name == "y");
        let mut aa = part_style(e, a, &style, "axis", Some(name), &object_args(a, name))?;
        aa.insert(
            "_no_pad".into(),
            V::Bool(layers.iter().any(|l| {
                (if dim == 0 {
                    l.xaxis == name
                } else {
                    l.yaxis == name
                }) && matches!(
                    l.kind.as_str(),
                    "bar" | "hist" | "boxplot" | "violin" | "heatmap" | "contour" | "contourf"
                )
            })),
        );
        let vals: Vec<_> = layers
            .iter()
            .filter(|l| {
                if dim == 0 {
                    l.xaxis == name
                } else {
                    l.yaxis == name
                }
            })
            .flat_map(|l| {
                let mut vals = layer_values(l, dim);
                if string(&aa, "scale", "linear") != "log" && l.zero == Some(dim) {
                    vals.push(0.);
                }
                vals
            })
            .collect();
        axes.push(Axis::new(e, aa, name, side, 0., &vals, o.loc)?);
    }
    for (kind, aa) in &o.layers {
        if kind != "add_axis" {
            continue;
        }
        let name = string(aa, "name", "");
        let side = string(aa, "side", "right");
        if name.is_empty()
            || axes.iter().any(|v| v.name == name)
            || !matches!(side.as_str(), "left" | "right" | "top" | "bottom")
        {
            return Err(e.error("E_PLOT", "附加轴名称重复或方向无效", o.loc));
        }
        let dim = usize::from(matches!(side.as_str(), "left" | "right"));
        let mut spec = part_style(e, a, &style, "axis", Some(&name), &object_args(aa, "axis"))?;
        spec.insert(
            "_no_pad".into(),
            V::Bool(layers.iter().any(|l| {
                (if dim == 0 {
                    l.xaxis == name
                } else {
                    l.yaxis == name
                }) && matches!(
                    l.kind.as_str(),
                    "bar" | "hist" | "boxplot" | "violin" | "heatmap" | "contour" | "contourf"
                )
            })),
        );
        let values: Vec<_> = layers
            .iter()
            .filter(|l| {
                if dim == 0 {
                    l.xaxis == name
                } else {
                    l.yaxis == name
                }
            })
            .flat_map(|l| layer_values(l, dim))
            .collect();
        axes.push(Axis::new(
            e,
            spec,
            &name,
            &side,
            length_arg(e, aa, "offset", 0.),
            &values,
            o.loc,
        )?);
    }
    for l in &layers {
        if !axes.iter().any(|a| a.name == l.xaxis && a.horizontal())
            || !axes.iter().any(|a| a.name == l.yaxis && !a.horizontal())
        {
            return Err(e.error("E_PLOT", "图层坐标轴不存在或方向不匹配", o.loc));
        }
    }
    // Broken-axis tick density uses each visible segment's final physical span.
    for ax in &mut axes {
        if !array(&ax.args, "breaks").is_empty() {
            ax.map_segments(e, if ax.horizontal() { area[2] } else { area[3] }, o.loc)?;
        }
    }
    let advanced = layers.iter().any(|l| {
        matches!(l.kind.as_str(), "hist" | "boxplot" | "violin" | "ecdf")
            || [
                "hatch",
                "fill",
                "stroke",
                "line_style",
                "marker_sizes",
                "marker_fills",
                "point_opacities",
            ]
            .iter()
            .any(|k| l.args.contains_key(*k))
    }) || axes.len() > 2
        || axes.iter().any(|ax| {
            ax.scale == "symlog"
                || ax.reverse
                || [
                    "breaks",
                    "minor_auto",
                    "tick_text",
                    "tick_rotation",
                    "tick_offset",
                    "tick_font_size",
                    "tick_color",
                    "tick_font_family",
                    "tick_font_weight",
                    "tick_font_style",
                ]
                .iter()
                .any(|k| ax.args.contains_key(*k))
        });
    let mut labels: BTreeMap<String, Vec<(f64, Json)>> = BTreeMap::new();
    for ax in &mut axes {
        let mut ticknodes = vec![];
        let custom = array(&ax.args, "tick_text");
        let mut st = style.clone();
        st.insert("color".into(), axis_text_color(&ax.args, &style));
        for (k, v) in &ax.args {
            if k.starts_with("font_") {
                st.insert(k.clone(), v.clone());
            }
        }
        st = part_style(e, a, &st, "tick-label", Some(&ax.name), &Args::new())?;
        for key in ["font_family", "font_weight", "font_style"] {
            if let Some(v) = ax.args.get(key) {
                st.insert(key.into(), v.clone());
            }
        }
        if let Some(v) = ax.args.get("tick_color") {
            st.insert("color".into(), v.clone());
        }
        let tick_size = length_arg(
            e,
            &ax.args,
            "tick_font_size",
            length_arg(e, &st, "font_size", ts),
        );
        let mut count = 5;
        loop {
            ticknodes.clear();
            ax.args.insert("_tick_count".into(), V::num(count as f64));
            for (i, v) in ax.ticks().into_iter().enumerate() {
                if !yes(&ax.args, "tick_labels", true) {
                    continue;
                }
                let mut n = if let Some(custom) = custom.get(i) {
                    label(e, custom, &st, tick_size, o)?
                } else {
                    number_label(e, v, &ax.args, ax.domain, &st, tick_size, o)?
                };
                if !advanced && n["kind"] == "text" && n["content"] == "" {
                    continue;
                }
                n["rotation"] = json!(angle(&ax.args, "tick_rotation", 0.));
                ticknodes.push((v, n));
            }
            if advanced || ax.args.contains_key("ticks") || count <= 1 {
                break;
            }
            let length = if ax.horizontal() { area[2] } else { area[3] };
            let overlaps = ticknodes.windows(2).any(|pair| {
                let distance =
                    (interpolate(pair[1].0, ax.domain, [0., length], &ax.scale, ax.constant)
                        - interpolate(pair[0].0, ax.domain, [0., length], &ax.scale, ax.constant))
                    .abs();
                let dimension = if ax.horizontal() { "width" } else { "height" };
                distance
                    < (jnum(&pair[0].1, dimension, 0.) + jnum(&pair[1].1, dimension, 0.)) / 2. + 1.2
            });
            if !overlaps {
                break;
            }
            count -= 1;
        }
        labels.insert(ax.name.clone(), ticknodes);
    }
    if !fixed {
        let mut needed = [1f64; 4];
        for ax in &axes {
            let ticks = &labels[&ax.name];
            let mut labelsize = if let Some(v) = ax.args.get("label") {
                let mut parent = style.clone();
                parent.insert("color".into(), axis_text_color(&ax.args, &style));
                parent.insert("font_size".into(), V::mm(ls));
                let st = part_style(e, a, &parent, "axis-label", Some(&ax.name), &Args::new())?;
                let n = label_at(
                    e,
                    v,
                    &st,
                    length_arg(e, &st, "font_size", ls),
                    o,
                    &ax.args,
                    "label",
                )?;
                jnum(&n, "height", 0.) + 1.2
            } else {
                0.
            };
            let shift = pair(&ax.args, "label_offset", [0., 0.], &e.unit, e.dpi);
            labelsize += match ax.side.as_str() {
                "left" => (-shift[0]).max(0.),
                "right" => shift[0].max(0.),
                "top" => (-shift[1]).max(0.),
                _ => shift[1].max(0.),
            };
            let extent = ticks
                .iter()
                .map(|(_, n)| {
                    let r = angle(&ax.args, "tick_rotation", 0.).to_radians();
                    if ax.horizontal() {
                        jnum(n, "height", 0.) * r.cos().abs() + jnum(n, "width", 0.) * r.sin().abs()
                    } else {
                        jnum(n, "width", 0.) * r.cos().abs() + jnum(n, "height", 0.) * r.sin().abs()
                    }
                })
                .fold(0., f64::max)
                + labelsize
                + ax.offset
                + 3.4;
            let half = ticks
                .iter()
                .map(|(_, n)| {
                    let r = angle(&ax.args, "tick_rotation", 0.).to_radians();
                    if ax.horizontal() {
                        (jnum(n, "width", 0.) * r.cos().abs()
                            + jnum(n, "height", 0.) * r.sin().abs())
                            / 2.
                    } else {
                        (jnum(n, "height", 0.) * r.cos().abs()
                            + jnum(n, "width", 0.) * r.sin().abs())
                            / 2.
                    }
                })
                .fold(0., f64::max)
                + 1.;
            for i in if ax.horizontal() { [0, 2] } else { [1, 3] } {
                let half = if advanced {
                    let length = if ax.horizontal() { area[2] } else { area[3] };
                    ticks
                        .iter()
                        .map(|(v, n)| {
                            let r = angle(&ax.args, "tick_rotation", 0.).to_radians();
                            let half = if ax.horizontal() {
                                jnum(n, "width", 0.) * r.cos().abs()
                                    + jnum(n, "height", 0.) * r.sin().abs()
                            } else {
                                jnum(n, "height", 0.) * r.cos().abs()
                                    + jnum(n, "width", 0.) * r.sin().abs()
                            } / 2.;
                            let mut fraction = (transform(*v, &ax.scale, ax.constant)
                                - transform(ax.domain[0], &ax.scale, ax.constant))
                                / (transform(ax.domain[1], &ax.scale, ax.constant)
                                    - transform(ax.domain[0], &ax.scale, ax.constant));
                            if ax.reverse != !ax.horizontal() {
                                fraction = 1. - fraction;
                            }
                            let offset = pair(&ax.args, "tick_offset", [0., 0.], &e.unit, e.dpi)
                                [usize::from(!ax.horizontal())];
                            let position = fraction * length + offset;
                            1. + if i < 2 {
                                half - position
                            } else {
                                position + half - length
                            }
                        })
                        .fold(1., f64::max)
                } else {
                    half
                };
                margins[i] = margins[i].max(half);
                needed[i] = needed[i].max(half);
            }
            let i = match ax.side.as_str() {
                "left" => 0,
                "top" => 1,
                "right" => 2,
                _ => 3,
            };
            margins[i] = margins[i].max(extent);
            needed[i] = needed[i].max(extent);
        }
        for ax in &axes {
            if string(&ax.args, "notation", "plain") != "offset"
                || !yes(&ax.args, "tick_labels", true)
                || exponent(&ax.args, ax.domain) == 0
            {
                continue;
            }
            let factor = formula(
                e,
                format!("\\times 10^{{{}}}", exponent(&ax.args, ax.domain)),
                &style,
                ts,
                o,
            )?;
            let offset = pair(&ax.args, "exponent_offset", [0., 0.], &e.unit, e.dpi);
            if ax.horizontal() {
                let tick_height = labels[&ax.name]
                    .iter()
                    .map(|(_, n)| jnum(n, "height", 0.))
                    .fold(0., f64::max);
                let mut reach = 3.6 + tick_height + jnum(&factor, "height", 0.);
                if let Some(v) = ax.args.get("label") {
                    let mut parent = style.clone();
                    parent.insert("font_size".into(), V::mm(ls));
                    let st = part_style(e, a, &parent, "axis-label", Some(&ax.name), &Args::new())?;
                    let title = label_at(
                        e,
                        v,
                        &st,
                        length_arg(e, &st, "font_size", ls),
                        o,
                        &ax.args,
                        "label",
                    )?;
                    let available = w - margins[0] - margins[2];
                    if (available + jnum(&title, "width", 0.)) / 2.
                        > available - jnum(&factor, "width", 0.)
                    {
                        reach += 1.2 + jnum(&title, "height", 0.);
                    }
                }
                let side = if ax.side == "top" { 1 } else { 3 };
                margins[side] =
                    margins[side].max(1. + reach + offset[1] * if side == 1 { -1. } else { 1. });
            } else {
                margins[1] = margins[1].max(2.2 + jnum(&factor, "height", 0.) - offset[1]);
            }
        }
        for (kind, aa) in &o.layers {
            if kind == "colorbar" && !matches!(aa.get("position"), Some(V::List(_))) {
                if let Some(scale) = colorbar_source(o, &layers, aa) {
                    let side = string(aa, "position", "right");
                    let cb = colorbar_layout(e, o, aa, &style, scale, 40.)?;
                    let gap = length_arg(e, aa, "gap", 2.4);
                    let i = match side.as_str() {
                        "left" => 0,
                        "top" => 1,
                        "bottom" => 3,
                        _ => 2,
                    };
                    let span = jnum(&cb, if i % 2 == 0 { "width" } else { "height" }, 0.);
                    let legacy_right = !advanced
                        && i == 2
                        && !aa.contains_key("length")
                        && (length_arg(e, aa, "thickness", 3.) - 3.).abs() < 1e-8
                        && (gap - 2.4).abs() < 1e-8
                        && string(aa, "notation", "plain") == "plain"
                        && !aa.contains_key("label_offset");
                    let base = if legacy_right {
                        3.
                    } else if i == 2 {
                        0.
                    } else {
                        needed[i] - 1.
                    };
                    margins[i] = margins[i].max(base + gap + span + 1.);
                }
            }
        }

        area = [
            margins[0],
            margins[1],
            w - margins[0] - margins[2],
            h - margins[1] - margins[3],
        ];
    }
    if area[2] <= 0. || area[3] <= 0. {
        return Err(diagnostic_origin(
            e.error("E_LAYOUT", "图表空间不足，绘图区宽高必须大于零", o.loc),
            a.get("__placement_origin"),
        ));
    }
    for ax in &mut axes {
        ax.map_segments(e, if ax.horizontal() { area[2] } else { area[3] }, o.loc)?;
    }
    let mut children = vec![];
    let area_style = part_style(e, a, &style, "area", None, &Args::new())?;
    if let Some(fill) = area_style.get("background") {
        for x in &axes[0].segments {
            for y in &axes[1].segments {
                let (x0, x1) = (x.range[0].min(x.range[1]), x.range[0].max(x.range[1]));
                let (y0, y1) = (y.range[0].min(y.range[1]), y.range[0].max(y.range[1]));
                let mut bg = path(
                    &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                    true,
                    area[2],
                    area[3],
                    fill.json(),
                    "none",
                    0.,
                );
                bg["x"] = json!(area[0]);
                bg["y"] = json!(area[1]);
                bg["id"] = json!("plot-area-background");
                children.push(bg);
            }
        }
    }
    let mut grid = vec![];
    let mut data = vec![];
    let (mut decorations, mut axisnodes) = (vec![], vec![]);
    let gc = string(&style, "grid_color", "#dddddd");
    let gw = length_arg(e, &style, "grid_line_width", 0.3 * PT);
    for ax in &axes {
        let axis_start = axisnodes.len();
        let major = ax.ticks();
        let minor = ax.minor_ticks(&major);
        let gridtype = string(&ax.args, "grid", "none");
        for v in major.iter().chain(if gridtype == "both" {
            minor.iter()
        } else {
            [].iter()
        }) {
            if gridtype == "none" {
                break;
            }
            if let Some(pos) = ax.map(*v) {
                let mut n = if ax.horizontal() {
                    line([pos, 0.], [pos, area[3]], area[2], area[3], &gc, gw)
                } else {
                    line([0., pos], [area[2], pos], area[2], area[3], &gc, gw)
                };
                let mut gs = Args::new();
                gs.insert("line_color".into(), V::text(&gc));
                gs.insert("line_width".into(), V::mm(gw));
                if let Some(v) = style
                    .get("grid_line_dash")
                    .or_else(|| style.get("grid_dash"))
                {
                    gs.insert("line_dash".into(), v.clone());
                }
                n["strokeStyle"] = e.stroke(&gs, true, gw);
                if !major.contains(v) {
                    n["opacity"] = json!(0.5);
                }
                grid.push(n);
            }
        }
        let color = string(
            &ax.args,
            "line_color",
            &string(
                &ax.args,
                "color",
                &string(&style, "line_color", &string(&style, "color", "#222222")),
            ),
        );
        let sw = length_arg(
            e,
            &ax.args,
            "line_width",
            length_arg(e, &style, "line_width", 0.6 * PT),
        );
        let horizontal = ax.horizontal();
        let coord = match ax.side.as_str() {
            "left" => area[0] - ax.offset,
            "right" => area[0] + area[2] + ax.offset,
            "top" => area[1] - ax.offset,
            _ => area[1] + area[3] + ax.offset,
        };
        let outward = if matches!(ax.side.as_str(), "left" | "top") {
            -1.
        } else {
            1.
        };
        if yes(&ax.args, "spine", true) && string(a, "frame", "axes") != "none" {
            for seg in &ax.segments {
                axisnodes.push(if horizontal {
                    line(
                        [area[0] + seg.range[0], coord],
                        [area[0] + seg.range[1], coord],
                        w,
                        h,
                        &color,
                        sw,
                    )
                } else {
                    line(
                        [coord, area[1] + seg.range[0]],
                        [coord, area[1] + seg.range[1]],
                        w,
                        h,
                        &color,
                        sw,
                    )
                });
            }
            if ax.segments.len() > 1 {
                let mark = length_arg(e, &ax.args, "break_mark_size", 1.) / 2.;
                for ss in ax.segments.windows(2) {
                    for pos in [ss[0].range[1], ss[1].range[0]] {
                        axisnodes.push(if horizontal {
                            line(
                                [area[0] + pos - mark, coord + mark],
                                [area[0] + pos + mark, coord - mark],
                                w,
                                h,
                                &color,
                                sw,
                            )
                        } else {
                            line(
                                [coord - mark, area[1] + pos + mark],
                                [coord + mark, area[1] + pos - mark],
                                w,
                                h,
                                &color,
                                sw,
                            )
                        });
                    }
                }
            }
        }
        for (is_minor, values) in [(false, &major), (true, &minor)] {
            let size = length_arg(
                e,
                &ax.args,
                if is_minor {
                    "minor_tick_length"
                } else {
                    "tick_length"
                },
                if is_minor { 0.6 } else { 1.2 },
            );
            let direction = string(&ax.args, "tick_direction", "out");
            let inside = if direction == "in" {
                size
            } else if direction == "inout" {
                size / 2.
            } else {
                0.
            };
            let outside = if direction == "out" {
                size
            } else if direction == "inout" {
                size / 2.
            } else {
                0.
            };
            for v in values {
                if let Some(pos) = ax.map(*v) {
                    axisnodes.push(if horizontal {
                        line(
                            [area[0] + pos, coord - outward * inside],
                            [area[0] + pos, coord + outward * outside],
                            w,
                            h,
                            &color,
                            sw,
                        )
                    } else {
                        line(
                            [coord - outward * inside, area[1] + pos],
                            [coord + outward * outside, area[1] + pos],
                            w,
                            h,
                            &color,
                            sw,
                        )
                    });
                }
            }
        }
        let offset = pair(&ax.args, "tick_offset", [0., 0.], &e.unit, e.dpi);
        let mut extent: f64 = 0.;
        for (v, node) in &labels[&ax.name] {
            if let Some(pos) = ax.map(*v) {
                let mut n = node.clone();
                let direction = string(&ax.args, "tick_direction", "out");
                let outside = length_arg(e, &ax.args, "tick_length", 1.2)
                    * if direction == "in" {
                        0.
                    } else if direction == "inout" {
                        0.5
                    } else {
                        1.
                    };
                let gap = outside + 1.2;
                let r = angle(&ax.args, "tick_rotation", 0.).to_radians();
                let rw =
                    jnum(&n, "width", 0.) * r.cos().abs() + jnum(&n, "height", 0.) * r.sin().abs();
                let rh =
                    jnum(&n, "height", 0.) * r.cos().abs() + jnum(&n, "width", 0.) * r.sin().abs();
                let (x, y) = if horizontal {
                    (area[0] + pos, coord + outward * (gap + rh / 2.))
                } else {
                    (coord + outward * (gap + rw / 2.), area[1] + pos)
                };
                place(&mut n, x + offset[0], y + offset[1], [0.5, 0.5]);
                extent = extent.max(if horizontal { rh } else { rw });
                decorations.push(decoration_bounds(&n, &format!("{}.tick:{v}", ax.name)));
                axisnodes.push(n);
            }
        }
        if let Some(value) = ax.args.get("label") {
            if !value.as_str().is_empty() || value.object().is_some() {
                let mut parent = style.clone();
                parent.insert("color".into(), axis_text_color(&ax.args, &style));
                parent.insert("font_size".into(), V::mm(ls));
                let st = part_style(e, a, &parent, "axis-label", Some(&ax.name), &Args::new())?;
                let mut n = label_at(
                    e,
                    value,
                    &st,
                    length_arg(e, &st, "font_size", ls),
                    o,
                    &ax.args,
                    "label",
                )?;
                let offset = pair(&ax.args, "label_offset", [0., 0.], &e.unit, e.dpi);
                let outside = length_arg(e, &ax.args, "tick_length", 1.2)
                    * match string(&ax.args, "tick_direction", "out").as_str() {
                        "in" => 0.,
                        "inout" => 0.5,
                        _ => 1.,
                    };
                if horizontal {
                    let mut reach = extent + outside + 2.4;
                    let has_factor = string(&ax.args, "notation", "plain") == "offset"
                        && exponent(&ax.args, ax.domain) != 0
                        && yes(&ax.args, "tick_labels", true);
                    if has_factor {
                        let factor = formula(
                            e,
                            format!("\\times 10^{{{}}}", exponent(&ax.args, ax.domain)),
                            &style,
                            ts,
                            o,
                        )?;
                        if (area[2] + jnum(&n, "width", 0.)) / 2.
                            > area[2] - jnum(&factor, "width", 0.)
                        {
                            reach += jnum(&factor, "height", 0.) + 1.2;
                        }
                    }
                    let pinned = !advanced
                        && !fixed
                        && !has_factor
                        && ax.side == "bottom"
                        && !o.layers.iter().any(|(k, a)| {
                            k == "colorbar" && string(a, "position", "right") == "bottom"
                        });
                    let ypos = if pinned {
                        h - 1. - jnum(&n, "height", 0.)
                    } else {
                        coord + outward * reach
                    };
                    place(
                        &mut n,
                        area[0] + area[2] / 2. + offset[0],
                        ypos + offset[1],
                        [0.5, if outward > 0. { 0. } else { 1. }],
                    );
                } else {
                    n["rotation"] = json!(-90.);
                    let nw = jnum(&n, "width", 0.);
                    let nh = jnum(&n, "height", 0.);
                    let pinned = !advanced
                        && !fixed
                        && ax.side == "left"
                        && !o.layers.iter().any(|(k, a)| {
                            k == "colorbar" && string(a, "position", "right") == "left"
                        });
                    let xpos = if pinned {
                        1. + nh / 2.
                    } else {
                        coord + outward * (extent + outside + 2.4 + nh / 2.)
                    };
                    place(
                        &mut n,
                        xpos + offset[0],
                        area[1] + area[3] / 2. + offset[1],
                        [0.5, 0.5],
                    );
                    let _ = nw;
                }
                decorations.push(decoration_bounds(&n, &format!("{}.label", ax.name)));
                axisnodes.push(n);
            }
        }
        if string(&ax.args, "notation", "plain") == "offset" && yes(&ax.args, "tick_labels", true) {
            let exponent = exponent(&ax.args, ax.domain);
            if exponent != 0 {
                let mut n = formula(e, format!("\\times 10^{{{exponent}}}"), &style, ts, o)?;
                let offset = pair(&ax.args, "exponent_offset", [0., 0.], &e.unit, e.dpi);
                place(
                    &mut n,
                    if horizontal {
                        area[0] + area[2] + offset[0]
                    } else {
                        coord + offset[0]
                    },
                    if horizontal {
                        coord + outward * (extent + 3.6) + offset[1]
                    } else {
                        area[1] - 1.2 + offset[1]
                    },
                    if horizontal { [1., 0.] } else { [0., 1.] },
                );
                decorations.push(decoration_bounds(&n, &format!("{}.exponent", ax.name)));
                axisnodes.push(n);
            }
        }
        for node in &mut axisnodes[axis_start..] {
            if node["kind"] == "path" {
                let color = node["strokeStyle"]["color"].clone();
                node["strokeStyle"] = e.stroke(&ax.args, true, sw);
                node["strokeStyle"]["color"] = color;
            }
        }
    }
    if string(a, "frame", "axes") == "box" {
        let color = string(&style, "color", "#222222");
        let sw = length_arg(e, &style, "line_width", 0.6 * PT);
        axisnodes.push(line(
            [area[0], area[1]],
            [area[0] + area[2], area[1]],
            w,
            h,
            &color,
            sw,
        ));
        axisnodes.push(line(
            [area[0] + area[2], area[1]],
            [area[0] + area[2], area[1] + area[3]],
            w,
            h,
            &color,
            sw,
        ));
    }
    for layer in &layers {
        let x = axes.iter().find(|a| a.name == layer.xaxis).unwrap();
        let y = axes.iter().find(|a| a.name == layer.yaxis).unwrap();
        data.extend(cartesian_layer(e, layer, x, y, area[2], area[3])?);
    }
    for (id, nodes) in [("plot-grid", grid), ("plot-data", data)] {
        let mut g = group(area[2], area[3], nodes);
        g["id"] = json!(id);
        g["x"] = json!(area[0]);
        g["y"] = json!(area[1]);
        if id == "plot-grid" {
            g["clip"] = json!({"x":0.,"y":0.,"width":area[2],"height":area[3]});
        }
        children.push(g);
    }
    for ax in &axes {
        if !ax.args.contains_key("background") && !ax.args.contains_key("border_color") {
            continue;
        }
        let mut x0 = if ax.side == "right" {
            area[0] + area[2] + ax.offset
        } else {
            area[0] - if ax.side == "left" { ax.offset } else { 0. }
        };
        let mut x1 = if ax.horizontal() {
            area[0] + area[2]
        } else {
            x0
        };
        let mut y0 = if ax.side == "bottom" {
            area[1] + area[3] + ax.offset
        } else {
            area[1] - if ax.side == "top" { ax.offset } else { 0. }
        };
        let mut y1 = if ax.horizontal() {
            y0
        } else {
            area[1] + area[3]
        };
        for b in &decorations {
            if jstr(b, "name", "").starts_with(&format!("{}.", ax.name)) {
                x0 = x0.min(jnum(b, "left", x0));
                x1 = x1.max(jnum(b, "right", x1));
                y0 = y0.min(jnum(b, "top", y0));
                y1 = y1.max(jnum(b, "bottom", y1));
            }
        }
        let values = ax
            .args
            .get("padding")
            .map(|v| match v {
                V::List(v) => v
                    .iter()
                    .map(|v| e.len(v, o.loc).unwrap_or(0.))
                    .collect::<Vec<_>>(),
                _ => vec![e.len(v, o.loc).unwrap_or(0.)],
            })
            .unwrap_or(vec![0.]);
        let p = match values.as_slice() {
            [a] => [*a; 4],
            [a, b] => [*a, *b, *a, *b],
            [a, b, c, d] => [*a, *b, *c, *d],
            _ => return Err(e.error("E_LAYOUT", "padding 需要一、二或四个长度", o.loc)),
        };
        x0 -= p[3];
        x1 += p[1];
        y0 -= p[0];
        y1 += p[2];
        if x1 > x0 && y1 > y0 {
            let mut node = path(
                &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                true,
                w,
                h,
                ax.args
                    .get("background")
                    .map(V::json)
                    .unwrap_or(json!("none")),
                "none",
                0.,
            );
            node["strokeStyle"] = e.stroke(&ax.args, false, 0.3);
            node["id"] = json!(format!("axis-background-{}", ax.name));
            children.insert(0, node);
            decorations.push(json!({"name":format!("{}.background",ax.name),"left":x0,"right":x1,"top":y0,"bottom":y1}));
        }
    }
    children.extend(axisnodes);
    add_decorations(
        e,
        o,
        &style,
        &layers,
        area,
        size,
        &mut children,
        &mut decorations,
    )?;
    warn_layout(e, o, &decorations, size, fixed, a.get("__placement_origin"));
    let mut n = group(w, h, children);
    n["plotBounds"] = json!({"x":area[0],"y":area[1],"width":area[2],"height":area[3]});
    n["plotAxes"] = json!(
        axes.iter()
            .map(|ax| (ax.name.clone(), ax.json()))
            .collect::<BTreeMap<_, _>>()
    );
    n["plotDecorations"] = json!(decorations);
    Ok(n)
}
fn angle(a: &Args, k: &str, d: f64) -> f64 {
    match a.get(k) {
        Some(V::Number(v, u)) => {
            if u == "rad" {
                v.to_degrees()
            } else {
                *v
            }
        }
        Some(V::Text(v, _)) => match v.as_str() {
            "north" => 90.,
            "west" => 180.,
            "south" => 270.,
            _ => 0.,
        },
        _ => d,
    }
}
pub fn local_to_parent(n: &Json, p: [f64; 2]) -> [f64; 2] {
    let w = jnum(n, "width", 0.);
    let h = jnum(n, "height", 0.);
    let sx = w / jnum(n, "contentWidth", w).max(1e-12);
    let sy = h / jnum(n, "contentHeight", h).max(1e-12);
    let r = jnum(n, "rotation", 0.).to_radians();
    let x = p[0] * sx - w / 2.;
    let y = p[1] * sy - h / 2.;
    [
        jnum(n, "x", 0.) + w / 2. + x * r.cos() - y * r.sin(),
        jnum(n, "y", 0.) + h / 2. + x * r.sin() + y * r.cos(),
    ]
}
impl Engine {
    pub fn data_anchor(
        &mut self,
        owner: Rc<RefCell<Object>>,
        method: &str,
        pos: Vec<V>,
        mut a: Args,
        l: Loc,
    ) -> Result<V> {
        a.retain(|k, _| !k.starts_with("__"));
        if a.get("r").and_then(V::number).is_some_and(|r| r < 0.)
            && owner
                .borrow()
                .node
                .as_ref()
                .is_some_and(|n| n["plotProjection"]["kind"] == "polar")
        {
            self.warn(
                "W_POLAR_NEGATIVE_RADIUS",
                "负半径已转换为正半径并旋转半周",
                l,
            );
        }
        let id = owner.borrow().id;
        let q = crate::geometry_query::Query {
            instance: Some(id),
            steps: vec![],
        };
        self.geometry_step(
            &q,
            crate::geometry_query::Step::Call(method.into(), pos, a),
            l,
        )
    }
}

pub(crate) fn query_anchor(
    e: &Engine,
    n: &Json,
    method: &str,
    pos: &[V],
    a: &Args,
    l: Loc,
) -> Result<kurbo::Point> {
    if !pos.is_empty() {
        return Err(e.error("E_ARG", "需要 0 个位置参数", l));
    }
    if !n["plotBounds"].is_object() {
        return Err(e.error("E_TYPE", "data/axis 需要已放置的原生图表实例", l));
    }
    if method == "data" && n.get("plotProjection").is_none() {
        for key in ["x", "y"] {
            e.scalar(
                a.get(key)
                    .ok_or_else(|| e.error("E_ARG", format!("缺少参数 {key}"), l))?,
                l,
            )?;
        }
    }
    let b = &n["plotBounds"];
    let point = if n.get("plotProjection").is_some() {
        projected_anchor(e, n, method, &pos, &a, l)?
    } else if method == "axis" {
        let name = string(
            &a,
            "name",
            &pos.first().map(V::as_str).unwrap_or("x".into()),
        );
        let axis = &n["plotAxes"][&name];
        if axis.is_null() {
            return Err(e.error("E_PLOT", "坐标轴不存在", l));
        }
        let anc = string(&a, "anchor", "center");
        if !matches!(anc.as_str(), "start" | "center" | "end") {
            return Err(e.error("E_PLOT", "轴锚点须为 start/center/end", l));
        }
        let side = jstr(axis, "side", "bottom");
        let horizontal = matches!(side, "bottom" | "top");
        let value = if anc == "center" {
            jnum(b, if horizontal { "width" } else { "height" }, 0.) / 2.
        } else {
            map_json(
                axis,
                axis["domain"][if anc == "start" { 0 } else { 1 }]
                    .as_f64()
                    .unwrap(),
            )
            .unwrap()
        };
        let off = jnum(axis, "offset", 0.);
        if horizontal {
            [
                jnum(b, "x", 0.) + value,
                jnum(b, "y", 0.)
                    + if side == "top" {
                        -off
                    } else {
                        jnum(b, "height", 0.) + off
                    },
            ]
        } else {
            [
                jnum(b, "x", 0.)
                    + if side == "left" {
                        -off
                    } else {
                        jnum(b, "width", 0.) + off
                    },
                jnum(b, "y", 0.) + value,
            ]
        }
    } else {
        let mut q = [0., 0.];
        for (dim, name) in ["x", "y"].iter().enumerate() {
            let value = a
                .get(*name)
                .or_else(|| pos.get(dim))
                .and_then(V::number)
                .ok_or_else(|| e.error("E_PLOT", "数据锚点需要有限数值", l))?;
            let key = string(&a, &format!("{name}_axis"), name);
            let axis = &n["plotAxes"][&key];
            let horizontal = matches!(jstr(axis, "side", ""), "top" | "bottom");
            if axis.is_null() || horizontal != (dim == 0) {
                return Err(e.error("E_PLOT", "坐标轴不存在或方向不匹配", l));
            }
            if axis["scale"] == "log" && value <= 0. {
                return Err(e.error("E_PLOT", "对数轴数据必须大于零", l));
            }
            q[dim] = jnum(b, name, 0.)
                + map_json(axis, value)
                    .ok_or_else(|| e.error("E_PLOT", "数据锚点超出轴范围或位于断轴隐藏区间", l))?;
        }
        q
    };
    let p = local_to_parent(n, point);
    Ok(kurbo::Point::new(p[0], p[1]))
}

/// Constructor-time diagnostics also apply to definitions which are never placed.
pub fn validate_definition(e: &Engine, name: &str, a: &Args, l: Loc) -> Result<()> {
    if matches!(
        name,
        "line"
            | "head"
            | "rgb"
            | "hsv"
            | "oklch"
            | "shadow"
            | "glow"
            | "text_path"
            | "text_warp"
            | "text_extrude"
    ) {
        return Ok(());
    }
    let scalar = |v: &V| match v {
        V::Number(n, u) if u.is_empty() && n.is_finite() => Ok(*n),
        _ => Err(e.error("E_UNIT", "需要有限的无单位数值", l)),
    };
    for (key, choices) in [
        ("tick_direction", &["in", "out", "inout"][..]),
        ("grid", &["none", "major", "both"][..]),
        (
            "marker",
            &[
                "none",
                "circle",
                "square",
                "triangle",
                "triangle_down",
                "diamond",
            ][..],
        ),
        ("where", &["pre", "mid", "post"][..]),
        ("orientation", &["vertical", "horizontal"][..]),
        ("origin", &["lower", "upper"][..]),
        ("mode", &["raster", "vector"][..]),
        ("stat", &["count", "probability", "density"][..]),
    ] {
        if a.contains_key(key) && !choices.contains(&string(a, key, "").as_str()) {
            return Err(e.error("E_PLOT", format!("{key} 须为 {}", choices.join("/")), l));
        }
    }
    for key in [
        "tick_labels",
        "reverse",
        "spine",
        "outliers",
        "closed",
        "periodic",
    ] {
        if a.contains_key(key) && !matches!(a.get(key), Some(V::Bool(_))) {
            return Err(e.error("E_PLOT", format!("{key} 需要布尔值"), l));
        }
    }
    for key in ["font_size", "grid_line_width", "bandwidth"] {
        if let Some(v) = a.get(key) {
            if key == "bandwidth" && v.as_str() == "scott" {
                continue;
            }
            let n = if key == "bandwidth" {
                scalar(v)?
            } else {
                e.len(v, l)?
            };
            if n <= 0. {
                return Err(e.error("E_PLOT", format!("{key} 必须大于零"), l));
            }
        }
    }
    if let Some(v) = a.get("grid_line_dash") {
        let v = v.list();
        if v.len() % 2 != 0
            || v.iter()
                .any(|v| value_length(v, &e.unit, e.dpi).is_none_or(|v| v <= 0.))
        {
            return Err(e.error("E_PLOT", "dash 需要成对的正长度", l));
        }
    }
    if matches!(name, "legend" | "colorbar") {
        if let Some(v) = a.get("position") {
            if matches!(v, V::List(_)) {
                let vs = v.list();
                if vs.len() != 2 {
                    return Err(e.error("E_PLOT", "position 需要 2 个值", l));
                }
                for v in vs {
                    e.len(&v, l)?;
                }
            }
        }
        if let Some(v) = a.get("gap") {
            if e.len(v, l)? < 0. {
                return Err(e.error("E_PLOT", "gap 不能为负数", l));
            }
        }
        if let Some(v) = a.get("background") {
            if matches!(v, V::Text(..)) {
                e.validate_color(v, l, "E_PLOT")?;
            }
        }
        if name == "legend" {
            if let Some(v) = a.get("columns") {
                let n = scalar(v)?;
                if n < 1. || n.fract() != 0. {
                    return Err(e.error("E_PLOT", "columns 需要正整数", l));
                }
            }
            if a.contains_key("frame") && !matches!(a.get("frame"), Some(V::Bool(_))) {
                return Err(e.error("E_PLOT", "frame 需要布尔值", l));
            }
        }
        if name == "colorbar" {
            if let Some(V::Text(side, _)) = a.get("position") {
                let horizontal = matches!(side.as_str(), "top" | "bottom");
                if a.contains_key("orientation")
                    && (string(a, "orientation", "") == "horizontal") != horizontal
                {
                    return Err(e.error("E_PLOT", "orientation 与色标所在侧边冲突", l));
                }
            } else if a.contains_key("position") && a.contains_key("gap") {
                return Err(e.error("E_ARG", "手动色标位置不使用 gap", l));
            }
            let mut last = f64::NEG_INFINITY;
            for v in array(a, "ticks") {
                let n = scalar(&v)?;
                if n <= last {
                    return Err(e.error("E_PLOT", "色标刻度须递增且位于颜色范围内", l));
                }
                last = n;
            }
        }
    }
    for key in ["label", "title"] {
        if let Some(v) = a.get(key) {
            if !matches!(v, V::Text(..))
                && !v
                    .object()
                    .is_some_and(|o| matches!(o.borrow().kind.as_str(), "text" | "formula"))
            {
                return Err(e.error("E_ARG", "label 需要字符串、text(...) 或 formula(...)", l));
            }
        }
    }
    for key in ["label_offset", "tick_offset", "exponent_offset"] {
        if let Some(v) = a.get(key) {
            let vs = v.list();
            if vs.len() != 2 {
                return Err(e.error("E_PLOT", format!("{key} 需要两个物理长度"), l));
            }
            for v in vs {
                e.len(&v, l)?;
            }
        }
    }
    for key in [
        "label_font_size",
        "tick_font_size",
        "tick_length",
        "minor_tick_length",
        "break_mark_size",
        "sample_width",
        "thickness",
        "length",
    ] {
        if let Some(v) = a.get(key) {
            if e.len(v, l)? < 0.
                || (matches!(
                    key,
                    "length" | "thickness" | "sample_width" | "label_font_size" | "tick_font_size"
                ) && e.len(v, l)? == 0.)
            {
                return Err(e.error("E_PLOT", format!("{key} 必须为正"), l));
            }
        }
    }
    if name == "axis" {
        let range = nums(a, "range");
        if a.contains_key("range") && range.len() != 2 {
            return Err(e.error("E_PLOT", "range 需要两个值", l));
        }
        for v in array(a, "range") {
            scalar(&v)?;
        }
        if a.contains_key("breaks") && !a.contains_key("range") {
            return Err(e.error("E_PLOT", "断轴需要显式 range", l));
        }
        if !a.contains_key("breaks")
            && ["segment_lengths", "break_gap", "break_mark_size"]
                .iter()
                .any(|k| a.contains_key(*k))
        {
            return Err(e.error("E_PLOT", "分段参数需要 breaks", l));
        }
        for key in ["ticks", "minor_ticks"] {
            let mut last = f64::NEG_INFINITY;
            for v in array(a, key) {
                let n = scalar(&v)?;
                if n <= last || string(a, "scale", "linear") == "log" && n <= 0. {
                    return Err(e.error("E_PLOT", "刻度须严格递增；对数刻度必须为正值", l));
                }
                last = n;
            }
        }
        let mut initial = a.clone();
        initial.remove("minor_ticks");
        let _ = Axis::new(e, initial, "x", "bottom", 0., &[], l)?;
        let n = array(a, "breaks").len();
        if let Some(V::List(g)) = a.get("break_gap") {
            if g.len() != n {
                return Err(e.error("E_PLOT", "break_gap 需要每个断口一个长度", l));
            }
        }
        if let Some(V::List(g)) = a.get("segment_lengths") {
            if g.len() != n + 1 {
                return Err(e.error("E_PLOT", "segment_lengths 需要每段一个长度", l));
            }
        }
    }
    if matches!(name, "axis" | "colorbar") {
        let notation = string(a, "notation", "plain");
        if !matches!(notation.as_str(), "plain" | "scientific" | "offset") {
            return Err(e.error("E_PLOT", "notation 须为 plain/scientific/offset", l));
        }
        if notation != "offset" && (a.contains_key("exponent") || a.contains_key("exponent_offset"))
        {
            return Err(e.error("E_PLOT", "exponent/exponent_offset 仅用于 offset 计数法", l));
        }
        if let Some(v) = a.get("exponent") {
            let n = scalar(v)?;
            if n.fract() != 0. || !(-323. ..=308.).contains(&n) {
                return Err(e.error("E_PLOT", "exponent 须为 -323 到 308 的整数", l));
            }
        }
        if notation == "scientific"
            && a.contains_key("format")
            && !string(a, "format", "").ends_with('e')
        {
            return Err(e.error("E_PLOT", "scientific 的 format 必须为指数格式", l));
        }
    }
    if name == "color_scale" || a.contains_key("cmap") {
        if matches!(a.get("cmap"), Some(V::List(_))) {
            let cs = array(a, "cmap");
            if cs.len() < 2 {
                return Err(e.error("E_PLOT", "颜色序列少于两种颜色", l));
            }
            for c in cs {
                e.validate_color(&c, l, "E_PLOT")?;
            }
        } else if !match a.get("cmap") {
            None | Some(V::Cmap(_)) => true,
            Some(V::Text(name, _)) => crate::colormap::Colormap::get(name).is_some(),
            _ => false,
        } {
            return Err(e.error("E_PLOT", "未知配色；需要预设名称、颜色序列或 cmap 对象", l));
        }
    }
    if name == "color_scale" {
        let norm = string(a, "norm", "linear");
        if !matches!(
            norm.as_str(),
            "linear" | "log" | "symlog" | "centered" | "boundary"
        ) {
            return Err(e.error("E_PLOT", "未知颜色归一化", l));
        }
        let bounds = nums(a, "boundaries");
        if norm == "boundary" {
            if bounds.len() < 2 || bounds.windows(2).any(|v| v[0] >= v[1]) {
                return Err(e.error("E_PLOT", "boundary 需要严格递增的 boundaries", l));
            }
            if a.contains_key("vmin") || a.contains_key("vmax") {
                return Err(e.error("E_PLOT", "离散标尺使用 boundaries，不同时指定 vmin/vmax", l));
            }
        } else if a.contains_key("boundaries") {
            return Err(e.error("E_PLOT", "boundaries 仅用于 boundary 标尺", l));
        } else if !a.contains_key("vmin") || !a.contains_key("vmax") {
            return Err(e.error("E_ARG", "color_scale 需要 vmin/vmax", l));
        }
        let [lo, hi] = color_domain(a);
        if !lo.is_finite() || !hi.is_finite() || lo >= hi || norm == "log" && lo <= 0. {
            return Err(e.error("E_PLOT", "颜色范围须递增、有限；对数颜色范围必须为正", l));
        }
        let c = num(a, "center", 0.);
        if norm == "centered" && !(lo < c && c < hi) {
            return Err(e.error("E_PLOT", "center 须严格位于颜色范围内", l));
        }
        if num(a, "constant", 1.) <= 0. {
            return Err(e.error("E_PLOT", "constant 必须大于零", l));
        }
    }
    if name == "plot" {
        if !["axes", "box", "none"].contains(&string(a, "frame", "axes").as_str()) {
            return Err(e.error("E_PLOT", "frame 须为 axes/box/none", l));
        }
        if let Some(v) = a.get("plot_area").and_then(V::object) {
            let v = v.borrow();
            let xy = pair(&v.args, "offset", [0., 0.], &e.unit, e.dpi);
            let size = pair(&v.args, "size", [0., 0.], &e.unit, e.dpi);
            if xy.iter().any(|v| *v < 0.) || size.iter().any(|v| *v <= 0.) {
                return Err(e.error("E_LAYOUT", "plot_area 的左、上须非负，宽、高须大于零", l));
            }
        }

        for key in ["x", "y", "theta", "r"] {
            if let Some(v) = a.get(key) {
                if !v.object().is_some_and(|o| o.borrow().kind == "axis") {
                    return Err(e.error("E_PLOT", format!("{key} 需要 axis(...)"), l));
                }
            }
        }
        if a.contains_key("margins") && a.contains_key("plot_area") {
            return Err(e.error("E_ARG", "margins 与 plot_area 互斥", l));
        }
        let p = string(a, "projection", "cartesian");
        if !matches!(p.as_str(), "cartesian" | "polar" | "radar") {
            return Err(e.error("E_PLOT", "未知图表投影", l));
        }
        if p == "cartesian"
            && [
                "theta",
                "r",
                "angle_unit",
                "theta_zero",
                "theta_direction",
                "inner_radius",
                "categories",
                "ranges",
            ]
            .iter()
            .any(|k| a.contains_key(*k))
        {
            return Err(e.error("E_PLOT", "极坐标参数需要 projection=polar 或 radar", l));
        }
    }
    Ok(())
}
pub fn validate_layer(
    e: &mut Engine,
    owner: &Object,
    method: &str,
    a: &Args,
    l: Loc,
) -> Result<()> {
    validate_definition(e, method, a, l)?;
    if method == "heatmap"
        && a.contains_key("color_scale")
        && ["cmap", "vmin", "vmax"].iter().any(|k| a.contains_key(*k))
    {
        return Err(e.error("E_PLOT", "color_scale 不与 cmap/vmin/vmax 同时指定", l));
    }
    let projection = string(&owner.args, "projection", "cartesian");
    if method == "bar"
        && ((projection == "polar" && a.contains_key("data_width"))
            || (projection == "cartesian" && a.contains_key("angle_width")))
    {
        return Err(e.error(
            "E_ARG",
            "极坐标柱使用 angle_width；直角坐标柱使用 data_width",
            l,
        ));
    }

    if projection != "cartesian" {
        let valid = if projection == "radar" {
            vec!["line", "scatter", "area", "legend"]
        } else {
            vec![
                "line", "scatter", "step", "errorbar", "area", "band", "bar", "hist", "heatmap",
                "contour", "contourf", "legend", "colorbar",
            ]
        };
        if !valid.contains(&method) {
            return Err(e.error("E_PLOT", format!("{projection} 不支持 {method}"), l));
        }
        if [
            "x",
            "y",
            "xerr",
            "yerr",
            "x_edges",
            "y_edges",
            "x_axis",
            "y_axis",
            "orientation",
        ]
        .iter()
        .any(|k| a.contains_key(*k))
        {
            return Err(e.error("E_PLOT", "极坐标图层使用 theta/r 数据", l));
        }
    }
    if method == "add_axis" {
        let name = string(a, "name", "");
        if name.is_empty()
            || matches!(name.as_str(), "x" | "y")
            || owner
                .layers
                .iter()
                .any(|(k, a)| k == "add_axis" && string(a, "name", "") == name)
        {
            return Err(e.error("E_PLOT", "坐标轴名称重复或为空", l));
        }
        return Ok(());
    }
    if method == "legend" {
        let c = num(a, "columns", 1.);
        if c < 1. || c.fract() != 0. {
            return Err(e.error("E_PLOT", "columns 需要正整数", l));
        }
        return validate_definition(e, "legend", a, l);
    }
    if method == "colorbar" {
        return validate_definition(e, "colorbar", a, l);
    }
    for (key, horizontal) in [("x_axis", true), ("y_axis", false)] {
        if let Some(v) = a.get(key) {
            let name = v.as_str();
            if name == if horizontal { "x" } else { "y" } {
                continue;
            }
            if !owner.layers.iter().any(|(k, a)| {
                k == "add_axis"
                    && string(a, "name", "") == name
                    && matches!(string(a, "side", "right").as_str(), "top" | "bottom") == horizontal
            }) {
                return Err(e.error("E_PLOT", "图层坐标轴不存在或方向不匹配", l));
            }
        }
    }
    for key in [
        "x",
        "y",
        "theta",
        "r",
        "positions",
        "values",
        "lower",
        "upper",
        "c",
    ] {
        if let Some(v) = a.get(key) {
            for value in v.list() {
                if !matches!(value, V::Null) && !matches!(&value,V::Number(_,u)if u.is_empty()) {
                    if !matches!(value, V::Null) {
                        return Err(e.error("E_UNIT", "绘图数据需要无单位数值", l));
                    }
                }
            }
        }
    }
    let style = object_args(&owner.args, "style");
    let layer = build(
        e,
        method,
        a,
        owner.layers.len(),
        &style,
        &projection,
        &owner.args,
        l,
    )?;
    if projection == "radar" {
        let ranges = array(&owner.args, "ranges");
        let values = nums(a, "values");
        for (i, range) in ranges.iter().enumerate() {
            let r = range.list();
            if r.len() == 2
                && values.get(i).is_some_and(|v| {
                    v.is_finite()
                        && (*v < r[0].number().unwrap_or(0.) || *v > r[1].number().unwrap_or(0.))
                })
            {
                return Err(e.error("E_PLOT", "雷达指标超出范围", l));
            }
        }
    } else if projection == "polar" {
        if string(&object_args(&owner.args, "r"), "scale", "linear") == "log"
            && layer_values(&layer, 1).iter().any(|v| *v == 0.)
        {
            return Err(e.error("E_PLOT", "对数半径的数据必须非零", l));
        }
    } else {
        for (dim, name) in [(0, &layer.xaxis), (1, &layer.yaxis)] {
            let spec = if name == "x" || name == "y" {
                object_args(&owner.args, name)
            } else {
                owner
                    .layers
                    .iter()
                    .find(|(k, a)| k == "add_axis" && string(a, "name", "") == *name)
                    .map(|(_, a)| object_args(a, "axis"))
                    .unwrap_or_default()
            };
            if string(&spec, "scale", "linear") == "log"
                && layer_values(&layer, dim).iter().any(|v| *v <= 0.)
            {
                return Err(e.error("E_PLOT", format!("{name} 对数轴的数据及端点必须大于零"), l));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eng() -> Engine {
        Engine::new(Host::default())
    }
    fn a(j: Json) -> Args {
        match V::from_json(&j) {
            V::Map(a) => a,
            _ => panic!("args"),
        }
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
    #[test]
    fn broken_axes_keep_physical_segment_lengths_and_reverse() {
        let e = eng();
        let aa = a(
            json!({"scale":"log","range":[1,1000],"breaks":[[10,100]],"break_gap":2,"reverse":true}),
        );
        let mut axis = Axis::new(&e, aa, "x", "bottom", 0., &[], Loc::default()).unwrap();
        axis.map_segments(&e, 60., Loc::default()).unwrap();
        near(axis.segments[0].range[0], 60.);
        near(axis.segments[0].range[1], 31.);
        near(axis.map(100.).unwrap(), 29.);
        assert!(axis.map(50.).is_none());
    }
    #[test]
    fn symlog_mapping_is_symmetric() {
        let e = eng();
        let mut axis = Axis::new(
            &e,
            a(json!({"scale":"symlog","constant":2,"range":[-100,100]})),
            "y",
            "left",
            0.,
            &[],
            Loc::default(),
        )
        .unwrap();
        axis.map_segments(&e, 50., Loc::default()).unwrap();
        near(axis.map(0.).unwrap(), 25.);
        near(axis.map(-100.).unwrap(), 50.);
        near(axis.map(100.).unwrap(), 0.);
    }
    #[test]
    fn weighted_histogram_mass_and_density_match_hand_calculation() {
        let mut e = eng();
        let l=build(&mut e,"hist",&a(json!({"values":[-1,0,0.5,1,2,3,4],"bins":[0,1,3],"weights":[1,2,3,4,5,6,7],"stat":"density"})),0,&Args::new(),"cartesian",&Args::new(),Loc::default()).unwrap();
        assert_eq!(l.statistics["counts"], json!([5., 15.]));
        near(l.statistics["total"].as_f64().unwrap(), 20.);
        let h = l.statistics["heights"].as_array().unwrap();
        near(h[0].as_f64().unwrap() + h[1].as_f64().unwrap() * 2., 1.);
        assert!(e.warnings.iter().any(|w| w.message.contains("分箱")));
    }
    #[test]
    fn r7_quartiles_and_kde_scott_bandwidth() {
        let mut e = eng();
        let b = build(
            &mut e,
            "boxplot",
            &a(json!({"values":[100,1,2,3,4,5,6,7,8]})),
            0,
            &Args::new(),
            "cartesian",
            &Args::new(),
            Loc::default(),
        )
        .unwrap();
        for (k, v) in [
            ("q1", 3.),
            ("median", 5.),
            ("q3", 7.),
            ("lower", 1.),
            ("upper", 8.),
        ] {
            near(b.statistics[k].as_f64().unwrap(), v);
        }
        assert_eq!(b.statistics["outliers"], json!([100.]));
        let k = build(
            &mut e,
            "violin",
            &a(json!({"values":[-1,1]})),
            0,
            &Args::new(),
            "cartesian",
            &Args::new(),
            Loc::default(),
        )
        .unwrap();
        near(
            k.statistics["bandwidth"].as_f64().unwrap(),
            2f64.sqrt() * 2f64.powf(-0.2),
        );
    }
    #[test]
    fn contour_nonuniform_grid_and_missing_nodes() {
        let mut e = eng();
        let mut args =
            a(json!({"z":[[0,1,2],[0,1,2],[0,1,2]],"x":[0,2,6],"y":[0,1,2],"levels":[0.5]}));
        let c = build(
            &mut e,
            "contour",
            &args,
            0,
            &Args::new(),
            "cartesian",
            &Args::new(),
            Loc::default(),
        )
        .unwrap();
        assert!(!c.primitives.is_empty());
        for p in &c.primitives {
            for q in &p.points {
                near(q[0], 1.);
            }
        }
        args.insert(
            "z".into(),
            V::from_json(&json!([[0, 1, 2], [0, null, 2], [0, 1, 2]])),
        );
        let c = build(
            &mut e,
            "contourf",
            &args,
            0,
            &Args::new(),
            "cartesian",
            &Args::new(),
            Loc::default(),
        )
        .unwrap();
        assert!(c.primitives.is_empty());
    }
    #[test]
    fn shared_palette_interpolates_typed_colors_and_alpha() {
        let mut args = a(json!({"vmin":0,"vmax":1}));
        args.insert(
            "cmap".into(),
            V::List(vec![
                V::text("#ff000040"),
                V::Color(Rc::new(
                    crate::color::Color::new("hsv", [120., 1., 1.], 0.75).unwrap(),
                )),
            ]),
        );
        let color = crate::color::Color::parse(&scale_color(&args, 0.5)).unwrap();
        near(color.rgba[0], 0.5);
        near(color.rgba[1], 0.5);
        near(color.alpha, (64. / 255. + 0.75) / 2.);
    }
    #[test]
    fn color_normalizations_and_inverse() {
        let s = a(json!({"vmin":0,"vmax":10,"cmap":["#000000","#ffffff"]}));
        assert_eq!(scale_color(&s, 5.), "#808080");
        assert_eq!(scale_color(&s, f64::NAN), "none");
        let s = a(json!({"norm":"log","vmin":1,"vmax":100}));
        near(color_position(&s, 10.), 0.5);
        near(color_value(&s, 0.5), 10.);
        let s = a(json!({"norm":"symlog","vmin":-9,"vmax":9}));
        near(color_value(&s, color_position(&s, -3.)), -3.);
        let s = a(json!({"norm":"boundary","boundaries":[0,1,10],"cmap":["#f00","#00f"]}));
        assert_eq!(scale_color(&s, 0.99), "#f00");
        assert_eq!(scale_color(&s, 1.), "#00f");
    }
    #[test]
    fn native_plot_anchors_work_without_installed_fonts() {
        let source = r##"page=canvas(size=(120 mm,100 mm))
p=plot(size=(100 mm,80 mm),plot_area=box(offset=(20 mm,10 mm),size=(60 mm,52 mm)),x=axis(range=(0,10),breaks=[(2,8)],break_gap=2 mm,segment_lengths=[20 mm,38 mm]),y=axis(range=(0,100)))
p.line(x=[0,10],y=[0,100])
a=page.add(p)
page.add(rect(size=(1 mm,1 mm)),anchor=center,target=a.data(x=8,y=50))"##;
        let scene =
            crate::engine::compile_source(source, "/fixture/test.lay", Host::default()).unwrap();
        near(jnum(&scene.nodes[1], "x", 0.) + 0.5, 42.);
        near(jnum(&scene.nodes[1], "y", 0.) + 0.5, 36.);
        assert!(!scene.warnings.is_empty());
    }
    #[test]
    fn radar_anchors_use_independent_negative_domains() {
        let source = r##"page=canvas(size=(120 mm,100 mm))
p=plot(projection="radar",size=(110 mm,100 mm),plot_area=box(offset=(20 mm,15 mm),size=(60 mm,60 mm)),categories=["Strength","Temperature","Cost"],ranges=[(0,100),(-20,20),(10,30)])
p.line(values=[100,-10,20])
a=page.add(p)
page.add(rect(size=(1 mm,1 mm)),anchor=center,target=a.data(category="Temperature",value=-10))
page.add(rect(size=(1 mm,1 mm)),anchor=center,target=a.axis(name="Strength",anchor="end"))"##;
        let s =
            crate::engine::compile_source(source, "/fixture/test.lay", Host::default()).unwrap();
        near(
            (jnum(&s.nodes[1], "x", 0.) + 0.5 - 50.).hypot(jnum(&s.nodes[1], "y", 0.) + 0.5 - 45.),
            7.5,
        );
        near(jnum(&s.nodes[2], "x", 0.) + 0.5, 50.);
        near(jnum(&s.nodes[2], "y", 0.) + 0.5, 15.);
    }
    #[test]
    fn imported_plot_style_resolves_font_next_to_its_module() {
        let mut host = Host::default();
        host.files.insert(
            "/fixture/components/theme.lay".into(),
            br#"export s=plot_style(font_family="../fonts/body.ttf")"#.to_vec(),
        );
        host.files.insert(
            "/fixture/fonts/body.ttf".into(),
            include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
        );
        let source = r#"import { s } from "./components/theme.lay"
page=canvas(size=(100,80))
p=plot(size=(90,70),style=s)
p.line(x=[0,1],y=[1,2])
page.add(p)"#;
        let scene = crate::engine::compile_source(source, "/fixture/page.lay", host).unwrap();
        assert!(!scene.fonts.is_empty());
        assert!(
            !scene.warnings.iter().any(|w| w.code == "W_FONT"),
            "{:?}",
            scene.warnings
        );
    }
    #[test]
    fn missing_and_negative_radius_warnings_are_emitted_once_at_layer() {
        let source = r#"page=canvas(size=(100,80))
p=plot(projection="polar",size=(90,70))
p.line(theta=[0,90,180],r=[1,-1,2])
page.add(p)"#;
        let scene =
            crate::engine::compile_source(source, "/fixture/page.lay", Host::default()).unwrap();
        let warnings: Vec<_> = scene
            .warnings
            .iter()
            .filter(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].loc.line, 3);
        let mut host = Host::default();
        host.files.insert(
            "/fixture/data.json".into(),
            br#"{"x":[0,1,2],"y":[1,null,3]}"#.to_vec(),
        );
        let source = r#"d=table(src="data.json")
page=canvas(size=(100,80))
p=plot(size=(90,70))
p.line(x=d["x"],y=d["y"])
page.add(p)"#;
        let scene = crate::engine::compile_source(source, "/fixture/page.lay", host).unwrap();
        let warnings: Vec<_> = scene
            .warnings
            .iter()
            .filter(|w| w.code == "W_PLOT_MISSING")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].loc.line, 4);
    }

    #[test]
    fn scientific_format_preserves_accounting_and_padding() {
        assert_eq!(
            format_number(-120000., &a(json!({"format":"(.2e"}))),
            "(1.20e+5)"
        );
        assert_eq!(
            format_number(120000., &a(json!({"format":"0<12.2e"}))),
            "1.20e+500000"
        );
        assert_eq!(
            scale_color(&a(json!({"cmap":"rdbu","vmin":0,"vmax":1})), 0.5),
            "#f6f7f7"
        );
    }
    #[test]
    fn plot_parts_share_variables_and_preserve_explicit_marker_values() {
        let mut e = eng();
        e.stylesheet("plot.graph {--ink:#ff0000;} plot.graph::axis(x){line-color:var(--ink);} plot.graph::grid{line-color:#123456;} plot.graph::marker {fill:#00ff00;} plot.graph::area {background:#abcdef;} plot.graph::legend {background:#fedcba;}","/fixture/style.lcss",Loc::default()).unwrap();
        let scene = e
            .compile(
                r##"page=canvas(size=(100,80))
p=plot(size=(90,70),class="graph",x=axis(grid="major"))
p.scatter(x=[0,1],y=[1,2],label="series",marker_fill="#0000ff")
p.legend()
page.add(p)"##,
                "/fixture/page.lay",
            )
            .unwrap();
        fn nodes<'a>(n: &'a Json, v: &mut Vec<&'a Json>) {
            v.push(n);
            for c in n["children"].as_array().into_iter().flatten() {
                nodes(c, v);
            }
        }
        let mut all = vec![];
        nodes(&scene.nodes[0], &mut all);
        assert!(all.iter().any(|n| n["strokeStyle"]["color"] == "#ff0000"));
        assert!(all.iter().any(|n| n["strokeStyle"]["color"] == "#123456"));
        assert!(all.iter().any(|n| n["markerFill"] == "#0000ff"));
        assert!(all.iter().any(|n| n["fill"] == "#abcdef"));
        assert!(all.iter().any(|n| n["fill"] == "#fedcba"));
    }
    #[test]
    fn plotting_errors_retain_layer_and_placement_locations() {
        for (definition, layer, line) in [
            (
                "plot(projection=\"radar\",categories=[\"A\",\"B\",\"C\"],ranges=[(0,1),(0,1),(0,1)])",
                "p.line(values=[1,2,1])",
                3,
            ),
            (
                "plot(projection=\"polar\",r=axis(scale=\"log\"))",
                "p.scatter(theta=[0],r=[0])",
                3,
            ),
            ("plot(y=axis(scale=\"log\"))", "p.line(x=[0,1],y=[0,1])", 3),
            (
                "plot(x=axis(range=(0,1),minor_ticks=[2]))",
                "p.line(x=[0,1],y=[1,2])",
                4,
            ),
        ] {
            let definition = definition.replacen("plot(", "plot(size=(90,70),", 1);
            let source =
                format!("page=canvas(size=(100,80))\np={definition}\n{layer}\npage.add(p)");
            let error =
                crate::engine::compile_source(&source, "/fixture/error.lay", Host::default())
                    .unwrap_err();
            assert_eq!(error.code, "E_PLOT", "{error:?}");
            assert_eq!(error.loc.line, line, "{error:?}");
        }
        let source = "page=canvas(size=(100,80))\np=plot(size=(90,70))\nh=p.heatmap(z=[[0,1],[2,3]])\np.colorbar(h,ticks=[4])\npage.add(p)";
        let error = crate::engine::compile_source(source, "/fixture/error.lay", Host::default())
            .unwrap_err();
        assert_eq!(error.code, "E_PLOT", "{error:?}");
        assert_eq!(error.loc.line, 4, "{error:?}");
    }
    #[test]
    fn diagnostic_origins_match_captured_definition_errors() {
        let corpus =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
        for (capture, row) in [
            ("cases-1044564.jsonl", 11),
            ("cases-1044575.jsonl", 11),
            ("cases-1044575.jsonl", 12),
            ("cases-1044575.jsonl", 13),
            ("cases-1044575.jsonl", 14),
            ("cases-1044575.jsonl", 16),
            ("cases-1044587.jsonl", 12),
        ] {
            let input = std::fs::read_to_string(corpus.join(capture)).unwrap();
            let case: Json = serde_json::from_str(input.lines().nth(row - 1).unwrap()).unwrap();
            let mut host = Host::default();
            for (file, hash) in case["files"].as_object().unwrap() {
                host.files.insert(
                    file.clone(),
                    std::fs::read(corpus.join("blobs").join(hash.as_str().unwrap())).unwrap(),
                );
            }
            let error = crate::engine::compile_source(
                case["source"].as_str().unwrap(),
                case["file"].as_str().unwrap(),
                host,
            )
            .unwrap_err();
            let expected = &case["result"];
            assert_eq!(
                error.code,
                expected["code"].as_str().unwrap(),
                "{capture}:{row}"
            );
            assert_eq!(
                error.file,
                expected["file"].as_str().unwrap(),
                "{capture}:{row}"
            );
            assert_eq!(
                error.loc.line,
                expected["loc"]["line"].as_u64().unwrap() as usize,
                "{capture}:{row}"
            );
            assert_eq!(
                error.loc.column,
                expected["loc"]["column"].as_u64().unwrap() as usize,
                "{capture}:{row}"
            );
        }
    }
}

#[cfg(test)]
mod regressions;
