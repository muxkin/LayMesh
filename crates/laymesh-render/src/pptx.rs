//! Single-slide DrawingML export. The existing SVG lowering is the
//! geometry authority; this module preserves supported subtrees as native objects
//! and rasterizes only a subtree whose compositing cannot be represented exactly.
use super::*;
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
use resvg::{tiny_skia, usvg};
use roxmltree::Node;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};

mod geometry;
mod paint;

const MM_TO_EMU: f64 = 36_000.;
const PX_TO_MM: f64 = 25.4 / 96.;
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
fn emu(v: f64) -> i64 {
    (v * MM_TO_EMU).round() as i64
}
fn attr(n: Node<'_, '_>, key: &str, default: f64) -> f64 {
    n.attribute(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn solid(value: &str, opacity: f64) -> Result<String> {
    if value == "none" {
        return Ok("<a:noFill/>".into());
    }
    let c: svgtypes::Color = value
        .parse()
        .map_err(|_| error(format!("PPTX 无效颜色：{value}")))?;
    Ok(format!(
        "<a:solidFill><a:srgbClr val=\"{:02X}{:02X}{:02X}\"><a:alpha val=\"{}\"/></a:srgbClr></a:solidFill>",
        c.red,
        c.green,
        c.blue,
        (opacity.clamp(0., 1.) * c.alpha as f64 / 255. * 100_000.).round() as u32
    ))
}
fn affine(n: Node<'_, '_>) -> Result<Affine> {
    let t: svgtypes::Transform = n
        .attribute("transform")
        .unwrap_or("")
        .parse()
        .map_err(|e| error(format!("PPTX 无效变换：{e}")))?;
    Ok(Affine::new([t.a, t.b, t.c, t.d, t.e, t.f]))
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
}
struct Writer<'a> {
    scene: &'a Scene,
    object_kinds: BTreeMap<String, String>,
    recipes: BTreeMap<String, Json>,
    source: &'a str,
    defs: String,
    dpi: f64,
    next_id: u32,
    media: Vec<Media>,
    warnings: Vec<Diagnostic>,
    fonts: BTreeSet<String>,
    bounds: Vec<Rect>,
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
    fn subtree_svg(&self, n: Node<'_, '_>, parent: Affine) -> String {
        let [a, b, c, d, e, f] = parent.as_coeffs();
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\"><defs>{}</defs><g transform=\"matrix({a} {b} {c} {d} {e} {f})\">{}</g></svg>",
            self.scene.width,
            self.scene.height,
            self.scene.width,
            self.scene.height,
            self.defs,
            &self.source[n.range()]
        )
    }
    fn fallback(
        &mut self,
        n: Node<'_, '_>,
        parent: Affine,
        opacity: f64,
        name: &str,
        reason: &str,
    ) -> Result<String> {
        self.warnings.push(Diagnostic::new(
            "W_PPTX_RASTER",
            format!("对象 {name} 转为透明 PNG：{reason}"),
            "",
            Loc::default(),
        ));
        let tree = native::tree_svg(self.scene, &self.subtree_svg(n, parent))?;
        if tree.root().children().is_empty() {
            return Ok(String::new());
        }
        // usvg supplies visual bounds including stroke/filter overflow. Restrict to
        // the original canvas, then align the crop to the export pixel grid.
        let b = tree.root().abs_layer_bounding_box();
        let scale = self.dpi / 96.;
        let x0 = (b.x().max(0.) as f64 * scale).floor();
        let y0 = (b.y().max(0.) as f64 * scale).floor();
        let x1 = ((b.right().min(tree.size().width())) as f64 * scale).ceil();
        let y1 = ((b.bottom().min(tree.size().height())) as f64 * scale).ceil();
        if x1 <= x0 || y1 <= y0 {
            return Ok(String::new());
        }
        let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
        if w as u64 * h as u64 > 100_000_000 {
            return Err(error("PPTX 局部回退像素数超过 100000000 上限"));
        }
        let mut pixels =
            tiny_skia::Pixmap::new(w, h).ok_or_else(|| error("无法分配 PPTX 回退图片"))?;
        let transform = tiny_skia::Transform::from_row(
            scale as f32,
            0.,
            0.,
            scale as f32,
            -x0 as f32,
            -y0 as f32,
        );
        resvg::render(&tree, transform, &mut pixels.as_mut());
        let png = pixels
            .encode_png()
            .map_err(|e| error(format!("PPTX PNG 编码失败：{e}")))?;
        let mm = 25.4 / self.dpi;
        Ok(self.picture(
            png,
            "png",
            Rect::new(x0 * mm, y0 * mm, x1 * mm, y1 * mm),
            0.,
            opacity,
            &format!("{name} [raster: {reason}]"),
        ))
    }
    fn text(
        &mut self,
        n: Node<'_, '_>,
        transform: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<Option<String>> {
        let Some((scale, angle)) = similarity(transform) else {
            return Ok(None);
        };
        let key = n.attribute("font-family").unwrap_or("");
        let missing_family = n.attribute("data-text-family");
        let asset = self.scene.fonts.get(key).or_else(|| {
            missing_family.and_then(|family| {
                self.scene
                    .fonts
                    .values()
                    .find(|asset| asset.family == family)
            })
        });
        if asset.is_none() && missing_family.is_none() {
            return Ok(None);
        }
        let face = asset
            .map(|asset| ttf_parser::Face::parse(&asset.data, asset.index))
            .transpose()
            .map_err(|_| error("PPTX 无效字体"))?;
        let size = attr(
            n,
            if missing_family.is_some() {
                "data-font-size"
            } else {
                "font-size"
            },
            3.,
        );
        let units = face
            .as_ref()
            .map_or(1000., |face| face.units_per_em() as f64);
        let ascent = face
            .as_ref()
            .map_or(size * 0.8, |face| face.ascender() as f64 / units * size);
        let descent = face
            .as_ref()
            .map_or(size * 0.2, |face| -(face.descender() as f64) / units * size);
        let content = n
            .attribute("data-source-content")
            .unwrap_or_else(|| n.text().unwrap_or(""));
        let baseline = attr(
            n,
            if missing_family.is_some() {
                "data-baseline"
            } else {
                "y"
            },
            0.,
        );
        // DrawingML top-anchored paragraphs at 100% line spacing place the first
        // baseline one em below the box top (independent of font ink bounds).
        // No wrapping or auto-fit: width is deliberately generous. The original
        // layout already positions each run independently, including mixed styles.
        let width = content
            .chars()
            .map(|ch| {
                face.as_ref()
                    .and_then(|face| {
                        face.glyph_index(ch)
                            .and_then(|id| face.glyph_hor_advance(id))
                    })
                    .map_or(size * 0.7, |advance| advance as f64 / units * size)
            })
            .sum::<f64>()
            + size;
        let height = (ascent + descent).max(size);
        let rect = placed_rect(
            transform,
            Rect::new(
                attr(n, "x", 0.),
                baseline - size,
                attr(n, "x", 0.) + width,
                baseline - size + height,
            ),
            scale,
        );
        let center = rect.center();
        let rotation_transform = Affine::translate(center.to_vec2())
            * Affine::rotate(angle)
            * Affine::translate(-center.to_vec2());
        self.bounds
            .push(rotation_transform.transform_rect_bbox(rect));
        let family_name = asset
            .map(|asset| asset.family.as_str())
            .unwrap_or(missing_family.unwrap_or("sans-serif"));
        self.fonts.insert(family_name.into());
        let id = self.id();
        let size_pt = (size * scale * 72. / 25.4 * 100.).round() as i64;
        let color = solid(
            n.attribute(if missing_family.is_some() {
                "stroke"
            } else {
                "fill"
            })
            .unwrap_or("#000000"),
            opacity * attr(n, "fill-opacity", 1.),
        )?;
        let weight = attr(
            n,
            if missing_family.is_some() {
                "data-font-weight"
            } else {
                "font-weight"
            },
            400.,
        );
        let italic = n.attribute(if missing_family.is_some() {
            "data-font-style"
        } else {
            "font-style"
        }) == Some("italic");
        let family = escape(family_name);
        Ok(Some(format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap=\"none\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"t\" vertOverflow=\"overflow\" horzOverflow=\"overflow\"><a:noAutofit/></a:bodyPr><a:lstStyle/><a:p><a:pPr marL=\"0\" indent=\"0\"><a:lnSpc><a:spcPct val=\"100000\"/></a:lnSpc><a:spcBef><a:spcPts val=\"0\"/></a:spcBef><a:spcAft><a:spcPts val=\"0\"/></a:spcAft></a:pPr><a:r><a:rPr sz=\"{size_pt}\" b=\"{}\" i=\"{}\" dirty=\"0\">{color}<a:latin typeface=\"{family}\"/><a:ea typeface=\"{family}\"/><a:cs typeface=\"{family}\"/></a:rPr><a:t xml:space=\"preserve\">{}</a:t></a:r><a:endParaRPr sz=\"{size_pt}\"/></a:p></p:txBody></p:sp>",
            escape(name),
            xfrm(rect, angle),
            u8::from(weight >= 600.),
            u8::from(italic),
            escape(content)
        )))
    }
    fn outline_text(
        &mut self,
        n: Node<'_, '_>,
        parent: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<String> {
        let tree = native::tree_svg(self.scene, &self.subtree_svg(n, parent))?;
        fn visit(w: &mut Writer<'_>, g: &usvg::Group, opacity: f64, name: &str) -> Result<String> {
            let mut out = String::new();
            for n in g.children() {
                match n {
                    usvg::Node::Group(g) => {
                        out += &visit(w, g, opacity * g.opacity().get() as f64, name)?
                    }
                    usvg::Node::Text(t) => out += &visit(w, t.flattened(), opacity, name)?,
                    usvg::Node::Path(p) => {
                        let mut path = BezPath::new();
                        for s in p.data().segments() {
                            match s {
                                tiny_skia::PathSegment::MoveTo(p) => {
                                    path.move_to((p.x as f64, p.y as f64))
                                }
                                tiny_skia::PathSegment::LineTo(p) => {
                                    path.line_to((p.x as f64, p.y as f64))
                                }
                                tiny_skia::PathSegment::QuadTo(a, b) => {
                                    path.quad_to((a.x as f64, a.y as f64), (b.x as f64, b.y as f64))
                                }
                                tiny_skia::PathSegment::CubicTo(a, b, c) => path.curve_to(
                                    (a.x as f64, a.y as f64),
                                    (b.x as f64, b.y as f64),
                                    (c.x as f64, c.y as f64),
                                ),
                                tiny_skia::PathSegment::Close => path.close_path(),
                            }
                        }
                        if let Some(f) = p.fill() {
                            let usvg::Paint::Color(c) = f.paint() else {
                                return Err(error("PPTX 公式轮廓包含不支持的填充"));
                            };
                            let t = p.abs_transform();
                            let transform = Affine::scale(PX_TO_MM)
                                * Affine::new([
                                    t.sx as f64,
                                    t.ky as f64,
                                    t.kx as f64,
                                    t.sy as f64,
                                    t.tx as f64,
                                    t.ty as f64,
                                ]);
                            out += &w.path(
                                transform * path,
                                &solid(
                                    &format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue),
                                    opacity * f.opacity().get() as f64,
                                )?,
                                name,
                            );
                        }
                    }
                    _ => return Err(error("PPTX 公式文字无法转换为轮廓")),
                }
            }
            Ok(out)
        }
        visit(self, tree.root(), opacity, name)
    }
    fn shape(
        &mut self,
        n: Node<'_, '_>,
        parent: Affine,
        transform: Affine,
        opacity: f64,
        inherited_opacity: f64,
        name: &str,
    ) -> Result<String> {
        let mut path = geometry::svg_path(n)?;
        if path.is_empty() {
            return Ok(String::new());
        }
        let value = n.attribute("fill").unwrap_or("#000000");
        if n.attribute("fill-rule") == Some("evenodd")
            && value != "none"
            && path
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count()
                > 1
        {
            if n.attribute("fill-opacity").is_some() && transform.determinant().abs() > 1e-12 {
                // Shared SVG marks generated compound-stroke outlines with
                // fill-opacity. Normalize their even-odd contours in final mm
                // so DrawingML's nonzero fill keeps the exact vector region.
                let physical = transform * path;
                path =
                    transform.inverse() * laymesh_core::geometry::union(&physical, &BezPath::new());
            } else {
                return self.fallback(n, parent, inherited_opacity, name, "多轮廓奇偶填充");
            }
        }
        let line = geometry::native_stroke(n, transform, opacity)?;
        let mut g = geometry::geometry(n, &path, transform, line.is_some(), self.recipes.get(name));
        let Some(fill) = self.fill(n, opacity, path.bounding_box(), &g)? else {
            return self.fallback(
                n,
                parent,
                inherited_opacity,
                name,
                "无法准确映射的渐变、纹理或图片填充",
            );
        };
        if let Some(line) = line {
            if n.attribute("stroke").unwrap_or("none") != "none" {
                let outline = laymesh_core::geometry::outline_transformed(
                    &path,
                    &geometry::stroke_style(n)?,
                    transform,
                );
                if !outline.is_empty() {
                    g.bounds = g.bounds.union(outline.bounding_box());
                }
            }
            return Ok(self.emit_shape(name, g, &fill, &line));
        }
        let stroke = n.attribute("stroke").unwrap_or("none");
        if stroke.starts_with("url(") {
            return self.fallback(n, parent, inherited_opacity, name, "无法准确映射的渐变描边");
        }
        let mut out = if value == "none" {
            String::new()
        } else {
            self.emit_shape(name, g, &fill, "<a:ln><a:noFill/></a:ln>")
        };
        let style = geometry::stroke_style(n)?;
        let outline = laymesh_core::geometry::outline_transformed(&path, &style, transform);
        out += &self.path(
            outline,
            &solid(stroke, opacity * attr(n, "stroke-opacity", 1.))?,
            &format!("{name} stroke outline"),
        );
        Ok(out)
    }
    fn node(
        &mut self,
        n: Node<'_, '_>,
        parent: Affine,
        inherited_opacity: f64,
        in_formula: bool,
        owner: &str,
    ) -> Result<String> {
        let tag = n.tag_name().name();
        if matches!(tag, "defs" | "style" | "title" | "desc") {
            return Ok(String::new());
        }
        let name = n
            .attribute("data-id")
            .filter(|s| !s.is_empty())
            .or_else(|| n.attribute("data-latex-source"))
            .unwrap_or(owner);
        let transform = parent * affine(n)?;
        let opacity = inherited_opacity * attr(n, "opacity", 1.);
        let formula = in_formula || n.attribute("data-latex-source").is_some();
        if n.attribute("clip-path").is_some()
            && n.attribute("filter").is_none()
            && n.attribute("mask").is_none()
        {
            if let Some(image) = self.clipped_image(n, transform, opacity, name)? {
                return Ok(image);
            }
        }
        let reason = if n.attribute("clip-path").is_some() {
            Some("复杂裁剪")
        } else if n.attribute("filter").is_some() {
            Some("滤镜或特效")
        } else if n.attribute("mask").is_some() {
            Some("蒙版")
        } else if matches!(tag, "g" | "svg") && attr(n, "opacity", 1.) != 1. && paint_count(n) > 1 {
            Some("分组透明度合成")
        } else if tag == "svg" {
            Some("嵌套 SVG 图片或视口")
        } else {
            None
        };
        if let Some(reason) = reason {
            return self.fallback(n, parent, inherited_opacity, name, reason);
        }
        match tag {
            "g" => {
                let start = self.bounds.len();
                let mut out = String::new();
                for child in n.children().filter(Node::is_element) {
                    out += &self.node(child, transform, opacity, formula, name)?;
                }
                let kind = n
                    .attribute("data-id")
                    .and_then(|id| self.object_kinds.get(id))
                    .map(String::as_str);
                let logical_group =
                    kind == Some("group") || n.attribute("data-latex-source").is_some();
                let compound = n.attribute("data-id").is_some()
                    && kind != Some("formula")
                    && self.bounds.len() - start > 1;
                if logical_group || compound {
                    Ok(self.group(name, &out, start))
                } else {
                    Ok(out)
                }
            }
            "text" => {
                if formula {
                    return self.outline_text(n, parent, inherited_opacity, name);
                }
                if let Some(text) = self.text(n, transform, opacity, name)? {
                    Ok(text)
                } else {
                    self.fallback(
                        n,
                        parent,
                        inherited_opacity,
                        name,
                        "文字变换或字体无法用文本框表达",
                    )
                }
            }
            "rect" if !formula && n.attribute("data-source-content").is_some() => {
                if let Some(text) = self.text(n, transform, opacity, name)? {
                    Ok(text)
                } else {
                    self.fallback(
                        n,
                        parent,
                        inherited_opacity,
                        name,
                        "文字变换无法用文本框表达",
                    )
                }
            }
            "path" | "rect" | "ellipse" | "circle" => {
                self.shape(n, parent, transform, opacity, inherited_opacity, name)
            }
            "image" => {
                let Some(data) = paint::image_data(n)? else {
                    return self.fallback(n, parent, inherited_opacity, name, "SVG 或其他图片格式");
                };
                let mut rect = Rect::new(
                    attr(n, "x", 0.),
                    attr(n, "y", 0.),
                    attr(n, "x", 0.) + attr(n, "width", 1.),
                    attr(n, "y", 0.) + attr(n, "height", 1.),
                );
                let aspect = n
                    .attribute("preserveAspectRatio")
                    .unwrap_or("xMidYMid meet");
                let mut crop = [0.; 4];
                if aspect.contains("slice") {
                    let ratio = data.width / data.height;
                    let target = rect.width() / rect.height();
                    if ratio > target {
                        crop[0] = (1. - target / ratio) / 2.;
                        crop[2] = crop[0];
                    } else {
                        crop[1] = (1. - ratio / target) / 2.;
                        crop[3] = crop[1];
                    }
                } else if aspect != "none" {
                    let ratio = (rect.width() / data.width).min(rect.height() / data.height);
                    rect = Rect::from_center_size(
                        rect.center(),
                        (data.width * ratio, data.height * ratio),
                    );
                }
                let Some((b, xfrm, paint)) = geometry::frame(transform, rect) else {
                    return self.fallback(n, parent, inherited_opacity, name, "图片斜切变换");
                };
                let g = geometry::Geometry {
                    rect: b,
                    bounds: transform.transform_rect_bbox(rect),
                    xml: geometry::preset("rect", 0., b),
                    transform: xfrm,
                    paint_transform: paint,
                };
                let fill = self.blip_fill(data, opacity, crop, [0.; 4]);
                Ok(self.emit_shape(name, g, &fill, "<a:ln><a:noFill/></a:ln>"))
            }
            _ => self.fallback(
                n,
                parent,
                inherited_opacity,
                name,
                &format!("不支持的 SVG 元素 {tag}"),
            ),
        }
    }
}
// Count independently composited paints, so single-picture/text opacity remains
// editable while overlapping shapes preserve group-alpha semantics via fallback.
fn paint_count(n: Node<'_, '_>) -> usize {
    match n.tag_name().name() {
        "g" => n.children().filter(Node::is_element).map(paint_count).sum(),
        "defs" | "style" | "title" | "desc" => 0,
        "path" | "rect" | "ellipse" | "circle" => {
            usize::from(n.attribute("fill").unwrap_or("#000000") != "none")
                + usize::from(
                    n.attribute("stroke").unwrap_or("none") != "none"
                        && attr(n, "stroke-width", 1.) > 0.,
                )
        }
        _ => 1,
    }
}

