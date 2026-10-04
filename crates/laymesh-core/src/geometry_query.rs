//! Typed, replayable selectors for instance parts, paths, and anchor collections.
use crate::query_path::{self as qp, Curve, PathPoint, Paths, Position, Route, Segment};
use crate::{Loc, Result, engine::Engine, model::*, parser::Expr};
use kurbo::{Affine, Arc, BezPath, CubicBez, Line, PathSeg, Point, QuadBez, Rect, Vec2};
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug)]
pub struct Query {
    pub instance: Option<usize>,
    pub steps: Vec<Step>,
}
#[derive(Clone, Debug)]
pub enum Step {
    Member(String),
    Index(V),
    Call(String, Vec<V>, Args),
}
pub fn root(instance: Option<usize>) -> V {
    V::Geometry(Rc::new(Query {
        instance,
        steps: vec![],
    }))
}
#[derive(Clone, Debug)]
pub struct AnchorPoint {
    pub point: Point,
    pub owner: usize,
    pub path: Option<PathPoint>,
}
impl AnchorPoint {
    pub fn tangent(&self, e: &Engine, l: Loc) -> Result<Vec2> {
        self.path
            .as_ref()
            .ok_or_else(|| e.error("E_ANCHOR_DIRECTION", "此锚点没有路径方向", l))?
            .tangent()
            .map_err(|m| e.error("E_ANCHOR_DIRECTION", m, l))
    }
}
#[derive(Clone, Debug)]
struct PathView {
    paths: Rc<Paths>,
    route: Option<usize>,
    segment: Option<usize>,
}
#[derive(Clone, Debug)]
struct NodeView {
    node: Json,
    transform: Affine,
}
#[derive(Clone, Debug)]
enum Geometry {
    Node(NodeView),
    Bounds(Rect, Affine),
    Ink(NodeView),
    Path(PathView),
    Subpaths(PathView),
    Segments(PathView),
    Nodes(PathView),
    Controls(PathView),
    Axes(NodeView),
    Axis(NodeView, String),
    Decorations(NodeView, String),
    Anchor(AnchorPoint),
    Collection(Vec<AnchorPoint>),
    Value(V),
}
impl Query {
    pub fn is_self_contained(&self) -> bool {
        fn value(v: &V) -> bool {
            match v {
                V::Geometry(q) | V::Measured(q, _) => q.is_self_contained(),
                V::List(v) => v.iter().all(value),
                V::Map(v) => v.values().all(value),
                V::Dict(v) => v.values().all(value),
                V::Anchor { .. } => false,
                _ => true,
            }
        }
        self.instance.is_none()
            && self.steps.iter().all(|s| match s {
                Step::Member(_) => true,
                Step::Index(v) => value(v),
                Step::Call(_, p, a) => p.iter().all(value) && a.values().all(value),
            })
    }

