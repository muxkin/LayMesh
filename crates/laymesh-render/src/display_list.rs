//! Backend-independent drawing commands. No generated XML enters this layer.
use super::*;
use kurbo::{Affine, BezPath, Rect, Shape};
use laymesh_core::{model::FontAsset, text::PositionedGlyph};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DISPLAY_LIST_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_transition: Option<laymesh_core::page_transition::PageTransition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<laymesh_core::animation::Animation>,
    pub schema_version: u32,
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<DrawNode>,
    pub resources: BTreeMap<String, Resource>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub mime: String,
    pub data: Vec<u8>,
    pub font_index: Option<u32>,
    pub family: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawNode {
    pub name: String,
    pub transform: Affine,
    pub opacity: f64,
    pub effects: Vec<Effect>,
    pub semantic: Option<String>,
    pub hint: Json,
    pub kind: DrawKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DrawKind {
    Group {
        children: Vec<DrawNode>,
        clip: Option<Clip>,
        mask: Option<Mask>,
    },
    Path {
        path: BezPath,
        fill: Paint,
        stroke: Option<Stroke>,
        even_odd: bool,
    },
    Glyphs {
        resource: String,
        text: String,
        size: f64,
        glyphs: Vec<PositionedGlyph>,
        outlines: Vec<BezPath>,
        color: [f64; 4],
        family: String,
        weight: u16,
        italic: bool,
        width: f64,
        ascent: f64,
        descent: f64,
    },
    Image {
        resource: String,
        rect: Rect,
        nearest: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub path: BezPath,
    pub even_odd: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    pub node: Box<DrawNode>,
    pub luminance: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Paint {
    None,
    Solid {
        rgba: [f64; 4],
    },
    Linear {
        start: [f64; 2],
        end: [f64; 2],
        stops: Vec<Stop>,
        transform: Affine,
        spread: String,
    },
    Radial {
        center: [f64; 2],
        focal: [f64; 2],
        radius: f64,
        stops: Vec<Stop>,
        transform: Affine,
        spread: String,
    },
    Pattern {
        width: f64,
        height: f64,
        transform: Affine,
        nodes: Vec<DrawNode>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub offset: f64,
    pub rgba: [f64; 4],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub width: f64,
    pub rgba: [f64; 4],
    pub cap: String,
    pub join: String,
    pub miter: f64,
    pub dash: Vec<f64>,
    pub offset: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub inner: bool,
    pub blur: f64,
    pub spread: f64,
    pub offset: [f64; 2],
    pub rgba: [f64; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sourcePath")]
    pub source_path: Option<BezPath>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub knockout: bool,
}
pub(crate) fn rgba(s: &str) -> Result<[f64; 4]> {
    if s == "none" {
        return Ok([0.; 4]);
    }
    match laymesh_core::color::Color::parse(s) {
        Ok(color) => Ok(color.rgba),
        Err(message) => {
            // RaTeX color commands preserve CSS color names in formula items.
            let color: svgtypes::Color = s.parse().map_err(|_| error(message))?;
            Ok([
                color.red as f64 / 255.,
                color.green as f64 / 255.,
                color.blue as f64 / 255.,
                color.alpha as f64 / 255.,
            ])
        }
    }
}
impl DrawNode {
    pub fn new(kind: DrawKind) -> Self {
        Self {
            name: String::new(),
            transform: Affine::IDENTITY,
            opacity: 1.,
            effects: Vec::new(),
            semantic: None,
            hint: Json::Null,
            kind,
        }
    }
    pub fn group(children: Vec<Self>) -> Self {
        Self::new(DrawKind::Group {
            children,
            clip: None,
            mask: None,
        })
    }
    pub fn local_bounds(&self) -> Rect {
        match &self.kind {
            DrawKind::Group { children, clip, .. } => {
                let mut b = children
                    .iter()
                    .map(DrawNode::bounds)
                    .reduce(|a, b| a.union(b))
                    .unwrap_or(Rect::ZERO);
                if let Some(c) = clip {
                    b = b.intersect(c.path.bounding_box());
                }
                b
            }
            DrawKind::Path { path, stroke, .. } => {
                let b = path.bounding_box();
                if let Some(s) = stroke {
                    b.inflate(s.width * s.miter.max(1.), s.width * s.miter.max(1.))
                } else {
                    b
                }
            }
            DrawKind::Glyphs { outlines, .. } => outlines
                .iter()
                .map(Shape::bounding_box)
                .reduce(|a, b| a.union(b))
                .unwrap_or(Rect::ZERO),
            DrawKind::Image { rect, .. } => *rect,
        }
    }
    pub fn bounds(&self) -> Rect {
        let b = self.local_bounds();
        let mut result = b;
        for e in &self.effects {
            if !e.inner {
                let pad = 4. * e.blur + e.spread.abs();
                result =
                    result.union(b.inflate(pad, pad) + kurbo::Vec2::new(e.offset[0], e.offset[1]));
            }
        }
        self.transform.transform_rect_bbox(result)
    }
}
pub(crate) fn stroke(s: &Json) -> Result<Option<Stroke>> {
    let color = jstr(s, "color", "none");
    let width = jnum(s, "width", 0.);
    if color == "none" || width <= 0. {
        return Ok(None);
    }
    let mut color = rgba(color)?;
    color[3] *= jnum(s, "opacity", 1.);
    Ok(Some(Stroke {
        width,
        rgba: color,
        cap: jstr(s, "cap", "butt").into(),
        join: jstr(s, "join", "miter").into(),
        miter: jnum(s, "miterLimit", 4.),
        offset: jnum(s, "dashOffset", 0.),
        dash: s["dash"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Json::as_f64)
            .collect(),
    }))
}
pub(crate) fn path_node(
    path: BezPath,
    fill: Paint,
    stroke: Option<Stroke>,
    even_odd: bool,
) -> DrawNode {
    DrawNode::new(DrawKind::Path {
        path,
        fill,
        stroke,
        even_odd,
    })
}
pub(crate) fn solid(s: &str) -> Result<Paint> {
    if s == "none" {
        Ok(Paint::None)
    } else {
        Ok(Paint::Solid { rgba: rgba(s)? })
    }
}
struct Builder<'a> {
    scene: &'a Scene,
    preview: bool,
    resources: BTreeMap<String, Resource>,
    font_resources: BTreeMap<String, String>,
}
impl Builder<'_> {
    fn resource(&mut self, mime: &str, bytes: Vec<u8>, font: Option<&FontAsset>) -> String {
        let id = format!(
            "{}-{}",
            laymesh_core::asset_cache::digest(&bytes),
            font.map(|f| f.index).unwrap_or(0)
        );
        self.resources
            .entry(id.clone())
            .or_insert_with(|| Resource {
                mime: mime.into(),
                data: bytes,
                font_index: font.map(|f| f.index),
                family: font.map(|f| f.family.clone()),
            });
        id
    }
    fn paint(&mut self, p: &Json, bbox: Rect) -> Result<Paint> {
        let computed = laymesh_core::css_values::gradient_box(
            p,
            [bbox.x0, bbox.y0, bbox.width(), bbox.height()],
        );
        let p = computed.as_ref();
        if p.is_null() {
            return Ok(Paint::None);
        }
        if let Some(s) = p.as_str() {
            return solid(s);
        }
        match jstr(p, "kind", "") {
            "hatch" | "pattern" => {
                let spacing = jnum(p, "spacing", 2.).max(0.001);
                let sw = jnum(p, "lineWidth", jnum(p, "line_width", 0.2));
                let c = jstr(p, "color", "#000000");
                let mut nodes = vec![path_node(
                    Rect::new(0., 0., spacing, spacing).to_path(0.001),
                    solid(jstr(p, "background", "none"))?,
                    None,
                    false,
                )];
                if jstr(p, "pattern", jstr(p, "style", "lines")) == "dots" {
                    nodes.push(path_node(
                        kurbo::Circle::new((spacing / 2., spacing / 2.), sw / 2.).to_path(0.001),
                        solid(c)?,
                        None,
                        false,
                    ));
                } else {
                    let mut path = BezPath::new();
                    for a in [-spacing, 0., spacing] {
                        path.move_to((a, spacing));
                        path.line_to((a + spacing, 0.));
                    }
                    if jstr(p, "pattern", jstr(p, "style", "lines")) == "cross" {
                        for a in [-spacing, 0., spacing] {
                            path.move_to((a, 0.));
                            path.line_to((a + spacing, spacing));
                        }
                    }
                    nodes.push(path_node(
                        path,
                        Paint::None,
                        stroke(&json!({"color":c,"width":sw}))?,
                        false,
                    ));
                }
                Ok(Paint::Pattern {
                    width: spacing,
                    height: spacing,
                    transform: Affine::rotate(jnum(p, "angle", 0.).to_radians()),
                    nodes,
                })
            }
            "imagePaint" | "image_paint" | "image_fill" => {
                let mut image = p.clone();
                image["kind"] = json!("image");
                image["width"] = json!(bbox.width());
                image["height"] = json!(bbox.height());
                image["intrinsicWidth"] = p["width"].clone();
                image["intrinsicHeight"] = p["height"].clone();
                let node = self.image(&image)?;
                Ok(Paint::Pattern {
                    width: bbox.width().max(1e-12),
                    height: bbox.height().max(1e-12),
                    transform: Affine::translate((bbox.x0, bbox.y0)),
                    nodes: vec![node],
                })
            }
            kind => {
                let stops = p["stops"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| {
                        let offset = s["at"].as_f64().or_else(|| s[0].as_f64()).unwrap_or(0.);
                        let mut rgba = rgba(
                            s["color"]
                                .as_str()
                                .or_else(|| s[1].as_str())
                                .unwrap_or("#000000"),
                        )?;
                        rgba[3] *= s["opacity"]
                            .as_f64()
                            .or_else(|| s[2].as_f64())
                            .unwrap_or(1.);
                        Ok(Stop { offset, rgba })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let pb = p["paintBox"].as_array().map(|b| {
                    Rect::new(
                        b[0].as_f64().unwrap_or(0.),
                        b[1].as_f64().unwrap_or(0.),
                        b[0].as_f64().unwrap_or(0.) + b[2].as_f64().unwrap_or(1.),
                        b[1].as_f64().unwrap_or(0.) + b[3].as_f64().unwrap_or(1.),
                    )
                });
                let b = pb.unwrap_or(bbox);
                if matches!(kind, "linearGradient" | "linear_gradient") {
                    let a = pair(&p["start"], [0., 0.]);
                    let z = pair(&p["end"], [1., 1.]);
                    Ok(Paint::Linear {
                        start: [b.x0 + a[0] * b.width(), b.y0 + a[1] * b.height()],
                        end: [b.x0 + z[0] * b.width(), b.y0 + z[1] * b.height()],
                        stops,
                        transform: Affine::IDENTITY,
                        spread: "pad".into(),
                    })
                } else {
                    let center = pair(&p["center"], [0.5, 0.5]);
                    if pb.is_some() {
                        let center = [b.x0 + center[0] * b.width(), b.y0 + center[1] * b.height()];
                        Ok(Paint::Radial {
                            center,
                            focal: center,
                            radius: jnum(p, "radius", 0.5) * b.width().max(b.height()),
                            stops,
                            transform: Affine::IDENTITY,
                            spread: "pad".into(),
                        })
                    } else {
                        let b = p["cssEllipseBox"]
                            .as_array()
                            .map(|v| {
                                Rect::new(
                                    v[0].as_f64().unwrap(),
                                    v[1].as_f64().unwrap(),
                                    v[0].as_f64().unwrap() + v[2].as_f64().unwrap(),
                                    v[1].as_f64().unwrap() + v[3].as_f64().unwrap(),
                                )
                            })
                            .unwrap_or(b);
                        Ok(Paint::Radial {
                            center,
                            focal: center,
                            radius: jnum(p, "radius", 0.5),
                            stops,
                            transform: Affine::translate((b.x0, b.y0))
                                * Affine::scale_non_uniform(b.width(), b.height()),
                            spread: "pad".into(),
                        })
                    }
                }
            }
        }
    }
    fn glyph(&mut self, n: &Json) -> Result<DrawNode> {
        let key = jstr(n, "fontFamily", "");
        let font = self
            .scene
            .fonts
            .get(key)
            .ok_or_else(|| error(format!("缺少绘制字体：{key}")))?;
        let face =
            ttf_parser::Face::parse(&font.data, font.index).map_err(|_| error("无效绘制字体"))?;
        let text = jstr(n, "content", "").to_owned();
        let size = jnum(n, "fontSize", 3.);
        let glyphs = laymesh_core::text::shape_glyphs(font, &text, size);
        let scale = size / face.units_per_em() as f64;
        let mut outlines = Vec::new();
        for g in &glyphs {
            let mut outline = OutlinePath::default();
            face.outline_glyph(ttf_parser::GlyphId(g.id), &mut outline);
            outlines.push(Affine::new([scale, 0., 0., -scale, g.x, g.y]) * outline.0);
        }
        let resource = if let Some(id) = self.font_resources.get(key) {
            id.clone()
        } else {
            let id = self.resource("font/ttf", font.data.clone(), Some(font));
            self.font_resources.insert(key.into(), id.clone());
            id
        };
        let mut result = DrawNode::new(DrawKind::Glyphs {
            resource,
            text,
            size,
            glyphs,
            outlines,
            color: rgba(jstr(n, "color", "#000000"))?,
            family: font.family.clone(),
            weight: jnum(n, "fontWeight", 400.) as u16,
            italic: n["fontItalic"].as_bool().unwrap_or(false),
            width: jnum(n, "width", 0.),
            ascent: face.ascender() as f64 * scale,
            descent: -(face.descender() as f64) * scale,
        });
        result.transform = Affine::translate((jnum(n, "x", 0.), jnum(n, "baseline", 0.)));
        Ok(result)
    }
    fn image(&mut self, n: &Json) -> Result<DrawNode> {
        // Isolate the selected source pixels before interpolation, so pixels
        // outside a crop cannot bleed into its visible boundary. Keep the
        // original source for editable DrawingML crop coordinates.
        if n["mime"] == "image/png" && n["crop"].is_object() && n["_pptxKeepSourceCrop"] != true {
            let c = &n["crop"];
            let asset = laymesh_core::assets::crop_asset(
                n,
                [
                    jnum(c, "x", 0.),
                    jnum(c, "y", 0.),
                    jnum(c, "width", 0.),
                    jnum(c, "height", 0.),
                ],
                "",
                Loc::default(),
            )?;
            let mut cropped = n.clone();
            cropped["crop"] = Json::Null;
            cropped["data"] = asset["data"].clone();
            cropped["rasterKey"] = asset["rasterKey"].clone();
            cropped["intrinsicWidth"] = asset["width"].clone();
            cropped["intrinsicHeight"] = asset["height"].clone();
            return self.image(&cropped);
        }
        let mime = jstr(n, "mime", "image/png");
        // Native previews already own normalized source pixels. Keep their
        // content key until preview_assets encodes the configured derivative;
        // a PNG encode/decode round trip here only duplicates that work.
        let cached_pixels = if self.preview && mime != "image/svg+xml" {
            n["rasterKey"]
                .as_str()
                .filter(|key| laymesh_core::asset_cache::pixels(key).is_some())
        } else {
            None
        };
        let data = if cached_pixels.is_some() {
            Vec::new()
        } else if jstr(n, "data", "").is_empty() {
            let key = n["rasterKey"]
                .as_str()
                .ok_or_else(|| error("缺少图片资源"))?;
            laymesh_core::asset_cache::png(key)?.as_ref().clone()
        } else {
            base64::engine::general_purpose::STANDARD
                .decode(jstr(n, "data", ""))
                .map_err(|e| error(e.to_string()))?
        };
        let w = jnum(n, "width", 0.);
        let h = jnum(n, "height", 0.);
        let iw = jnum(n, "intrinsicWidth", w).max(1e-12);
        let ih = jnum(n, "intrinsicHeight", h).max(1e-12);
        let c = &n["crop"];
        let cx = jnum(c, "x", 0.);
        let cy = jnum(c, "y", 0.);
        let cw = jnum(c, "width", iw).max(1e-12);
        let ch = jnum(c, "height", ih).max(1e-12);
        let fit = jstr(n, "fit", "contain");
        let scale = if fit == "cover" {
            (w / cw).max(h / ch)
        } else {
            (w / cw).min(h / ch)
        };
        let (sx, sy) = if fit == "stretch" {
            (w / cw, h / ch)
        } else {
            (scale, scale)
        };
        let tr = Affine::translate(((w - cw * sx) / 2. - cx * sx, (h - ch * sy) / 2. - cy * sy))
            * Affine::scale_non_uniform(sx, sy);
        let mut result = if mime == "image/svg+xml" {
            super::svg_import::import(&data, self.scene, &mut self.resources)?
        } else {
            let resource = if let Some(key) = cached_pixels {
                let id = format!("pixels-{key}");
                self.resources
                    .entry(id.clone())
                    .or_insert_with(|| Resource {
                        mime: mime.into(),
                        data: Vec::new(),
                        font_index: None,
                        family: None,
                    });
                id
            } else {
                self.resource(mime, data, None)
            };
            DrawNode::new(DrawKind::Image {
                resource,
                rect: Rect::new(0., 0., iw, ih),
                nearest: n["interpolation"] == "nearest",
            })
        };
        result.transform = tr * result.transform;
        if fit == "cover" || c.is_object() {
            result = DrawNode::new(DrawKind::Group {
                children: vec![result],
                clip: Some(Clip {
                    path: Rect::new(0., 0., w, h).to_path(0.001),
                    even_odd: false,
                }),
                mask: None,
            });
        }
        Ok(result)
    }
    fn item(&mut self, n: &Json, parent: Affine) -> Result<DrawNode> {
        match jstr(n, "kind", "") {
            "glyph" => self.glyph(n),
            "box" => {
                let r = Rect::new(
                    jnum(n, "x", 0.),
                    jnum(n, "y", 0.),
                    jnum(n, "x", 0.) + jnum(n, "width", 2.),
                    jnum(n, "y", 0.) + jnum(n, "height", 3.),
                );
                let mut node = path_node(
                    r.to_path(0.001),
                    Paint::None,
                    stroke(
                        &json!({"color":jstr(n,"color","#000000"),"width":jnum(n,"strokeWidth",0.15)}),
                    )?,
                    false,
                );
                node.hint = n.clone();
                Ok(node)
            }
            "rule" => {
                let (x, y, w, h) = (
                    jnum(n, "x", 0.),
                    jnum(n, "y", 0.),
                    jnum(n, "width", 0.),
                    jnum(n, "height", 0.1),
                );
                let mut result = if n["dashed"] == true {
                    let mut p = BezPath::new();
                    p.move_to((x, y));
                    p.line_to((x + w, y));
                    path_node(
                        p,
                        Paint::None,
                        stroke(
                            &json!({"color":jstr(n,"color","#000000"),"width":h,"dash":[h*3.,h*2.]}),
                        )?,
                        false,
                    )
                } else {
                    path_node(
                        Rect::new(x, y, x + w, y + h).to_path(0.001),
                        solid(jstr(n, "color", "#000000"))?,
                        None,
                        false,
                    )
                };
                result.opacity = jnum(n, "opacity", 1.);
                Ok(result)
            }
            "path" if n["strokeStyle"].is_null() => {
                let p = BezPath::from_svg(jstr(n, "d", "")).map_err(|_| error("无效绘制路径"))?;
                let mut result = path_node(
                    p.clone(),
                    self.paint(&n["fill"], p.bounding_box())?,
                    stroke(
                        &json!({"color":jstr(n,"stroke","none"),"width":jnum(n,"strokeWidth",0.)}),
                    )?,
                    false,
                );
                result.opacity = jnum(n, "opacity", 1.);
                Ok(result)
            }
            "formula" => {
                let mut children = Vec::new();
                for item in n["items"].as_array().into_iter().flatten() {
                    children.push(self.item(item, parent)?);
                }
                let mut result = DrawNode::group(children);
                result.semantic = Some(jstr(n, "source", "").into());
                result.transform = Affine::translate((jnum(n, "x", 0.), jnum(n, "y", 0.)));
                Ok(result)
            }
            _ => self.node(n, parent),
        }
    }
    fn node(&mut self, n: &Json, parent: Affine) -> Result<DrawNode> {
        let transform = parent * laymesh_core::geometry::node_transform(n);
        let w = jnum(n, "width", 0.);
        let h = jnum(n, "height", 0.);
        let mut result = match jstr(n, "kind", "") {
            "group" => {
                let sx = if jnum(n, "contentWidth", 0.) > 0. {
                    w / jnum(n, "contentWidth", w)
                } else {
                    1.
                };
                let sy = if jnum(n, "contentHeight", 0.) > 0. {
                    h / jnum(n, "contentHeight", h)
                } else {
                    1.
                };
                let scale = Affine::scale_non_uniform(sx, sy);
                let clip = if n["clipPath"].is_object() {
                    let mut p = laymesh_core::geometry::resolved_path(&n["clipPath"], transform);
                    if transform.determinant().abs() > 1e-30 {
                        p = transform.inverse() * p;
                    }
                    Some(Clip {
                        path: p,
                        even_odd: n["clipPath"]["fillRule"] == "evenodd",
                    })
                } else if n["clip"].is_object() {
                    let c = &n["clip"];
                    let x = jnum(c, "x", 0.);
                    let y = jnum(c, "y", 0.);
                    Some(Clip {
                        path: Rect::new(x, y, x + jnum(c, "width", w), y + jnum(c, "height", h))
                            .to_path(0.001),
                        even_odd: false,
                    })
                } else {
                    None
                };
                let mut children = Vec::new();
                for child in n["children"].as_array().into_iter().flatten() {
                    children.push(self.node(child, transform * scale)?);
                }
                let mut content = DrawNode::new(DrawKind::Group {
                    children,
                    clip,
                    mask: None,
                });
                content.transform = scale;
                DrawNode::group(vec![content])
            }
            "text" => {
                let mut children = Vec::new();
                for r in n["runs"].as_array().into_iter().flatten() {
                    children.push(self.item(r, transform)?);
                }
                let mut text = DrawNode::group(children);
                text.semantic = n["artText"].as_str().map(str::to_owned);
                text
            }
            "formula" => {
                let mut local = n.clone();
                local["x"] = json!(0.);
                local["y"] = json!(0.);
                self.item(&local, transform)?
            }
            "path" => {
                if n["endpointRecipe"].is_object() {
                    let mut children = Vec::new();
                    for layer in laymesh_core::endpoints::layers(n, transform).map_err(error)? {
                        let mut child = path_node(
                            transform.inverse() * layer.path,
                            solid(&layer.color)?,
                            None,
                            false,
                        );
                        child.opacity = layer.opacity;
                        children.push(child);
                    }
                    DrawNode::group(children)
                } else {
                    let sx = if jnum(n, "intrinsicWidth", 0.) > 0. {
                        w / jnum(n, "intrinsicWidth", w)
                    } else {
                        1.
                    };
                    let sy = if jnum(n, "intrinsicHeight", 0.) > 0. {
                        h / jnum(n, "intrinsicHeight", h)
                    } else {
                        1.
                    };
                    let scale = Affine::scale_non_uniform(sx, sy);
                    let final_tr = transform * scale;
                    let mut path = laymesh_core::geometry::resolved_path(n, final_tr);
                    if final_tr.determinant().abs() > 1e-30 {
                        path = final_tr.inverse() * path;
                    }
                    let st = &n["strokeStyle"];
                    let compound = matches!(jstr(st, "compound", "single"), "double" | "triple");
                    let fill = self.paint(&n["fill"], path.bounding_box())?;
                    let mut children = vec![path_node(
                        path.clone(),
                        fill,
                        if compound || n["outlineD"].is_string() {
                            None
                        } else {
                            stroke(st)?
                        },
                        n["fillRule"] == "evenodd",
                    )];
                    if compound || n["outlineD"].is_string() {
                        let outline = if let Some(d) = n["outlineD"].as_str() {
                            BezPath::from_svg(d).map_err(|_| error("无效描边轮廓"))?
                        } else {
                            final_tr.inverse()
                                * laymesh_core::geometry::compound_outline_transformed(
                                    &path, st, final_tr,
                                )
                        };
                        let mut child =
                            path_node(outline, solid(jstr(st, "color", "#000000"))?, None, true);
                        child.opacity = jnum(st, "opacity", 1.);
                        children.push(child);
                    }
                    let mut result = DrawNode::group(children);
                    result.transform = scale;
                    result
                }
            }
            "rect" | "ellipse" => {
                let path = if n["kind"] == "ellipse" {
                    kurbo::Ellipse::new((w / 2., h / 2.), (w / 2., h / 2.), 0.).to_path(0.001)
                } else {
                    Rect::new(0., 0., w, h)
                        .to_rounded_rect(jnum(n, "radius", 0.))
                        .to_path(0.001)
                };
                path_node(
                    path,
                    self.paint(&n["fill"], Rect::new(0., 0., w, h))?,
                    stroke(
                        &json!({"color":jstr(n,"stroke","none"),"width":jnum(n,"strokeWidth",0.)}),
                    )?,
                    false,
                )
            }
            "image" => self.image(n)?,
            "points" => {
                let mut children = Vec::new();
                for (i, xy) in n["positions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .chunks_exact(2)
                    .enumerate()
                {
                    let x = xy[0].as_f64().unwrap_or(0.);
                    let y = xy[1].as_f64().unwrap_or(0.);
                    let size = n["markerSizes"][i]
                        .as_f64()
                        .unwrap_or(jnum(n, "markerSize", 2.));
                    let stroke_color = jstr(n, "stroke", jstr(n, "markerStroke", "none"));
                    let sw = if stroke_color == "none" {
                        0.
                    } else {
                        jnum(n, "strokeWidth", jnum(n, "markerStrokeWidth", 0.))
                    };
                    let r = (size - sw).max(0.) / 2.;
                    let path = marker(jstr(n, "marker", "circle"), x, y, r);
                    let fill = n["markerFills"][i].as_str().unwrap_or(jstr(
                        n,
                        "markerFill",
                        jstr(n, "fill", jstr(n, "color", "#000000")),
                    ));
                    let mut child = path_node(
                        path,
                        solid(fill)?,
                        stroke(&json!({"color":stroke_color,"width":sw,"join":"round"}))?,
                        false,
                    );
                    child.opacity =
                        n["pointOpacities"][i]
                            .as_f64()
                            .unwrap_or(jnum(n, "pointOpacity", 1.));
                    children.push(child);
                }
                DrawNode::group(children)
            }
            kind => return Err(error(format!("未知绘制对象：{kind}"))),
        };
        result.transform = laymesh_core::geometry::node_transform(n) * result.transform;
        result.opacity *= jnum(n, "opacity", 1.);
        result.name = jstr(n, "id", "").into();
        result.hint = if n["geometryRecipe"].is_object() {
            n["geometryRecipe"].clone()
        } else {
            json!({"kind":n["kind"],"radius":n["radius"]})
        };
        result.hint["logicalKind"] = n["kind"].clone();
        if self.scene.animation.is_some() {
            result.hint["animationLayout"] =
                json!(laymesh_core::geometry::node_transform(n).as_coeffs());
            result.hint["animationSize"] = json!([jnum(n, "width", 0.), jnum(n, "height", 0.)]);
            result.hint["plotBounds"] = n["plotBounds"].clone();
        }
        if let DrawKind::Group { children, .. } = &mut result.kind {
            for child in children {
                if child.hint.is_null() && matches!(child.kind, DrawKind::Path { .. }) {
                    child.hint = result.hint.clone();
                    child.hint["logicalKind"] = Json::Null;
                }
            }
        }
        result.effects = n["effects"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| {
                let mut color = rgba(jstr(e, "color", "#000000"))?;
                color[3] *= jnum(e, "opacity", 0.5);
                Ok(Effect {
                    inner: e["mode"] == "inner",
                    blur: jnum(e, "blur", 1.),
                    spread: jnum(e, "spread", 0.),
                    offset: pair(&e["offset"], [0., 0.]),
                    rgba: color,
                    source_path: e["sourcePath"]
                        .as_str()
                        .map(BezPath::from_svg)
                        .transpose()
                        .map_err(|e| error(format!("Invalid CSS shadow box: {e}")))?,
                    knockout: e["knockout"].as_bool().unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(result)
    }
}
fn marker(kind: &str, x: f64, y: f64, r: f64) -> BezPath {
    if kind == "circle" {
        return kurbo::Circle::new((x, y), r).to_path(0.001);
    }
    let pts = match kind {
        "square" => vec![[-r, -r], [r, -r], [r, r], [-r, r]],
        "diamond" => vec![[0., -r], [r, 0.], [0., r], [-r, 0.]],
        "triangle_down" => vec![[-r, -r], [r, -r], [0., r]],
        "triangle_left" => vec![[r, -r], [r, r], [-r, 0.]],
        "triangle_right" => vec![[-r, -r], [r, 0.], [-r, r]],
        "cross" | "plus" => {
            let t = r / 3.;
            vec![
                [-t, -r],
                [t, -r],
                [t, -t],
                [r, -t],
                [r, t],
                [t, t],
                [t, r],
                [-t, r],
                [-t, t],
                [-r, t],
                [-r, -t],
                [-t, -t],
            ]
        }
        "star" => (0..10)
            .map(|i| {
                let a = (i as f64 * 36. - 90.).to_radians();
                let rr = if i % 2 == 0 { r } else { r * 0.4 };
                [rr * a.cos(), rr * a.sin()]
            })
            .collect(),
        _ => vec![[0., -r], [r, r], [-r, r]],
    };
    let mut p = BezPath::new();
    for (i, v) in pts.iter().enumerate() {
        if i == 0 {
            p.move_to((x + v[0], y + v[1]));
        } else {
            p.line_to((x + v[0], y + v[1]));
        }
    }
    p.close_path();
    p
}
#[derive(Default)]
pub(crate) struct OutlinePath(pub BezPath);
impl ttf_parser::OutlineBuilder for OutlinePath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x as f64, y as f64));
    }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.0.quad_to((a as f64, b as f64), (x as f64, y as f64));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.0.curve_to(
            (a as f64, b as f64),
            (c as f64, d as f64),
            (x as f64, y as f64),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}
pub fn to_display_list(scene: &Scene) -> Result<DisplayList> {
    build(scene, false)
}
#[cfg(feature = "native")]
pub(crate) fn preview_display_list(scene: &Scene) -> Result<DisplayList> {
    build(scene, true)
}
fn build(scene: &Scene, preview: bool) -> Result<DisplayList> {
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.
        || scene.height <= 0.
    {
        return Err(error("画布尺寸必须为正"));
    }
    let mut b = Builder {
        scene,
        preview,
        resources: BTreeMap::new(),
        font_resources: BTreeMap::new(),
    };
    let mut nodes = Vec::new();
    if !scene.background.is_null() && scene.background != "none" {
        let rect = Rect::new(0., 0., scene.width, scene.height);
        let paint = b.paint(&scene.background, rect)?;
        nodes.push(path_node(rect.to_path(0.001), paint, None, false));
    }
    for n in &scene.nodes {
        nodes.push(b.node(n, Affine::IDENTITY)?);
    }
    let mut page_transition = scene.page_transition.clone();
    if let Some(transition) = &mut page_transition {
        for (name, asset) in std::mem::take(&mut transition.assets) {
            let data = if jstr(&asset, "data", "").is_empty() {
                let key = asset["rasterKey"]
                    .as_str()
                    .ok_or_else(|| error("Missing transition texture"))?;
                laymesh_core::asset_cache::png(key)?.as_ref().clone()
            } else {
                base64::engine::general_purpose::STANDARD
                    .decode(jstr(&asset, "data", ""))
                    .map_err(|e| error(e.to_string()))?
            };
            let id = b.resource(jstr(&asset, "mime", "image/png"), data, None);
            transition.textures.insert(name, id);
        }
    }
    Ok(DisplayList {
        page_transition,
        animation: scene.animation.clone(),
        schema_version: DISPLAY_LIST_VERSION,
        width: scene.width,
        height: scene.height,
        nodes,
        resources: b.resources,
    })
}

#[cfg(test)]
mod color_tests {
    use super::*;
    #[test]
    fn formula_css_colors_preserve_svg_semantics() {
        assert_eq!(rgba("red").unwrap(), rgba("#ff0000").unwrap());
        assert_eq!(rgba("aliceblue").unwrap(), rgba("#f0f8ff").unwrap());
        assert_eq!(rgba("transparent").unwrap(), [0.; 4]);
        assert!(rgba("not-a-color").is_err());
    }
}
