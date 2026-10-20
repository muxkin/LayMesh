//! Serialize the retained drawing commands at the export boundary. Offline
//! packages keep these commands and shared images once, not an extra SVG/page.
//! PDF callers supply the existing renderer for effect-only raster layers.
use crate::{display_list::*, effects, error};
use base64::Engine;
use kurbo::{Affine, Rect};
use laymesh_core::{Result, model::escape};
use serde_json::json;
use std::collections::BTreeMap;

type Raster<'a> = &'a mut dyn FnMut(&DrawNode, Rect, u32, u32, Option<bool>) -> Result<Vec<u8>>;

pub(crate) fn max_scale(t: Affine) -> f64 {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let sum = a * a + b * b + c * c + d * d;
    let det = a * d - b * c;
    ((sum + (sum * sum - 4. * det * det).max(0.).sqrt()) / 2.).sqrt()
}

// Matches the renderer's filter input support. Nested filters need the sum
// of parent and descendant halos, transformed into the parent's coordinates.
fn effect_halo(n: &DrawNode) -> f64 {
    let own = n
        .effects
        .iter()
        .map(|e| {
            (4. * e.blur + e.spread.abs() + e.offset[0].abs().max(e.offset[1].abs()))
                .max(2.5 * e.soft_edge.unwrap_or(0.))
        })
        .fold(0., f64::max);
    let child = if let DrawKind::Group { children, mask, .. } = &n.kind {
        children
            .iter()
            .chain(mask.iter().map(|m| m.node.as_ref()))
            .map(|c| max_scale(c.transform) * effect_halo(c))
            .fold(0., f64::max)
    } else if let DrawKind::Path {
        fill: Paint::Pattern {
            nodes, transform, ..
        },
        ..
    } = &n.kind
    {
        max_scale(*transform)
            * nodes
                .iter()
                .map(|c| max_scale(c.transform) * effect_halo(c))
                .fold(0., f64::max)
    } else {
        0.
    };
    own + child
}