/// DrawingML text/picture rotation can express positive uniform scale + rotation.
fn similarity(t: Affine) -> Option<(f64, f64)> {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let scale = a.hypot(b);
    if scale <= 1e-12 || (a - d).abs() > scale * 1e-8 || (b + c).abs() > scale * 1e-8 {
        None
    } else {
        Some((scale, b.atan2(a)))
    }
}
fn placed_rect(t: Affine, r: Rect, scale: f64) -> Rect {
    let center = t * r.center();
    Rect::from_center_size(center, (r.width() * scale, r.height() * scale))
}

// Symmetric, undecorated caps have an exact native line property. Keep the
// shared endpoint geometry for arrows, asymmetric caps and head-only paths.
// Reuse unmodified JPEG bytes only when core's color/orientation policy marked
// them safe for passthrough. Managed/rotated JPEGs keep the normalized PNG.
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
fn normalize_caps(nodes: &mut [Json]) {
    for n in nodes {
        if n["kind"] == "image" && n["sourceImage"].is_object() {
            let source = n["sourceImage"].clone();
            for key in ["data", "mime", "rasterKey", "sourceJpegHash", "crop"] {
                n[key] = source[key].clone();
            }
            n["intrinsicWidth"] = source["width"].clone();
            n["intrinsicHeight"] = source["height"].clone();
        }
        preserve_jpeg(n);
        if n["fill"].is_object() {
            preserve_jpeg(&mut n["fill"]);
        }
        // SVG normally physically crops PNGs for PDF edge sampling. PPTX
        // retains the original resource and maps this crop to DrawingML.
        if n["kind"] == "image" && n["crop"].is_object() {
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
fn object_recipes(nodes: &[Json]) -> BTreeMap<String, Json> {
    fn visit(nodes: &[Json], out: &mut BTreeMap<String, Json>) {
        for n in nodes {
            if let Some(id) = n["id"].as_str() {
                if n["geometryRecipe"].is_object() {
                    out.insert(id.into(), n["geometryRecipe"].clone());
                }
            }
            if let Some(children) = n["children"].as_array() {
                visit(children, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(nodes, &mut out);
    out
}

fn object_kinds(nodes: &[Json]) -> BTreeMap<String, String> {
    fn visit(nodes: &[Json], out: &mut BTreeMap<String, String>) {
        for n in nodes {
            if let (Some(id), Some(kind)) = (n["id"].as_str(), n["kind"].as_str()) {
                out.insert(id.into(), kind.into());
            }
            if let Some(children) = n["children"].as_array() {
                visit(children, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(nodes, &mut out);
    out
}

/// Return bytes and export-specific diagnostics. CLI honors --warnings; other
/// native callers may use this function to expose the same fallback diagnostics.
pub fn render_pptx(scene: &Scene, options: &ExportOptions) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
    options.validate("pptx")?;
    // ST_SlideSizeCoordinate permits 1–56 inches on each axis. Reject before
    // serializing rather than silently resizing physical geometry.
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
    normalize_caps(&mut lowered.nodes);
    let source = svg(&lowered, false)?;
    let doc = roxmltree::Document::parse(&source).map_err(|e| error(e.to_string()))?;
    let defs = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("defs"))
        .map(|n| {
            n.children()
                .filter(Node::is_element)
                .map(|n| &source[n.range()])
                .collect::<String>()
        })
        .unwrap_or_default();
    let mut writer = Writer {
        scene,
        object_kinds: object_kinds(&scene.nodes),
        recipes: object_recipes(&scene.nodes),
        source: &source,
        defs,
        dpi,
        next_id: 2,
        media: vec![],
        warnings: vec![],
        fonts: BTreeSet::new(),
        bounds: vec![],
    };
    let mut body = String::new();
    for n in doc.root_element().children().filter(Node::is_element) {
        body += &writer.node(n, Affine::IDENTITY, 1., false, "canvas")?;
    }
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
    let bytes = package(scene, &body, &writer.media)?;
    Ok((bytes, writer.warnings))
}
fn package(scene: &Scene, body: &str, media: &[Media]) -> Result<Vec<u8>> {
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
    let content_types=format!("{XML}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>{}</Types>",[
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
    add("ppt/slides/slide1.xml",format!("{XML}<p:sld xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\"><p:cSld><p:spTree>{root_group}{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>").as_bytes())?;
    let mut slide_rels = vec![(
        "rId1".to_string(),
        "slideLayout",
        "../slideLayouts/slideLayout1.xml".to_string(),
    )];
    for (i, m) in media.iter().enumerate() {
        slide_rels.push((
            format!("rId{}", i + 2),
            "image",
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
                "<Relationship Id=\"{id}\" Type=\"{R}/{kind}\" Target=\"{target}\"/>"
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
