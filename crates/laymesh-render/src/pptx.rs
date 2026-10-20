//! Direct DrawingML export from the shared display list.
use super::*;
use crate::display_list::{DrawKind, DrawNode, Paint, Stroke};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
use std::{
    collections::BTreeSet,
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};
mod document;
mod geometry;
mod media;
mod paint;
pub use document::PptxDocument;
const MM_TO_EMU: f64 = 36000.;
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
fn emu(v: f64) -> i64 {
    (v * MM_TO_EMU).round() as i64
}
fn solid(c: [f64; 4], opacity: f64) -> Result<String> {
    Ok(format!(
        "<a:solidFill><a:srgbClr val=\"{:02X}{:02X}{:02X}\"><a:alpha val=\"{}\"/></a:srgbClr></a:solidFill>",
        (c[0].clamp(0., 1.) * 255.).round() as u8,
        (c[1].clamp(0., 1.) * 255.).round() as u8,
        (c[2].clamp(0., 1.) * 255.).round() as u8,
        (c[3] * opacity.clamp(0., 1.) * 100000.).round() as u32
    ))
}
fn xfrm(rect: Rect, rotation: f64) -> String {
    format!(
        "<a:xfrm rot=\"{}\"><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>",
        (rotation.to_degrees().rem_euclid(360.) * 60_000.).round() as u32,
        emu(rect.x0),
        emu(rect.y0),
        emu(rect.width()).max(1),
        emu(rect.height()).max(1)
    )
}
fn group_xfrm(b: Rect) -> String {
    let (x, y, w, h) = (
        emu(b.x0),
        emu(b.y0),
        emu(b.width()).max(1),
        emu(b.height()).max(1),
    );
    // off == chOff and ext == chExt retain absolute child geometry while giving
    // each editable group its own content bounds rather than a page-sized box.
    format!(
        "<a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/><a:chOff x=\"{x}\" y=\"{y}\"/><a:chExt cx=\"{w}\" cy=\"{h}\"/></a:xfrm>"
    )
}

