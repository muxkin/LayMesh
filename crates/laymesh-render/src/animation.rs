//! Shared presentation geometry, native Skia rendering and retained sampling.
use crate::{display_list::*, error};
#[cfg(feature = "native")]
use crate::{
    display_update::DisplayUpdate,
    raster::{PreparedRaster, RasterFrame},
};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveArclen, PathSeg, Rect, Shape};
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
    pub fn set_media_frame(&mut self, resource:&str, width:u32, height:u32, rgba:&[u8])->Result<()> {
        self.drawing.set_media_frame(resource,width,height,rgba)
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
    fn trim(n: &mut DrawNode, left: &mut f64) {
        match &mut n.kind {
            DrawKind::Path {
                path,
                fill: Paint::None,
                stroke: Some(_),
                ..
            } => {
                let mut out = BezPath::new();
                let mut last = None;
                for seg in path.segments() {
                    let length = seg.arclen(1e-6);
                    if *left <= 0. {
                        break;
                    }
                    let t = if *left >= length {
                        1.
                    } else {
                        seg.inv_arclen(*left, 1e-6)
                    };
                    let segment = seg.subsegment(0.0..t);
                    if last != Some(segment.start()) {
                        out.move_to(segment.start());
                    }
                    match segment {
                        PathSeg::Line(l) => out.line_to(l.p1),
                        PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
                        PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
                    }
                    last = Some(segment.end());
                    *left -= length;
                }
                *path = out;
            }
            DrawKind::Group { children, .. } => {
                for c in children {
                    trim(c, left)
                }
            }
            _ => {}
        }
    }
    let length = total(node);
    if length <= 0. {
        return Err(error("draw requires an open stroked line or curve"));
    }
    let mut left = length * progress;
    trim(node, &mut left);
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
