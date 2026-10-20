//! Deterministic object-local geometry and rasterization, shared by native and WASM.
//! The front texture is supplied by the existing 2D renderer at the target density.
use crate::{display_list::*, error};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{Affine, BezPath, ParamCurve, PathEl, Point, Rect, Shape};
use laymesh_core::{
    Result,
    model::{jnum, jstr},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const MAX_VERTICES: usize = 100_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub p: [f64; 3],
    pub uv: [f64; 2],
    pub w: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal: Option<[f64; 3]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Triangle {
    pub v: [Vertex; 3],
    pub light: [f64; 3],
    pub specular: [f64; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mesh {
    pub source: [f64; 4],
    pub bounds: [f64; 4],
    pub triangles: Vec<Triangle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<Box<Recipe>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Box<DrawNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_alpha: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<Lighting>,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    #[serde(default = "one")]
    pub texture_scale: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub node: DrawNode,
    pub config: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lighting {
    pub direction: [f64; 3],
    pub color: [f64; 3],
    pub ambient: f64,
    pub intensity: f64,
    pub power: f64,
    pub strength: f64,
    pub flat: bool,
    pub camera: Option<[f64; 3]>,
}
fn one() -> f64 {
    1.
}
fn default_tolerance() -> f64 {
    0.015
}
fn point_key(p: Point) -> (i64, i64) {
    ((p.x * 1e7).round() as i64, (p.y * 1e7).round() as i64)
}
fn hard_corners(p: &BezPath, hard: &mut HashSet<(i64, i64)>) {
    let segments = p.segments().collect::<Vec<_>>();
    let outgoing = segments
        .iter()
        .map(|s| (point_key(s.start()), s.eval(1e-5) - s.start()))
        .collect::<HashMap<_, _>>();
    for s in segments {
        let incoming = s.end() - s.eval(1. - 1e-5);
        let key = point_key(s.end());
        if let Some(next) = outgoing.get(&key) {
            if incoming.dot(*next) < 0.99999 * incoming.hypot() * next.hypot() {
                hard.insert(key);
            }
        } else {
            hard.insert(key);
        }
    }
}
fn contours(p: &BezPath, tolerance: f64, hard: &mut HashSet<(i64, i64)>) -> Vec<Vec<[f64; 2]>> {
    hard_corners(p, hard);
    let mut out = vec![];
    let mut c = vec![];
    kurbo::flatten(p.iter(), tolerance, |e| match e {
        PathEl::MoveTo(p) => {
            if c.len() > 2 {
                out.push(std::mem::take(&mut c));
            } else {
                c.clear();
            }
            c.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => {
            if c.last() != Some(&[p.x, p.y]) {
                c.push([p.x, p.y]);
            }
        }
        PathEl::ClosePath => {
            if c.first() == c.last() {
                c.pop();
            }
            if c.len() > 2 {
                out.push(std::mem::take(&mut c));
            } else {
                c.clear();
            }
        }
        _ => {}
    });
    if c.len() > 2 {
        out.push(c);
    }
    out
}
fn as_path(cs: &[Vec<[f64; 2]>]) -> BezPath {
    let mut p = BezPath::new();
    for c in cs {
        if c.len() < 3 {
            continue;
        }
        p.move_to((c[0][0], c[0][1]));
        for q in &c[1..] {
            p.line_to((q[0], q[1]));
        }
        p.close_path();
    }
    p
}
fn normalized(cs: &[Vec<[f64; 2]>], even: bool) -> Vec<Vec<Vec<[f64; 2]>>> {
    cs.to_vec().overlay(
        &Vec::<Vec<[f64; 2]>>::new(),
        OverlayRule::Union,
        if even {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        },
    )
}
fn silhouette(
    n: &DrawNode,
    t: Affine,
    tolerance: f64,
    hard: &mut HashSet<(i64, i64)>,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let mut cs = vec![];
    match &n.kind {
        DrawKind::Path {
            path,
            fill,
            stroke,
            even_odd,
        } => {
            if !matches!(fill, Paint::None) {
                cs.extend(
                    normalized(&contours(&(t * path), tolerance, hard), *even_odd)
                        .into_iter()
                        .flatten(),
                );
            }
            if let Some(s) = stroke {
                if s.width > 0. && s.rgba[3] > 0. {
                    let mut st = kurbo::Stroke::new(s.width);
                    st.join = match s.join.as_str() {
                        "round" => kurbo::Join::Round,
                        "bevel" => kurbo::Join::Bevel,
                        _ => kurbo::Join::Miter,
                    };
                    st.start_cap = match s.cap.as_str() {
                        "round" => kurbo::Cap::Round,
                        "square" => kurbo::Cap::Square,
                        _ => kurbo::Cap::Butt,
                    };
                    st.end_cap = st.start_cap;
                    st.miter_limit = s.miter;
                    st.dash_pattern = s.dash.iter().copied().collect();
                    st.dash_offset = s.offset;
                    let p =
                        kurbo::stroke(path.iter(), &st, &kurbo::StrokeOpts::default(), tolerance);
                    cs.extend(
                        normalized(&contours(&(t * p), tolerance, hard), false)
                            .into_iter()
                            .flatten(),
                    );
                }
            }
        }
        DrawKind::Glyphs { outlines, .. } => {
            for p in outlines {
                cs.extend(
                    normalized(&contours(&(t * p), tolerance, hard), false)
                        .into_iter()
                        .flatten(),
                );
            }
        }
        DrawKind::Image { rect, .. } => {
            cs.extend(contours(&(t * rect.to_path(tolerance)), tolerance, hard))
        }
        DrawKind::Group {
            children,
            clip,
            mask,
        } => {
            if mask.is_some() {
                return Err(error(
                    "three_d does not support arbitrary alpha masks; use a cropped image",
                ));
            }
            for child in children {
                cs.extend(silhouette(child, t * child.transform, tolerance, hard)?);
            }
            if let Some(clip) = clip {
                let subject = normalized(&cs, false)
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let clips =
                    normalized(&contours(&(t * &clip.path), tolerance, hard), clip.even_odd)
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                return Ok(subject
                    .overlay(&clips, OverlayRule::Intersect, FillRule::NonZero)
                    .into_iter()
                    .flatten()
                    .collect());
            }
        }
    }
    Ok(cs)
}
fn area(c: &[[f64; 2]]) -> f64 {
    c.iter()
        .zip(c.iter().cycle().skip(1))
        .take(c.len())
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        / 2.
}
fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = v.iter().map(|v| v * v).sum::<f64>().sqrt();
    if l < 1e-15 {
        [0., 0., 1.]
    } else {
        v.map(|v| v / l)
    }
}
fn rotate(mut p: [f64; 3], r: [f64; 3]) -> [f64; 3] {
    let (s, c) = r[0].sin_cos();
    p = [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c];
    let (s, c) = r[1].sin_cos();
    p = [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c];
    let (s, c) = r[2].sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]]
}
fn vec3(v: &Value, d: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| v[i].as_f64().unwrap_or(d[i]))
}
fn inset(cs: &[Vec<[f64; 2]>], w: f64) -> Result<Vec<Vec<[f64; 2]>>> {
    if w == 0. {
        return Ok(cs.to_vec());
    }
    let sign = area(&cs[0]).signum();
    let path = as_path(cs);
    let mut out = vec![];
    for c in cs {
        let mut ring = vec![];
        for i in 0..c.len() {
            let (a, b, d) = (c[(i + c.len() - 1) % c.len()], c[i], c[(i + 1) % c.len()]);
            let edge = |a: [f64; 2], b: [f64; 2]| {
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let l = dx.hypot(dy);
                [-dy / l * sign, dx / l * sign]
            };
            let (n, m) = (edge(a, b), edge(b, d));
            let div = 1. + n[0] * m[0] + n[1] * m[1];
            if div < 1e-6 {
                return Err(error("Bevel has a degenerate corner; reduce bevel width"));
            }
            let p = [
                b[0] + w * (n[0] + m[0]) / div,
                b[1] + w * (n[1] + m[1]) / div,
            ];
            if !p.iter().all(|v| v.is_finite()) || path.winding(Point::new(p[0], p[1])) == 0 {
                return Err(error("Bevel exceeds the local contour width"));
            }
            ring.push(p);
        }
        if area(&ring) * area(c) <= 0. {
            return Err(error("Bevel collapses a contour"));
        }
        for i in 0..c.len() {
            let j = (i + 1) % c.len();
            if (ring[j][0] - ring[i][0]) * (c[j][0] - c[i][0])
                + (ring[j][1] - ring[i][1]) * (c[j][1] - c[i][1])
                <= 0.
            {
                return Err(error("Bevel collapses a contour edge"));
            }
        }
        // Every edge must stay within the original material, including concave corners.
        for (a, b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            if path.winding(Point::new((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.)) == 0 {
                return Err(error("Bevel crosses a contour; reduce bevel width"));
            }
        }
        out.push(ring);
    }
    let check = normalized(&out, false);
    if check.len() != 1 || check[0].len() != cs.len() {
        return Err(error("Bevel changes contour topology; reduce bevel width"));
    }
    Ok(out)
}
/// Geometry coverage is independent of fill/image opacity. Dividing by this
/// mask removes the old 2D edge coverage before the mesh adds its own coverage.
pub fn coverage_node(node: &DrawNode) -> DrawNode {
    let mut n = node.clone();
    n.opacity = 1.;
    n.effects.clear();
    n.hint = Value::Null;
    let white = Paint::Solid { rgba: [1.; 4] };
    match &mut n.kind {
        DrawKind::Path { fill, stroke, .. } => {
            if !matches!(fill, Paint::None) {
                *fill = white;
            }
            if let Some(s) = stroke {
                if s.rgba[3] > 0. {
                    s.rgba = [1.; 4];
                }
            }
        }
        DrawKind::Glyphs { color, .. } => *color = [1.; 4],
        DrawKind::Image { rect, .. } => {
            n.kind = DrawKind::Path {
                path: rect.to_path(0.001),
                fill: white,
                stroke: None,
                even_odd: false,
            }
        }
        DrawKind::Group { children, .. } => {
            *children = children.iter().map(coverage_node).collect()
        }
    }
    n
}
fn uniform_material_alpha(n: &DrawNode) -> Option<f64> {
    match &n.kind {
        DrawKind::Path {
            fill, stroke: None, ..
        } => match fill {
            Paint::Solid { rgba } => Some(rgba[3]),
            Paint::Linear { stops, .. }
            | Paint::Radial { stops, .. }
            | Paint::Conic { stops, .. } => {
                let alpha = stops.first()?.rgba[3];
                stops.iter().all(|s| s.rgba[3] == alpha).then_some(alpha)
            }
            _ => None,
        },
        DrawKind::Glyphs { color, .. } => Some(color[3]),
        DrawKind::Group {
            children,
            mask: None,
            ..
        } if children.len() == 1 && children[0].opacity == 1. => {
            uniform_material_alpha(&children[0])
        }
        _ => None,
    }
}
pub fn material_texture(
    texture: &[u8],
    coverage: &[u8],
    w: u32,
    h: u32,
    material_alpha: Option<f64>,
) -> Result<Vec<u8>> {
    let count = w as usize * h as usize;
    if count > 16_777_216 || texture.len() != count * 4 || coverage.len() != count * 4 {
        return Err(error("Invalid three_d material texture"));
    }
    let mut out = texture.to_vec();
    for (i, pixel) in out.chunks_exact_mut(4).enumerate() {
        let c = coverage[i * 4 + 3] as u32;
        if c > 0 {
            pixel[3] = ((pixel[3] as u32 * 255 + c / 2) / c).min(255) as u8;
        }
    }
    // Resolve ambiguous edge alpha against an interior texel only when it is
    // consistent with the original 8-bit quantization interval. This avoids
    // amplifying a one-byte coverage error without flattening real image alpha.
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let at = (y as usize * w as usize + x as usize) * 4;
            let c = coverage[at + 3] as f64;
            let a = texture[at + 3] as f64;
            // A known translucent fill can round to zero at the geometry edge.
            // Recover its color before restoring material alpha. For images,
            // zero alpha may be authored transparency and must stay untouched.
            if c == 255. || c > 0. && a == 0. && material_alpha.is_none_or(|alpha| alpha <= 0.) {
                continue;
            }
            let low = (a - 0.5).max(0.) * 255. / (c + 0.5);
            let high = (a + 0.5) * 255. / (c - 0.5).max(0.5);
            let mut best = None;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let xx = x + dx;
                    let yy = y + dy;
                    if xx < 0 || yy < 0 || xx >= w as i32 || yy >= h as i32 {
                        continue;
                    }
                    let k = (yy as usize * w as usize + xx as usize) * 4;
                    let cover = coverage[k + 3];
                    if cover == 0 {
                        continue;
                    }
                    let alpha = ((texture[k + 3] as u32 * 255 + cover as u32 / 2) / cover as u32)
                        .min(255) as u8;
                    // Known uniform alpha takes precedence over the interval:
                    // raster coverage multiplication can round down as well.
                    if c > 0.
                        && (cover != 255
                            || material_alpha.is_none()
                                && ((alpha as f64) < low || (alpha as f64) > high))
                    {
                        continue;
                    }
                    // Prefer a fully covered material texel for the exterior gutter;
                    // use partial coverage only for contours narrower than one pixel.
                    let score = i32::from(cover != 255) * 65536
                        + (dx * dx + dy * dy) * 256
                        + (255 - cover as i32);
                    if best.is_none_or(|(s, _, _)| score < s) {
                        best = Some((score, k, alpha));
                    }
                }
            }
            if let Some((_, k, alpha)) = best {
                if c == 0. {
                    out[at..at + 3].copy_from_slice(&texture[k..k + 3]);
                } else {
                    // Unpremultiplying a one-byte, partially covered edge can
                    // amplify RGB rounding by 255 / alpha. Removing coverage
                    // from alpha alone leaves that error painted down the wall.
                    // Recover only colors compatible with the original byte's
                    // quantization interval; real image/color boundaries remain.
                    let uncertainty = 255. / a.max(1.) + 1.;
                    for channel in 0..3 {
                        if texture[at + channel].abs_diff(texture[k + channel]) as f64
                            <= uncertainty
                        {
                            out[at + channel] = texture[k + channel];
                        }
                    }
                }
                out[at + 3] = alpha;
            }
        }
    }
    if let Some(alpha) = material_alpha {
        let alpha = (alpha.clamp(0., 1.) * 255.).round() as u8;
        for p in out.chunks_exact_mut(4) {
            p[3] = alpha;
        }
    }
    Ok(out)
}
pub fn prepare(node: &DrawNode, cfg: &Value) -> Result<Mesh> {
    let mut mesh = prepare_at(node, cfg, default_tolerance())?;
    let mut original = node.clone();
    if let Some(hint) = original.hint.as_object_mut() {
        hint.remove("surface3d");
        hint.remove("_identity");
    }
    original.effects.clear();
    original.transform = Affine::IDENTITY;
    original.opacity = 1.;
    mesh.recipe = Some(Box::new(Recipe {
        node: original,
        config: cfg.clone(),
    }));
    Ok(mesh)
}
/// Refine from vector geometry, never by subdividing the old polygonal mesh.
pub fn refine(mesh: &Mesh, density: f64) -> Result<Mesh> {
    if !density.is_finite() || density <= 0. {
        return Err(error("Invalid three_d rendering scale"));
    }
    let tolerance = default_tolerance().min(0.125 / (density * mesh.texture_scale));
    let mut result = if tolerance < mesh.tolerance {
        if let Some(r) = &mesh.recipe {
            prepare_at(&r.node, &r.config, tolerance)?
        } else {
            mesh.clone()
        }
    } else {
        mesh.clone()
    };
    result.recipe = None;
    Ok(result)
}
fn prepare_at(node: &DrawNode, cfg: &Value, tolerance: f64) -> Result<Mesh> {
    let mut hard = HashSet::new();
    let cs = silhouette(node, Affine::IDENTITY, tolerance, &mut hard)?;
    if cs.iter().map(Vec::len).sum::<usize>() > MAX_VERTICES {
        return Err(error("three_d contour exceeds the 100000-vertex budget"));
    }
    let shapes = normalized(&cs, false);
    if shapes.is_empty() {
        return Err(error("three_d requires nondegenerate visible geometry"));
    }
    let b = as_path(&shapes.iter().flatten().cloned().collect::<Vec<_>>()).bounding_box();
    if b.width() <= 1e-8 || b.height() <= 1e-8 {
        return Err(error("three_d requires a nondegenerate surface"));
    }
    let rotation = vec3(&cfg["rotation"], [0.; 3]);
    let light = unit(vec3(&cfg["lightDirection"], [-1., -1., 2.]));
    let color = rgba(jstr(cfg, "lightColor", "#ffffff"))?;
    let depth = jnum(cfg, "depth", 2.);
    let perspective = cfg["projection"] == "perspective";
    let distance = jnum(cfg, "distance", 500.);
    let top = crate::pair(&cfg["bevelTop"], [0.; 2]);
    let bottom = crate::pair(&cfg["bevelBottom"], [0.; 2]);
    let mut mesh = Mesh {
        source: [b.x0, b.y0, b.x1, b.y1],
        bounds: [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        triangles: vec![],
        recipe: None,
        coverage: Some(Box::new(coverage_node(node))),
        material_alpha: uniform_material_alpha(node),
        lighting: None,
        tolerance,
        texture_scale: 1.,
    };
    let center = b.center();
    let material = jstr(cfg, "material", "matte");
    let (power, strength) = match material {
        "plastic" => (40., 0.5),
        "metal" => (90., 0.9),
        _ => (1., 0.),
    };
    mesh.lighting = Some(Lighting {
        direction: light,
        color: [color[0], color[1], color[2]],
        ambient: jnum(cfg, "ambient", 0.25),
        intensity: jnum(cfg, "intensity", 1.),
        power,
        strength,
        flat: material == "flat",
        camera: perspective.then_some([center.x, center.y, distance]),
    });
    if perspective {
        let mut min_w: f64 = 1.;
        for x in [b.x0, b.x1] {
            for y in [b.y0, b.y1] {
                for z in [-depth / 2., depth / 2.] {
                    min_w = min_w
                        .min(1. - rotate([x - center.x, y - center.y, z], rotation)[2] / distance);
                }
            }
        }
        if min_w <= 0.01 {
            return Err(error(
                "Perspective camera intersects the object; increase distance",
            ));
        }
        mesh.texture_scale =
            ((1. + b.width().hypot(b.height()) / (2. * distance)) / min_w.powi(2)).max(1.);
    }
    let mut emit =
        |pts: [[f64; 3]; 3], normal: [f64; 3], smooth: Option<[[f64; 3]; 3]>| -> Result<()> {
            let normal = unit(rotate(normal, rotation));
            let v = std::array::from_fn(|i| {
                let p = pts[i];
                let q = rotate([p[0] - center.x, p[1] - center.y, p[2]], rotation);
                let w = if perspective {
                    1. - q[2] / distance
                } else {
                    1.
                };
                Vertex {
                    p: [q[0] / w + center.x, q[1] / w + center.y, q[2]],
                    uv: [p[0], p[1]],
                    w,
                    normal: Some(smooth.map_or(normal, |ns| unit(rotate(ns[i], rotation)))),
                }
            });
            if v.iter()
                .any(|v| v.w <= 0.01 || !v.p.iter().all(|x| x.is_finite()))
            {
                return Err(error(
                    "Perspective camera intersects the object; increase distance",
                ));
            }
            // Camera is on +Z; perspective views use the direction from the face to camera.
            let q = rotate(
                [pts[0][0] - center.x, pts[0][1] - center.y, pts[0][2]],
                rotation,
            );
            let view = if perspective {
                unit([-q[0], -q[1], distance - q[2]])
            } else {
                [0., 0., 1.]
            };
            if normal.iter().zip(view).map(|(a, b)| a * b).sum::<f64>() <= 1e-10 {
                return Ok(());
            }
            let diffuse = normal
                .iter()
                .zip(light)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .max(0.);
            let intensity = jnum(cfg, "intensity", 1.);
            let ambient = jnum(cfg, "ambient", 0.25);
            let half = unit([light[0] + view[0], light[1] + view[1], light[2] + view[2]]);
            let shine = normal
                .iter()
                .zip(half)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .max(0.);
            let (power, strength) = match material {
                "plastic" => (40., 0.5),
                "metal" => (90., 0.9),
                _ => (1., 0.),
            };
            let specular = if diffuse > 0. {
                shine.powf(power) * strength * intensity
            } else {
                0.
            };
            let light = std::array::from_fn(|i| {
                if material == "flat" {
                    1.
                } else {
                    ambient + (1. - ambient) * diffuse * intensity * color[i]
                }
            });
            for v in &v {
                mesh.bounds[0] = mesh.bounds[0].min(v.p[0]);
                mesh.bounds[1] = mesh.bounds[1].min(v.p[1]);
                mesh.bounds[2] = mesh.bounds[2].max(v.p[0]);
                mesh.bounds[3] = mesh.bounds[3].max(v.p[1]);
            }
            mesh.triangles.push(Triangle {
                v,
                light,
                specular: std::array::from_fn(|i| specular * color[i]),
            });
            Ok(())
        };
    for cs in shapes {
        let front = inset(&cs, top[0])?;
        let back = inset(&cs, bottom[0])?;
        for (rings, z, n) in [
            (&front, depth / 2., [0., 0., 1.]),
            (&back, -depth / 2., [0., 0., -1.]),
        ] {
            let mut data = vec![];
            let mut holes = vec![];
            for (i, c) in rings.iter().enumerate() {
                if i > 0 {
                    holes.push(data.len() / 2);
                }
                for p in c {
                    data.extend(p);
                }
            }
            let indices = earcutr::earcut(&data, &holes, 2)
                .map_err(|e| error(format!("Triangulation failed: {e}")))?;
            for tri in indices.chunks_exact(3) {
                emit(
                    std::array::from_fn(|i| [data[tri[i] * 2], data[tri[i] * 2 + 1], z]),
                    n,
                    None,
                )?;
            }
        }
        let sign = area(&cs[0]).signum();
        for (ring, original) in cs.iter().enumerate() {
            let levels = [
                (&front[ring], depth / 2.),
                (original, depth / 2. - top[1]),
                (original, -depth / 2. + bottom[1]),
                (&back[ring], -depth / 2.),
            ];
            for pair in levels.windows(2) {
                if pair[0].1 == pair[1].1 && pair[0].0 == pair[1].0 {
                    continue;
                }
                for i in 0..original.len() {
                    let j = (i + 1) % original.len();
                    let a = [pair[0].0[i][0], pair[0].0[i][1], pair[0].1];
                    let b = [pair[0].0[j][0], pair[0].0[j][1], pair[0].1];
                    let c = [pair[1].0[j][0], pair[1].0[j][1], pair[1].1];
                    let d = [pair[1].0[i][0], pair[1].0[i][1], pair[1].1];
                    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let v = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
                    let n = [
                        (u[1] * v[2] - u[2] * v[1]) * -sign,
                        (u[2] * v[0] - u[0] * v[2]) * -sign,
                        (u[0] * v[1] - u[1] * v[0]) * -sign,
                    ];
                    let edge_normal = |index: usize| {
                        let next = (index + 1) % original.len();
                        let x = [
                            pair[0].0[next][0] - pair[0].0[index][0],
                            pair[0].0[next][1] - pair[0].0[index][1],
                            0.,
                        ];
                        let y = [
                            pair[1].0[index][0] - pair[0].0[index][0],
                            pair[1].0[index][1] - pair[0].0[index][1],
                            pair[1].1 - pair[0].1,
                        ];
                        unit([
                            (x[1] * y[2] - x[2] * y[1]) * -sign,
                            (x[2] * y[0] - x[0] * y[2]) * -sign,
                            (x[0] * y[1] - x[1] * y[0]) * -sign,
                        ])
                    };
                    let smooth_at = |index: usize| {
                        let current = unit(n);
                        if hard.contains(&point_key(Point::new(
                            original[index][0],
                            original[index][1],
                        ))) {
                            return current;
                        }
                        let prev = edge_normal((index + original.len() - 1) % original.len());
                        let next = edge_normal(index);
                        if prev.iter().zip(next).map(|(a, b)| a * b).sum::<f64>() < 0.5 {
                            current
                        } else {
                            unit(std::array::from_fn(|k| prev[k] + next[k]))
                        }
                    };
                    let ni = smooth_at(i);
                    let nj = smooth_at(j);
                    emit([a, b, c], n, Some([ni, nj, nj]))?;
                    emit([a, c, d], n, Some([ni, nj, ni]))?;
                }
            }
        }
    }
    if mesh.triangles.len() > MAX_VERTICES * 8 {
        return Err(error("three_d triangle budget exceeded"));
    }
    if mesh.triangles.is_empty() {
        mesh.bounds = [center.x, center.y, center.x, center.y];
    }
    if !mesh.triangles.is_empty() {
        let pad = tolerance * mesh.texture_scale;
        mesh.bounds[0] -= pad;
        mesh.bounds[1] -= pad;
        mesh.bounds[2] += pad;
        mesh.bounds[3] += pad;
    }
    Ok(mesh)
}

/// Render one output tile. Texture is unpremultiplied sRGB. Lighting is linear RGB.
fn decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn decode_table() -> &'static [f64; 256] {
    static TABLE: std::sync::OnceLock<[f64; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| std::array::from_fn(|i| decode(i as f64 / 255.)))
}
fn encode_channel(v: f64) -> u8 {
    static TABLE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        (0..=65535)
            .map(|i| (encode(i as f64 / 65535.) * 255.).round() as u8)
            .collect()
    });
    t[(v.clamp(0., 1.) * 65535.).round() as usize]
}
fn raster_single(
    mesh: &Mesh,
    texture: &[u8],
    tw: u32,
    th: u32,
    region: [f64; 4],
    w: u32,
    h: u32,
) -> Result<Vec<u8>> {
    if w == 0
        || h == 0
        || tw == 0
        || th == 0
        || texture.len() != tw as usize * th as usize * 4
        || !region.iter().all(|x| x.is_finite())
        || region[2] <= region[0]
        || region[3] <= region[1]
    {
        return Err(error("Invalid three_d raster dimensions"));
    }
    let count = w as usize * h as usize * 16;
    if count * 8 > 64 * 1024 * 1024 {
        return Err(error(
            "three_d tile exceeds the 64 MiB working buffer budget",
        ));
    }
    let mut pixels = vec![0u8; count * 4];
    let mut depth = vec![f32::NEG_INFINITY; count];
    let sx = w as f64 / (region[2] - region[0]);
    let sy = h as f64 / (region[3] - region[1]);
    let decode = decode_table();
    for tri in &mesh.triangles {
        let p = tri
            .v
            .each_ref()
            .map(|v| [(v.p[0] - region[0]) * sx, (v.p[1] - region[1]) * sy]);
        let edge = |a: [f64; 2], b: [f64; 2], q: [f64; 2]| {
            (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
        };
        let area = edge(p[0], p[1], p[2]);
        if area.abs() < 1e-12 {
            continue;
        }
        let x0 = p
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .clamp(0., w as f64) as u32;
        let x1 = p
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .clamp(0., w as f64) as u32;
        let y0 = p
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .clamp(0., h as f64) as u32;
        let y1 = p
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .clamp(0., h as f64) as u32;
        let gx =
            std::array::from_fn::<_, 3, _>(|i| -(p[(i + 2) % 3][1] - p[(i + 1) % 3][1]) / area);
        let gy = std::array::from_fn::<_, 3, _>(|i| (p[(i + 2) % 3][0] - p[(i + 1) % 3][0]) / area);
        let iw = tri.v.each_ref().map(|v| 1. / v.w);
        let iz = std::array::from_fn::<_, 3, _>(|i| tri.v[i].p[2] * iw[i]);
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let idx = dot(gx, iw);
        let idy = dot(gy, iw);
        let zdx = dot(gx, iz);
        let zdy = dot(gy, iz);
        for y in y0..y1 {
            for x in x0..x1 {
                let at = (y as usize * w as usize + x as usize) * 16;
                let q = [x as f64 + 0.5, y as f64 + 0.5];
                let center = [
                    edge(p[1], p[2], q) / area,
                    edge(p[2], p[0], q) / area,
                    edge(p[0], p[1], q) / area,
                ];
                let margin =
                    std::array::from_fn::<_, 3, _>(|i| 0.375 * (gx[i].abs() + gy[i].abs()));
                if (0..3).any(|i| center[i] + margin[i] < -1e-9) {
                    continue;
                }
                let full = (0..3).all(|i| center[i] - margin[i] >= -1e-9);
                let inv_center = dot(center, iw);
                let z_center = dot(center, iz);
                let mut hits = [(0usize, 0f32); 16];
                let mut nh = 0;
                let mut mx = 0.;
                let mut my = 0.;
                for dy in 0..4 {
                    for dx in 0..4 {
                        let ox = dx as f64 * 0.25 - 0.375;
                        let oy = dy as f64 * 0.25 - 0.375;
                        if !full && (0..3).any(|i| center[i] + gx[i] * ox + gy[i] * oy < -1e-9) {
                            continue;
                        }
                        let z = ((z_center + zdx * ox + zdy * oy)
                            / (inv_center + idx * ox + idy * oy))
                            as f32;
                        let k = at + dy * 4 + dx;
                        if z < depth[k] {
                            continue;
                        }
                        hits[nh] = (k, z);
                        nh += 1;
                        mx += ox;
                        my += oy;
                    }
                }
                if nh == 0 {
                    continue;
                }
                let weights = std::array::from_fn::<_, 3, _>(|i| {
                    center[i] + gx[i] * mx / nh as f64 + gy[i] * my / nh as f64
                });
                let inv = (0..3).map(|i| weights[i] / tri.v[i].w).sum::<f64>();
                let bary = std::array::from_fn::<_, 3, _>(|i| weights[i] / tri.v[i].w / inv);
                let u = (0..3).map(|i| bary[i] * tri.v[i].uv[0]).sum::<f64>();
                let v = (0..3).map(|i| bary[i] * tri.v[i].uv[1]).sum::<f64>();
                let tx = ((u - mesh.source[0]) / (mesh.source[2] - mesh.source[0]) * tw as f64
                    - 0.5)
                    .clamp(0., tw as f64 - 1.);
                let ty = ((v - mesh.source[1]) / (mesh.source[3] - mesh.source[1]) * th as f64
                    - 0.5)
                    .clamp(0., th as f64 - 1.);
                let (ix, iy) = (tx.floor() as u32, ty.floor() as u32);
                let (fx, fy) = (tx.fract(), ty.fract());
                let mut rgba = [0.; 4];
                for (xx, yy, weight) in [
                    (ix, iy, (1. - fx) * (1. - fy)),
                    ((ix + 1).min(tw - 1), iy, fx * (1. - fy)),
                    (ix, (iy + 1).min(th - 1), (1. - fx) * fy),
                    ((ix + 1).min(tw - 1), (iy + 1).min(th - 1), fx * fy),
                ] {
                    let at = (yy as usize * tw as usize + xx as usize) * 4;
                    let alpha = texture[at + 3] as f64 / 255.;
                    for c in 0..3 {
                        rgba[c] += decode[texture[at + c] as usize] * alpha * weight;
                    }
                    rgba[3] += alpha * weight;
                }
                if rgba[3] < 1. / 255. {
                    continue;
                }
                let (mut light, mut specular) = (tri.light, tri.specular);
                if let Some(l) = &mesh.lighting {
                    if l.flat {
                        light = [1.; 3];
                        specular = [0.; 3];
                    } else if tri.v.iter().all(|v| v.normal.is_some()) {
                        let n = unit(std::array::from_fn(|k| {
                            (0..3).map(|i| bary[i] * tri.v[i].normal.unwrap()[k]).sum()
                        }));
                        let view = l.camera.map_or([0., 0., 1.], |camera| {
                            unit(std::array::from_fn(|k| {
                                camera[k]
                                    - (0..3)
                                        .map(|i| {
                                            bary[i]
                                                * if k < 2 {
                                                    (tri.v[i].p[k] - camera[k]) * tri.v[i].w
                                                        + camera[k]
                                                } else {
                                                    tri.v[i].p[k]
                                                }
                                        })
                                        .sum::<f64>()
                            }))
                        });
                        let diffuse = (0..3).map(|i| n[i] * l.direction[i]).sum::<f64>().max(0.);
                        let half = unit(std::array::from_fn(|i| l.direction[i] + view[i]));
                        let shine = if diffuse > 0. && l.strength > 0. {
                            (0..3)
                                .map(|i| n[i] * half[i])
                                .sum::<f64>()
                                .max(0.)
                                .powf(l.power)
                                * l.strength
                                * l.intensity
                        } else {
                            0.
                        };
                        light = std::array::from_fn(|i| {
                            l.ambient + (1. - l.ambient) * diffuse * l.intensity * l.color[i]
                        });
                        specular = l.color.map(|v| v * shine);
                    }
                }
                let color = [
                    encode_channel(rgba[0] / rgba[3] * light[0] + specular[0]),
                    encode_channel(rgba[1] / rgba[3] * light[1] + specular[1]),
                    encode_channel(rgba[2] / rgba[3] * light[2] + specular[2]),
                    (rgba[3] * 255.).round() as u8,
                ];
                for &(k, z) in &hits[..nh] {
                    depth[k] = z;
                    pixels[k * 4..k * 4 + 4].copy_from_slice(&color);
                }
            }
        }
    }
    let mut out = vec![0; w as usize * h as usize * 4];
    for (i, pixel) in out.chunks_exact_mut(4).enumerate() {
        let mut alpha = 0u32;
        let mut color = [0.; 3];
        for sample in pixels[i * 64..(i + 1) * 64].chunks_exact(4) {
            alpha += sample[3] as u32;
            for c in 0..3 {
                color[c] += decode[sample[c] as usize] * sample[3] as f64;
            }
        }
        if alpha > 0 {
            for c in 0..3 {
                pixel[c] = encode_channel(color[c] / alpha as f64);
            }
        }
        pixel[3] = ((alpha + 8) / 16) as u8;
    }
    Ok(out)
}

pub fn mesh(n: &DrawNode) -> Option<Mesh> {
    serde_json::from_value(n.hint.get("surface3d")?.clone()).ok()
}
pub fn rect(b: [f64; 4]) -> Rect {
    Rect::new(b[0], b[1], b[2], b[3])
}

/// Texture bounds of the portion of the projected mesh intersecting one tile.
/// Clip in screen space while interpolating UV/w and 1/w, then recover UV.
pub fn texture_region(mesh: &Mesh, region: [f64; 4], density: f64) -> Option<[f64; 4]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for t in &mesh.triangles {
        let mut poly =
            t.v.iter()
                .map(|v| [v.p[0], v.p[1], v.uv[0] / v.w, v.uv[1] / v.w, 1. / v.w])
                .collect::<Vec<_>>();
        for (axis, limit, lower) in [
            (0, region[0], true),
            (0, region[2], false),
            (1, region[1], true),
            (1, region[3], false),
        ] {
            let mut next = vec![];
            for i in 0..poly.len() {
                let a = poly[i];
                let z = poly[(i + 1) % poly.len()];
                let inside = |p: [f64; 5]| {
                    if lower {
                        p[axis] >= limit
                    } else {
                        p[axis] <= limit
                    }
                };
                if inside(a) {
                    next.push(a);
                }
                if inside(a) != inside(z) {
                    let u = (limit - a[axis]) / (z[axis] - a[axis]);
                    next.push(std::array::from_fn(|j| a[j] + u * (z[j] - a[j])));
                }
            }
            poly = next;
            if poly.is_empty() {
                break;
            }
        }
        for p in poly {
            let x = p[2] / p[4];
            let y = p[3] / p[4];
            b[0] = b[0].min(x);
            b[1] = b[1].min(y);
            b[2] = b[2].max(x);
            b[3] = b[3].max(y);
        }
    }
    if !b[0].is_finite() {
        return None;
    }
    // Align source sampling to a single object-wide grid: neighboring output
    // tiles must not sample the same source point at different pixel phases.
    Some([
        (b[0] * density).floor() / density - 2. / density,
        (b[1] * density).floor() / density - 2. / density,
        (b[2] * density).ceil() / density + 2. / density,
        (b[3] * density).ceil() / density + 2. / density,
    ])
}

/// Sixteen depth/coverage samples, with one material/light evaluation per
/// triangle/pixel. This keeps high-quality MSAA affordable in software WASM.
pub fn raster(
    mesh: &Mesh,
    texture: &[u8],
    tw: u32,
    th: u32,
    region: [f64; 4],
    w: u32,
    h: u32,
) -> Result<Vec<u8>> {
    if w == 0 || h == 0 || w as u64 * h as u64 > 2_000_000 {
        return Err(error(
            "three_d output must be split into tiles of at most 2000000 pixels",
        ));
    }
    let band = (524_288 / w).max(1).min(h);
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in (0..h).step_by(band as usize) {
        let rows = band.min(h - y);
        let mut r = region;
        r[1] = region[1] + (region[3] - region[1]) * y as f64 / h as f64;
        r[3] = region[1] + (region[3] - region[1]) * (y + rows) as f64 / h as f64;
        out.extend(raster_single(mesh, texture, tw, th, r, w, rows)?);
    }
    Ok(out)
}