struct Media {
    data: Vec<u8>,
    extension: &'static str,
    kind: &'static str,
}
struct Writer<'a> {
    list: &'a DisplayList,
    dpi: f64,
    next_id: u32,
    playback: Vec<(u32, laymesh_core::media::Cue, bool)>,
    media: Vec<Media>,
    warnings: Vec<Diagnostic>,
    fonts: BTreeSet<String>,
    bounds: Vec<Rect>,
    rasterize: &'a mut dyn FnMut(&DisplayList, Rect, u32, u32) -> Result<Vec<u8>>,
}
impl Writer<'_> {
    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    fn group(&mut self, name: &str, children: &str, start: usize) -> String {
        if children.is_empty() {
            return String::new();
        }
        let id = self.id();
        let bounds = self.bounds[start..]
            .iter()
            .copied()
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::ZERO);
        format!(
            "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{}</p:grpSpPr>{children}</p:grpSp>",
            escape(name),
            group_xfrm(bounds)
        )
    }
    fn path(&mut self, path: BezPath, fill: &str, name: &str) -> String {
        if path.is_empty() {
            return String::new();
        }
        let b = path.bounding_box();
        self.bounds.push(b);
        let mut commands = String::new();
        let pt = |p: Point| {
            format!(
                "<a:pt x=\"{}\" y=\"{}\"/>",
                emu(p.x - b.x0),
                emu(p.y - b.y0)
            )
        };
        for el in path.iter() {
            commands += &match el {
                PathEl::MoveTo(p) => format!("<a:moveTo>{}</a:moveTo>", pt(p)),
                PathEl::LineTo(p) => format!("<a:lnTo>{}</a:lnTo>", pt(p)),
                PathEl::QuadTo(a, b) => format!("<a:quadBezTo>{}{}</a:quadBezTo>", pt(a), pt(b)),
                PathEl::CurveTo(a, b, c) => {
                    format!("<a:cubicBezTo>{}{}{}</a:cubicBezTo>", pt(a), pt(b), pt(c))
                }
                PathEl::ClosePath => "<a:close/>".into(),
            };
        }
        let id = self.id();
        format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{}<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/><a:pathLst><a:path w=\"{}\" h=\"{}\" stroke=\"0\" extrusionOk=\"0\">{commands}</a:path></a:pathLst></a:custGeom>{fill}<a:ln><a:noFill/></a:ln></p:spPr></p:sp>",
            escape(name),
            xfrm(b, 0.),
            emu(b.width()).max(1),
            emu(b.height()).max(1)
        )
    }
    fn picture(
        &mut self,
        data: Vec<u8>,
        extension: &'static str,
        rect: Rect,
        rotation: f64,
        opacity: f64,
        name: &str,
    ) -> String {
        let id = self.id();
        let center = rect.center();
        let rotation_transform = Affine::translate(center.to_vec2())
            * Affine::rotate(rotation)
            * Affine::translate(-center.to_vec2());
        self.bounds
            .push(rotation_transform.transform_rect_bbox(rect));
        let rid = self.image_relationship(data, extension);
        format!(
            "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"rId{rid}\"><a:alphaModFix amt=\"{}\"/></a:blip><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:ln><a:noFill/></a:ln></p:spPr></p:pic>",
            escape(name),
            (opacity.clamp(0., 1.) * 100_000.).round() as u32,
            xfrm(rect, rotation)
        )
    }

    fn fallback(
        &mut self,
        n: &DrawNode,
        parent: Affine,
        opacity: f64,
        name: &str,
        reason: &str,
    ) -> Result<String> {
        // Successful local fallback is an export detail, not an actionable
        // warning. Keep its reason and quality in the picture's diagnostic name.
        let mut node = n.clone();
        node.transform = parent * node.transform;
        node.opacity *= opacity;
        let b = node
            .bounds()
            .intersect(Rect::new(0., 0., self.list.width, self.list.height));
        let density = self.dpi / 25.4;
        let rect = Rect::new(
            (b.x0 * density + 1e-9).floor() / density,
            (b.y0 * density + 1e-9).floor() / density,
            (b.x1 * density - 1e-9).ceil() / density,
            (b.y1 * density - 1e-9).ceil() / density,
        );
        if rect.width() <= 0. || rect.height() <= 0. {
            return Ok(String::new());
        }
        let list = DisplayList {
            nodes: vec![node],
            ..self.list.clone()
        };
        let (w, h) = crate::raster_dimensions(rect.width(), rect.height(), self.dpi, "png")?;
        let mut out = String::new();
        // Bounded tile allocations also apply to large gradient fills.
        for y in (0..h).step_by(4096) {
            for x in (0..w).step_by(4096) {
                let tw = (w - x).min(4096);
                let th = (h - y).min(4096);
                let tile = Rect::new(
                    rect.x0 + x as f64 / density,
                    rect.y0 + y as f64 / density,
                    rect.x0 + (x + tw) as f64 / density,
                    rect.y0 + (y + th) as f64 / density,
                );
                let bytes = (self.rasterize)(&list, tile, tw, th)?;
                out += &self.picture(
                    bytes,
                    "png",
                    tile,
                    0.,
                    1.,
                    &format!(
                        "{name} [raster: {}; {} dpi]",
                        if reason.is_empty() {
                            "gradient fill"
                        } else {
                            reason
                        },
                        self.dpi
                    ),
                );
            }
        }
        Ok(out)
    }

    fn text(
        &mut self,
        n: &DrawNode,
        t: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<Option<String>> {
        let Some((scale, angle)) = similarity(t) else {
            return Ok(None);
        };
        let (content, size, width, ascent, descent, family_name, weight, italic, c, baseline, x) =
            match &n.kind {
                DrawKind::Glyphs {
                    text,
                    size,
                    width,
                    ascent,
                    descent,
                    family,
                    weight,
                    italic,
                    color,
                    ..
                } => (
                    text.as_str(),
                    *size,
                    *width,
                    *ascent,
                    *descent,
                    family.as_str(),
                    *weight,
                    *italic,
                    *color,
                    0.,
                    0.,
                ),
                _ if n.hint["sourceContent"].is_string() => {
                    let h = &n.hint;
                    let size = jnum(h, "fontSize", 3.);
                    (
                        jstr(h, "sourceContent", ""),
                        size,
                        jnum(h, "width", size),
                        size * 0.8,
                        size * 0.2,
                        jstr(h, "requestedFamily", "sans-serif"),
                        jnum(h, "fontWeight", 400.) as u16,
                        h["fontItalic"].as_bool().unwrap_or(false),
                        display_list::rgba(jstr(h, "color", "#000000"))?,
                        jnum(h, "baseline", 0.),
                        jnum(h, "x", 0.),
                    )
                }
                _ => return Ok(None),
            };
        let rect = placed_rect(
            t,
            Rect::new(
                x,
                baseline - size,
                x + width + size,
                baseline - size + (ascent + descent).max(size),
            ),
            scale,
        );
        let center = rect.center();
        let rotation_transform = Affine::translate(center.to_vec2())
            * Affine::rotate(angle)
            * Affine::translate(-center.to_vec2());
        self.bounds
            .push(rotation_transform.transform_rect_bbox(rect));
        self.fonts.insert(family_name.into());
        let id = self.id();
        let size_pt = (size * scale * 72. / 25.4 * 100.).round() as i64;
        let color = solid(c, opacity)?;
        let family = escape(family_name);
        Ok(Some(format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap=\"none\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"t\" vertOverflow=\"overflow\" horzOverflow=\"overflow\"><a:noAutofit/></a:bodyPr><a:lstStyle/><a:p><a:pPr marL=\"0\" indent=\"0\"><a:lnSpc><a:spcPct val=\"100000\"/></a:lnSpc><a:spcBef><a:spcPts val=\"0\"/></a:spcBef><a:spcAft><a:spcPts val=\"0\"/></a:spcAft></a:pPr><a:r><a:rPr sz=\"{size_pt}\" b=\"{}\" i=\"{}\" dirty=\"0\">{color}<a:latin typeface=\"{family}\"/><a:ea typeface=\"{family}\"/><a:cs typeface=\"{family}\"/></a:rPr><a:t xml:space=\"preserve\">{}</a:t></a:r><a:endParaRPr sz=\"{size_pt}\"/></a:p></p:txBody></p:sp>",
            escape(name),
            xfrm(rect, angle),
            u8::from(weight >= 600),
            u8::from(italic),
            escape(content)
        )))
    }
    fn shape(
        &mut self,
        n: &DrawNode,
        path: &BezPath,
        fill: &Paint,
        stroke: Option<&Stroke>,
        even_odd: bool,
        parent: Affine,
        t: Affine,
        opacity: f64,
        inherited: f64,
        name: &str,
    ) -> Result<String> {
        if path.is_empty() {
            return Ok(String::new());
        }
        let mut path = path.clone();
        if even_odd
            && path
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count()
                > 1
            && t.determinant().abs() > 1e-12
        {
            path = t.inverse() * laymesh_core::geometry::union(&(t * path), &BezPath::new());
        }
        let line = geometry::native_stroke(stroke, t, opacity)?;
        let mut g = geometry::geometry(&path, t, line.is_some(), Some(&n.hint));
        let Some(fill_xml) = self.fill(fill, opacity, path.bounding_box(), &g)? else {
            fn has_gradient(p: &Paint) -> bool {
                match p {
                    Paint::Linear { .. } | Paint::Radial { .. } | Paint::Conic { .. } => true,
                    Paint::Pattern { nodes, .. } => nodes.iter().any(|n| match &n.kind {
                        DrawKind::Path { fill, .. } => has_gradient(fill),
                        _ => false,
                    }),
                    _ => false,
                }
            }
            if has_gradient(fill) {
                // Only the unsupported fill is rasterized. The path's outline
                // stays editable; opacity is baked into the fill exactly once.
                let mut fill_node = n.clone();
                if let DrawKind::Path { stroke, .. } = &mut fill_node.kind {
                    *stroke = None;
                }
                let mut out = self.fallback(&fill_node, parent, inherited, name, "")?;
                if stroke.is_some() {
                    out += &self.shape(
                        n,
                        &path,
                        &Paint::None,
                        stroke,
                        even_odd,
                        parent,
                        t,
                        opacity,
                        inherited,
                        name,
                    )?;
                }
                return Ok(out);
            }
            return self.fallback(n, parent, inherited, name, "无法准确映射的纹理或图片填充");
        };
        let outline = stroke.map(|s| {
            laymesh_core::geometry::outline_transformed(&path, &geometry::stroke_style(s), t)
        });
        if let Some(line) = line {
            if let Some(outline) = outline {
                if !outline.is_empty() {
                    g.bounds = g.bounds.union(outline.bounding_box());
                }
            }
            return Ok(self.emit_shape(name, g, &fill_xml, &line));
        }
        let mut out = if matches!(fill, Paint::None) {
            String::new()
        } else {
            self.emit_shape(name, g, &fill_xml, "<a:ln><a:noFill/></a:ln>")
        };
        if let (Some(s), Some(outline)) = (stroke, outline) {
            out += &self.path(
                outline,
                &solid(s.rgba, opacity)?,
                &format!("{name} stroke outline"),
            );
        }
        Ok(out)
    }
    fn node(
        &mut self,
        n: &DrawNode,
        parent: Affine,
        inherited: f64,
        in_formula: bool,
        owner: &str,
    ) -> Result<String> {
        let name = if n.name.is_empty() {
            owner
        } else {
            n.name.as_str()
        };
        let t = parent * n.transform;
        let opacity = inherited * n.opacity;
        let formula = in_formula || n.semantic.is_some();
        if n.hint["mediaId"].is_string() {
            return self.video(n, t, opacity, name);
        }
        // Keep descendants playable when a group effect cannot be represented.
        // Regular siblings retain the normal editable-object export pipeline.
        fn contains_media(n: &DrawNode) -> bool {
            n.hint["mediaId"].is_string()
                || matches!(&n.kind,DrawKind::Group{children,..} if children.iter().any(contains_media))
        }
        if let DrawKind::Group {
            children,
            clip,
            mask,
        } = &n.kind
        {
            if children.iter().any(contains_media)
                && (!n.effects.is_empty() || clip.is_some() || mask.is_some() || n.opacity != 1.)
            {
                self.warnings.push(Diagnostic::new("W_PPTX_MEDIA",format!("Group {name}: unsupported group effects, clipping, mask or opacity omitted; native media playback retained"),"",Loc::default()));
                let start = self.bounds.len();
                let mut out = String::new();
                for child in children {
                    out += &self.node(child, t, inherited, formula, name)?;
                }
                return Ok(self.group(name, &out, start));
            }
        }
        if n.effects.iter().any(|e| e.soft_edge.is_none()) {
            // An isolated translucent composition cannot be split without changing overlaps.
            if n.opacity != 1. {
                return self.fallback(n, parent, inherited, name, "透明效果组合");
            }
            let mut out = String::new();
            let mut subject = n.clone();
            subject.effects.retain(|e| e.soft_edge.is_some());
            for inner in [false, true] {
                if inner {
                    out += &self.node(&subject, parent, inherited, in_formula, name)?;
                }
                let mut layer = n.clone();
                layer
                    .effects
                    .retain(|e| e.soft_edge.is_some() || e.inner == inner);
                if layer.effects.iter().any(|e| e.soft_edge.is_none()) {
                    layer.hint["effectsOnly"] = json!(true);
                    out += &self.fallback(&layer, parent, inherited, name, "独立效果层")?;
                }
            }
            return Ok(out);
        }
        if n.hint["surface3d"].is_object() || n.effects.iter().any(|e| e.soft_edge.is_some()) {
            return self.fallback(n, parent, inherited, name, "对象级三维或柔化边缘");
        }
        if opacity != 1.
            && matches!(&n.kind, DrawKind::Path { fill, stroke: Some(_), .. } if !matches!(fill, Paint::None))
        {
            // DrawingML applies fill/line alpha independently. Their overlap
            // must be composited before applying this object's opacity once.
            return self.fallback(n, parent, inherited, name, "填充与描边透明度合成");
        }
        if !formula && n.hint.get("exportText").is_some() {
            let mut text = n.clone();
            let mut kind = n.hint["exportText"].clone();
            kind["kind"] = json!("glyphs");
            kind["resource"] = json!("");
            kind["glyphs"] = json!([]);
            kind["outlines"] = json!([]);
            text.kind = serde_json::from_value(kind).map_err(|e| error(e.to_string()))?;
            if let Some(result) = self.text(&text, t, opacity, name)? {
                return Ok(result);
            }
        }
        if let DrawKind::Group {
            children,
            clip,
            mask,
        } = &n.kind
        {
            if mask.is_some() {
                return self.fallback(n, parent, inherited, name, "蒙版");
            }
            if let Some(clip) = clip {
                if let Some(out) = self.clipped_image(children, &clip.path, t, opacity, name)? {
                    return Ok(out);
                }
                return self.fallback(n, parent, inherited, name, "复杂裁剪");
            }
            if n.opacity != 1. && paint_count(n) > 1 {
                return self.fallback(n, parent, inherited, name, "分组透明度合成");
            }
            let start = self.bounds.len();
            let mut out = String::new();
            for child in children {
                out += &self.node(child, t, opacity, formula, name)?;
            }
            if n.hint["logicalKind"] == "group"
                || n.semantic.is_some()
                || (!n.name.is_empty() && self.bounds.len() - start > 1)
            {
                return Ok(self.group(name, &out, start));
            }
            return Ok(out);
        }
        match &n.kind {
            DrawKind::Glyphs {
                outlines, color, ..
            } if formula => {
                let fill = solid(*color, opacity)?;
                let mut out = String::new();
                for path in outlines {
                    out += &self.path(t * path.clone(), &fill, name);
                }
                Ok(out)
            }
            DrawKind::Glyphs { .. } => {
                if let Some(text) = self.text(n, t, opacity, name)? {
                    Ok(text)
                } else {
                    self.fallback(n, parent, inherited, name, "文字变换或字体无法用文本框表达")
                }
            }
            DrawKind::Path {
                path,
                fill,
                stroke,
                even_odd,
            } => {
                if !formula && n.hint["sourceContent"].is_string() {
                    if let Some(text) = self.text(n, t, opacity, name)? {
                        return Ok(text);
                    }
                }
                self.shape(
                    n,
                    path,
                    fill,
                    stroke.as_ref(),
                    *even_odd,
                    parent,
                    t,
                    opacity,
                    inherited,
                    name,
                )
            }
            DrawKind::Image { resource, rect, .. } => {
                let Some((b, transform, paint)) = geometry::frame(t, *rect) else {
                    return self.fallback(n, parent, inherited, name, "图片斜切变换");
                };
                let data = self.image_data(resource)?;
                let fill = self.blip_fill(data, opacity, [0.; 4], [0.; 4]);
                Ok(self.emit_shape(
                    name,
                    geometry::Geometry {
                        rect: b,
                        bounds: t.transform_rect_bbox(*rect),
                        xml: geometry::preset("rect", 0., b),
                        transform,
                        paint_transform: paint,
                    },
                    &fill,
                    "<a:ln><a:noFill/></a:ln>",
                ))
            }
            DrawKind::Group { .. } => unreachable!(),
        }
    }
}
fn paint_count(n: &DrawNode) -> usize {
    match &n.kind {
        DrawKind::Group { children, .. } => children.iter().map(paint_count).sum(),
        DrawKind::Path { fill, stroke, .. } => {
            usize::from(!matches!(fill, Paint::None)) + usize::from(stroke.is_some())
        }
        _ => 1,
    }
}
fn similarity(t: Affine) -> Option<(f64, f64)> {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let s = a.hypot(b);
    if s <= 1e-12 || (a - d).abs() > s * 1e-8 || (b + c).abs() > s * 1e-8 {
        None
    } else {
        Some((s, b.atan2(a)))
    }
}
fn placed_rect(t: Affine, r: Rect, scale: f64) -> Rect {
    Rect::from_center_size(t * r.center(), (r.width() * scale, r.height() * scale))
}
#[cfg(feature = "native")]
fn preserve_jpeg(value: &mut Json) {
    if let (Some(key), Some(hash)) = (
        value["rasterKey"].as_str(),
        value["sourceJpegHash"].as_str(),
    ) {
        if let Some(bytes) = laymesh_core::asset_cache::source_jpeg(key) {
            if laymesh_core::asset_cache::digest(&bytes) == hash {
                value["mime"] = Json::String("image/jpeg".into());
                value["data"] =
                    Json::String(base64::engine::general_purpose::STANDARD.encode(&*bytes));
            }
        }
    }
}
#[cfg(feature = "native")]
fn normalize_caps(nodes: &mut [Json]) {
    for n in nodes {
        let raster_subject =
            n["threeD"].is_object() || n["effects"].as_array().is_some_and(|e| !e.is_empty());
        if !raster_subject && n["kind"] == "image" && n["sourceImage"].is_object() {
            let source = n["sourceImage"].clone();
            for key in ["data", "mime", "rasterKey", "sourceJpegHash", "crop"] {
                n[key] = source[key].clone();
            }
            n["intrinsicWidth"] = source["width"].clone();
            n["intrinsicHeight"] = source["height"].clone();
        }
        if !raster_subject {
            preserve_jpeg(n);
        }
        if n["fill"].is_object() {
            preserve_jpeg(&mut n["fill"]);
        }
        // The display list physically crops PNGs for raster/PDF edge sampling. PPTX
        // retains the original resource and maps this crop to DrawingML.
        if !raster_subject && n["kind"] == "image" && n["crop"].is_object() {
            n["_pptxKeepSourceCrop"] = Json::Bool(true);
        }
        let r = &n["endpointRecipe"];
        if r.is_object()
            && !r["start_head"].is_object()
            && !r["end_head"].is_object()
            && r["start_cap"] == r["end_cap"]
            // DrawingML applies the cap to every dash. Endpoint-only caps
            // with a different dash-body cap must retain the exact outline.
            && (n["strokeStyle"]["dash"].as_array().is_none_or(|d| d.is_empty())
                || r["start_cap"] == n["strokeStyle"]["cap"])
            && n["zeroAngle"].is_null()
        {
            let cap = r["start_cap"].clone();
            n["strokeStyle"]["cap"] = cap;
            n.as_object_mut().unwrap().remove("endpointRecipe");
        }
        if let Some(children) = n["children"].as_array_mut() {
            normalize_caps(children);
        }
    }
}
#[cfg(feature = "native")]
pub fn render_pptx(scene: &Scene, options: &ExportOptions) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
    options.validate("pptx")?;
    if ![scene.width, scene.height]
        .into_iter()
        .all(|v| (25.4..=1422.4).contains(&v))
    {
        return Err(error("PPTX 画布宽高须为 25.4–1422.4 mm（1–56 英寸）"));
    }
    let dpi = options.dpi.unwrap_or(scene.export_dpi);
    ExportOptions {
        dpi: Some(dpi),
        ..Default::default()
    }
    .validate("pptx")?;
    let mut lowered = scene.clone();
    for node in &mut lowered.nodes {
        normalize_caps(std::slice::from_mut(std::sync::Arc::make_mut(node)));
    }
    let list = to_display_list(&lowered)?;
    render_pptx_list(&list, dpi, &mut |list, rect, w, h| {
        let frame = raster::render_region(list, rect, w, h)?;
        let im = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba)
            .ok_or_else(|| error("PPTX 图片尺寸无效"))?;
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(im)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| error(e.to_string()))?;
        Ok(bytes.into_inner())
    })
}