fn matrix(t: Affine) -> String {
    format!(
        "matrix({})",
        t.as_coeffs()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    )
}
fn rgb(c: [f64; 4]) -> String {
    format!(
        "rgb({},{},{})",
        c[0].clamp(0., 1.) * 255.,
        c[1].clamp(0., 1.) * 255.,
        c[2].clamp(0., 1.) * 255.
    )
}
fn href(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
fn image_at(uri: &str, b: Rect) -> String {
    format!(
        "<image x='{}' y='{}' width='{}' height='{}' preserveAspectRatio='none' xlink:href='{uri}'/>",
        b.x0,
        b.y0,
        b.width(),
        b.height()
    )
}
struct Writer<'a> {
    list: &'a DisplayList,
    defs: Vec<String>,
    next: usize,
    images: BTreeMap<String, String>,
    dpi: f64,
    raster: Option<Raster<'a>>,
    pdf: bool,
}
impl Writer<'_> {
    fn id(&mut self) -> String {
        self.next += 1;
        format!("lm{}", self.next)
    }
    fn nodes(&mut self, nodes: &[DrawNode], parent: Affine) -> Result<String> {
        nodes.iter().map(|n| self.node(n, parent)).collect()
    }
    fn paint(&mut self, p: &Paint, parent: Affine) -> Result<(String, f64)> {
        match p {
            Paint::None => return Ok(("none".into(), 1.)),
            Paint::Solid { rgba } => return Ok((rgb(*rgba), rgba[3])),
            _ => {}
        }
        let id = self.id();
        let gradient = |stops: &[Stop], transform: Affine, spread: &str| {
            let stops = stops
                .iter()
                .map(|s| {
                    format!(
                        "<stop offset='{}' stop-color='{}' stop-opacity='{}'/>",
                        s.offset,
                        rgb(s.rgba),
                        s.rgba[3]
                    )
                })
                .collect::<String>();
            (
                format!(
                    "id='{id}' gradientUnits='userSpaceOnUse' gradientTransform='{}' spreadMethod='{}'",
                    matrix(transform),
                    escape(spread)
                ),
                stops,
            )
        };
        let def = match p {
            Paint::Linear {
                start,
                end,
                stops,
                transform,
                spread,
            } => {
                let (attrs, stops) = gradient(stops, *transform, spread);
                format!(
                    "<linearGradient {attrs} x1='{}' y1='{}' x2='{}' y2='{}'>{stops}</linearGradient>",
                    start[0], start[1], end[0], end[1]
                )
            }
            Paint::Radial {
                center,
                focal,
                radius,
                stops,
                transform,
                spread,
            } => {
                let (attrs, stops) = gradient(stops, *transform, spread);
                format!(
                    "<radialGradient {attrs} cx='{}' cy='{}' fx='{}' fy='{}' r='{radius}'>{stops}</radialGradient>",
                    center[0], center[1], focal[0], focal[1]
                )
            }
            Paint::Pattern {
                width,
                height,
                transform,
                nodes,
            } => {
                let body = self.nodes(nodes, parent * *transform)?;
                format!(
                    "<pattern id='{id}' patternUnits='userSpaceOnUse' width='{width}' height='{height}' patternTransform='{}'>{body}</pattern>",
                    matrix(*transform)
                )
            }
            _ => unreachable!(),
        };
        self.defs.push(def);
        Ok((format!("url(#{id})"), 1.))
    }
    fn image(&mut self, resource: &str, rect: Rect, nearest: bool) -> Result<String> {
        let id = if let Some(id) = self.images.get(resource) {
            id.clone()
        } else {
            let r = self
                .list
                .resources
                .get(resource)
                .ok_or_else(|| error(format!("Missing export image: {resource}")))?;
            if r.data.is_empty() {
                return Err(error(format!("Empty export image: {resource}")));
            }
            // SVG/PDF consumers consistently support PNG/JPEG. Reuse the
            // packaged encoding until export, then convert other codecs once.
            let uri = if matches!(r.mime.as_str(), "image/png" | "image/jpeg") {
                href(&r.mime, &r.data)
            } else {
                let decoded = image::load_from_memory(&r.data).map_err(|e| error(e.to_string()))?;
                let mut png = std::io::Cursor::new(Vec::new());
                decoded
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| error(e.to_string()))?;
                href("image/png", &png.into_inner())
            };
            let id = self.id();
            self.defs.push(format!("<image id='{id}' width='1' height='1' preserveAspectRatio='none' xlink:href='{uri}'/>"));
            self.images.insert(resource.into(), id.clone());
            id
        };
        Ok(format!(
            "<use xlink:href='#{id}' transform='translate({} {}) scale({} {})' image-rendering='{}'/>",
            rect.x0,
            rect.y0,
            rect.width(),
            rect.height(),
            if nearest { "pixelated" } else { "auto" }
        ))
    }
    fn layer(&mut self, node: &DrawNode, transform: Affine, inner: Option<bool>) -> Result<String> {
        if let Some(inner) = inner {
            if node
                .effects
                .iter()
                .filter(|e| e.soft_edge.is_none() && e.inner == inner)
                .count()
                > 1
            {
                let mut body = String::new();
                for effect in node
                    .effects
                    .iter()
                    .filter(|e| e.soft_edge.is_none() && e.inner == inner)
                {
                    let mut single = node.clone();
                    single.effects.retain(|e| e.soft_edge.is_some());
                    single.effects.push(effect.clone());
                    body += &self.layer(&single, transform, Some(inner))?;
                }
                return Ok(body);
            }
        }
        let mut local = node.clone();
        local.transform = Affine::IDENTITY;
        local.opacity = 1.;
        // The largest singular value ensures the requested physical DPI even
        // under skew, rotation and nonuniform scale.
        let scale = max_scale(transform);
        if scale <= 0. {
            return Ok(String::new());
        }
        let b = local.bounds();
        let density = self.dpi / 25.4 * scale;
        let b = Rect::new(
            (b.x0 * density).floor() / density,
            (b.y0 * density).floor() / density,
            (b.x1 * density).ceil() / density,
            (b.y1 * density).ceil() / density,
        );
        if b.width() <= 0. || b.height() <= 0. {
            return Ok(String::new());
        }
        let (w, h) =
            crate::raster_dimensions(b.width() * scale, b.height() * scale, self.dpi, "png")?;
        let halo = (effect_halo(&local) * density).ceil() as u32 + 2;
        let edge = 4096u32.saturating_sub(halo.saturating_mul(2));
        if edge < 64 {
            return Err(error("Filter halo exceeds the 64 MiB export tile budget"));
        }
        let mut body = String::new();
        for y in (0..h).step_by(edge as usize) {
            for x in (0..w).step_by(edge as usize) {
                let tw = (w - x).min(edge);
                let th = (h - y).min(edge);
                let tile = Rect::new(
                    b.x0 + x as f64 / density,
                    b.y0 + y as f64 / density,
                    b.x0 + (x + tw) as f64 / density,
                    b.y0 + (y + th) as f64 / density,
                );
                let data = self.raster.as_mut().unwrap()(&local, tile, tw, th, inner)?;
                body += &image_at(&href("image/png", &data), tile);
            }
        }
        Ok(body)
    }
    fn node(&mut self, n: &DrawNode, parent: Affine) -> Result<String> {
        if n.opacity <= 0. {
            return Ok(String::new());
        }
        let t = parent * n.transform;
        if n.hint["surface3d"].is_object()
            || (self.pdf && n.effects.iter().any(|e| e.soft_edge.is_some()))
        {
            let mut subject = n.clone();
            subject.effects.retain(|e| e.soft_edge.is_some());
            let mut body = String::new();
            if n.effects.iter().any(|e| e.soft_edge.is_none() && !e.inner) {
                body += &self.layer(n, t, Some(false))?;
            }
            body += &self.layer(&subject, t, None)?;
            if n.effects.iter().any(|e| e.soft_edge.is_none() && e.inner) {
                body += &self.layer(n, t, Some(true))?;
            }
            return Ok(format!(
                "<g transform='{}' opacity='{}'>{body}</g>",
                matrix(n.transform),
                n.opacity
            ));
        }
        let body = match &n.kind {
            DrawKind::Group {
                children,
                clip,
                mask,
            } => {
                let mut attrs = String::new();
                if let Some(c) = clip {
                    let id = self.id();
                    self.defs.push(format!("<clipPath id='{id}' clipPathUnits='userSpaceOnUse'><path d='{}' clip-rule='{}'/></clipPath>",c.path.to_svg(),if c.even_odd {"evenodd"} else {"nonzero"}));
                    attrs += &format!(" clip-path='url(#{id})'");
                }
                if let Some(m) = mask {
                    let id = self.id();
                    let body = self.node(&m.node, t)?;
                    let b = n.local_bounds().union(m.node.bounds());
                    self.defs.push(format!("<mask id='{id}' maskUnits='userSpaceOnUse' maskContentUnits='userSpaceOnUse' mask-type='{}' x='{}' y='{}' width='{}' height='{}'>{body}</mask>",if m.luminance {"luminance"} else {"alpha"},b.x0,b.y0,b.width(),b.height()));
                    attrs += &format!(" mask='url(#{id})'");
                }
                // An animation-only reveal clip must not alter static export.
                let body = self.nodes(children, t)?;
                format!("<g{attrs}>{body}</g>")
            }
            DrawKind::Path {
                path,
                fill,
                stroke,
                even_odd,
            } => {
                // svg2pdf does not support reflected/repeated gradients. Use a
                // local CanvasKit fallback for those paints instead of silently
                // changing them to pad; other paths remain vectors.
                if matches!(fill, Paint::Conic { .. })
                    || (self.pdf
                        && self.raster.is_some()
                        && (matches!(fill,Paint::Linear{spread,..}|Paint::Radial{spread,..} if spread!="pad")
                            || matches!(fill,Paint::Radial{stops,..} if stops.windows(2).any(|s| s[0].offset==s[1].offset && s[0].rgba!=s[1].rgba))))
                {
                    if self.raster.is_none() {
                        return Err(error("Conic SVG export requires a local paint rasterizer"));
                    }
                    let mut subject = n.clone();
                    subject.effects.clear();
                    if let DrawKind::Path { stroke, .. } = &mut subject.kind {
                        *stroke = None;
                    }
                    let mut body = self.layer(&subject, t, None)?;
                    if stroke.is_some() {
                        let mut outline = n.clone();
                        outline.effects.clear();
                        outline.opacity = 1.;
                        outline.transform = Affine::IDENTITY;
                        if let DrawKind::Path { fill, .. } = &mut outline.kind {
                            *fill = Paint::None;
                        }
                        body += &self.node(&outline, t)?;
                    }
                    body
                } else {
                    let (fill, alpha) = self.paint(fill, t)?;
                    let stroke=stroke.as_ref().map(|s|format!("stroke='{}' stroke-opacity='{}' stroke-width='{}' stroke-linecap='{}' stroke-linejoin='{}' stroke-miterlimit='{}' stroke-dasharray='{}' stroke-dashoffset='{}'",rgb(s.rgba),s.rgba[3],s.width,escape(&s.cap),escape(&s.join),s.miter,if s.dash.is_empty(){"none".into()}else{s.dash.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")},s.offset)).unwrap_or_default();
                    format!(
                        "<path d='{}' fill='{fill}' fill-opacity='{alpha}' fill-rule='{}' {stroke}/>",
                        path.to_svg(),
                        if *even_odd { "evenodd" } else { "nonzero" }
                    )
                }
            }
            DrawKind::Glyphs {
                outlines, color, ..
            } => outlines
                .iter()
                .map(|p| {
                    format!(
                        "<path d='{}' fill='{}' fill-opacity='{}'/>",
                        p.to_svg(),
                        rgb(*color),
                        color[3]
                    )
                })
                .collect(),
            DrawKind::Image {
                resource,
                rect,
                nearest,
            } => self.image(resource, *rect, *nearest)?,
        };
        let body = if n.effects.is_empty() {
            body
        } else if self.pdf && self.raster.is_some() {
            let back = if n.effects.iter().any(|e| !e.inner) {
                self.layer(n, t, Some(false))?
            } else {
                String::new()
            };
            let front = if n.effects.iter().any(|e| e.inner) {
                self.layer(n, t, Some(true))?
            } else {
                String::new()
            };
            back + &body + &front
        } else {
            let b = n.local_bounds();
            let effects=n.effects.iter().map(|e| if let Some(radius)=e.soft_edge {json!({"kind":"soft_edge","radius":radius})} else if let Some(r)=&e.reflection {json!({"kind":"reflection","distance":r.distance,"length":r.length,"opacity":r.opacity,"fade":r.fade,"blur":e.blur})} else {json!({"mode":if e.inner{"inner"}else{"outer"},"blur":e.blur,"spread":e.spread,"offset":e.offset,"color":rgb(e.rgba),"opacity":e.rgba[3],"sourcePath":e.source_path.as_ref().map(|p|p.to_svg()),"knockout":e.knockout})}).collect::<Vec<_>>();
            effects::apply(
                &json!({"subjectBounds":{"x":b.x0,"y":b.y0,"width":b.width(),"height":b.height()},"effects":effects}),
                body,
                &mut self.defs,
            )
        };
        Ok(format!(
            "<g transform='{}' opacity='{}'>{body}</g>",
            matrix(n.transform),
            n.opacity
        ))
    }
}

pub fn render<'a>(list: &'a DisplayList, dpi: f64, raster: Option<Raster<'a>>) -> Result<String> {
    let pdf = raster.is_some();
    render_with_mode(list, dpi, raster, pdf)
}
pub fn render_with_mode<'a>(
    list: &'a DisplayList,
    dpi: f64,
    raster: Option<Raster<'a>>,
    pdf: bool,
) -> Result<String> {
    if !supported_version(list.schema_version)
        || ![list.width, list.height, dpi]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
    {
        return Err(error("Invalid vector export dimensions"));
    }
    let mut writer = Writer {
        list,
        defs: Vec::new(),
        next: 0,
        images: BTreeMap::new(),
        dpi,
        raster,
        pdf,
    };
    let body = writer.nodes(&list.nodes, Affine::IDENTITY)?;
    Ok(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='{}mm' height='{}mm' viewBox='0 0 {} {}'><defs>{}</defs>{body}</svg>",
        list.width,
        list.height,
        list.width,
        list.height,
        writer.defs.join("")
    ))
}