    pub fn rebound(&self, instances: &BTreeMap<usize, V>) -> Self {
        fn value(v: &V, map: &BTreeMap<usize, V>) -> V {
            match v {
                V::Geometry(q) => V::Geometry(Rc::new(q.rebound(map))),
                V::Measured(q, v) => V::Measured(Rc::new(q.rebound(map)), v.clone()),
                V::List(v) => V::List(v.iter().map(|v| value(v, map)).collect()),
                V::Map(v) => V::Map(v.iter().map(|(k, v)| (k.clone(), value(v, map))).collect()),
                V::Dict(v) => V::Dict(v.iter().map(|(k, v)| (k.clone(), value(v, map))).collect()),
                V::Anchor {
                    reference: Some(r), ..
                } => {
                    if let Some(o) = map.get(&r.instance).and_then(V::object) {
                        let o = o.borrow();
                        let mut r = r.clone();
                        r.instance = o.id;
                        V::Anchor {
                            owner: o.parent,
                            x: 0.,
                            y: 0.,
                            reference: Some(r),
                        }
                    } else {
                        v.clone()
                    }
                }
                _ => v.clone(),
            }
        }
        let instance = self.instance.map(|id| {
            instances
                .get(&id)
                .and_then(V::object)
                .map(|o| o.borrow().id)
                .unwrap_or(id)
        });
        let steps = self
            .steps
            .iter()
            .map(|step| match step {
                Step::Member(m) => Step::Member(m.clone()),
                Step::Index(v) => Step::Index(value(v, instances)),
                Step::Call(m, p, a) => Step::Call(
                    m.clone(),
                    p.iter().map(|v| value(v, instances)).collect(),
                    a.iter()
                        .map(|(k, v)| (k.clone(), value(v, instances)))
                        .collect(),
                ),
            })
            .collect();
        Self { instance, steps }
    }
}
fn node_frame(n: &Json) -> Affine {
    let mut tr = crate::geometry::node_transform(n);
    if n["kind"] == "group" {
        tr = tr
            * Affine::scale_non_uniform(
                jnum(n, "width", 1.) / jnum(n, "contentWidth", jnum(n, "width", 1.)).max(1e-12),
                jnum(n, "height", 1.) / jnum(n, "contentHeight", jnum(n, "height", 1.)).max(1e-12),
            );
    } else if jnum(n, "intrinsicWidth", 0.) > 0. || jnum(n, "intrinsicHeight", 0.) > 0. {
        tr = tr
            * Affine::scale_non_uniform(
                if jnum(n, "intrinsicWidth", 0.) > 0. {
                    jnum(n, "width", 1.) / jnum(n, "intrinsicWidth", 1.)
                } else {
                    1.
                },
                if jnum(n, "intrinsicHeight", 0.) > 0. {
                    jnum(n, "height", 1.) / jnum(n, "intrinsicHeight", 1.)
                } else {
                    1.
                },
            );
    }
    tr
}
fn rectangle_node(b: Rect) -> Json {
    let mut n = base("rect", b.width(), b.height());
    n["x"] = json!(b.x0);
    n["y"] = json!(b.y0);
    n
}
fn paths(node: &NodeView, e: &Engine, l: Loc) -> Result<Paths> {
    let n = &node.node;
    let tr = node.transform * node_frame(n);
    let recipe = &n["geometryRecipe"];
    let kind = jstr(recipe, "kind", "");
    let mut routes = vec![];
    if n["kind"] == "group" {
        for child in n["children"].as_array().into_iter().flatten() {
            routes.extend(
                paths(
                    &NodeView {
                        node: child.clone(),
                        transform: tr,
                    },
                    e,
                    l,
                )?
                .routes,
            );
        }
    } else if kind == "rect" {
        let (w, h) = (jnum(recipe, "width", 0.), jnum(recipe, "height", 0.));
        let radius = jnum(recipe, "radius", 0.).max(0.).min(w / 2.).min(h / 2.);
        if radius == 0. {
            let points = [
                Point::new(0., 0.),
                Point::new(w, 0.),
                Point::new(w, h),
                Point::new(0., h),
                Point::new(0., 0.),
            ];
            routes.push(Route {
                zero_direction: None,
                segments: points
                    .windows(2)
                    .map(|p| Segment {
                        curve: Curve::Poly(PathSeg::Line(Line::new(p[0], p[1]))),
                        transform: tr,
                    })
                    .collect(),
                closed: true,
            });
        } else {
            let mut segments = vec![];
            for (center, start, next) in [
                (
                    Point::new(w - radius, radius),
                    -std::f64::consts::FRAC_PI_2,
                    Point::new(w, h - radius),
                ),
                (
                    Point::new(w - radius, h - radius),
                    0.,
                    Point::new(radius, h),
                ),
                (
                    Point::new(radius, h - radius),
                    std::f64::consts::FRAC_PI_2,
                    Point::new(0., radius),
                ),
                (
                    Point::new(radius, radius),
                    std::f64::consts::PI,
                    Point::new(w - radius, 0.),
                ),
            ] {
                let arc = Arc {
                    center,
                    radii: Vec2::new(radius, radius),
                    start_angle: start,
                    sweep_angle: std::f64::consts::FRAC_PI_2,
                    x_rotation: 0.,
                };
                segments.push(Segment {
                    curve: Curve::Arc(arc),
                    transform: tr,
                });
                if arc_point(arc, 1.).distance(next) > 1e-12 {
                    segments.push(Segment {
                        curve: Curve::Poly(PathSeg::Line(Line::new(arc_point(arc, 1.), next))),
                        transform: tr,
                    });
                }
            }
            routes.push(Route {
                zero_direction: None,
                segments,
                closed: true,
            });
        }
    } else if n["endpoints"].is_array() {
        let p = &n["endpoints"];
        routes.push(Route {
            zero_direction: n["zeroAngle"]
                .as_f64()
                .map(|a| qp::vector(tr, Vec2::new(a.to_radians().cos(), a.to_radians().sin()))),
            segments: vec![Segment {
                curve: Curve::Poly(PathSeg::Line(Line::new(
                    Point::new(p[0][0].as_f64().unwrap(), p[0][1].as_f64().unwrap()),
                    Point::new(p[1][0].as_f64().unwrap(), p[1][1].as_f64().unwrap()),
                ))),
                transform: tr,
            }],
            closed: false,
        });
    } else if kind == "path" {
        let origin = &recipe["origin"];
        let tr = tr
            * Affine::translate((
                -origin[0].as_f64().unwrap_or(0.),
                -origin[1].as_f64().unwrap_or(0.),
            ));
        let mut r = Route {
            zero_direction: None,
            segments: vec![],
            closed: false,
        };
        let mut p = Point::ORIGIN;
        let mut start = p;
        for c in recipe["commands"].as_array().into_iter().flatten() {
            let g = |key| jnum(c, key, 0.);
            let end = Point::new(g("x"), g("y"));
            let curve = match jstr(c, "kind", "") {
                "move_to" => {
                    if !r.segments.is_empty() {
                        routes.push(r);
                        r = Route {
                            zero_direction: None,
                            segments: vec![],
                            closed: false,
                        };
                    }
                    p = end;
                    start = p;
                    continue;
                }
                "line_to" => Curve::Poly(PathSeg::Line(Line::new(p, end))),
                "quad_to" => Curve::Poly(PathSeg::Quad(QuadBez::new(
                    p,
                    Point::new(g("cx"), g("cy")),
                    end,
                ))),
                "cubic_to" => Curve::Poly(PathSeg::Cubic(CubicBez::new(
                    p,
                    Point::new(g("c1x"), g("c1y")),
                    Point::new(g("c2x"), g("c2y")),
                    end,
                ))),
                "arc_to" => {
                    let a = kurbo::SvgArc {
                        from: p,
                        to: end,
                        radii: Vec2::new(g("rx"), g("ry")),
                        x_rotation: g("rotation"),
                        large_arc: c["large_arc"] == true,
                        sweep: c["sweep"] == true,
                    };
                    if let Some(a) = Arc::from_svg_arc(&a) {
                        Curve::Arc(a)
                    } else {
                        Curve::Poly(PathSeg::Line(Line::new(p, end)))
                    }
                }
                "close" => {
                    if p.distance(start) > 1e-12 {
                        r.segments.push(Segment {
                            curve: Curve::Poly(PathSeg::Line(Line::new(p, start))),
                            transform: tr,
                        });
                    }
                    r.closed = true;
                    p = start;
                    continue;
                }
                _ => return Err(e.error("E_PATH", "无效几何路径", l)),
            };
            r.segments.push(Segment {
                curve,
                transform: tr,
            });
            p = end;
        }
        if !r.segments.is_empty() {
            routes.push(r);
        }
    } else if matches!(kind, "ellipse" | "ring" | "arc" | "sector") {
        let origin = &recipe["origin"];
        let tr = tr
            * Affine::translate((
                -origin[0].as_f64().unwrap_or(0.),
                -origin[1].as_f64().unwrap_or(0.),
            ));
        let (center, radii) = if kind == "ellipse" {
            let w = jnum(recipe, "width", 0.);
            let h = jnum(recipe, "height", 0.);
            (Point::new(w / 2., h / 2.), Vec2::new(w / 2., h / 2.))
        } else {
            let r = jnum(recipe, "radius", 0.);
            (Point::new(r, r), Vec2::new(r, r))
        };
        let start = jnum(recipe, "start", 0.);
        let sweep = if kind == "arc" || kind == "sector" {
            jnum(recipe, "sweep", std::f64::consts::TAU)
        } else {
            std::f64::consts::TAU
        };
        let arc = Arc {
            center,
            radii,
            start_angle: start,
            sweep_angle: sweep,
            x_rotation: 0.,
        };
        let s = Segment {
            curve: Curve::Arc(arc),
            transform: tr,
        };
        let mut r = Route {
            zero_direction: None,
            segments: vec![s.clone()],
            closed: kind != "arc",
        };
        if kind == "sector" {
            r.segments.push(Segment {
                curve: Curve::Poly(PathSeg::Line(Line::new(arc_point(arc, 1.), center))),
                transform: tr,
            });
            r.segments.push(Segment {
                curve: Curve::Poly(PathSeg::Line(Line::new(center, arc_point(arc, 0.)))),
                transform: tr,
            });
        }
        routes.push(r);
        if kind == "ring" {
            let inner = jnum(recipe, "inner", 0.);
            if inner > 0. {
                routes.push(Route {
                    zero_direction: None,
                    segments: vec![Segment {
                        curve: Curve::Arc(Arc {
                            radii: Vec2::new(inner, inner),
                            ..arc
                        }),
                        transform: tr,
                    }],
                    closed: true,
                });
            }
        }
    } else if let Ok(p) = BezPath::from_svg(jstr(n, "d", "")) {
        routes = qp::from_bez(&p, tr);
    } else {
        return Err(e.error("E_GEOMETRY", "path 仅适用于矢量几何", l));
    }
    Ok(Paths {
        identity: String::new(),
        routes,
        frame: tr,
        local: false,
    })
}
fn vector_only(node: &Json) -> bool {
    node["kind"] == "path"
        || (node["kind"] == "group"
            && node["children"]
                .as_array()
                .is_some_and(|children| children.iter().all(vector_only)))
}
fn remove_invisible_ink(node: &mut Json) {
    if node["opacity"] == 0. {
        node["fill"] = json!("none");
        node["strokeStyle"]["width"] = json!(0.);
        if node["children"].is_array() {
            node["children"] = json!([]);
        }
    }
    if node["strokeStyle"]["opacity"] == 0. {
        node["strokeStyle"]["width"] = json!(0.);
    }
    for child in node["children"].as_array_mut().into_iter().flatten() {
        remove_invisible_ink(child);
    }
}
/// Keep analytic boundaries where their fill/stroke region has a known exact
/// boundary. In particular a circle-center nearest query must remain infinite,
/// rather than becoming a collection of tessellation vertices.
fn analytic_ink(node: &NodeView, e: &Engine, l: Loc) -> Result<Option<Paths>> {
    let n = &node.node;
    let kind = jstr(&n["geometryRecipe"], "kind", "");
    let filled = n["fill"] != "none" && !n["fill"].is_null();
    let width = if n["strokeStyle"]["color"] == "none" {
        0.
    } else {
        jnum(&n["strokeStyle"], "width", 0.)
    };
    if filled && width == 0. && matches!(kind, "rect" | "ellipse" | "ring" | "sector") {
        return Ok(Some(paths(node, e, l)?));
    }
    if kind == "ellipse"
        && width > 0.
        && (jnum(&n["geometryRecipe"], "width", 0.) - jnum(&n["geometryRecipe"], "height", 0.))
            .abs()
            < 1e-12
        && n["strokeStyle"]["dash"]
            .as_array()
            .is_none_or(|dash| dash.is_empty())
        && jstr(&n["strokeStyle"], "compound", "single") == "single"
    {
        let mut geometry = paths(node, e, l)?;
        let mut outer = geometry.routes[0].clone();
        let Curve::Arc(mut arc) = outer.segments[0].curve else {
            return Ok(None);
        };
        let inner = arc.radii.x - width / 2.;
        arc.radii += Vec2::new(width / 2., width / 2.);
        outer.segments[0].curve = Curve::Arc(arc);
        geometry.routes = vec![outer.clone()];
        if !filled && inner > 0. {
            arc.radii = Vec2::new(inner, inner);
            outer.segments[0].curve = Curve::Arc(arc);
            geometry.routes.push(outer);
        }
        return Ok(Some(geometry));
    }
    Ok(None)
}
fn arc_point(a: Arc, t: f64) -> Point {
    Segment {
        curve: Curve::Arc(a),
        transform: Affine::IDENTITY,
    }
    .point(t)
}
impl Engine {
    fn query_argument(&self, v: &V, binding: Option<(&Json, usize)>, l: Loc) -> Result<V> {
        Ok(match v {
            V::Measured(q, _) => match self.resolve_geometry(q, binding, l)? {
                Geometry::Value(value) => V::Measured(q.clone(), Box::new(value)),
                _ => return Err(self.error("E_TYPE", "测量引用没有产生数值", l)),
            },
            V::Geometry(q) if q.instance.is_none() && binding.is_some() => {
                match self.resolve_geometry(q, binding, l)? {
                    Geometry::Value(value) => value,
                    _ => v.clone(),
                }
            }
            V::List(values) => V::List(
                values
                    .iter()
                    .map(|v| self.query_argument(v, binding, l))
                    .collect::<Result<_>>()?,
            ),
            V::Map(values) => V::Map(
                values
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.query_argument(v, binding, l)?)))
                    .collect::<Result<_>>()?,
            ),
            V::Dict(values) => V::Dict(
                values
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.query_argument(v, binding, l)?)))
                    .collect::<Result<_>>()?,
            ),
            _ => v.clone(),
        })
    }
    pub(crate) fn geometry_items(&self, q: &Query, l: Loc) -> Result<Vec<V>> {
        let geometry = self.resolve_geometry(q, None, l)?;
        let count = match geometry {
            Geometry::Collection(ref v) => v.len(),
            Geometry::Subpaths(ref p) => p.paths.routes.len(),
            Geometry::Segments(ref p) => p.paths.routes[self.route_id(p, l)?].segments.len(),
            Geometry::Nodes(ref p) => {
                let r = &p.paths.routes[self.route_id(p, l)?];
                r.segments.len() + usize::from(!r.closed)
            }
            Geometry::Controls(ref p) => p.paths.routes[self.route_id(p, l)?].segments
                [p.segment.unwrap()]
            .controls()
            .len(),
            Geometry::Value(V::List(ref v)) => v.len(),
            _ => return Err(self.error("E_TYPE", "需要几何集合", l)),
        };
        (0..count)
            .map(|i| self.geometry_step(q, Step::Index(V::num(i as f64)), l))
            .collect()
    }
    pub(crate) fn replay_query_value(
        &self,
        v: &V,
        instances: &BTreeMap<usize, V>,
        l: Loc,
    ) -> Result<V> {
        Ok(match v {
            V::Geometry(q) => V::Geometry(Rc::new(q.rebound(instances))),
            V::Measured(q, _) => {
                let q = q.rebound(instances);
                let Geometry::Value(value) = self.resolve_geometry(&q, None, l)? else {
                    return Err(self.error("E_TYPE", "测量引用没有产生数值", l));
                };
                V::Measured(Rc::new(q), Box::new(value))
            }
            V::List(values) => V::List(
                values
                    .iter()
                    .map(|v| self.replay_query_value(v, instances, l))
                    .collect::<Result<_>>()?,
            ),
            V::Map(values) => V::Map(
                values
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.replay_query_value(v, instances, l)?)))
                    .collect::<Result<_>>()?,
            ),
            V::Dict(values) => V::Dict(
                values
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.replay_query_value(v, instances, l)?)))
                    .collect::<Result<_>>()?,
            ),
            _ => v.clone(),
        })
    }
    pub(crate) fn eval_arguments(
        &mut self,
        args: &[(Option<String>, Expr)],
        s: &Scope,
        l: Loc,
    ) -> Result<(Vec<V>, Args)> {
        let mut pos = vec![];
        let mut a = Args::new();
        let mut named = false;
        for (n, expr) in args {
            let v = self.eval(expr, s)?;
            if let Some(n) = n {
                named = true;
                if a.insert(n.clone(), v).is_some() {
                    return Err(self.error("E_ARG", format!("重复参数 {n}"), l));
                }
            } else {
                if named {
                    return Err(self.error("E_ARG", "位置参数必须位于命名参数之前", l));
                }
                pos.push(v);
            }
        }
        Ok((pos, a))
    }
    pub(crate) fn geometry_step(&self, q: &Query, step: Step, l: Loc) -> Result<V> {
        let mut q = q.clone();
        q.steps.push(step);
        if q.instance.is_some() {
            if let Geometry::Value(v) = self.resolve_geometry(&q, None, l)? {
                return Ok(V::Measured(Rc::new(q), Box::new(v)));
            }
        }
        Ok(V::Geometry(Rc::new(q)))
    }
    pub(crate) fn geometry_method(
        &self,
        v: V,
        name: &str,
        pos: Vec<V>,
        mut a: Args,
        l: Loc,
    ) -> Result<V> {
        a.retain(|k, _| !k.starts_with("__"));
        let V::Geometry(q) = v else {
            return Err(self.error("E_CALL", "此对象没有几何查询方法", l));
        };
        self.geometry_step(&q, Step::Call(name.into(), pos, a), l)
    }
    pub(crate) fn geometry_anchor(
        &self,
        v: &V,
        binding: Option<(&Json, usize)>,
        l: Loc,
    ) -> Result<AnchorPoint> {
        match v {
            V::Geometry(q) => match self.resolve_geometry(q, binding, l)? {
                Geometry::Anchor(a) => Ok(a),
                Geometry::Collection(_) => Err(self.error(
                    "E_ANCHOR",
                    "候选集合必须显式使用 [0] 等索引，即使只有一个候选",
                    l,
                )),
                _ => Err(self.error("E_ANCHOR", "需要一个锚点，不能使用几何视图", l)),
            },
            V::Anchor {
                owner,
                x,
                y,
                reference,
            } => {
                if let Some(r) = reference {
                    if let Some(o) = self.objects.get(&r.instance) {
                        let v = self.member(V::Object(o.clone()), &r.name, l)?;
                        if let V::Anchor { x, y, .. } = v {
                            return Ok(AnchorPoint {
                                point: Point::new(x, y),
                                owner: *owner,
                                path: None,
                            });
                        }
                    }
                }
                Ok(AnchorPoint {
                    point: Point::new(*x, *y),
                    owner: *owner,
                    path: None,
                })
            }
            _ => Err(self.error("E_ANCHOR", "需要锚点", l)),
        }
    }
    fn resolve_geometry(
        &self,
        q: &Query,
        binding: Option<(&Json, usize)>,
        l: Loc,
    ) -> Result<Geometry> {
        let (node, owner) = if let Some(id) = q.instance {
            let o = self
                .objects
                .get(&id)
                .ok_or_else(|| self.error("E_ANCHOR", "锚点实例不存在", l))?
                .borrow();
            if yes(&o.args, "__consumed", false) {
                return Err(self.error("E_FUSE", "已融合的实例不能再次引用", l));
            }
            let n = o
                .node
                .clone()
                .unwrap_or_else(|| base("rect", o.width, o.height));
            (n, if o.kind == "canvas" { o.id } else { o.parent })
        } else {
            let (n, owner) = binding
                .ok_or_else(|| self.error("E_ANCHOR", "self 只能作为 anchor 的自身选择器", l))?;
            (n.clone(), owner)
        };
        let mut v = Geometry::Node(NodeView {
            node,
            transform: Affine::IDENTITY,
        });
        for (index, step) in q.steps.iter().enumerate() {
            v = match step {
                Step::Member(m) => self.geometry_member(v, m, owner, l)?,
                Step::Index(i) => {
                    self.geometry_index(v, &self.query_argument(i, binding, l)?, owner, l)?
                }
                Step::Call(m, p, a) => {
                    let p = p
                        .iter()
                        .map(|v| self.query_argument(v, binding, l))
                        .collect::<Result<Vec<_>>>()?;
                    let a = a
                        .iter()
                        .map(|(k, v)| Ok((k.clone(), self.query_argument(v, binding, l)?)))
                        .collect::<Result<Args>>()?;
                    self.geometry_call(v, m, &p, &a, owner, binding, l)?
                }
            };
            if let Geometry::Path(path) = &mut v {
                if path.paths.identity.is_empty() {
                    let mut paths = (*path.paths).clone();
                    paths.identity = format!("{:?}:{owner}:{:?}", q.instance, &q.steps[..=index]);
                    path.paths = Rc::new(paths);
                }
            }
            if let Geometry::Anchor(anchor) = &mut v {
                if let Some(point) = anchor.path.as_mut().filter(|p| p.paths.identity.is_empty()) {
                    let mut steps = q.steps[..=index].to_vec();
                    if matches!(steps.last(),Some(Step::Member(name)) if name=="min"||name=="max") {
                        *steps.last_mut().unwrap() = Step::Member("spine".into());
                    }
                    let mut paths = (*point.paths).clone();
                    paths.identity = format!("{:?}:{owner}:{steps:?}", q.instance);
                    point.paths = Rc::new(paths);
                }
            }
        }
        Ok(v)
    }
    fn geometry_member(&self, v: Geometry, m: &str, owner: usize, l: Loc) -> Result<Geometry> {
        let point = |p, path| {
            Geometry::Anchor(AnchorPoint {
                point: p,
                owner,
                path,
            })
        };
        match v {
            Geometry::Node(n) => {
                if anchor(m).is_some() {
                    let a = node_anchor(&n.node, "top_left");
                    let b = node_anchor(&n.node, "bottom_right");
                    let b = n
                        .transform
                        .transform_rect_bbox(Rect::new(a[0], a[1], b[0], b[1]));
                    let f = anchor(m).unwrap();
                    return Ok(point(
                        Point::new(b.x0 + f[0] * b.width(), b.y0 + f[1] * b.height()),
                        None,
                    ));
                }
                match m {
                    "bounds" => {
                        let a = node_anchor(&n.node, "top_left");
                        let b = node_anchor(&n.node, "bottom_right");
                        Ok(Geometry::Bounds(
                            n.transform
                                .transform_rect_bbox(Rect::new(a[0], a[1], b[0], b[1])),
                            Affine::IDENTITY,
                        ))
                    }
                    "path" => Ok(Geometry::Path(PathView {
                        paths: Rc::new(paths(&n, self, l)?),
                        route: None,
                        segment: None,
                    })),
                    "ink" => {
                        if !vector_only(&n.node) {
                            return Err(self.error("E_GEOMETRY", "ink 仅适用于矢量几何", l));
                        }
                        Ok(Geometry::Ink(n))
                    }
                    "plot_area" => {
                        let b = &n.node["plotBounds"];
                        if !b.is_object() {
                            return Err(self.error("E_TYPE", "plot_area 需要原生图表实例", l));
                        }
                        Ok(Geometry::Node(NodeView {
                            node: rectangle_node(Rect::new(
                                jnum(b, "x", 0.),
                                jnum(b, "y", 0.),
                                jnum(b, "x", 0.) + jnum(b, "width", 0.),
                                jnum(b, "y", 0.) + jnum(b, "height", 0.),
                            )),
                            transform: n.transform * node_frame(&n.node),
                        }))
                    }
                    "axes" => {
                        if !n.node["plotAxes"].is_object() {
                            return Err(self.error("E_TYPE", "axes 需要原生图表实例", l));
                        }
                        Ok(Geometry::Axes(n))
                    }
                    _ => Err(self.error("E_NAME", format!("未知几何属性 {m}"), l)),
                }
            }
            Geometry::Bounds(b, tr) => {
                if let Some(f) = anchor(m) {
                    Ok(point(
                        tr * Point::new(b.x0 + f[0] * b.width(), b.y0 + f[1] * b.height()),
                        None,
                    ))
                } else if matches!(m, "width" | "height") {
                    Ok(Geometry::Value(V::mm(if m == "width" {
                        b.width()
                    } else {
                        b.height()
                    })))
                } else {
                    Err(self.error("E_NAME", "矩形区域提供九点锚点与 width/height", l))
                }
            }
            Geometry::Ink(n) => {
                let mut node = n.node.clone();
                if node["fill"] == "none" && jnum(&node["strokeStyle"], "width", 0.) == 0. {
                    return Err(self.error("E_GEOMETRY", "绘制区域为空", l));
                }
                if node["opacity"] == 0. {
                    return Err(self.error("E_GEOMETRY", "绘制区域为空", l));
                }
                remove_invisible_ink(&mut node);
                let ps = if let Some(paths) = analytic_ink(
                    &NodeView {
                        node: node.clone(),
                        transform: n.transform,
                    },
                    self,
                    l,
                )? {
                    paths
                } else {
                    let path = crate::geometry::visible_transformed(&node, n.transform);
                    if path.elements().is_empty() {
                        return Err(self.error("E_GEOMETRY", "绘制区域为空", l));
                    }
                    Paths {
                        identity: String::new(),
                        routes: qp::from_bez(&path, Affine::IDENTITY),
                        frame: n.transform * node_frame(&node),
                        local: false,
                    }
                };
                let view = Geometry::Path(PathView {
                    paths: Rc::new(ps),
                    route: None,
                    segment: None,
                });
                if m == "boundary" {
                    Ok(view)
                } else if m == "bounds" {
                    self.geometry_member(view, "bounds", owner, l)
                } else {
                    Err(self.error("E_NAME", "ink 提供 bounds/boundary", l))
                }
            }
            Geometry::Path(p) if anchor(m).is_some() => {
                let bounds = self.geometry_member(Geometry::Path(p), "bounds", owner, l)?;
                self.geometry_member(bounds, m, owner, l)
            }
            Geometry::Path(p) => match m {
                "path" => Ok(Geometry::Path(p)),
                "bounds" => {
                    let mut ps = (*p.paths).clone();
                    if let Some(i) = p.route {
                        ps.routes = vec![ps.routes[i].clone()];
                    }
                    if let Some(s) = p.segment {
                        ps.routes[0].segments = vec![ps.routes[0].segments[s].clone()];
                    }
                    let b = ps
                        .bounds()
                        .ok_or_else(|| self.error("E_PATH", "路径为空", l))?;
                    Ok(Geometry::Bounds(
                        b,
                        if ps.local { ps.frame } else { Affine::IDENTITY },
                    ))
                }
                "subpaths" => Ok(Geometry::Subpaths(p)),
                "segments" => {
                    self.route_id(&p, l)?;
                    Ok(Geometry::Segments(p))
                }
                "nodes" => {
                    self.route_id(&p, l)?;
                    Ok(Geometry::Nodes(p))
                }
                "controls" => {
                    if p.segment.is_none() {
                        return Err(self.error("E_PATH", "controls 需要先选择曲线段", l));
                    }
                    Ok(Geometry::Controls(p))
                }
                "start" | "end" => {
                    let ri = self.route_id(&p, l)?;
                    let r = &p.paths.routes[ri];
                    if r.segments.is_empty() {
                        return Err(self.error("E_PATH", "路径为空", l));
                    }
                    let si = p.segment.unwrap_or(if m == "start" {
                        0
                    } else {
                        r.segments.len() - 1
                    });
                    let pp = PathPoint {
                        paths: p.paths,
                        position: Position {
                            route: ri,
                            segment: si,
                            t: if m == "start" { 0. } else { 1. },
                        },
                        side: Some(if m == "start" { "outgoing" } else { "incoming" }.into()),
                    };
                    Ok(point(pp.point(), Some(pp)))
                }
                _ => Err(self.error("E_NAME", format!("未知路径属性 {m}"), l)),
            },
            Geometry::Axis(n, name) => self.axis_member(n, &name, m, owner, l),
            Geometry::Anchor(a) => {
                let t = if matches!(m, "tangent" | "normal" | "tangent_angle") {
                    Some(a.tangent(self, l)?)
                } else {
                    None
                };
                match m {
                    "x" => Ok(Geometry::Value(V::mm(a.point.x))),
                    "y" => Ok(Geometry::Value(V::mm(a.point.y))),
                    "tangent_angle" => {
                        let t = t.unwrap();
                        Ok(Geometry::Value(V::Number(
                            t.y.atan2(t.x).to_degrees(),
                            "deg".into(),
                        )))
                    }
                    "tangent" | "normal" => {
                        let t = t.unwrap();
                        let v = if m == "normal" {
                            Vec2::new(t.y, -t.x)
                        } else {
                            t
                        };
                        Ok(Geometry::Value(V::List(vec![V::num(v.x), V::num(v.y)])))
                    }
                    _ => Err(self.error("E_NAME", format!("未知锚点属性 {m}"), l)),
                }
            }
            _ => Err(self.error("E_TYPE", "先选择集合中的元素，再访问属性", l)),
        }
    }
    fn route_id(&self, p: &PathView, l: Loc) -> Result<usize> {
        p.route
            .or_else(|| (p.paths.routes.len() == 1).then_some(0))
            .ok_or_else(|| self.error("E_PATH", "连续路径操作需要先选择 subpaths[i]", l))
    }
    fn integer(&self, v: &V, l: Loc) -> Result<usize> {
        let i = self.scalar(v, l)?;
        if i < 0. || i.fract() != 0. {
            return Err(self.error("E_INDEX", "索引需要非负整数", l));
        }
        Ok(i as usize)
    }
    fn geometry_index(&self, v: Geometry, i: &V, owner: usize, l: Loc) -> Result<Geometry> {
        if let Geometry::Axes(n) = v {
            let name = i.as_str();
            if !n.node["plotAxes"][&name].is_object() {
                return Err(self.error("E_PLOT", "坐标轴不存在", l));
            }
            return Ok(Geometry::Axis(n, name));
        }
        if let Geometry::Decorations(n, prefix) = v {
            let i = self.integer(i, l)?;
            let bs = n.node["plotDecorations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| jstr(b, "name", "").starts_with(&prefix))
                .collect::<Vec<_>>();
            let b = bs
                .get(i)
                .ok_or_else(|| self.error("E_INDEX", "刻度标签索引越界", l))?;
            return Ok(Geometry::Node(NodeView {
                node: rectangle_node(Rect::new(
                    jnum(b, "left", 0.),
                    jnum(b, "top", 0.),
                    jnum(b, "right", 0.),
                    jnum(b, "bottom", 0.),
                )),
                transform: n.transform * node_frame(&n.node),
            }));
        }
        let i = self.integer(i, l)?;
        match v {
            Geometry::Value(V::List(values)) => Ok(Geometry::Value(
                values
                    .get(i)
                    .cloned()
                    .ok_or_else(|| self.error("E_INDEX", "方向索引越界", l))?,
            )),
            Geometry::Collection(v) => {
                Ok(Geometry::Anchor(v.get(i).cloned().ok_or_else(|| {
                    self.error("E_INDEX", "锚点候选索引越界", l)
                })?))
            }
            Geometry::Subpaths(mut p) => {
                if i >= p.paths.routes.len() {
                    return Err(self.error("E_INDEX", "子路径索引越界", l));
                }
                p.route = Some(i);
                p.segment = None;
                Ok(Geometry::Path(p))
            }
            Geometry::Segments(mut p) => {
                let r = self.route_id(&p, l)?;
                if i >= p.paths.routes[r].segments.len() {
                    return Err(self.error("E_INDEX", "路径段索引越界", l));
                }
                p.route = Some(r);
                p.segment = Some(i);
                Ok(Geometry::Path(p))
            }
            Geometry::Nodes(p) => {
                let r = self.route_id(&p, l)?;
                let route = &p.paths.routes[r];
                let count = route.segments.len() + if route.closed { 0 } else { 1 };
                if i >= count {
                    return Err(self.error("E_INDEX", "节点索引越界", l));
                }
                let pp = PathPoint {
                    paths: p.paths.clone(),
                    position: Position {
                        route: r,
                        segment: if i < route.segments.len() { i } else { i - 1 },
                        t: if i < route.segments.len() { 0. } else { 1. },
                    },
                    side: None,
                };
                Ok(Geometry::Anchor(AnchorPoint {
                    point: pp.point(),
                    owner,
                    path: Some(pp),
                }))
            }
            Geometry::Controls(p) => {
                let r = self.route_id(&p, l)?;
                let controls = p.paths.routes[r].segments[p.segment.unwrap()].controls();
                let p = controls
                    .get(i)
                    .ok_or_else(|| self.error("E_INDEX", "控制点索引越界", l))?;
                Ok(Geometry::Anchor(AnchorPoint {
                    point: *p,
                    owner,
                    path: None,
                }))
            }
            _ => Err(self.error("E_TYPE", "此几何值不能索引", l)),
        }
    }
    fn anchor_position(
        &self,
        v: &V,
        binding: Option<(&Json, usize)>,
        owner: usize,
        l: Loc,
    ) -> Result<Point> {
        if let V::List(v) = v {
            if v.len() != 2 {
                return Err(self.error("E_ARG", "坐标需要两个物理长度", l));
            }
            return Ok(Point::new(self.len(&v[0], l)?, self.len(&v[1], l)?));
        }
        let p = self.geometry_anchor(v, binding, l)?;
        if p.owner != owner {
            return Err(self.error("E_LAYOUT", "查询参考点须位于同一容器", l));
        }
        Ok(p.point)
    }
    fn geometry_call(
        &self,
        v: Geometry,
        m: &str,
        pos: &[V],
        a: &Args,
        owner: usize,
        binding: Option<(&Json, usize)>,
        l: Loc,
    ) -> Result<Geometry> {
        if let Geometry::Node(n) = &v {
            if matches!(m, "data" | "axis") {
                let p = crate::plot::query_anchor(self, &n.node, m, pos, a, l)?;
                return Ok(Geometry::Anchor(AnchorPoint {
                    point: n.transform * p,
                    owner,
                    path: None,
                }));
            }
        }
        if let Geometry::Anchor(mut p) = v {
            if pos.len() + a.len() != 1 || a.keys().any(|k| k != "side") {
                return Err(self.error("E_ARG", "with_side 需要一个 side 参数", l));
            }
            if m != "with_side" {
                return Err(self.error("E_NAME", "锚点仅支持 with_side", l));
            }
            let side = pos
                .first()
                .or_else(|| a.get("side"))
                .map(V::as_str)
                .unwrap_or_default();
            if !matches!(side.as_str(), "incoming" | "outgoing") {
                return Err(self.error("E_ARG", "side 须为 incoming/outgoing", l));
            }
            p.path
                .as_mut()
                .ok_or_else(|| self.error("E_ANCHOR_DIRECTION", "此锚点没有路径方向", l))?
                .side = Some(side);
            return Ok(Geometry::Anchor(p));
        }
        let Geometry::Path(mut view) = v else {
            return Err(self.error("E_CALL", "此几何视图没有该方法", l));
        };
        if m == "in_space" {
            if pos.len() + a.len() != 1 || a.keys().any(|k| k != "space") {
                return Err(self.error("E_ARG", "in_space 需要一个 space 参数", l));
            }
            let space = pos
                .first()
                .or_else(|| a.get("space"))
                .map(V::as_str)
                .unwrap_or_default();
            if !matches!(space.as_str(), "local" | "parent") {
                return Err(self.error("E_ARG", "space 须为 local/parent", l));
            }
            let mut paths = (*view.paths).clone();
            if paths.frame.determinant().abs() < 1e-20 {
                return Err(self.error("E_GEOMETRY", "变换不可逆", l));
            }
            paths.local = space == "local";
            view.paths = Rc::new(paths);
            return Ok(Geometry::Path(view));
        }
        if m == "between" {
            return self.path_between(view, pos, a, owner, binding, l);
        }
        let chosen = self.selected_segments(&view);
        if m == "at" {
            if !pos.is_empty() {
                return Err(self.error("E_ARG", "at 使用具名参数", l));
            }
            let allowed = ["fraction", "distance", "t"];
            if a.keys().any(|k| !allowed.contains(&k.as_str())) || a.len() != 1 {
                return Err(self.error("E_ARG", "at 恰好选择 fraction、distance、t 之一", l));
            }
            let _ = self.route_id(&view, l)?;
            let position = if let Some(v) = a.get("t") {
                if view.segment.is_none() {
                    return Err(self.error("E_PATH", "t 需要先选择曲线段", l));
                }
                let t = self.scalar(v, l)?;
                if !(0.0..=1.).contains(&t) {
                    return Err(self.error("E_PATH", "t 超出 0..1", l));
                }
                Position {
                    route: view.route.unwrap_or(0),
                    segment: view.segment.unwrap(),
                    t,
                }
            } else {
                let lengths = chosen
                    .iter()
                    .map(|(_, s)| s.measured(&view.paths).length(1.))
                    .collect::<Vec<_>>();
                let total = lengths.iter().sum::<f64>();
                let mut distance = if let Some(v) = a.get("fraction") {
                    let f = self.scalar(v, l)?;
                    if !(0.0..=1.).contains(&f) {
                        return Err(self.error("E_PATH", "fraction 超出 0..1", l));
                    }
                    total * f
                } else {
                    self.len(a.get("distance").unwrap(), l)?
                };
                if distance < 0. || distance > total + qp::EPS || (total == 0. && distance != 0.) {
                    return Err(self.error("E_PATH", "distance 超出路径范围", l));
                }
                let mut p = chosen.last().unwrap().0;
                p.t = 1.;
                for ((position, s), length) in chosen.iter().zip(lengths) {
                    if distance <= length + 1e-12 {
                        p = *position;
                        p.t = s.measured(&view.paths).parameter_at_length(distance);
                        break;
                    }
                    distance -= length;
                }
                p
            };
            let pp = PathPoint {
                paths: view.paths,
                position,
                side: None,
            };
            return Ok(Geometry::Anchor(AnchorPoint {
                point: pp.point(),
                owner,
                path: Some(pp),
            }));
        }
        if !pos.is_empty() && !(m == "intersections" && pos.len() == 1) {
            return Err(self.error("E_ARG", "搜索查询使用具名参数", l));
        }
        let mut candidates = vec![];
        if m == "nearest" {
            if a.len() != 1 || !a.contains_key("to") {
                return Err(self.error("E_ARG", "nearest 需要 to", l));
            }
            let p = view
                .paths
                .metric(self.anchor_position(&a["to"], binding, owner, l)?);
            let mut scored = vec![];
            let mut infinite = vec![];
            for (mut position, s) in chosen {
                let s = s.measured(&view.paths);
                let roots = match s.nearest(p) {
                    Ok(roots) => roots,
                    Err(message) => {
                        infinite.push((s.point(0.).distance(p), message));
                        continue;
                    }
                };
                for t in roots {
                    position.t = t;
                    scored.push((position, s.point(t).distance(p)));
                }
            }
            let best = scored
                .iter()
                .map(|(_, d)| *d)
                .chain(infinite.iter().map(|(d, _)| *d))
                .fold(f64::INFINITY, f64::min);
            if let Some((_, message)) = infinite.into_iter().find(|(d, _)| *d <= best + qp::EPS) {
                return Err(self.error("E_GEOMETRY", message, l));
            }
            candidates.extend(
                scored
                    .into_iter()
                    .filter(|(_, d)| *d <= best + qp::EPS)
                    .map(|(p, _)| p),
            );
        } else if m == "intersections" {
            if pos.len() + a.len() != 1 || a.keys().any(|k| k != "ray") {
                return Err(self.error("E_ARG", "intersections 需要一个 ray 参数", l));
            }
            let ray = a
                .get("ray")
                .or_else(|| pos.first())
                .ok_or_else(|| self.error("E_ARG", "intersections 需要 ray", l))?;
            let V::Map(ray) = ray else {
                return Err(self.error("E_ARG", "需要 ray(...) 查询射线", l));
            };
            let origin_value = ray
                .get("origin")
                .ok_or_else(|| self.error("E_ARG", "ray 缺少 origin", l))?;
            let direction_value = ray
                .get("direction")
                .ok_or_else(|| self.error("E_ARG", "ray 缺少 direction", l))?;
            let origin =
                view.paths
                    .metric(self.anchor_position(origin_value, binding, owner, l)?);
            let d = direction_value.list();
            if d.len() != 2 {
                return Err(self.error("E_ARG", "direction 需要两个无单位数值", l));
            }
            let direction = Vec2::new(self.scalar(&d[0], l)?, self.scalar(&d[1], l)?);
            if direction.hypot() < 1e-12 {
                return Err(self.error("E_ARG", "射线方向不能为零", l));
            }
            for (mut p, s) in chosen {
                for t in s
                    .measured(&view.paths)
                    .intersections(origin, direction)
                    .map_err(|m| self.error("E_GEOMETRY", m, l))?
                {
                    p.t = t;
                    candidates.push(p);
                }
            }
        } else if matches!(m, "extrema" | "inflections" | "corners") {
            let axis = string(a, "axis", "y");
            if m == "extrema" && !matches!(axis.as_str(), "x" | "y") {
                return Err(self.error("E_ARG", "axis 须为 x/y", l));
            }
            if a.keys().any(|k| m != "extrema" || k != "axis") {
                return Err(self.error("E_ARG", "未知特征查询参数", l));
            }
            for (mut p, s) in chosen.iter().copied() {
                let s = s.measured(&view.paths);
                let ts = if m == "extrema" {
                    s.extrema(usize::from(axis == "y"))
                } else if m == "inflections" {
                    s.inflections()
                } else {
                    vec![]
                };
                for t in ts {
                    p.t = t;
                    candidates.push(p);
                }
            }
            for ri in view
                .route
                .map(|i| vec![i])
                .unwrap_or_else(|| (0..view.paths.routes.len()).collect())
            {
                let r = &view.paths.routes[ri];
                let count = r.segments.len();
                for i in if r.closed { 0..count } else { 1..count } {
                    if view.segment.is_some() {
                        continue;
                    }
                    let before = r.segments[(i + count - 1) % count].measured(&view.paths);
                    let after = r.segments[i].measured(&view.paths);
                    let vin = before.deriv(1. - 1e-8);
                    let vout = after.deriv(1e-8);
                    let smooth = vin.hypot() > 1e-12
                        && vout.hypot() > 1e-12
                        && (vin / vin.hypot() - vout / vout.hypot()).hypot() < 1e-6;
                    let take = if m == "corners" {
                        !smooth
                    } else if m == "extrema" {
                        let get = |v: Vec2| if axis == "x" { v.x } else { v.y };
                        get(vin) * get(vout) < 0.
                    } else {
                        smooth
                            && vin.cross(before.second(1. - 1e-8)) * vout.cross(after.second(1e-8))
                                < 0.
                    };
                    if take {
                        candidates.push(Position {
                            route: ri,
                            segment: i,
                            t: 0.,
                        });
                    }
                }
            }
        } else {
            return Err(self.error("E_NAME", format!("未知路径方法 {m}"), l));
        }
        let points = view
            .paths
            .ordered(candidates)
            .into_iter()
            .map(|position| {
                let p = PathPoint {
                    paths: view.paths.clone(),
                    position,
                    side: None,
                };
                AnchorPoint {
                    point: p.point(),
                    owner,
                    path: Some(p),
                }
            })
            .collect();
        Ok(Geometry::Collection(points))
    }
    fn selected_segments<'a>(&self, p: &'a PathView) -> Vec<(Position, &'a Segment)> {
        let mut out = vec![];
        for (ri, r) in p.paths.routes.iter().enumerate() {
            if p.route.is_some_and(|i| i != ri) {
                continue;
            }
            for (si, s) in r.segments.iter().enumerate() {
                if p.segment.is_some_and(|i| i != si) {
                    continue;
                }
                out.push((
                    Position {
                        route: ri,
                        segment: si,
                        t: 0.,
                    },
                    s,
                ));
            }
        }
        out
    }
    fn path_between(
        &self,
        p: PathView,
        pos: &[V],
        a: &Args,
        _owner: usize,
        binding: Option<(&Json, usize)>,
        l: Loc,
    ) -> Result<Geometry> {
        if pos.len() != 2 || a.keys().any(|k| k != "wrap") {
            return Err(self.error("E_ARG", "between 需要两个路径锚点，可选 wrap=true", l));
        }
        if a.get("wrap").is_some_and(|v| !matches!(v, V::Bool(_))) {
            return Err(self.error("E_ARG", "wrap 须为布尔值", l));
        }
        let ri = self.route_id(&p, l)?;
        let left = self
            .geometry_anchor(&pos[0], binding, l)?
            .path
            .ok_or_else(|| self.error("E_PATH", "between 端点须在路径上", l))?;
        let right = self
            .geometry_anchor(&pos[1], binding, l)?
            .path
            .ok_or_else(|| self.error("E_PATH", "between 端点须在路径上", l))?;
        if left.paths.identity != p.paths.identity
            || right.paths.identity != p.paths.identity
            || left.position.route != ri
            || right.position.route != ri
            || left.paths.routes[ri].segments.len() != p.paths.routes[ri].segments.len()
            || right.paths.routes[ri].segments.len() != p.paths.routes[ri].segments.len()
        {
            return Err(self.error("E_PATH", "between 端点须来自同一子路径", l));
        }
        let route = &p.paths.routes[ri];
        let start = left.position.segment as f64 + left.position.t;
        let mut end = right.position.segment as f64 + right.position.t;
        let wrap = yes(a, "wrap", false);
        if end < start {
            if !route.closed || !wrap {
                return Err(self.error("E_PATH", "反向区间或跨闭合接缝须显式 wrap=true", l));
            }
            end += route.segments.len() as f64;
        }
        let mut segments = vec![];
        let n = route.segments.len();
        for i in start.floor() as usize..end.ceil() as usize {
            let lo = (start - i as f64).max(0.);
            let hi = (end - i as f64).min(1.);
            if hi <= lo {
                continue;
            }
            let s = &route.segments[i % n];
            let curve = match s.curve {
                Curve::Poly(c) => {
                    use kurbo::ParamCurve;
                    Curve::Poly(c.subsegment(lo..hi))
                }
                Curve::Arc(mut c) => {
                    c.start_angle += c.sweep_angle * lo;
                    c.sweep_angle *= hi - lo;
                    Curve::Arc(c)
                }
            };
            segments.push(Segment {
                curve,
                transform: s.transform,
            });
        }
        Ok(Geometry::Path(PathView {
            paths: Rc::new(Paths {
                identity: format!(
                    "{}:between:{:?}:{:?}",
                    p.paths.identity, left.position, right.position
                ),
                routes: vec![Route {
                    zero_direction: route.zero_direction,
                    segments,
                    closed: false,
                }],
                frame: p.paths.frame,
                local: p.paths.local,
            }),
            route: Some(0),
            segment: None,
        }))
    }
    fn axis_member(
        &self,
        n: NodeView,
        name: &str,
        m: &str,
        owner: usize,
        l: Loc,
    ) -> Result<Geometry> {
        let axis = &n.node["plotAxes"][name];
        let area = &n.node["plotBounds"];
        let side = jstr(axis, "side", "bottom");
        let horizontal = matches!(side, "bottom" | "top");
        let offset = jnum(axis, "offset", 0.);
        let coord = |v: f64| {
            if horizontal {
                Point::new(
                    jnum(area, "x", 0.) + v,
                    jnum(area, "y", 0.)
                        + if side == "top" {
                            -offset
                        } else {
                            jnum(area, "height", 0.) + offset
                        },
                )
            } else {
                Point::new(
                    jnum(area, "x", 0.)
                        + if side == "left" {
                            -offset
                        } else {
                            jnum(area, "width", 0.) + offset
                        },
                    jnum(area, "y", 0.) + v,
                )
            }
        };
        let tr = n.transform * node_frame(&n.node);
        let fallback = if horizontal {
            [0., jnum(area, "width", 0.)]
        } else {
            [jnum(area, "height", 0.), 0.]
        };
        let segments = axis["segments"].as_array().cloned().unwrap_or_else(|| {
            vec![json!({"range":if axis["reverse"]==true{[fallback[1],fallback[0]]}else{fallback}})]
        });
        let mut routes = vec![];
        for s in &segments {
            let p = &s["range"];
            routes.push(Route {
                zero_direction: None,
                segments: vec![Segment {
                    curve: Curve::Poly(PathSeg::Line(Line::new(
                        coord(p[0].as_f64().unwrap()),
                        coord(p[1].as_f64().unwrap()),
                    ))),
                    transform: tr,
                }],
                closed: false,
            });
        }
        let path = PathView {
            paths: Rc::new(Paths {
                identity: String::new(),
                routes,
                frame: tr,
                local: false,
            }),
            route: None,
            segment: None,
        };
        if m == "spine" {
            return Ok(Geometry::Path(path));
        }
        if m == "min" || m == "max" {
            let ri = if m == "min" {
                0
            } else {
                path.paths.routes.len() - 1
            };
            let pp = PathPoint {
                paths: path.paths,
                position: Position {
                    route: ri,
                    segment: 0,
                    t: if m == "min" { 0. } else { 1. },
                },
                side: Some(if m == "min" { "outgoing" } else { "incoming" }.into()),
            };
            return Ok(Geometry::Anchor(AnchorPoint {
                point: pp.point(),
                owner,
                path: Some(pp),
            }));
        }
        let decorations = n.node["plotDecorations"]
            .as_array()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if m == "ticks" {
            return Ok(Geometry::Decorations(n, format!("{name}.tick:")));
        }
        let mut b = Rect::from_points(coord(fallback[0]), coord(fallback[1]));
        if m == "label" || m == "exponent" {
            let d = decorations
                .iter()
                .find(|d| jstr(d, "name", "") == format!("{name}.{m}"))
                .ok_or_else(|| self.error("E_PLOT", format!("坐标轴没有 {m}"), l))?;
            b = Rect::new(
                jnum(d, "left", 0.),
                jnum(d, "top", 0.),
                jnum(d, "right", 0.),
                jnum(d, "bottom", 0.),
            );
            return Ok(Geometry::Node(NodeView {
                node: rectangle_node(b),
                transform: tr,
            }));
        }
        for d in decorations {
            if jstr(d, "name", "").starts_with(&format!("{name}.")) {
                b = b.union(Rect::new(
                    jnum(d, "left", 0.),
                    jnum(d, "top", 0.),
                    jnum(d, "right", 0.),
                    jnum(d, "bottom", 0.),
                ));
            }
        }
        self.geometry_member(
            Geometry::Node(NodeView {
                node: rectangle_node(b),
                transform: tr,
            }),
            m,
            owner,
            l,
        )
    }
}
