//! Shared native/WASM frame sampling. Projective effects flatten only their
//! target subtree; ordinary transforms stay vector until rasterization.
use crate::{
    display_list::*,
    display_update::DisplayUpdate,
    error,
    raster::{PreparedRaster, RasterFrame},
};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveArclen, PathSeg, Rect, Shape};
use laymesh_core::{
    Result,
    animation::{
        FrameStyle,
        values::{Matrix, identity, multiply, translate},
    },
};
use std::collections::BTreeMap;

pub struct AnimationRenderer {
    base: DisplayList,
    drawing: PreparedRaster,
    sources: BTreeMap<String, (String, RasterFrame, Rect)>,
    last: Option<(u64, bool, bool, u64)>,
    resource_view: std::sync::Arc<BTreeMap<String, Resource>>,
    projected: bool,
    source_budget: usize,
}
impl AnimationRenderer {
    pub fn new(base: DisplayList) -> Result<Self> {
        Ok(Self {
            drawing: PreparedRaster::new(base.clone())?,
            resource_view: std::sync::Arc::new(base.resources.clone()),
            base,
            sources: BTreeMap::new(),
            last: None,
            projected: false,
            source_budget: 64 * 1024 * 1024,
        })
    }
    pub fn base(&self) -> &DisplayList {
        &self.base
    }
    pub fn replace(&mut self, base: DisplayList) -> Result<()> {
        self.drawing
            .update(DisplayUpdate::between(self.drawing.display_list(), &base))?;
        self.resource_view = std::sync::Arc::new(base.resources.clone());
        self.base = base;
        self.sources.clear();
        self.last = None;
        self.projected = false;
        Ok(())
    }
    pub fn update(&mut self, update: DisplayUpdate) -> Result<()> {
        let mut next = self.base.clone();
        update.apply(&mut next)?;
        self.replace(next)
    }
    pub fn set_cache_budget(&mut self, mib: usize) {
        self.drawing.set_cache_budget(mib);
        self.source_budget = mib.saturating_mul(1024 * 1024);
        if self.sources.values().map(|v| v.1.rgba.len()).sum::<usize>() > self.source_budget {
            self.sources.clear();
        }
    }
    pub fn render(
        &mut self,
        rect: Rect,
        width: u32,
        height: u32,
        time: Option<f64>,
        reduced: bool,
    ) -> Result<RasterFrame> {
        self.render_environment(rect, width, height, time, reduced, false)
    }
    pub fn render_environment(
        &mut self,
        rect: Rect,
        width: u32,
        height: u32,
        time: Option<f64>,
        reduced: bool,
        print: bool,
    ) -> Result<RasterFrame> {
        if width == 0
            || height == 0
            || !rect.is_finite()
            || rect.width() <= 0.
            || rect.height() <= 0.
        {
            return Err(error("Invalid animation viewport"));
        }
        if let Some(t) = time {
            if !t.is_finite() || t < 0. {
                return Err(error("Animation time must be finite and nonnegative"));
            }
        }
        let density = (width as f64 / rect.width()).max(height as f64 / rect.height());
        // Projected resources are clipped to the complete canvas, independent
        // of viewport panning. Density changes invalidate projection sampling.
        let key = time.map(|t| (t.to_bits(), reduced, print, density.to_bits()));
        if key != self.last {
            let mut next = DisplayList {
                animation: None,
                schema_version: self.base.schema_version,
                width: self.base.width,
                height: self.base.height,
                nodes: self.base.nodes.clone(),
                resources: BTreeMap::new(),
            };
            if let (Some(time), Some(animation)) = (time, &self.base.animation) {
                let states = animation.timeline_with_media(reduced, print).sample(time);
                let viewport = Rect::new(0., 0., self.base.width, self.base.height);
                let resources = self.resource_view.clone();
                for node in &mut next.nodes {
                    self.sample_node(
                        node,
                        Affine::IDENTITY,
                        &states,
                        &resources,
                        &mut next.resources,
                        viewport,
                        density,
                    )?;
                }
            }
            let projected = !next.resources.is_empty();
            let mut update = DisplayUpdate::between(self.drawing.display_list(), &next);
            update.resources = if projected || self.projected {
                let mut resources = self.base.resources.clone();
                resources.extend(next.resources);
                Some(resources)
            } else {
                None
            };
            self.drawing.update(update)?;
            self.projected = projected;
            self.last = key;
        }
        self.drawing.render_retained_region(rect, width, height)
    }
    fn sample_node(
        &mut self,
        node: &mut DrawNode,
        parent: Affine,
        states: &BTreeMap<String, FrameStyle>,
        resources: &BTreeMap<String, Resource>,
        output: &mut BTreeMap<String, Resource>,
        viewport: Rect,
        density: f64,
    ) -> Result<()> {
        if let DrawKind::Group { children, .. } = &mut node.kind {
            for child in children {
                self.sample_node(
                    child,
                    parent * node.transform,
                    states,
                    resources,
                    output,
                    viewport,
                    density,
                )?;
            }
        }
        let Some(style) = states.get(&node.name) else {
            return Ok(());
        };
        if let Some(opacity) = style.opacity {
            node.opacity = opacity;
        }
        if style.visible == Some(false) {
            node.opacity = 0.;
        }
        if let Some(progress) = style.draw {
            trim_paths(node, progress)?;
        }
        if let Some(progress) = style.reveal {
            reveal(node, progress);
        }
        let Some(m) = style.transform else {
            return Ok(());
        };
        let origin = style.origin.unwrap_or([0.; 3]);
        let m = multiply(
            multiply(translate(origin[0], origin[1], origin[2]), m),
            translate(-origin[0], -origin[1], -origin[2]),
        );
        let layout = node.hint["animationLayout"]
            .as_array()
            .filter(|v| v.len() == 6)
            .map(|v| Affine::new(std::array::from_fn(|i| v[i].as_f64().unwrap_or(0.))))
            .unwrap_or(node.transform);
        let combined = multiply(
            multiply(affine_matrix(layout), m),
            affine_matrix(layout.inverse() * node.transform),
        );
        if style.backface == Some(false) && back_facing(combined) {
            node.opacity = 0.;
            return Ok(());
        }
        if combined[12].abs() < 1e-12 && combined[13].abs() < 1e-12 && combined[15].abs() > 1e-12 {
            let w = combined[15];
            node.transform = Affine::new([
                combined[0] / w,
                combined[4] / w,
                combined[1] / w,
                combined[5] / w,
                combined[3] / w,
                combined[7] / w,
            ]);
            return Ok(());
        }
        self.project(node, combined, parent, output, resources, viewport, density)
    }
    fn project(
        &mut self,
        node: &mut DrawNode,
        m: Matrix,
        parent: Affine,
        output: &mut BTreeMap<String, Resource>,
        resources: &BTreeMap<String, Resource>,
        viewport: Rect,
        density: f64,
    ) -> Result<()> {
        if node.opacity <= 0. {
            return Ok(());
        }
        let mut local = node.clone();
        local.transform = Affine::IDENTITY;
        local.opacity = 1.;
        let source_rect = local.bounds().inflate(1. / density, 1. / density);
        if source_rect.width() <= 0. || source_rect.height() <= 0. {
            node.opacity = 0.;
            return Ok(());
        }
        let sx = (parent.as_coeffs()[0].hypot(parent.as_coeffs()[1]))
            .max(parent.as_coeffs()[2].hypot(parent.as_coeffs()[3]))
            .max(1e-6);
        let source_density = (density * sx * 2.).min(100.);
        let sw = (source_rect.width() * source_density).ceil().max(1.) as u32;
        let sh = (source_rect.height() * source_density).ceil().max(1.) as u32;
        if sw as u64 * sh as u64 > 16_000_000 {
            return Err(error(
                "Projected animation source exceeds 16 megapixels; reduce animation output width",
            ));
        }
        let key = laymesh_core::asset_cache::digest(
            &serde_json::to_vec(&(local.clone(), sw, sh)).map_err(|e| error(e.to_string()))?,
        );
        if self.sources.get(&node.name).is_none_or(|v| v.0 != key) {
            let mut rs = resources.clone();
            rs.extend(output.clone());
            let list = DisplayList {
                animation: None,
                schema_version: DISPLAY_LIST_VERSION,
                width: self.base.width,
                height: self.base.height,
                nodes: vec![local],
                resources: rs,
            };
            let frame = crate::raster::render_region(&list, source_rect, sw, sh)?;
            if self.sources.values().map(|v| v.1.rgba.len()).sum::<usize>() + frame.rgba.len()
                > self.source_budget
            {
                self.sources.clear();
            }
            self.sources
                .insert(node.name.clone(), (key, frame, source_rect));
        }
        let (_, source, source_rect) = &self.sources[&node.name];
        let Some(inv) = inverse_homography(m) else {
            node.opacity = 0.;
            return Ok(());
        };
        let parent_clip = parent
            .inverse()
            .transform_rect_bbox(viewport)
            .inflate(2. / density, 2. / density);
        let mut corners = vec![];
        for (x, y) in [
            (source_rect.x0, source_rect.y0),
            (source_rect.x1, source_rect.y0),
            (source_rect.x1, source_rect.y1),
            (source_rect.x0, source_rect.y1),
        ] {
            corners.push([
                m[0] * x + m[1] * y + m[3],
                m[4] * x + m[5] * y + m[7],
                m[12] * x + m[13] * y + m[15],
            ]);
        }
        // Clip the homogeneous polygon against the positive camera half-space.
        let mut clipped = vec![];
        for i in 0..corners.len() {
            let a = corners[i];
            let b = corners[(i + 1) % corners.len()];
            let ia = a[2] >= 1e-6;
            let ib = b[2] >= 1e-6;
            if ia {
                clipped.push(a);
            }
            if ia != ib {
                let t = (1e-6 - a[2]) / (b[2] - a[2]);
                clipped.push(std::array::from_fn(|j| a[j] + (b[j] - a[j]) * t));
            }
        }
        let Some(bounds) = clipped
            .iter()
            .map(|p| Rect::new(p[0] / p[2], p[1] / p[2], p[0] / p[2], p[1] / p[2]))
            .reduce(|a, b| a.union(b))
        else {
            node.opacity = 0.;
            return Ok(());
        };
        let bounds = bounds
            .inflate(1. / density, 1. / density)
            .intersect(parent_clip);
        if bounds.width() <= 0. || bounds.height() <= 0. {
            node.opacity = 0.;
            return Ok(());
        }
        let (w, h) = (
            (bounds.width() * density * sx).ceil().max(1.) as u32,
            (bounds.height() * density * sx).ceil().max(1.) as u32,
        );
        if w as u64 * h as u64 > 32_000_000 {
            return Err(error("Projected animation frame exceeds 32 megapixels"));
        }
        let mut rgba = vec![0u8; w as usize * h as usize * 4];
        for y in 0..h {
            for x in 0..w {
                let px = bounds.x0 + (x as f64 + 0.5) * bounds.width() / w as f64;
                let py = bounds.y0 + (y as f64 + 0.5) * bounds.height() / h as f64;
                let z = inv[6] * px + inv[7] * py + inv[8];
                if z.abs() < 1e-15 {
                    continue;
                }
                let u = (inv[0] * px + inv[1] * py + inv[2]) / z;
                let v = (inv[3] * px + inv[4] * py + inv[5]) / z;
                if m[12] * u + m[13] * v + m[15] <= 1e-6 {
                    continue;
                }
                let u = (u - source_rect.x0) / source_rect.width() * source.width as f64 - 0.5;
                let v = (v - source_rect.y0) / source_rect.height() * source.height as f64 - 0.5;
                let pixel = bilinear(source, u, v);
                let offset = (y as usize * w as usize + x as usize) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
        let mut data = Vec::with_capacity(rgba.len() + 8);
        data.extend(w.to_le_bytes());
        data.extend(h.to_le_bytes());
        data.extend(rgba);
        let id = format!("animation-project:{}", node.name);
        output.insert(
            id.clone(),
            Resource {
                mime: "image/x-laymesh-rgba".into(),
                data,
                font_index: None,
                family: None,
            },
        );
        node.kind = DrawKind::Image {
            resource: id,
            rect: bounds,
            nearest: false,
        };
        node.effects.clear();
        node.transform = Affine::IDENTITY;
        Ok(())
    }
}
fn affine_matrix(a: Affine) -> Matrix {
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
fn inverse_homography(m: Matrix) -> Option<[f64; 9]> {
    let [a, b, c, d, e, f, g, h, i] = [m[0], m[1], m[3], m[4], m[5], m[7], m[12], m[13], m[15]];
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-15 {
        return None;
    }
    Some(
        [
            e * i - f * h,
            c * h - b * i,
            b * f - c * e,
            f * g - d * i,
            a * i - c * g,
            c * d - a * f,
            d * h - e * g,
            b * g - a * h,
            a * e - b * d,
        ]
        .map(|x| x / det),
    )
}
fn back_facing(m: Matrix) -> bool {
    laymesh_core::animation::matrix::inverse(m).is_some_and(|inv| inv[10] < 0.)
}
fn bilinear(frame: &RasterFrame, x: f64, y: f64) -> [u8; 4] {
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - ix as f64;
    let fy = y - iy as f64;
    let mut p = [0.; 4];
    for (dy, wy) in [(0, 1. - fy), (1, fy)] {
        for (dx, wx) in [(0, 1. - fx), (1, fx)] {
            let (xx, yy) = (ix + dx, iy + dy);
            if xx < 0 || yy < 0 || xx >= frame.width as i64 || yy >= frame.height as i64 {
                continue;
            }
            let off = (yy as usize * frame.width as usize + xx as usize) * 4;
            let a = frame.rgba[off + 3] as f64 / 255.;
            for c in 0..3 {
                p[c] += frame.rgba[off + c] as f64 * a * wx * wy;
            }
            p[3] += a * wx * wy;
        }
    }
    if p[3] <= 1e-12 {
        [0; 4]
    } else {
        [
            (p[0] / p[3]).round() as u8,
            (p[1] / p[3]).round() as u8,
            (p[2] / p[3]).round() as u8,
            (p[3] * 255.).round() as u8,
        ]
    }
}
fn trim_paths(node: &mut DrawNode, progress: f64) -> Result<()> {
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
fn reveal(node: &mut DrawNode, progress: f64) {
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
    node.kind = DrawKind::Group {
        children: vec![inner],
        clip: Some(Clip {
            path: Rect::new(b.x0, b.y0, b.x0 + b.width() * progress, b.y1).to_path(0.001),
            even_odd: false,
        }),
        mask: None,
    };
}
