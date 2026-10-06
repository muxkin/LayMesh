//! Decorations are compiled once into physical vector geometry and alpha effects.
use crate::{Loc, Result, engine::Engine, model::*};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
use serde_json::{Value as Json, json};
use ttf_parser::OutlineBuilder;

#[derive(Default)]
struct GlyphOutline(BezPath);
impl OutlineBuilder for GlyphOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x as f64, -y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x as f64, -y as f64));
    }
    fn quad_to(&mut self, x: f32, y: f32, a: f32, b: f32) {
        self.0.quad_to((x as f64, -y as f64), (a as f64, -b as f64));
    }
    fn curve_to(&mut self, x: f32, y: f32, a: f32, b: f32, c: f32, d: f32) {
        self.0.curve_to(
            (x as f64, -y as f64),
            (a as f64, -b as f64),
            (c as f64, -d as f64),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}
fn config(v: &V, kind: &str) -> Option<Args> {
    v.object()
        .filter(|o| o.borrow().kind == kind)
        .map(|o| o.borrow().args.clone())
}
fn angle(a: &Args, k: &str, d: f64) -> f64 {
    match a.get(k).map(V::value) {
        Some(V::Number(n, u)) if u == "rad" => *n,
        Some(V::Number(n, _)) => n.to_radians(),
        _ => d.to_radians(),
    }
}
fn mapped(p: &BezPath, step: f64, f: impl Fn(Point) -> Point) -> Option<BezPath> {
    fn edge(
        out: &mut BezPath,
        a: Point,
        b: Point,
        step: f64,
        f: &impl Fn(Point) -> Point,
        depth: u32,
    ) -> bool {
        if out.elements().len() > 200_000 || depth > 24 {
            return false;
        }
        let fa = f(a);
        let fb = f(b);
        if !fa.x.is_finite() || !fa.y.is_finite() || !fb.x.is_finite() || !fb.y.is_finite() {
            return false;
        }
        let delta = fb - fa;
        let norm = delta.hypot2();
        let error = [0.25, 0.5, 0.75]
            .into_iter()
            .map(|t| {
                let q = f(a + (b - a) * t);
                let u = if norm > 1e-24 {
                    ((q - fa).dot(delta) / norm).clamp(0., 1.)
                } else {
                    0.
                };
                q.distance(fa + delta * u)
            })
            .fold(0f64, f64::max);
        if a.distance(b) > step || error > 0.002 {
            let mid = a + (b - a) * 0.5;
            edge(out, a, mid, step, f, depth + 1) && edge(out, mid, b, step, f, depth + 1)
        } else {
            out.line_to(fb);
            true
        }
    }
    let mut out = BezPath::new();
    let mut first = Point::ORIGIN;
    let mut last = first;
    let mut valid = true;
    kurbo::flatten(p.iter(), 0.002, |el| match el {
        PathEl::MoveTo(p) => {
            out.move_to(f(p));
            first = p;
            last = p
        }
        PathEl::LineTo(p) => {
            if valid {
                valid = edge(&mut out, last, p, step, &f, 0);
            }
            last = p
        }
        PathEl::ClosePath => {
            if valid {
                valid = edge(&mut out, last, first, step, &f, 0);
            }
            out.close_path();
            last = first
        }
        _ => {}
    });
    valid.then_some(out)
}
struct Route {
    points: Vec<Point>,
    dist: Vec<f64>,
}
impl Route {
    fn new(p: &BezPath) -> Option<Self> {
        let mut pts = Vec::new();
        let mut moves = 0;
        let mut closed = false;
        kurbo::flatten(p.iter(), 0.002, |e| match e {
            PathEl::MoveTo(p) => {
                moves += 1;
                pts.push(p)
            }
            PathEl::LineTo(p) => pts.push(p),
            PathEl::ClosePath => closed = true,
            _ => {}
        });
        if moves != 1 || closed || pts.len() < 2 {
            return None;
        }
        let mut dist = vec![0.];
        for w in pts.windows(2) {
            dist.push(dist.last().unwrap() + w[0].distance(w[1]));
        }
        if *dist.last().unwrap() < 1e-9 {
            return None;
        }
        Some(Self { points: pts, dist })
    }
    fn length(&self) -> f64 {
        *self.dist.last().unwrap()
    }
    fn at(&self, d: f64) -> (Point, f64) {
        let i = self
            .dist
            .partition_point(|x| *x <= d)
            .clamp(1, self.dist.len() - 1);
        let a = self.points[i - 1];
        let b = self.points[i];
        let t =
            ((d - self.dist[i - 1]) / (self.dist[i] - self.dist[i - 1]).max(1e-12)).clamp(0., 1.);
        (a + (b - a) * t, (b.y - a.y).atan2(b.x - a.x))
    }
}
impl Engine {
    pub(crate) fn validate_art(&self, name: &str, a: &Args, l: Loc) -> Result<()> {
        for key in ["blur", "spread", "depth", "text_stroke_width"] {
            if let Some(v) = a.get(key) {
                if self.len(v, l)? < 0. {
                    return Err(self.error("E_EFFECT", format!("{key} must be nonnegative"), l));
                }
            }
        }
        for key in ["angle", "phase"] {
            if let Some(v) = a.get(key) {
                if !matches!(v.value(),V::Number(n,u) if n.is_finite()&&(u.is_empty()||u=="deg"||u=="rad"))
                {
                    return Err(self.error(
                        "E_EFFECT",
                        format!("{key} requires a finite angle"),
                        l,
                    ));
                }
            }
        }
        if let Some(v) = a.get("wavelength") {
            if self.len(v, l)? <= 0. {
                return Err(self.error("E_EFFECT", "wavelength must be positive", l));
            }
        }
        if name == "shadow" {
            if let Some(v) = a.get("offset") {
                if v.list().len() != 2 {
                    return Err(self.error("E_EFFECT", "offset requires two lengths", l));
                }
            }
        }
        if name == "text_path" {
            let o = a["path"].object().ok_or_else(|| {
                self.error("E_EFFECT", "text_path requires a local path material", l)
            })?;
            if !matches!(
                o.borrow().kind.as_str(),
                "path" | "line" | "polyline" | "arc"
            ) {
                return Err(self.error("E_EFFECT", "text_path requires an open path material", l));
            }
            if let Some(v) = a.get("reverse") {
                if !matches!(v.value(), V::Bool(_)) {
                    return Err(self.error("E_EFFECT", "reverse requires boolean", l));
                }
            }
        }
        if name == "text_warp" && string(a, "kind", "arc") == "perspective" {
            let pts = array(a, "corners");
            if pts.len() != 4 || pts.iter().any(|v| v.list().len() != 2) {
                return Err(self.error("E_EFFECT","perspective requires four corners: top-left, top-right, bottom-right, bottom-left",l));
            }
            let pts = pts
                .iter()
                .map(|v| {
                    let p = v.list();
                    Ok(Point::new(self.len(&p[0], l)?, self.len(&p[1], l)?))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut sign = 0.;
            for i in 0..4 {
                let cross = (pts[(i + 1) % 4] - pts[i]).cross(pts[(i + 2) % 4] - pts[(i + 1) % 4]);
                if cross.abs() < 1e-8 || sign * cross < 0. {
                    return Err(self.error(
                        "E_EFFECT",
                        "perspective corners must form a nondegenerate convex quadrilateral",
                        l,
                    ));
                }
                sign = cross;
            }
        }
        if let Some(v) = a.get("effects") {
            let V::List(vs) = v.value() else {
                return Err(self.error("E_EFFECT", "effects requires a list", l));
            };
            if vs.len() > 32 {
                return Err(self.error("E_LIMIT", "at most 32 effects", l));
            }
            for v in vs {
                if config(v, "shadow").is_none() && config(v, "glow").is_none() {
                    return Err(self.error(
                        "E_EFFECT",
                        "effects accepts shadow(...) and glow(...) only",
                        l,
                    ));
                }
            }
        }
        if name == "text" {
            for (key, kind) in [
                ("path", "text_path"),
                ("warp", "text_warp"),
                ("extrude", "text_extrude"),
            ] {
                if let Some(v) = a.get(key) {
                    if config(v, kind).is_none() {
                        return Err(self.error(
                            "E_EFFECT",
                            format!("{key} requires {kind}(...)"),
                            l,
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn art_effects(&self, a: &Args, l: Loc) -> Result<Vec<Json>> {
        self.validate_art("material", a, l)?;
        array(a,"effects").iter().map(|v|{let o=v.object().unwrap();let o=o.borrow();let args=&o.args;
       self.validate_definition_readonly_effect(args,l)?;
       Ok(json!({"kind":o.kind,"color":string(args,"color",if o.kind=="glow"{"#ffffff"}else{"#000000"}),"opacity":num(args,"opacity",if o.kind=="glow"{1.}else{0.5}),"blur":length(args,"blur",1.,&self.unit,self.dpi),"spread":length(args,"spread",0.,&self.unit,self.dpi),"offset":pair(args,"offset",if o.kind=="shadow"{[1.,1.]}else{[0.,0.]},&self.unit,self.dpi),"mode":string(args,"mode","outer"),"target":string(args,"target","content")}))
    }).collect()
    }
    fn validate_definition_readonly_effect(&self, a: &Args, l: Loc) -> Result<()> {
        self.validate_art("effect", a, l)
    }
    pub(crate) fn decorate_text(&mut self, n: &mut Json, a: &Args, l: Loc) -> Result<()> {
        let active = [
            "text_fill",
            "text_stroke_color",
            "text_stroke_width",
            "path",
            "warp",
            "extrude",
        ]
        .iter()
        .any(|k| a.contains_key(*k));
        let active = active
            || n["runs"].as_array().into_iter().flatten().any(|r| {
                r.get("text_fill").is_some()
                    || r.get("text_stroke_color").is_some()
                    || r.get("text_stroke_width").is_some()
            });
        if !active && array(a, "effects").is_empty() {
            return Ok(());
        }
        self.validate_art("text", a, l)?;
        let cache_key = format!("{}:{}", n, args_json(a));
        if let Some(cached) = self.art_cache.get(&cache_key) {
            for k in ["runs", "artText", "subjectBounds"] {
                if let Some(value) = cached.get(k) {
                    if !value.is_null() {
                        n[k] = value.clone();
                    }
                }
            }
            return Ok(());
        }
        let path_args = a.get("path").and_then(|v| config(v, "text_path"));
        let route = if let Some(ref pa) = path_args {
            let mat = pa["path"].object().unwrap();
            let o = mat.borrow();
            let sz = pair(&o.args, "size", [f64::NAN; 2], &self.unit, self.dpi);
            let node = self.shape(&o, &o.args, sz)?;
            let p = BezPath::from_svg(jstr(&node, "d", ""))
                .map_err(|_| self.error("E_EFFECT", "invalid text path", l))?;
            Some(Route::new(&p).ok_or_else(|| {
                self.error("E_EFFECT", "text_path requires one nonzero open subpath", l)
            })?)
        } else {
            None
        };
        let width = jnum(n, "width", 0.);
        let height = jnum(n, "height", 0.);
        let baselines: Vec<f64> = n["runs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["kind"] == "glyph" || r["kind"] == "box")
            .map(|r| {
                if r["kind"] == "box" {
                    jnum(r, "y", 0.) + jnum(r, "height", 0.)
                } else {
                    jnum(r, "baseline", 0.)
                }
            })
            .collect();
        if route.is_some()
            && (jstr(n, "content", "").contains('\n')
                || baselines
                    .iter()
                    .any(|b| (*b - baselines.first().copied().unwrap_or(0.)).abs() > 1e-6))
        {
            return Err(self.error("E_EFFECT", "text_path accepts single-line text only", l));
        }
        if route.is_some()
            && n["runs"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|r| r["kind"] == "formula")
        {
            return Err(self.error(
                "E_EFFECT",
                "inline formulas cannot be placed on text_path",
                l,
            ));
        }
        let start = if let (Some(r), Some(pa)) = (&route, &path_args) {
            let offset = length(pa, "start", 0., &self.unit, self.dpi);
            let start = offset
                + match string(pa, "align", "left").as_str() {
                    "center" => (r.length() - width) / 2.,
                    "right" => r.length() - width,
                    _ => 0.,
                };
            if start < 0. || start + width > r.length() + 1e-6 {
                return Err(self.error(
                    "E_EFFECT",
                    "text exceeds path length; increase the path or reduce font size/start",
                    l,
                ));
            }
            start
        } else {
            0.
        };
        let reverse = path_args.as_ref().is_some_and(|pa| {
            pa.get("reverse")
                .is_some_and(|v| matches!(v.value(), V::Bool(true)))
        });
        let mut paths: Vec<(BezPath, Json, Json)> = Vec::new();
        let mut semantics = String::new();
        for run in n["runs"].as_array().into_iter().flatten() {
            if run["kind"] == "glyph" {
                let key = jstr(run, "fontFamily", "").to_owned();
                let font = &self.fonts.assets[&key];
                let face = rustybuzz::Face::from_slice(&font.data, font.index)
                    .ok_or_else(|| self.error("E_FONT", "cannot outline font", l))?;
                let mut b = rustybuzz::UnicodeBuffer::new();
                b.push_str(jstr(run, "content", ""));
                b.guess_segment_properties();
                let shaped = rustybuzz::shape(&face, &[], b);
                let scale = jnum(run, "fontSize", 3.) / face.units_per_em() as f64;
                let mut x = jnum(run, "x", 0.);
                let baseline = jnum(run, "baseline", 0.);
                semantics += jstr(run, "content", "");
                for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                    let gid = info.glyph_id as u16;
                    let ck = (key.clone(), gid);
                    let raw = self
                        .fonts
                        .glyph_paths
                        .entry(ck)
                        .or_insert_with(|| {
                            let mut out = GlyphOutline::default();
                            face.outline_glyph(ttf_parser::GlyphId(gid), &mut out);
                            out.0
                        })
                        .clone();
                    let center = x + pos.x_advance as f64 * scale / 2.;
                    let mut p = Affine::translate((
                        x + pos.x_offset as f64 * scale,
                        baseline - pos.y_offset as f64 * scale,
                    )) * Affine::scale(scale)
                        * raw;
                    if let Some(r) = &route {
                        let d = start + center;
                        let (q, mut theta) = r.at(if reverse { r.length() - d } else { d });
                        if reverse {
                            theta += std::f64::consts::PI
                        }
                        p = Affine::translate(q.to_vec2())
                            * Affine::rotate(theta)
                            * Affine::translate((-center, -baseline))
                            * p;
                    }
                    let fill = run
                        .get("text_fill")
                        .cloned()
                        .unwrap_or_else(|| run["color"].clone());
                    paths.push((p, fill, run.clone()));
                    x += pos.x_advance as f64 * scale;
                }
            } else if run["kind"] == "box" {
                let mut p = Rect::new(
                    jnum(run, "x", 0.),
                    jnum(run, "y", 0.),
                    jnum(run, "x", 0.) + jnum(run, "width", 1.),
                    jnum(run, "y", 0.) + jnum(run, "height", 1.),
                )
                .to_path(0.002);
                p = crate::geometry::outline(
                    &p,
                    &json!({"width":jnum(run,"strokeWidth",0.15),"color":jstr(run,"color","#000000")}),
                );
                if let Some(r) = &route {
                    let center = jnum(run, "x", 0.) + jnum(run, "width", 1.) / 2.;
                    let d = start + center;
                    let (q, mut theta) = r.at(if reverse { r.length() - d } else { d });
                    if reverse {
                        theta += std::f64::consts::PI
                    }
                    p = Affine::translate(q.to_vec2())
                        * Affine::rotate(theta)
                        * Affine::translate((
                            -center,
                            -jnum(run, "y", 0.) - jnum(run, "height", 0.),
                        ))
                        * p;
                }
                paths.push((
                    p,
                    run.get("text_fill")
                        .cloned()
                        .unwrap_or_else(|| run["color"].clone()),
                    run.clone(),
                ));
            } else if run["kind"] == "formula" {
                semantics += jstr(run, "source", "");
                for item in run["items"].as_array().into_iter().flatten() {
                    let p = if item["kind"] == "path" {
                        BezPath::from_svg(jstr(item, "d", "")).unwrap_or_default()
                    } else if item["kind"] == "rule" {
                        Rect::new(
                            jnum(item, "x", 0.),
                            jnum(item, "y", 0.),
                            jnum(item, "x", 0.) + jnum(item, "width", 0.),
                            jnum(item, "y", 0.) + jnum(item, "height", 0.),
                        )
                        .to_path(0.002)
                    } else {
                        continue;
                    };
                    paths.push((
                        Affine::translate((jnum(run, "x", 0.), jnum(run, "y", 0.))) * p,
                        run.get("text_fill").cloned().unwrap_or_else(|| {
                            item.get("fill")
                                .or_else(|| item.get("color"))
                                .cloned()
                                .unwrap_or(json!("#000000"))
                        }),
                        Json::Null,
                    ));
                }
            }
        }
        if let Some(wa) = a.get("warp").and_then(|v| config(v, "text_warp")) {
            let kind = string(&wa, "kind", "arc");
            let sweep = angle(&wa, "angle", 45.);
            let amp = length(&wa, "amplitude", 2., &self.unit, self.dpi);
            let wave = length(&wa, "wavelength", 20., &self.unit, self.dpi);
            let phase = angle(&wa, "phase", 0.);
            let corners = array(&wa, "corners")
                .iter()
                .map(|v| {
                    let p = v.list();
                    Point::new(
                        value_length(&p[0], &self.unit, self.dpi).unwrap_or(0.),
                        value_length(&p[1], &self.unit, self.dpi).unwrap_or(0.),
                    )
                })
                .collect::<Vec<_>>();
            let bounds = paths
                .iter()
                .map(|p| p.0.bounding_box())
                .reduce(|a, b| a.union(b))
                .unwrap_or(Rect::new(0., 0., width, height));
            let w = bounds.width().max(1e-9);
            let h = bounds.height().max(1e-9);
            for (p, _, _) in &mut paths {
                *p = mapped(
                    p,
                    if kind == "wave" {
                        (wave / 16.).min(0.1)
                    } else {
                        0.1
                    },
                    |p| {
                        let x = p.x - bounds.x0;
                        let y = p.y - bounds.y0;
                        match kind.as_str() {
                            "wave" => Point::new(
                                p.x,
                                p.y + amp * (std::f64::consts::TAU * x / wave + phase).sin(),
                            ),
                            "arc" if sweep.abs() > 1e-8 => {
                                let radius = w / sweep;
                                let t = (x / w - 0.5) * sweep;
                                Point::new(
                                    bounds.x0 + w / 2. + (radius - y + h / 2.) * t.sin(),
                                    bounds.y0 + radius - (radius - y + h / 2.) * t.cos() + h / 2.,
                                )
                            }
                            "perspective" => {
                                let u = x / w;
                                let v = y / h;
                                let [p0, p1, p2, p3] =
                                    [corners[0], corners[1], corners[2], corners[3]];
                                let dx = p0.x - p1.x + p2.x - p3.x;
                                let dy = p0.y - p1.y + p2.y - p3.y;
                                let ax = p1.x - p2.x;
                                let ay = p1.y - p2.y;
                                let bx = p3.x - p2.x;
                                let by = p3.y - p2.y;
                                let det = ax * by - bx * ay;
                                let (g, k) = if dx.abs() + dy.abs() < 1e-12 {
                                    (0., 0.)
                                } else {
                                    ((dx * by - bx * dy) / det, (ax * dy - dx * ay) / det)
                                };
                                let den = g * u + k * v + 1.;
                                Point::new(
                                    ((p1.x - p0.x + g * p1.x) * u
                                        + (p3.x - p0.x + k * p3.x) * v
                                        + p0.x)
                                        / den,
                                    ((p1.y - p0.y + g * p1.y) * u
                                        + (p3.y - p0.y + k * p3.y) * v
                                        + p0.y)
                                        / den,
                                )
                            }
                            _ => p,
                        }
                    },
                )
                .ok_or_else(|| {
                    self.error(
                        "E_LIMIT",
                        "Text warp exceeds the vector subdivision budget",
                        l,
                    )
                })?;
            }
        }
        let mut out = Vec::new();
        let stroke = string(a, "text_stroke_color", "none");
        let sw = length(a, "text_stroke_width", 0., "pt", self.dpi);
        if let Some(ex) = a.get("extrude").and_then(|v| config(v, "text_extrude")) {
            let depth = length(&ex, "depth", 1., &self.unit, self.dpi);
            let theta = angle(&ex, "angle", 45.);
            let delta = kurbo::Vec2::new(depth * theta.cos(), depth * theta.sin());
            let mut side = BezPath::new();
            for (source, _, _) in &paths {
                let normalized = if source.area() < 0. {
                    source.reverse_subpaths()
                } else {
                    source.clone()
                };
                let p = &normalized;
                let mut first = Point::ORIGIN;
                let mut prev = first;
                kurbo::flatten(p.iter(), 0.002, |el| match el {
                    PathEl::MoveTo(p) => {
                        first = p;
                        prev = p
                    }
                    PathEl::LineTo(p) => {
                        let vertices = if (p - prev).cross(delta) >= 0. {
                            [prev, p, p + delta, prev + delta]
                        } else {
                            [prev + delta, p + delta, p, prev]
                        };
                        side.move_to(vertices[0]);
                        for q in &vertices[1..] {
                            side.line_to(*q);
                        }
                        side.close_path();
                        prev = p
                    }
                    PathEl::ClosePath => {
                        let vertices = if (first - prev).cross(delta) >= 0. {
                            [prev, first, first + delta, prev + delta]
                        } else {
                            [prev + delta, first + delta, first, prev]
                        };
                        side.move_to(vertices[0]);
                        for q in &vertices[1..] {
                            side.line_to(*q);
                        }
                        side.close_path()
                    }
                    _ => {}
                });
                side.extend((Affine::translate(delta) * p).elements().iter().copied());
            }
            out.push(crate::geometry::path_node(
                side,
                json!(string(&ex, "color", "#555555")),
                "none",
                0.,
            ));
        }
        let bounds = paths
            .iter()
            .map(|p| p.0.bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::ZERO);
        for (p, mut fill, style) in paths {
            if fill.is_object() {
                fill["paintBox"] = json!([bounds.x0, bounds.y0, bounds.width(), bounds.height()]);
            }
            out.push(crate::geometry::path_node(
                p,
                fill,
                jstr(&style, "text_stroke_color", &stroke),
                jnum(&style, "text_stroke_width", sw),
            ));
        }
        let bb = out
            .iter()
            .filter_map(|v| {
                BezPath::from_svg(jstr(v, "d", "")).ok().map(|p| {
                    p.bounding_box().inflate(
                        jnum(&v["strokeStyle"], "width", 0.) * 2.,
                        jnum(&v["strokeStyle"], "width", 0.) * 2.,
                    )
                })
            })
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::ZERO);
        if active {
            n["runs"] = json!(out);
            n["artText"] = json!(if jstr(n, "content", "").contains('\u{fffc}') {
                semantics
            } else {
                jstr(n, "content", "").to_owned()
            });
        }
        n["subjectBounds"] = json!({"x":bb.x0,"y":bb.y0,"width":bb.width(),"height":bb.height()});
        if self.art_cache.len() >= 32 {
            self.art_cache.clear();
        }
        self.art_cache.insert(
            cache_key,
            json!({"runs":n["runs"],"artText":n["artText"],"subjectBounds":n["subjectBounds"]}),
        );
        Ok(())
    }
}

/// Evaluate constructor literals only; no user functions, I/O, or arbitrary bindings.
pub(crate) fn css_config(s: &str, file: &str, l: Loc, unit: &str, dpi: f64) -> Result<V> {
    use crate::parser::{Expr, ExprKind};
    fn safe(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Number(..) | ExprKind::String(..) => true,
            ExprKind::Bool(_) => true,
            ExprKind::Ref(n) => {
                n.len() == 1 && (n[0] == "none" || crate::engine::predefined_value(&n[0]).is_some())
            }
            ExprKind::Unary(op, x) => matches!(op.as_str(), "+" | "-") && safe(x),
            ExprKind::List(xs) => xs.iter().all(safe),
            ExprKind::Call(names, args) => {
                names.len() == 1
                    && [
                        "shadow",
                        "glow",
                        "text_path",
                        "text_warp",
                        "text_extrude",
                        "path",
                        "line",
                        "polyline",
                        "arc",
                        "move_to",
                        "line_to",
                        "quad_to",
                        "cubic_to",
                        "arc_to",
                        "close",
                        "rgb",
                        "hsv",
                        "oklch",
                    ]
                    .contains(&names[0].as_str())
                    && args.iter().all(|(_, e)| safe(e))
            }
            _ => false,
        }
    }
    let expr = crate::parser::expression(s, file)?;
    if !safe(&expr) {
        return Err(crate::Diagnostic::new(
            "E_LCSS",
            "Effect styles require literal constructor expressions",
            file,
            l,
        ));
    }
    let mut engine = Engine::new(Host::default());
    engine.file = file.into();
    engine.unit = unit.into();
    engine.dpi = dpi;
    let value = engine.eval(&expr, &Environment::root())?;
    engine.objects.clear(); // Ownership moves to the returned literal configuration.
    Ok(value)
}

/// Conservative decorated occupancy; regular layout and anchor dimensions stay unchanged.
pub(crate) fn decorated_bounds(n: &Json) -> Option<Rect> {
    let active =
        n.get("artText").is_some() || n["effects"].as_array().is_some_and(|a| !a.is_empty());
    let mut b = if let Some(s) = n.get("subjectBounds") {
        Rect::new(
            jnum(s, "x", 0.),
            jnum(s, "y", 0.),
            jnum(s, "x", 0.) + jnum(s, "width", 0.),
            jnum(s, "y", 0.) + jnum(s, "height", 0.),
        )
    } else {
        Rect::new(0., 0., jnum(n, "width", 0.), jnum(n, "height", 0.))
    };
    let stroke_pad = jnum(&n["strokeStyle"], "width", 0.) * 2.;
    b = b.inflate(stroke_pad, stroke_pad);
    let mut any = active;
    if n["kind"] == "group" {
        let sx = jnum(n, "width", 0.) / jnum(n, "contentWidth", jnum(n, "width", 1.)).max(1e-12);
        let sy = jnum(n, "height", 0.) / jnum(n, "contentHeight", jnum(n, "height", 1.)).max(1e-12);
        for child in n["children"].as_array().into_iter().flatten() {
            if let Some(cb) = decorated_bounds(child) {
                any = true;
                b = b.union(Affine::scale_non_uniform(sx, sy).transform_rect_bbox(cb));
            }
        }
    }
    let sx = if n["kind"] == "group" {
        jnum(n, "width", 1.) / jnum(n, "contentWidth", 1.).max(1e-12)
    } else {
        1.
    };
    let sy = if n["kind"] == "group" {
        jnum(n, "height", 1.) / jnum(n, "contentHeight", 1.).max(1e-12)
    } else {
        1.
    };
    let subject = b;
    for e in n["effects"].as_array().into_iter().flatten() {
        if e["mode"] == "inner" {
            continue;
        }
        let pad = 4. * jnum(e, "blur", 1.) + jnum(e, "spread", 0.);
        let xy = e["offset"].as_array();
        let dx = xy.and_then(|v| v[0].as_f64()).unwrap_or(0.);
        let dy = xy.and_then(|v| v[1].as_f64()).unwrap_or(0.);
        b = b.union(subject.inflate(pad * sx, pad * sy) + kurbo::Vec2::new(dx * sx, dy * sy));
    }
    any.then(|| crate::geometry::node_transform(n).transform_rect_bbox(b))
}
