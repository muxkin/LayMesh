pub use crate::geometry_recipe::resolved_path;
use crate::{Loc, Result, engine::Engine, model::*};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{Affine, BezPath, PathEl, Point, Shape};
use serde_json::{Value as Json, json};
use std::{cell::RefCell, rc::Rc};
#[path = "geometry_nearest.rs"]
mod nearest;

pub fn path_node(path: BezPath, fill: Json, stroke: &str, width: f64) -> Json {
    let b = path.bounding_box();
    let mut n = base("path", b.width(), b.height());
    n["d"] = json!(path.to_svg());
    n["fill"] = fill;
    n["fillRule"] = json!("nonzero");
    n["strokeStyle"] = json!({"color":stroke,"width":width,"dash":[],"dashOffset":0,"cap":"butt","join":"miter","miterLimit":4});
    n
}
pub(crate) fn points(path: &BezPath, tolerance: f64) -> Vec<Vec<[f64; 2]>> {
    let mut contours = vec![];
    let mut p = vec![];
    kurbo::flatten(path.iter(), tolerance, |e| match e {
        PathEl::MoveTo(q) => {
            if !p.is_empty() {
                contours.push(std::mem::take(&mut p));
            }
            p.push([q.x, q.y]);
        }
        PathEl::LineTo(q) => p.push([q.x, q.y]),
        PathEl::ClosePath => {
            if !p.is_empty() {
                contours.push(std::mem::take(&mut p));
            }
        }
        _ => {}
    });
    if !p.is_empty() {
        contours.push(p);
    }
    contours
}
fn from_contours(cs: Vec<Vec<Vec<[f64; 2]>>>) -> BezPath {
    let mut p = BezPath::new();
    for shape in cs {
        for c in shape {
            if let Some(q) = c.first() {
                p.move_to((q[0], q[1]));
                for q in &c[1..] {
                    p.line_to((q[0], q[1]));
                }
                p.close_path();
            }
        }
    }
    p
}
fn boolean(a: &BezPath, b: &BezPath, op: OverlayRule) -> BezPath {
    from_contours(points(a, 0.0001).overlay(&points(b, 0.0001), op, FillRule::EvenOdd))
}
pub fn union(a: &BezPath, b: &BezPath) -> BezPath {
    boolean(a, b, OverlayRule::Union)
}
pub(crate) fn difference(a: &BezPath, b: &BezPath) -> BezPath {
    boolean(a, b, OverlayRule::Difference)
}
pub(crate) fn intersection(a: &BezPath, b: &BezPath) -> BezPath {
    boolean(a, b, OverlayRule::Intersect)
}
pub fn compound_outline(path: &BezPath, s: &Json) -> BezPath {
    compound_outline_transformed(path, s, Affine::IDENTITY)
}
/// Construct stroke bands in final physical coordinates. Applying the complete
/// ancestor transform before polygon operations keeps subdivision errors from
/// being magnified by nested groups or nonuniform path placement.
pub fn compound_outline_transformed(path: &BezPath, s: &Json, transform: Affine) -> BezPath {
    let outer = outline_transformed(path, s, transform);
    let kind = jstr(s, "compound", "single");
    if kind == "single" {
        return outer;
    }
    let mut narrow = s.clone();
    narrow["width"] = json!(jnum(s, "width", 0.) * if kind == "double" { 1. / 3. } else { 0.6 });
    let bands = difference(&outer, &outline_transformed(path, &narrow, transform));
    if kind == "triple" {
        narrow["width"] = json!(jnum(s, "width", 0.) * 0.2);
        union(&bands, &outline_transformed(path, &narrow, transform))
    } else {
        bands
    }
}
pub fn outline(path: &BezPath, s: &Json) -> BezPath {
    outline_transformed(path, s, Affine::IDENTITY)
}
pub fn outline_transformed(path: &BezPath, s: &Json, transform: Affine) -> BezPath {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    // Largest singular value: a conservative local tolerance also covers
    // rotations between different nonuniform scales (which introduce shear).
    let magnification = ((a + d).hypot(b - c) + (a - d).hypot(b + c)) / 2.;
    if magnification <= 1e-30 || jnum(s, "width", 0.) <= 0. {
        return BezPath::new();
    }
    // kurbo's dash splitter has a fixed absolute tolerance. Normalize to the
    // largest physical scale before splitting so that tolerance cannot grow.
    let path = Affine::scale(magnification) * path.clone();
    let transform = transform * Affine::scale(1. / magnification);
    let tolerance = 0.0001;
    let mut style = kurbo::Stroke::new(jnum(s, "width", 0.) * magnification);
    style.start_cap = match jstr(s, "startCap", jstr(s, "cap", "butt")) {
        "round" => kurbo::Cap::Round,
        "square" => kurbo::Cap::Square,
        _ => kurbo::Cap::Butt,
    };
    style.end_cap = match jstr(s, "endCap", jstr(s, "cap", "butt")) {
        "round" => kurbo::Cap::Round,
        "square" => kurbo::Cap::Square,
        _ => kurbo::Cap::Butt,
    };
    style.join = match jstr(s, "join", "miter") {
        "round" => kurbo::Join::Round,
        "bevel" => kurbo::Join::Bevel,
        _ => kurbo::Join::Miter,
    };
    style.miter_limit = jnum(s, "miterLimit", 4.);
    style.dash_pattern = s["dash"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Json::as_f64)
        .map(|n| n * magnification)
        .collect();
    style.dash_offset = jnum(s, "dashOffset", 0.) * magnification;
    let path = if style.dash_pattern.is_empty() {
        path
    } else {
        let dashed: BezPath =
            kurbo::dash(path.iter(), style.dash_offset, &style.dash_pattern).collect();
        style.dash_pattern.clear();
        dashed
    };
    // kurbo 0.13 hardcodes relative 1e-3 round-cap/join arc precision, even
    // when stroke() receives a stricter tolerance. Construct those disks with
    // Circle::to_path's actual physical tolerance and union with bevel/butt
    // stroke bodies. This is the same round stroke region without that limit.
    let round_start = style.start_cap == kurbo::Cap::Round;
    let round_end = style.end_cap == kurbo::Cap::Round;
    let round_caps = round_start || round_end;
    let round_joins = style.join == kurbo::Join::Round;
    if round_caps {
        if round_start {
            style.start_cap = kurbo::Cap::Butt;
        }
        if round_end {
            style.end_cap = kurbo::Cap::Butt;
        }
    }
    if round_joins {
        style.join = kurbo::Join::Bevel;
    }
    let mut centers = Vec::<Point>::new();
    if round_caps || round_joins {
        let mut vertices = Vec::<Point>::new();
        let mut curved = false;
        let collect =
            |vertices: &mut Vec<Point>, closed: bool, curved: bool, centers: &mut Vec<Point>| {
                if vertices.len() >= 2 {
                    if round_joins {
                        let range = if closed {
                            0..vertices.len()
                        } else {
                            1..vertices.len() - 1
                        };
                        for i in range {
                            let before =
                                vertices[i] - vertices[(i + vertices.len() - 1) % vertices.len()];
                            let after = vertices[(i + 1) % vertices.len()] - vertices[i];
                            let lengths = before.hypot() * after.hypot();
                            let cosine = if lengths > 0. {
                                (before.dot(after) / lengths).clamp(-1., 1.)
                            } else {
                                -1.
                            };
                            // On a finely flattened polygon, bevel and round joins
                            // differ only by this sagitta. Tiny turns need no disk:
                            // this bounds their physical error while avoiding many
                            // thousands of mutually intersecting overlapping disks.
                            let sagitta = style.width / 2. * (1. - ((1. + cosine) / 2.).sqrt());
                            if curved || sagitta > tolerance / 2. {
                                centers.push(vertices[i]);
                            }
                        }
                    }
                    if round_caps && !closed {
                        if round_start {
                            centers.push(vertices[0]);
                        }
                        if round_end {
                            centers.push(*vertices.last().unwrap());
                        }
                    }
                }
                vertices.clear();
            };
        for el in path.iter() {
            match el {
                PathEl::MoveTo(p) => {
                    collect(&mut vertices, false, curved, &mut centers);
                    curved = false;
                    vertices.push(p);
                }
                PathEl::LineTo(p) => vertices.push(p),
                PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                    curved = true;
                    vertices.push(p);
                }
                PathEl::ClosePath => {
                    collect(&mut vertices, true, curved, &mut centers);
                    curved = false;
                }
            }
        }
        collect(&mut vertices, false, curved, &mut centers);
    }
    let stroked = kurbo::stroke(
        path.iter(),
        &style,
        &kurbo::StrokeOpts::default(),
        tolerance,
    );
    let mut disks = Vec::<Vec<[f64; 2]>>::new();
    for center in centers {
        let circle = transform * kurbo::Circle::new(center, style.width / 2.).to_path(tolerance);
        disks.extend(points(&circle, tolerance));
    }
    // All disks have the same winding. Keep the stroke and disk winding
    // accumulators separate, then union them once; a union for every vertex
    // becomes quadratic during morphological closing of flattened contours.
    from_contours(points(&(transform * stroked), tolerance).overlay(
        &disks,
        OverlayRule::Union,
        FillRule::NonZero,
    ))
}
pub fn node_transform(n: &Json) -> Affine {
    let (w, h) = (jnum(n, "width", 0.), jnum(n, "height", 0.));
    Affine::translate((jnum(n, "x", 0.) + w / 2., jnum(n, "y", 0.) + h / 2.))
        * Affine::rotate(jnum(n, "rotation", 0.).to_radians())
        * Affine::translate((-w / 2., -h / 2.))
}
pub fn visible(n: &Json) -> BezPath {
    visible_transformed(n, Affine::IDENTITY)
}
pub fn visible_transformed(n: &Json, parent: Affine) -> BezPath {
    let transform = parent * node_transform(n);
    let mut p = BezPath::new();
    if n["endpointRecipe"].is_object() {
        return crate::endpoints::layers(n, transform)
            .unwrap_or_default()
            .iter()
            .fold(p, |p, q| union(&p, &q.path));
    }
    if jstr(n, "kind", "") == "group" {
        let sx = jnum(n, "width", 1.) / jnum(n, "contentWidth", 1.).max(1e-12);
        let sy = jnum(n, "height", 1.) / jnum(n, "contentHeight", 1.).max(1e-12);
        for c in n["children"].as_array().into_iter().flatten() {
            let q = visible_transformed(c, transform * Affine::scale_non_uniform(sx, sy));
            p = union(&p, &q);
        }
    } else if let Ok(q) = BezPath::from_svg(jstr(n, "d", "")) {
        let sx = if jnum(n, "intrinsicWidth", 0.) > 0. {
            jnum(n, "width", 1.) / jnum(n, "intrinsicWidth", 1.)
        } else {
            1.
        };
        let sy = if jnum(n, "intrinsicHeight", 0.) > 0. {
            jnum(n, "height", 1.) / jnum(n, "intrinsicHeight", 1.)
        } else {
            1.
        };
        let scale = transform * Affine::scale_non_uniform(sx, sy);
        let q = if n["geometryRecipe"].is_object() && scale.determinant().abs() > 1e-30 {
            scale.inverse() * resolved_path(n, scale)
        } else {
            q
        };
        if n["fill"] != "none" {
            p = from_contours(points(&(scale * q.clone()), 0.0001).overlay(
                &Vec::<Vec<[f64; 2]>>::new(),
                OverlayRule::Union,
                if jstr(n, "fillRule", "nonzero") == "evenodd" {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                },
            ));
        }
        if jnum(&n["strokeStyle"], "width", 0.) > 0. && n["strokeStyle"]["color"] != "none" {
            let stroke = compound_outline_transformed(&q, &n["strokeStyle"], scale);
            p = union(&p, &stroke);
        }
    }
    p
}
impl Engine {
    fn retain_replayed_name(&self, original: usize, replayed: &V, container: &Rc<RefCell<Object>>) {
        let Some(original) = self.objects.get(&original) else {
            return;
        };
        let name = original
            .borrow()
            .node
            .as_ref()
            .map(|node| node["id"].clone());
        let Some(name) = name else { return };
        let object = replayed.object().unwrap();
        let mut object = object.borrow_mut();
        let Some(node) = object.node.as_mut() else {
            return;
        };
        let old = node["id"].clone();
        node["id"] = name.clone();
        for node in &mut container.borrow_mut().nodes {
            if node["id"] == old {
                node["id"] = name.clone();
            }
        }
    }
    fn replay_anchor(
        &self,
        anchor: &V,
        owner: usize,
        instances: &std::collections::BTreeMap<usize, V>,
        loc: Loc,
    ) -> Result<V> {
        if let V::Geometry(query) = anchor {
            return Ok(V::Geometry(Rc::new(query.rebound(instances))));
        }
        let V::Anchor {
            x, y, reference, ..
        } = anchor
        else {
            return Err(self.error("E_ANCHOR", "target 需要锚点", loc));
        };
        if let Some(reference) = reference {
            let instance = instances
                .get(&reference.instance)
                .ok_or_else(|| self.error("E_ANCHOR", "锚点引用的实例尚未重放", loc))?;
            self.member(instance.clone(), &reference.name, loc)
        } else {
            // Container anchors and data-coordinate anchors are snapshots,
            // matching their original language semantics.
            Ok(V::Anchor {
                owner,
                x: *x,
                y: *y,
                reference: None,
            })
        }
    }
    pub(crate) fn text_spec(&mut self, a: &Args, file: &str, l: Loc) -> Result<Json> {
        let mut spec = args_json(a);
        spec["file"] = json!(file);
        spec["loc"] = json!(l);
        if let Some(V::Text(value, raw)) = a.get("content") {
            let origins = &spec["__arg_locations"];
            let at = origins.get("content").or_else(|| origins.get("_0"));
            if let Some(at) = at {
                spec["content"] =
                    json!({"value":value,"raw":raw,"file":at["file"],"loc":at["loc"]});
            }
        }
        for key in ["font_size", "line_height"] {
            if let Some(v) = a.get(key) {
                spec[key] = json!(value_length(v, "pt", self.dpi).unwrap_or(10. * PT));
            }
        }
        if let Some(v) = a.get("font_family") {
            self.fonts
                .load_requested(v, &self.host, file, l, &mut self.warnings);
        }
        if let Some(V::List(spans)) = a.get("spans") {
            let mut output = vec![];
            for span in spans {
                if let Some(o) = span.object() {
                    let o = o.borrow().clone();
                    let mut explicit = o.args.clone();
                    explicit.insert("__style_file".into(), V::text(&o.file));
                    let a = self.styled(&o.kind, &explicit, a)?;
                    let mut spec = self.text_spec(&a, &o.file, o.loc)?;
                    spec["kind"] = json!(o.kind);
                    output.push(spec);
                } else {
                    output.push(span.json());
                }
            }
            spec["spans"] = json!(output);
        }
        Ok(spec)
    }
    pub fn add(&mut self, owner: Rc<RefCell<Object>>, pos: Vec<V>, a: Args, l: Loc) -> Result<V> {
        if owner.borrow().sealed {
            return Err(self.error("E_GROUP", "组合放置后不能修改", l));
        }
        let mut a = a;
        self.validate_args("add", &mut a, &pos, l)?;
        if let Some(v) = a.get("size") {
            for (dimension, v) in v.list().iter().enumerate() {
                if let Some(x) = value_length(v, &self.unit, self.dpi) {
                    if x <= 0. {
                        return Err(self.error(
                            "E_VALUE",
                            format!(
                                "放置{}必须大于零",
                                if dimension == 0 { "宽度" } else { "高度" }
                            ),
                            l,
                        ));
                    }
                }
            }
        }
        if pos.len() != 1 {
            return Err(self.error("E_ARG", "add 需要一个素材", l));
        }
        let def = pos[0]
            .object()
            .ok_or_else(|| self.error("E_TYPE", "add 需要素材", l))?;
        if Rc::ptr_eq(&owner, &def) {
            return Err(self.error("E_GROUP", "组合不能包含自身", l));
        }
        if owner.borrow().kind == "group" {
            let depth = if def.borrow().kind == "group" {
                1. + num(&def.borrow().args, "__depth", 0.)
            } else {
                0.
            };
            if depth > 128. {
                return Err(self.error("E_LIMIT", "组嵌套深度超过 128", l));
            }
            let depth = depth.max(num(&owner.borrow().args, "__depth", 0.));
            owner
                .borrow_mut()
                .args
                .insert("__depth".into(), V::num(depth));
        }
        self.steps += 1;
        if self.steps > 1_000_000 {
            return Err(self.error("E_LIMIT", "执行步骤超过 1000000", l));
        }
        let parent = {
            let owner = owner.borrow();
            if owner.args.contains_key("__style_type") {
                owner.args.clone()
            } else {
                let mut args = owner.args.clone();
                args.insert("__style_file".into(), V::text(&owner.file));
                self.styled(&owner.kind, &args, &Args::new())?
            }
        };
        let mut n = self.materialize(def.clone(), &a, &parent)?;
        if !matches!(
            string(&a, "offset_space", "container").as_str(),
            "container" | "target"
        ) {
            return Err(self.error("E_ARG", "offset_space 须为 container/target", l));
        }
        let mut xy = pair(&a, "offset", [0., 0.], &self.unit, self.dpi);
        let target = match a.get("target") {
            Some(value @ V::Geometry(_)) => {
                let point = self.geometry_anchor(value, None, l)?;
                if point.owner != owner.borrow().id {
                    return Err(self.error("E_LAYOUT", "target 必须位于同一画布或组", l));
                }
                if string(&a, "offset_space", "container") == "target" {
                    let t = point.tangent(self, l)?;
                    xy = [xy[0] * t.x + xy[1] * t.y, xy[0] * t.y - xy[1] * t.x];
                }
                [point.point.x, point.point.y]
            }
            Some(V::Anchor {
                owner: target,
                x,
                y,
                ..
            }) => {
                if string(&a, "offset_space", "container") == "target" {
                    return Err(self.error(
                        "E_ANCHOR_DIRECTION",
                        "目标没有路径方向，请使用 target=instance.path...",
                        l,
                    ));
                }
                if *target != owner.borrow().id {
                    return Err(self.error("E_LAYOUT", "target 必须位于同一画布或组", l));
                }
                [*x, *y]
            }
            None => {
                if string(&a, "offset_space", "container") == "target" {
                    return Err(self.error(
                        "E_ANCHOR_DIRECTION",
                        "offset_space=target 需要路径目标",
                        l,
                    ));
                }
                [0., 0.]
            }
            _ => return Err(self.error("E_ANCHOR", "target 需要锚点", l)),
        };
        if owner.borrow().kind == "group" {
            let mut record = a.clone();
            record.insert("__material".into(), V::Object(def.clone()));
            record.insert("__result_id".into(), V::num((self.serial + 1) as f64));
            // `target` itself retains the instance and named anchor. Never
            // infer that relation from coordinates, which may coincide.
            owner.borrow_mut().layers.push(("__add".into(), record));
        }
        let rot = num(&a, "rotation", 0.);
        n["rotation"] = json!(rot);
        let opacity = num(&a, "opacity", 1.);
        if !(0.0..=1.0).contains(&opacity) {
            return Err(self.error("E_ARG", "opacity 必须在 0–1 之间", l));
        }
        n["opacity"] = json!(opacity * jnum(&n, "opacity", 1.));
        let anc = string(&a, "anchor", "top_left");
        let p = if let Some(V::Geometry(query)) = a.get("anchor") {
            if !query.is_self_contained() {
                return Err(self.error("E_ANCHOR", "anchor 选择器只能引用 self", l));
            }
            let point =
                self.geometry_anchor(a.get("anchor").unwrap(), Some((&n, owner.borrow().id)), l)?;
            [point.point.x, point.point.y]
        } else if matches!(anc.as_str(), "start" | "end") {
            let q = n["endpoints"][if anc == "start" { 0 } else { 1 }]
                .as_array()
                .ok_or_else(|| self.error("E_LAYOUT", "start/end 需要线段或箭头实例", l))?;
            let q = node_transform(&n) * Point::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap());
            [q.x, q.y]
        } else if let Some(name) = anc.strip_prefix("plot_") {
            let b = &n["plotBounds"];
            let f = anchor(name)
                .filter(|_| !b.is_null())
                .ok_or_else(|| self.error("E_LAYOUT", "plot 锚点需要原生图表实例", l))?;
            crate::plot::local_to_parent(
                &n,
                [
                    jnum(b, "x", 0.) + f[0] * jnum(b, "width", 0.),
                    jnum(b, "y", 0.) + f[1] * jnum(b, "height", 0.),
                ],
            )
        } else {
            if anchor(&anc).is_none() {
                return Err(self.error("E_LAYOUT", "未知 anchor", l));
            }
            node_anchor(&n, &anc)
        };
        n["x"] = json!(target[0] + xy[0] - p[0]);
        n["y"] = json!(target[1] + xy[1] - p[1]);
        self.serial += 1;
        n["id"] = json!(format!("instance-{}", self.serial));
        let mut instance = Object {
            kind: "instance".into(),
            args: a.clone(),
            file: self.file.clone(),
            loc: l,
            id: self.serial,
            nodes: vec![],
            layers: vec![],
            sealed: true,
            width: jnum(&n, "width", 0.),
            height: jnum(&n, "height", 0.),
            node: Some(n.clone()),
            parent: owner.borrow().id,
        };
        instance.args.insert("__material".into(), V::Object(def));
        owner.borrow_mut().nodes.push(n.clone());
        let mut o = owner.borrow_mut();
        if o.kind != "canvas" {
            o.width = o.width.max(jnum(&n, "x", 0.) + jnum(&n, "width", 0.));
            o.height = o.height.max(jnum(&n, "y", 0.) + jnum(&n, "height", 0.));
        }
        drop(o);
        let instance = Rc::new(RefCell::new(instance));
        self.objects.insert(instance.borrow().id, instance.clone());
        Ok(V::Object(instance))
    }
    pub fn materialize(
        &mut self,
        def: Rc<RefCell<Object>>,
        placement: &Args,
        parent: &Args,
    ) -> Result<Json> {
        let o = def.borrow().clone();
        let old = std::mem::replace(&mut self.file, o.file.clone());
        let mut explicit = o.args.clone();
        if let Some(class) = placement.get("class") {
            explicit.insert("class".into(), class.clone());
        }
        let mut a = self.styled(&o.kind, &explicit, parent)?;
        for (k, v) in placement {
            if !matches!(
                k.as_str(),
                "anchor"
                    | "target"
                    | "offset"
                    | "rotation"
                    | "opacity"
                    | "__arg_locations"
                    | "__call_origin"
            ) {
                a.insert(k.clone(), v.clone());
            }
        }
        if matches!(o.kind.as_str(), "rect" | "ellipse") && placement.contains_key("size") {
            let original = pair(&o.args, "size", [o.width, o.height], &self.unit, self.dpi);
            let size = pair(&a, "size", original, &self.unit, self.dpi);
            a.insert(
                "size".into(),
                V::List(size.into_iter().map(V::mm).collect()),
            );
        }
        if let Some(origin) = placement.get("__call_origin") {
            a.insert("__placement_origin".into(), origin.clone());
        }
        let result = self.make_node(&o, &a);
        self.file = old;
        if matches!(o.kind.as_str(), "group" | "plot") {
            def.borrow_mut().sealed = true;
        }
        result
    }
    fn make_node(&mut self, o: &Object, a: &Args) -> Result<Json> {
        let l = o.loc;
        let size = pair(a, "size", [f64::NAN, f64::NAN], &self.unit, self.dpi);
        let mut n = match o.kind.as_str() {
            "text" | "span" | "formula" => {
                let mut spec = self.text_spec(a, &o.file, l)?;
                spec["kind"] = json!(o.kind);
                for key in ["font_size", "line_height"] {
                    if let Some(v) = a.get(key) {
                        spec[key] = json!(value_length(v, "pt", self.dpi).unwrap_or(10. * PT));
                    }
                }
                if let Some(v) = a.get("font_family") {
                    self.fonts
                        .load_requested(v, &self.host, &o.file, l, &mut self.warnings);
                }
                crate::text::layout_text(
                    &spec,
                    if size[0].is_finite() {
                        Some(size[0])
                    } else {
                        None
                    },
                    &mut self.fonts,
                    &mut self.warnings,
                    &o.file,
                    l,
                )?
            }
            "image" => {
                let src = string(a, "src", "");
                let path = resolve(&o.file, &src);
                let data = self.host.read(&path, &o.file, l)?;
                let mut asset = crate::assets::load(&data, &path, &o.file, l)?;
                let (iw, ih) = (jnum(&asset, "width", 1.), jnum(&asset, "height", 1.));
                let crop = if let Some(c) = a.get("crop").and_then(V::object) {
                    let args = &c.borrow().args;
                    let offset = nums(args, "offset");
                    let size = nums(args, "size");
                    if offset.len() != 2
                        || size.len() != 2
                        || offset.iter().any(|x| *x < 0.)
                        || size.iter().any(|x| *x <= 0.)
                        || offset[0] + size[0] > 1. + 1e-10
                        || offset[1] + size[1] > 1. + 1e-10
                    {
                        return Err(self.error("E_CROP", "crop 使用 0–1 范围的归一化矩形", l));
                    }
                    Some(
                        json!({"x":offset[0]*iw,"y":offset[1]*ih,"width":size[0]*iw,"height":size[1]*ih}),
                    )
                } else {
                    None
                };
                if let Some(c) = crop {
                    if asset["mime"] == "image/svg+xml" {
                        return Err(self.error("E_IMAGE", "SVG 不支持像素裁剪", l));
                    }
                    asset = crate::assets::crop_raster(
                        jstr(&asset, "data", ""),
                        [
                            jnum(&c, "x", 0.),
                            jnum(&c, "y", 0.),
                            jnum(&c, "width", iw),
                            jnum(&c, "height", ih),
                        ],
                        &o.file,
                        l,
                    )?;
                }
                let (cw, ch) = (jnum(&asset, "width", iw), jnum(&asset, "height", ih));
                let mut w = size[0];
                let mut h = size[1];
                if w.is_nan() && h.is_nan() {
                    w = cw * 25.4 / self.dpi;
                    h = ch * 25.4 / self.dpi;
                } else if w.is_nan() {
                    w = h * cw / ch;
                } else if h.is_nan() {
                    h = w * ch / cw;
                }
                let mut n = base("image", w, h);
                n["data"] = asset["data"].clone();
                n["mime"] = asset["mime"].clone();
                n["intrinsicWidth"] = json!(cw);
                n["intrinsicHeight"] = json!(ch);
                n["fit"] = json!(string(a, "fit", "contain"));
                n
            }
            "group" => {
                let style_changed = [
                    "font_family",
                    "font_size",
                    "font_weight",
                    "font_style",
                    "color",
                    "line_height",
                ]
                .iter()
                .any(|k| a.get(*k).map(V::json) != o.args.get(*k).map(V::json));
                let (_, _, mut children) =
                    if o.layers.is_empty() || (!style_changed && self.styles.is_empty()) {
                        (o.width, o.height, o.nodes.clone())
                    } else {
                        let temp = self.object("group", a.clone(), o.loc).object().unwrap();
                        let mut replayed: std::collections::BTreeMap<usize, V> =
                            std::collections::BTreeMap::new();
                        for (operation, record) in &o.layers {
                            let mut args = record.clone();
                            let id = args
                                .remove("__result_id")
                                .and_then(|v| v.number())
                                .unwrap_or(0.) as usize;
                            if operation == "__fuse" {
                                let left = args.remove("__left").and_then(|v| v.number()).unwrap()
                                    as usize;
                                let right = args.remove("__right").and_then(|v| v.number()).unwrap()
                                    as usize;
                                let inputs = vec![
                                    replayed.get(&left).cloned().ok_or_else(|| {
                                        self.error("E_FUSE", "无法重放融合对象", l)
                                    })?,
                                    replayed.get(&right).cloned().ok_or_else(|| {
                                        self.error("E_FUSE", "无法重放融合对象", l)
                                    })?,
                                ];
                                if let Some(V::List(points)) = args.get("points") {
                                    let anchors = points
                                        .iter()
                                        .map(|point| {
                                            self.replay_anchor(
                                                point,
                                                temp.borrow().id,
                                                &replayed,
                                                o.loc,
                                            )
                                        })
                                        .collect::<Result<Vec<_>>>()?;
                                    args.insert("points".into(), V::List(anchors));
                                }
                                let result = self.fuse(temp.clone(), inputs, args, o.loc)?;
                                self.retain_replayed_name(id, &result, &temp);
                                replayed.insert(id, result);
                                continue;
                            }
                            let material = args.remove("__material").unwrap();
                            args = args
                                .iter()
                                .map(|(key, v)| {
                                    Ok((key.clone(), self.replay_query_value(v, &replayed, o.loc)?))
                                })
                                .collect::<Result<_>>()?;
                            if let Some(anchor) = args.get("target") {
                                let anchor =
                                    self.replay_anchor(anchor, temp.borrow().id, &replayed, o.loc)?;
                                args.insert("target".into(), anchor);
                            }
                            let result = self.add(temp.clone(), vec![material], args, o.loc)?;
                            self.retain_replayed_name(id, &result, &temp);
                            replayed.insert(id, result);
                        }
                        let temp = temp.borrow();
                        (temp.width, temp.height, temp.nodes.clone())
                    };
                if children.is_empty() {
                    return Err(self.error("E_GROUP", "不能放置空组", l));
                }
                let min_x = children
                    .iter()
                    .map(|n| node_anchor(n, "top_left")[0])
                    .fold(f64::INFINITY, f64::min);
                let min_y = children
                    .iter()
                    .map(|n| node_anchor(n, "top_left")[1])
                    .fold(f64::INFINITY, f64::min);
                let max_x = children
                    .iter()
                    .map(|n| node_anchor(n, "bottom_right")[0])
                    .fold(f64::NEG_INFINITY, f64::max);
                let max_y = children
                    .iter()
                    .map(|n| node_anchor(n, "bottom_right")[1])
                    .fold(f64::NEG_INFINITY, f64::max);
                let (content_width, content_height) = (max_x - min_x, max_y - min_y);
                for n in &mut children {
                    n["x"] = json!(jnum(n, "x", 0.) - min_x);
                    n["y"] = json!(jnum(n, "y", 0.) - min_y);
                }
                let (mut w, mut h) = (content_width, content_height);
                if size[0].is_finite() {
                    w = size[0];
                }
                if size[1].is_finite() {
                    h = size[1];
                }
                let mut n = base("group", w, h);
                n["contentWidth"] = json!(content_width);
                n["contentHeight"] = json!(content_height);
                n["children"] = json!(children);
                n
            }
            "plot" => return crate::plot::layout(self, o, a),
            "legend" | "colorbar" => return crate::plot::decoration(self, o, a),
            _ => self.shape(o, a, size)?,
        };
        if jnum(&n, "width", 0.) < 0. || jnum(&n, "height", 0.) < 0. {
            return Err(self.error("E_LAYOUT", "尺寸不能为负", l));
        }
        let padding = a
            .get("padding")
            .map(|v| {
                let v = if let V::List(v) = v {
                    v.clone()
                } else {
                    vec![v.clone()]
                };
                let v: Vec<_> = v
                    .iter()
                    .map(|v| value_length(v, &self.unit, self.dpi).unwrap_or(0.))
                    .collect();
                match v.as_slice() {
                    [a] => [*a; 4],
                    [v, h] => [*v, *h, *v, *h],
                    [t, h, b] => [*t, *h, *b, *h],
                    [t, r, b, l] => [*t, *r, *b, *l],
                    _ => [0.; 4],
                }
            })
            .unwrap_or([0.; 4]);
        if (a.contains_key("background")
            || a.contains_key("border_color")
            || a.contains_key("border_width")
            || a.contains_key("border_radius")
            || padding.iter().any(|x| *x != 0.))
            && matches!(
                o.kind.as_str(),
                "image" | "text" | "formula" | "group" | "plot" | "legend" | "colorbar"
            )
        {
            let (w, h) = (jnum(&n, "width", 0.), jnum(&n, "height", 0.));
            n["x"] = json!(padding[3]);
            n["y"] = json!(padding[0]);
            let mut g = base(
                "group",
                w + padding[1] + padding[3],
                h + padding[0] + padding[2],
            );
            g["contentWidth"] = g["width"].clone();
            g["contentHeight"] = g["height"].clone();
            let bg = kurbo::RoundedRect::new(
                0.,
                0.,
                jnum(&g, "width", 0.),
                jnum(&g, "height", 0.),
                length(a, "border_radius", 0., &self.unit, self.dpi),
            )
            .to_path(0.0001);
            let mut back = path_node(
                bg,
                a.get("background").map(V::json).unwrap_or(json!("none")),
                &string(a, "border_color", "none"),
                length(a, "border_width", 0., "pt", self.dpi),
            );
            back["strokeStyle"] =
                self.stroke(a, false, length(a, "border_width", 0., "pt", self.dpi));
            g["children"] = json!([back, n]);
            n = g;
        }
        n["opacity"] = json!(num(a, "opacity", 1.));
        Ok(n)
    }
    pub fn shape(&self, o: &Object, a: &Args, size: [f64; 2]) -> Result<Json> {
        let l = o.loc;
        let g = |key: &str, d: f64| length(a, key, d, &self.unit, self.dpi);
        let mut p = BezPath::new();
        let mut fixed = None;
        let mut recipe = Json::Null;
        match o.kind.as_str() {
            "rect" | "ellipse" => {
                let (w, h) = (size[0], size[1]);
                if !w.is_finite() || !h.is_finite() || w <= 0. || h <= 0. {
                    return Err(self.error("E_LAYOUT", "size 需要正尺寸", l));
                }
                p = if o.kind == "rect" {
                    kurbo::RoundedRect::new(0., 0., w, h, g("border_radius", 0.)).to_path(0.0001)
                } else {
                    kurbo::Ellipse::new((w / 2., h / 2.), (w / 2., h / 2.), 0.).to_path(0.0001)
                };
                fixed = Some((w, h));
                recipe = json!({"kind":o.kind,"width":w,"height":h,"radius":g("border_radius",0.)});
            }
            "line" => {
                let (dx, dy) = crate::endpoints::line_vector(self, a, l)?;
                let (w, h) = (dx.abs(), dy.abs());
                let start = Point::new(if dx < 0. { w } else { 0. }, if dy < 0. { h } else { 0. });
                let end = Point::new(if dx < 0. { 0. } else { w }, if dy < 0. { 0. } else { h });
                p.move_to(start);
                p.line_to(end);
                fixed = Some((w, h));
            }
            "polygon" | "polyline" => {
                let vs = array(a, "points");
                if vs.len() < 2 {
                    return Err(self.error("E_PATH", "至少需要两个点", l));
                }
                for (i, v) in vs.iter().enumerate() {
                    let v = v.list();
                    if v.len() != 2 {
                        return Err(self.error("E_PATH", "点需要两个坐标", l));
                    }
                    let q = (self.len(&v[0], l)?, self.len(&v[1], l)?);
                    if i == 0 { p.move_to(q) } else { p.line_to(q) }
                }
                if o.kind == "polygon" {
                    p.close_path();
                }
            }
            "star" => {
                let count = num(a, "points", 5.) as usize;
                if !(3..=1000).contains(&count) {
                    return Err(self.error("E_PATH", "星形顶点数须为 3–1000", l));
                }
                let (outer, inner) = (g("outer_radius", 10.), g("inner_radius", 5.));
                let rot = num(a, "rotation", 0.).to_radians() - std::f64::consts::FRAC_PI_2;
                for i in 0..count * 2 {
                    let r = if i % 2 == 0 { outer } else { inner };
                    let theta = rot + i as f64 * std::f64::consts::PI / count as f64;
                    let q = (outer + r * theta.cos(), outer + r * theta.sin());
                    if i == 0 { p.move_to(q) } else { p.line_to(q) }
                }
                p.close_path();
            }
            "ring" => {
                let (r, inner) = (g("outer_radius", 10.), g("inner_radius", 5.));
                if inner < 0. || inner >= r {
                    return Err(self.error("E_PATH", "圆环半径无效", l));
                }
                p = kurbo::Circle::new((r, r), r).to_path(0.0001);
                p.extend(kurbo::Circle::new((r, r), inner).to_path(0.0001).iter());
                recipe = json!({"kind":"ring","radius":r,"inner":inner});
            }
            "arc" | "sector" => {
                let r = g("radius", 10.);
                let start = num(a, "start", 0.).to_radians();
                let sweep = (num(a, "end", 360.) - num(a, "start", 0.)).to_radians();
                let arc = kurbo::Arc::new((r, r), (r, r), start, sweep, 0.);
                recipe = json!({"kind":o.kind,"radius":r,"start":start,"sweep":sweep});
                p = arc.to_path(0.0001);
                if o.kind == "sector" {
                    p.line_to((r, r));
                    p.close_path();
                }
            }
            "path" => {
                let cmds = array(a, "commands");
                let mut commands = vec![];
                if cmds.len() > 10_000 {
                    return Err(self.error("E_LIMIT", "路径命令超过上限", l));
                }
                for cmd in cmds {
                    let o = cmd
                        .object()
                        .ok_or_else(|| self.error("E_PATH", "无效路径命令", l))?;
                    let o = o.borrow();
                    let c = &o.args;
                    let coord = |k: &str| length(c, k, 0., &self.unit, self.dpi);
                    let mut command = args_json(c);
                    command["kind"] = json!(o.kind);
                    for key in ["x", "y", "cx", "cy", "c1x", "c1y", "c2x", "c2y", "rx", "ry"] {
                        if c.contains_key(key) {
                            command[key] = json!(coord(key));
                        }
                    }
                    command["rotation"] = json!(num(c, "rotation", 0.).to_radians());
                    commands.push(command);
                    match o.kind.as_str() {
                        "move_to" => p.move_to((coord("x"), coord("y"))),
                        "line_to" => p.line_to((coord("x"), coord("y"))),
                        "quad_to" => {
                            p.quad_to((coord("cx"), coord("cy")), (coord("x"), coord("y")))
                        }
                        "cubic_to" => p.curve_to(
                            (coord("c1x"), coord("c1y")),
                            (coord("c2x"), coord("c2y")),
                            (coord("x"), coord("y")),
                        ),
                        "close" => p.close_path(),
                        "arc_to" => {
                            let s = p
                                .elements()
                                .last()
                                .and_then(|e| match e {
                                    PathEl::MoveTo(p)
                                    | PathEl::LineTo(p)
                                    | PathEl::QuadTo(_, p)
                                    | PathEl::CurveTo(_, _, p) => Some(*p),
                                    _ => None,
                                })
                                .unwrap_or(Point::ORIGIN);
                            let arc = kurbo::SvgArc {
                                from: s,
                                to: Point::new(coord("x"), coord("y")),
                                radii: kurbo::Vec2::new(coord("rx"), coord("ry")),
                                x_rotation: num(c, "rotation", 0.).to_radians(),
                                large_arc: yes(c, "large_arc", false),
                                sweep: yes(c, "sweep", false),
                            };
                            if let Some(arc) = kurbo::Arc::from_svg_arc(&arc) {
                                arc.to_cubic_beziers(0.0001, |p1, p2, p3| p.curve_to(p1, p2, p3));
                            } else {
                                p.line_to(arc.to);
                            }
                        }
                        _ => return Err(self.error("E_PATH", "无效路径命令", l)),
                    }
                }
                recipe = json!({"kind":"path","commands":commands});
            }
            _ => return Err(self.error("E_TYPE", format!("{} 不能作为图形放置", o.kind), l)),
        }
        let b = crate::geometry_recipe::analytic_bounds(&recipe, &p);
        if fixed.is_none() {
            p = Affine::translate((-b.x0, -b.y0)) * p;
        }
        let line = matches!(o.kind.as_str(), "line" | "polyline" | "arc");
        let stroke_key = if line { "line_color" } else { "border_color" };
        let color = string(a, stroke_key, if line { "#000000" } else { "none" });
        let width = length(
            a,
            if line { "line_width" } else { "border_width" },
            if color == "none" { 0. } else { 0.3 },
            "pt",
            self.dpi,
        );
        let fill = a
            .get("fill")
            .map(V::json)
            .or_else(|| a.get("background").map(V::json))
            .unwrap_or(json!("none"));
        let mut n = path_node(p, fill, &color, width);
        if recipe.is_object() {
            recipe["origin"] = if fixed.is_none() {
                json!([b.x0, b.y0])
            } else {
                json!([0., 0.])
            };
            n["geometryRecipe"] = recipe;
        }
        if let Some((w, h)) = fixed {
            n["width"] = json!(w);
            n["height"] = json!(h);
        }
        if fixed.is_none() {
            n["width"] = json!(b.width());
            n["height"] = json!(b.height());
            n["intrinsicWidth"] = json!(b.width());
            n["intrinsicHeight"] = json!(b.height());
            for (key, requested, natural) in [
                ("width", size[0], b.width()),
                ("height", size[1], b.height()),
            ] {
                if requested.is_finite() {
                    if natural <= 0. {
                        return Err(self.error("E_PATH", "零尺寸路径不能缩放该方向", l));
                    }
                    n[key] = json!(requested);
                }
            }
        }
        n["strokeStyle"] = self.stroke(a, line, width);
        if o.kind == "line" {
            let (dx, dy) = crate::endpoints::line_vector(self, a, l)?;
            if dx == 0. && dy == 0. {
                n["zeroAngle"] = json!(num(a, "angle", 0.));
            }
            n["endpoints"] = json!([
                [
                    if dx < 0. { dx.abs() } else { 0. },
                    if dy < 0. { dy.abs() } else { 0. }
                ],
                [
                    if dx < 0. { 0. } else { dx.abs() },
                    if dy < 0. { 0. } else { dy.abs() }
                ]
            ]);
        }
        n["fillRule"] = json!(string(
            a,
            "fill_rule",
            if o.kind == "ring" {
                "evenodd"
            } else {
                "nonzero"
            }
        ));
        crate::endpoints::configure(self, &mut n, a, l)?;
        if !n["zeroAngle"].is_null() {
            crate::endpoints::zero_layout(&mut n);
        }
        Ok(n)
    }
    pub fn stroke(&self, a: &Args, line: bool, width: f64) -> Json {
        let p = if line { "line" } else { "border" };
        let style = string(a, &format!("{p}_style"), "solid");
        let dash = if a.contains_key(&format!("{p}_dash")) {
            array(a, &format!("{p}_dash"))
                .iter()
                .filter_map(|v| value_length(v, "pt", self.dpi))
                .collect::<Vec<_>>()
        } else {
            match style.as_str() {
                "dashed" => vec![width * 4., width * 2.],
                "dotted" => vec![width * 0.01, width * 1.5],
                "dash_dot" => vec![width * 4., width * 2., width * 0.01, width * 2.],
                _ => vec![],
            }
        };
        json!({"color":string(a,&format!("{p}_color"),if width>0.{"#000000"}else{"none"}),"width":if style=="none"{0.}else{width},"dash":dash,"dashOffset":length(a,&format!("{p}_dash_offset"),0.,"pt",self.dpi),"cap":string(a,&format!("{p}_cap"),if style=="dotted"{"round"}else{"butt"}),"join":string(a,&format!("{p}_join"),"miter"),"miterLimit":num(a,&format!("{p}_miter_limit"),4.),"opacity":num(a,&format!("{p}_opacity"),1.),"compound":if matches!(style.as_str(),"double"|"triple"){style}else{"single".into()}})
    }
    pub fn fuse(
        &mut self,
        owner: Rc<RefCell<Object>>,
        pos: Vec<V>,
        mut a: Args,
        l: Loc,
    ) -> Result<V> {
        self.validate_args("fuse", &mut a, &pos, l)?;
        if owner.borrow().sealed {
            return Err(self.error("E_GROUP", "组合放置后不能融合", l));
        }
        if pos.len() != 2 {
            return Err(self.error("E_ARG", "fuse 需要两个实例", l));
        }
        let objects = pos
            .iter()
            .map(|v| {
                v.object()
                    .ok_or_else(|| self.error("E_FUSE", "需要实例", l))
            })
            .collect::<Result<Vec<_>>>()?;
        if Rc::ptr_eq(&objects[0], &objects[1]) {
            return Err(self.error("E_FUSE", "需要两个不同实例", l));
        }
        let mut paths = vec![];
        let mut ids = vec![];
        for o in &objects {
            let o = o.borrow();
            if o.parent != owner.borrow().id
                || o.kind != "instance"
                || yes(&o.args, "__consumed", false)
            {
                return Err(self.error("E_FUSE", "实例必须属于当前容器且尚未融合", l));
            }
            let n = o.node.as_ref().unwrap();
            if jstr(n, "kind", "") != "path" {
                return Err(self.error("E_FUSE", "仅矢量形状和线条实例可以融合", l));
            }
            paths.push(visible(n));
            ids.push(n["id"].clone());
        }
        let junction = string(&a, "junction", "miter");
        let radius = length(&a, "radius", 0., &self.unit, self.dpi);
        if !matches!(junction.as_str(), "miter" | "bevel" | "round")
            || (junction == "round" && radius <= 0.)
            || (junction != "round" && a.contains_key("radius"))
        {
            return Err(self.error(
                "E_FUSE",
                "round 连接需要正 radius；其他连接不能指定 radius",
                l,
            ));
        }
        let mut best = ([0., 0.], [0., 0.], f64::INFINITY);
        if a.contains_key("points") {
            let anchors = array(&a, "points");
            if anchors.len() != 2 {
                return Err(self.error("E_FUSE", "points 需要两个实例锚点", l));
            }
            for (i, v) in anchors.iter().enumerate() {
                let instance = match v {
                    V::Anchor {
                        reference: Some(r), ..
                    } => Some(r.instance),
                    V::Geometry(q) => q.instance,
                    _ => None,
                };
                let point = self.geometry_anchor(v, None, l)?;
                if point.owner != owner.borrow().id || instance != Some(objects[i].borrow().id) {
                    return Err(self.error("E_FUSE", "points 须依次引用两个融合实例的锚点", l));
                }
                if i == 0 {
                    best.0 = [point.point.x, point.point.y];
                } else {
                    best.1 = [point.point.x, point.point.y];
                }
            }
            best.2 = (best.0[0] - best.1[0]).hypot(best.0[1] - best.1[1]);
        } else {
            let pa = points(&paths[0], 0.0001);
            let pb = points(&paths[1], 0.0001);
            best = nearest::boundaries(&pa, &pb);
            if !intersection(&paths[0], &paths[1]).is_empty() {
                best.1 = best.0;
                best.2 = 0.;
            }
        }
        if !best.2.is_finite() {
            return Err(self.error("E_FUSE", "连接对象没有可用边界", l));
        }
        if best.2 > 1e-6 && !a.contains_key("bridge_width") {
            return Err(self.error("E_FUSE", "有间隙时需要 bridge_width", l));
        }
        let width = length(&a, "bridge_width", 0., &self.unit, self.dpi);
        if a.contains_key("bridge_width") && width <= 0. {
            return Err(self.error("E_VALUE", "连接宽度必须为正", l));
        }
        if junction == "round" && best.2 > 1e-6 && radius > best.2 / 2. {
            return Err(self.error("E_FUSE", "圆角半径超过连接长度", l));
        }
        let mut bridge = BezPath::new();
        if best.2 > 1e-8 {
            if junction == "bevel" {
                let ux = (best.1[0] - best.0[0]) / best.2;
                let uy = (best.1[1] - best.0[1]) / best.2;
                let nx = -uy * width / 2.;
                let ny = ux * width / 2.;
                let trim = (width / 2.).min(best.2 / 4.);
                let (s, t) = (best.0, best.1);
                for (i, q) in [
                    (s[0] - ux * trim, s[1] - uy * trim),
                    (s[0] + ux * trim + nx, s[1] + uy * trim + ny),
                    (t[0] - ux * trim + nx, t[1] - uy * trim + ny),
                    (t[0] + ux * trim, t[1] + uy * trim),
                    (t[0] - ux * trim - nx, t[1] - uy * trim - ny),
                    (s[0] + ux * trim - nx, s[1] + uy * trim - ny),
                ]
                .iter()
                .enumerate()
                {
                    if i == 0 {
                        bridge.move_to(*q)
                    } else {
                        bridge.line_to(*q)
                    }
                }
                bridge.close_path();
            } else {
                bridge.move_to((best.0[0], best.0[1]));
                bridge.line_to((best.1[0], best.1[1]));
                bridge = outline(
                    &bridge,
                    &json!({"width":width,"cap":if junction=="round"{"round"}else{"butt"}}),
                );
            }
        }
        if junction == "round" {
            let contact = if best.2 <= 1e-8 {
                kurbo::Circle::new((best.0[0], best.0[1]), 0.01).to_path(0.0001)
            } else {
                bridge.clone()
            };
            if paths
                .iter()
                .any(|p| intersection(p, &contact).area().abs() <= 1e-10)
            {
                return Err(self.error("E_FUSE", "轮廓与连接段没有接触；请使用自动最近轮廓点", l));
            }
        }
        let mut combined = union(&union(&paths[0], &paths[1]), &bridge);
        if junction == "round" {
            let stroke = json!({"width":radius*2.,"cap":"round","join":"round"});
            let expanded = union(&combined, &outline(&combined, &stroke));
            let closed = difference(&expanded, &outline(&expanded, &stroke));
            let additions = difference(&closed, &combined);
            let reach = width / 2. + radius * 2.;
            let region = union(
                &kurbo::Circle::new((best.0[0], best.0[1]), reach).to_path(0.0001),
                &kurbo::Circle::new((best.1[0], best.1[1]), reach).to_path(0.0001),
            );
            combined = union(&combined, &intersection(&additions, &region));
        }
        let bounds = combined.bounding_box();
        let mut n = path_node(
            Affine::translate((-bounds.x0, -bounds.y0)) * combined,
            a.get("fill").map(V::json).unwrap_or(json!("none")),
            "none",
            0.,
        );
        n["geometryRecipe"] = json!({"kind":"fuse","origin":[bounds.x0,bounds.y0],"nodes":[objects[0].borrow().node,objects[1].borrow().node],"points":[best.0,best.1],"width":width,"radius":radius,"junction":junction,"bevel":bridge.to_svg()});
        // Fusion has no outline unless the caller supplies one. Ordinary shape
        // defaults must not add a black border to a filled union.
        a.entry("border_color".into())
            .or_insert_with(|| V::text("none"));
        n["strokeStyle"] = self.stroke(&a, false, length(&a, "border_width", 0.3, "pt", self.dpi));
        n["fillRule"] = json!(string(&a, "fill_rule", "evenodd"));
        n["opacity"] = json!(num(&a, "opacity", 1.));
        n["x"] = json!(bounds.x0);
        n["y"] = json!(bounds.y0);
        self.serial += 1;
        n["id"] = json!(format!("fused-{}", self.serial));
        owner.borrow_mut().nodes.retain(|n| !ids.contains(&n["id"]));
        owner.borrow_mut().nodes.push(n.clone());
        let mut record = a.clone();
        record.insert("__left".into(), V::num(objects[0].borrow().id as f64));
        record.insert("__right".into(), V::num(objects[1].borrow().id as f64));
        for o in objects {
            o.borrow_mut()
                .args
                .insert("__consumed".into(), V::Bool(true));
        }
        let v = self.object("instance", a, l);
        let o = v.object().unwrap();
        let mut o = o.borrow_mut();
        record.insert("__result_id".into(), V::num(o.id as f64));
        if owner.borrow().kind == "group" {
            owner.borrow_mut().layers.push(("__fuse".into(), record));
        }
        o.parent = owner.borrow().id;
        o.width = bounds.width();
        o.height = bounds.height();
        o.node = Some(n);
        drop(o);
        Ok(v)
    }
}