/// DrawingML is platform-independent; the host supplies raster fallbacks only.
pub fn render_pptx_list(
    list: &DisplayList,
    dpi: f64,
    rasterize: &mut dyn FnMut(&DisplayList, Rect, u32, u32) -> Result<Vec<u8>>,
) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
    if ![list.width, list.height]
        .into_iter()
        .all(|v| (25.4..=1422.4).contains(&v))
    {
        return Err(error("PPTX 画布宽高须为 25.4–1422.4 mm（1–56 英寸）"));
    }
    ExportOptions {
        dpi: Some(dpi),
        ..Default::default()
    }
    .validate("pptx")?;
    let mut writer = Writer {
        list,
        dpi,
        rasterize,
        next_id: 2,
        playback: vec![],
        media: vec![],
        warnings: vec![],
        fonts: BTreeSet::new(),
        bounds: vec![],
    };
    let mut body = String::new();
    for n in &list.nodes {
        body += &writer.node(n, Affine::IDENTITY, 1., false, "canvas")?;
    }
    body += &writer.audio()?;
    let timing = writer.timing();
    if !writer.fonts.is_empty() {
        writer.warnings.push(Diagnostic::new(
            "W_PPTX_FONT",
            format!(
                "可编辑文字未嵌入字体；目标机器需安装：{}。字体替换可能改变排版",
                writer.fonts.into_iter().collect::<Vec<_>>().join("、")
            ),
            "",
            Loc::default(),
        ));
    }
    Ok((
        package(list, &body, &writer.media, &timing)?,
        writer.warnings,
    ))
}
fn package(scene: &DisplayList, body: &str, media: &[Media], timing: &str) -> Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let mut add = |path: &str, data: &[u8]| -> Result<()> {
        zip.start_file(
            path,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(|e| error(e.to_string()))?;
        zip.write_all(data).map_err(|e| error(e.to_string()))?;
        Ok(())
    };
    let content_types=format!("{XML}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/><Default Extension=\"mp4\" ContentType=\"video/mp4\"/><Default Extension=\"m4a\" ContentType=\"audio/mp4\"/><Default Extension=\"mp3\" ContentType=\"audio/mpeg\"/><Default Extension=\"wav\" ContentType=\"audio/wav\"/>{}</Types>",[
        ("/ppt/presentation.xml","presentation.main"),("/ppt/slides/slide1.xml","slide"),("/ppt/slideMasters/slideMaster1.xml","slideMaster"),("/ppt/slideLayouts/slideLayout1.xml","slideLayout")].iter().map(|(part,kind)|format!("<Override PartName=\"{part}\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.{kind}+xml\"/>")).collect::<String>()+"<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>");
    add("[Content_Types].xml", content_types.as_bytes())?;
    add(
        "_rels/.rels",
        rels(&[("rId1", "officeDocument", "ppt/presentation.xml")]).as_bytes(),
    )?;
    add("ppt/presentation.xml",format!("{XML}<p:presentation xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/></p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\" type=\"custom\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/><p:defaultTextStyle/></p:presentation>",emu(scene.width),emu(scene.height)).as_bytes())?;
    add(
        "ppt/_rels/presentation.xml.rels",
        rels(&[
            ("rId1", "slideMaster", "slideMasters/slideMaster1.xml"),
            ("rId2", "slide", "slides/slide1.xml"),
        ])
        .as_bytes(),
    )?;
    let root_group = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>";
    add("ppt/slides/slide1.xml",format!("{XML}<p:sld xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\"><p:cSld><p:spTree>{root_group}{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>{timing}</p:sld>").as_bytes())?;
    let mut slide_rels = vec![(
        "rId1".to_string(),
        "slideLayout",
        "../slideLayouts/slideLayout1.xml".to_string(),
    )];
    for (i, m) in media.iter().enumerate() {
        slide_rels.push((
            format!("rId{}", i + 2),
            m.kind,
            format!("../media/image{}.{}", i + 1, m.extension),
        ));
    }
    for (i, m) in media.iter().enumerate().filter(|(_, m)| m.kind != "image") {
        slide_rels.push((
            format!("media{}", i + 2),
            "http://schemas.microsoft.com/office/2007/relationships/media",
            format!("../media/image{}.{}", i + 1, m.extension),
        ));
    }
    add(
        "ppt/slides/_rels/slide1.xml.rels",
        rels(
            &slide_rels
                .iter()
                .map(|(id, kind, target)| (id.as_str(), *kind, target.as_str()))
                .collect::<Vec<_>>(),
        )
        .as_bytes(),
    )?;
    let clr = "<p:clrMap accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" bg1=\"lt1\" bg2=\"lt2\" folHlink=\"folHlink\" hlink=\"hlink\" tx1=\"dk1\" tx2=\"dk2\"/>";
    add("ppt/slideMasters/slideMaster1.xml",format!("{XML}<p:sldMaster xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\"><p:cSld><p:spTree>{root_group}</p:spTree></p:cSld>{clr}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>").as_bytes())?;
    add(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        rels(&[
            ("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"),
            ("rId2", "theme", "../theme/theme1.xml"),
        ])
        .as_bytes(),
    )?;
    add("ppt/slideLayouts/slideLayout1.xml",format!("{XML}<p:sldLayout xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\" type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree>{root_group}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>").as_bytes())?;
    add(
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        rels(&[("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")]).as_bytes(),
    )?;
    add("ppt/theme/theme1.xml", theme().as_bytes())?;
    for (i, m) in media.iter().enumerate() {
        add(
            &format!("ppt/media/image{}.{}", i + 1, m.extension),
            &m.data,
        )?;
    }
    drop(add);
    Ok(zip.finish().map_err(|e| error(e.to_string()))?.into_inner())
}
fn rels(items: &[(&str, &str, &str)]) -> String {
    format!(
        "{XML}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}</Relationships>",
        items
            .iter()
            .map(|(id, kind, target)| format!(
                "<Relationship Id=\"{id}\" Type=\"{}\" Target=\"{target}\"/>",
                if kind.starts_with("http") {
                    kind.to_string()
                } else {
                    format!("{R}/{kind}")
                }
            ))
            .collect::<String>()
    )
}
fn theme() -> String {
    let colors = [
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "203864"),
        ("lt2", "EEEEEE"),
        ("accent1", "4472C4"),
        ("accent2", "ED7D31"),
        ("accent3", "A5A5A5"),
        ("accent4", "FFC000"),
        ("accent5", "5B9BD5"),
        ("accent6", "70AD47"),
        ("hlink", "0563C1"),
        ("folHlink", "954F72"),
    ]
    .iter()
    .map(|(key, value)| format!("<a:{key}><a:srgbClr val=\"{value}\"/></a:{key}>"))
    .collect::<String>();
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line = format!("<a:ln w=\"12700\">{fill}<a:prstDash val=\"solid\"/></a:ln>");
    format!(
        "{XML}<a:theme xmlns:a=\"{A}\" name=\"LayMesh\"><a:themeElements><a:clrScheme name=\"LayMesh\">{colors}</a:clrScheme><a:fontScheme name=\"LayMesh\"><a:majorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme><a:fmtScheme name=\"LayMesh\"><a:fillStyleLst>{}</a:fillStyleLst><a:lnStyleLst>{}</a:lnStyleLst><a:effectStyleLst>{}</a:effectStyleLst><a:bgFillStyleLst>{}</a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>",
        fill.repeat(3),
        line.repeat(3),
        "<a:effectStyle><a:effectLst/></a:effectStyle>".repeat(3),
        fill.repeat(3)
    )
}
