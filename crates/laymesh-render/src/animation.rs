//! Shared presentation geometry, native Skia rendering and retained sampling.
use crate::{display_list::*, error};
#[cfg(feature = "native")]
use crate::{
    display_update::DisplayUpdate,
    raster::{PreparedRaster, RasterFrame},
};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveArclen, PathEl, PathSeg, Point, Rect, Shape};
use laymesh_core::{
    Result,
    animation::values::{Matrix, identity},
};
#[cfg(feature = "native")]
pub struct AnimationRenderer {
    scene: crate::retained::RetainedScene,
    drawing: PreparedRaster,
}
#[cfg(feature = "native")]
impl AnimationRenderer {
    pub fn new(base: DisplayList) -> Result<Self> {
        Ok(Self {
            drawing: PreparedRaster::new(base.clone())?,
            scene: crate::retained::RetainedScene::new(base),
        })
    }
    pub fn base(&self) -> &DisplayList {
        self.scene.base()
    }
    pub fn replace(&mut self, base: DisplayList) -> Result<()> {
        self.drawing
            .update(DisplayUpdate::between(self.drawing.display_list(), &base))?;
        self.scene = crate::retained::RetainedScene::new(base);
        Ok(())
    }
    pub fn update(&mut self, update: DisplayUpdate) -> Result<()> {
        self.drawing.update(update.clone())?;
        self.scene.update(update)
    }
    pub fn set_cache_budget(&mut self, mib: usize) {
        self.drawing.set_cache_budget(mib);
    }
    pub fn render(
        &mut self,
        r: Rect,
        w: u32,
        h: u32,
        t: Option<f64>,
        reduced: bool,
    ) -> Result<RasterFrame> {
        self.render_environment(r, w, h, t, reduced, false)
    }
    pub fn render_environment(
        &mut self,
        r: Rect,
        w: u32,
        h: u32,
        t: Option<f64>,
        reduced: bool,
        print: bool,
    ) -> Result<RasterFrame> {
        let states = t
            .map(|t| self.scene.sample(t, reduced, print))
            .transpose()?
            .unwrap_or_default();
        self.drawing.render_states(r, w, h, &states)
    }
}
pub(crate) fn affine_matrix(a: Affine) -> Matrix {
    let [a, b, c, d, e, f] = a.as_coeffs();
    let mut m = identity();
    m[0] = a;
    m[4] = b;
    m[1] = c;
    m[5] = d;
    m[3] = e;
    m[7] = f;
    m
}
pub(crate) fn back_facing(m: Matrix) -> bool {
    laymesh_core::animation::matrix::inverse(m).is_some_and(|inv| inv[10] < 0.)
}
pub(crate) fn trim_paths(node: &mut DrawNode, progress: f64) -> Result<()> {
    fn total(n: &DrawNode) -> f64 {
        if let Some(d) = n.hint["drawGeometry"]["d"].as_str() {
            return BezPath::from_svg(d)
                .map(|p| p.segments().map(|s| s.arclen(1e-6)).sum())
                .unwrap_or(0.);
        }
        match &n.kind {
            DrawKind::Path {
                path,
                fill: Paint::None,
                stroke: Some(_),
                ..
            } => path.segments().map(|s| s.arclen(1e-6)).sum(),
            DrawKind::Group { children, .. } => children.iter().map(total).sum(),
            _ => 0.,
        }
    }
    fn trim(n: &mut DrawNode, left: &mut f64) -> Result<()> {
        if let Some(d) = n.hint["drawGeometry"]["d"].as_str() {
            let path = BezPath::from_svg(d).map_err(|_| error("Invalid draw geometry"))?;
            let length: f64 = path.segments().map(|s| s.arclen(1e-6)).sum();
            if *left >= length {
                *left -= length;
                return Ok(());
            }
            let mut simple = DrawNode::new(DrawKind::Path {
                path,
                fill: Paint::None,
                stroke: Some(Stroke {
                    rgba: [0., 0., 0., 1.],
                    width: 1.,
                    cap: "butt".into(),
                    join: "miter".into(),
                    miter: 4.,
                    dash: vec![],
                    offset: 0.,
                }),
                even_odd: false,
            });
            // Trim in page coordinates so differently scaled group children have
            // a consistent drawing speed, then regenerate physical endpoint ink.
            trim(&mut simple, left)?;
            let DrawKind::Path { path, .. } = simple.kind else {
                unreachable!()
            };
            let g = &n.hint["drawGeometry"];
            let matrix =
                |key: &str| Affine::new(std::array::from_fn(|i| g[key][i].as_f64().unwrap_or(0.)));
            let tr = matrix("transform");
            let local = matrix("container").inverse();
            let recipe = serde_json::json!({"d":(tr.inverse()*path).to_svg(),"fill":"none",
                "strokeStyle":g["strokeStyle"], "endpointRecipe":g["endpointRecipe"]});
            let mut children = vec![];
            for layer in laymesh_core::endpoints::layers(&recipe, tr).map_err(error)? {
                let mut child = DrawNode::new(DrawKind::Path {
                    path: local * layer.path,
                    fill: crate::display_list::solid(&layer.color)?,
                    stroke: None,
                    even_odd: false,
                });
                child.opacity = layer.opacity;
                children.push(child);
            }
            n.kind = DrawKind::Group {
                children,
                clip: None,
                mask: None,
            };
            return Ok(());
        }
        match &mut n.kind {
            DrawKind::Path {
                path,
                fill: Paint::None,
                stroke: Some(_),
                ..
            } => {
                let length: f64 = path.segments().map(|s| s.arclen(1e-6)).sum();
                if *left >= length {
                    *left -= length;
                    return Ok(());
                }
                let mut out = BezPath::new();
                let mut current = Point::ORIGIN;
                let mut start = current;
                for element in path.elements() {
                    if *left <= 0. {
                        break;
                    }
                    let (seg, closing) = match *element {
                        PathEl::MoveTo(p) => {
                            out.move_to(p);
                            current = p;
                            start = p;
                            continue;
                        }
                        PathEl::LineTo(p) => (kurbo::Line::new(current, p).into(), false),
                        PathEl::QuadTo(c, p) => (kurbo::QuadBez::new(current, c, p).into(), false),
                        PathEl::CurveTo(c1, c2, p) => {
                            (kurbo::CubicBez::new(current, c1, c2, p).into(), false)
                        }
                        PathEl::ClosePath => (kurbo::Line::new(current, start).into(), true),
                    };
                    let seg: PathSeg = seg;
                    let length = seg.arclen(1e-6);
                    let full = *left >= length;
                    let t = if full {
                        1.
                    } else {
                        seg.inv_arclen(*left, 1e-6)
                    };
                    let segment = seg.subsegment(0.0..t);
                    if closing && full {
                        out.close_path();
                    } else {
                        match segment {
                            PathSeg::Line(l) => out.line_to(l.p1),
                            PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
                            PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
                        }
                    }
                    current = segment.end();
                    *left -= length;
                }
                *path = out;
            }
            DrawKind::Group { children, .. } => {
                for c in children {
                    trim(c, left)?
                }
            }
            _ => {}
        }
        Ok(())
    }
    if progress >= 1. {
        return Ok(());
    }
    let length = total(node);
    if length <= 0. {
        return Ok(()); // Legacy packages may contain only filled endpoint outlines.
    }
    let mut left = length * progress;
    trim(node, &mut left)?;
    Ok(())
}
pub(crate) fn reveal(node: &mut DrawNode, progress: f64) {
    let size = node.hint["animationSize"]
        .as_array()
        .map(|v| [v[0].as_f64().unwrap_or(0.), v[1].as_f64().unwrap_or(0.)]);
    let b = node.hint["plotBounds"].clone();
    if b.is_object() {
        // Plot data already lives in clipped groups. Tighten those groups only,
        // leaving axes, ticks and legends outside the reveal mask.
        fn mask(n: &mut DrawNode, p: f64) {
            if let DrawKind::Group { children, clip, .. } = &mut n.kind {
                if let Some(c) = clip {
                    let b = c.path.bounding_box();
                    c.path = Rect::new(b.x0, b.y0, b.x0 + b.width() * p, b.y1).to_path(0.001);
                } else {
                    for c in children {
                        mask(c, p);
                    }
                }
            }
        }
        mask(node, progress);
        return;
    }
    let b = size
        .map(|s| Rect::new(0., 0., s[0], s[1]))
        .unwrap_or_else(|| node.local_bounds());
    let mut inner = node.clone();
    let layout = node.hint["animationLayout"]
        .as_array()
        .filter(|v| v.len() == 6)
        .map(|v| Affine::new(std::array::from_fn(|i| v[i].as_f64().unwrap_or(0.))))
        .unwrap_or(node.transform);
    inner.transform = layout.inverse() * node.transform;
    node.transform = layout;
    inner.opacity = 1.;
    inner.effects.clear();
    inner.name.clear();
    // The wrapper is presentation-only. Children must retain their scene IDs
    // so a simultaneous child animation is not applied to the wrapper itself.
    node.hint["animationRevealWrapper"] = serde_json::Value::Bool(true);
    node.kind = DrawKind::Group {
        children: vec![inner],
        clip: Some(Clip {
            path: Rect::new(b.x0, b.y0, b.x0 + b.width() * progress, b.y1).to_path(0.001),
            even_odd: false,
        }),
        mask: None,
    };
}
