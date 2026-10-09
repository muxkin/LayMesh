//! Direct, vector-preserving PDF adapter for the shared display list.
use super::*;
use display_list::{DisplayList, DrawKind, DrawNode, Paint};
use krilla::{
    color::rgb,
    geom::{Path, PathBuilder, Point, Size, Transform},
    image::Image,
    mask::{Mask, MaskType},
    num::NormalizedF32,
    paint::{
        Fill, FillRule, LineCap, LineJoin, LinearGradient, Pattern, RadialGradient, SpreadMethod,
        Stop, Stroke, StrokeDash,
    },
    surface::Surface,
    tagging::{ContentTag, SpanTag},
    text::{Font, GlyphId, KrillaGlyph},
};
use kurbo::{Affine, BezPath, PathEl};
use std::collections::BTreeMap;
fn norm(v: f64) -> NormalizedF32 {
    NormalizedF32::new(v.clamp(0., 1.) as f32).unwrap()
}
fn transform(t: Affine) -> Transform {
    let [a, b, c, d, e, f] = t.as_coeffs();
    Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}
fn color(c: [f64; 4]) -> rgb::Color {
    rgb::Color::new(
        (c[0] * 255.).round() as u8,
        (c[1] * 255.).round() as u8,
        (c[2] * 255.).round() as u8,
    )
}
fn path(p: &BezPath) -> Option<Path> {
    let mut out = PathBuilder::new();
    for e in p.iter() {
        match e {
            PathEl::MoveTo(p) => out.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => out.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(a, b) => out.quad_to(a.x as f32, a.y as f32, b.x as f32, b.y as f32),
            PathEl::CurveTo(a, b, c) => out.cubic_to(
                a.x as f32, a.y as f32, b.x as f32, b.y as f32, c.x as f32, c.y as f32,
            ),
            PathEl::ClosePath => out.close(),
        }
    }
    out.finish()
}
fn spread(s: &str) -> SpreadMethod {
    match s {
        "repeat" => SpreadMethod::Repeat,
        "reflect" => SpreadMethod::Reflect,
        _ => SpreadMethod::Pad,
    }
}
struct Writer<'a> {
    list: &'a DisplayList,
    dpi: f64,
    fonts: BTreeMap<String, Font>,
    images: BTreeMap<String, Image>,
    effect_pixels: u64,
    semantic_depth: usize,
    semantic_font: Option<Font>,
}
impl Writer<'_> {
    fn fill(&mut self, surface: &mut Surface, p: &Paint, even_odd: bool) -> Result<Option<Fill>> {
        let stops = |stops: &[display_list::Stop]| {
            stops
                .iter()
                .map(|s| Stop {
                    offset: norm(s.offset),
                    color: color(s.rgba).into(),
                    opacity: norm(s.rgba[3]),
                })
                .collect()
        };
        let (paint, opacity) = match p {
            Paint::None => return Ok(None),
            Paint::Solid { rgba } => (color(*rgba).into(), norm(rgba[3])),
            Paint::Linear {
                start,
                end,
                stops: s,
                transform: t,
                spread: spr,
            } => (
                LinearGradient {
                    x1: start[0] as f32,
                    y1: start[1] as f32,
                    x2: end[0] as f32,
                    y2: end[1] as f32,
                    transform: transform(*t),
                    spread_method: spread(spr),
                    stops: stops(s),
                    anti_alias: true,
                }
                .into(),
                NormalizedF32::ONE,
            ),
            Paint::Radial {
                center,
                focal,
                radius,
                stops: s,
                transform: t,
                spread: spr,
            } => (
                RadialGradient {
                    fx: focal[0] as f32,
                    fy: focal[1] as f32,
                    fr: 0.,
                    cx: center[0] as f32,
                    cy: center[1] as f32,
                    cr: *radius as f32,
                    transform: transform(*t),
                    spread_method: spread(spr),
                    stops: stops(s),
                    anti_alias: true,
                }
                .into(),
                NormalizedF32::ONE,
            ),
            Paint::Pattern {
                width,
                height,
                transform: t,
                nodes,
            } => {
                let mut builder = surface.stream_builder();
                let mut sub = builder.surface();
                for node in nodes {
                    self.node(&mut sub, node, Affine::IDENTITY)?;
                }
                sub.finish();
                let stream = builder.finish();
                (
                    Pattern {
                        stream,
                        transform: transform(*t),
                        width: *width as f32,
                        height: *height as f32,
                    }
                    .into(),
                    NormalizedF32::ONE,
                )
            }
        };
        Ok(Some(Fill {
            paint,
            opacity,
            rule: if even_odd {
                FillRule::EvenOdd
            } else {
                FillRule::NonZero
            },
        }))
    }
    fn image(&mut self, id: &str, nearest: bool) -> Result<Image> {
        if let Some(image) = self.images.get(id) {
            return Ok(image.clone());
        }
        let r = self
            .list
            .resources
            .get(id)
            .ok_or_else(|| error("PDF 缺少图片资源"))?;
        let image = match r.mime.as_str() {
            "image/jpeg" => Image::from_jpeg(r.data.clone().into(), !nearest),
            "image/png" => Image::from_png(r.data.clone().into(), !nearest),
            "image/webp" => Image::from_webp(r.data.clone().into(), !nearest),
            _ => return Err(error(format!("PDF 不支持的图片资源：{}", r.mime))),
        }
        .map_err(error)?;
        self.images.insert(id.into(), image.clone());
        Ok(image)
    }
    fn draw_image(surface: &mut Surface, image: Image, r: kurbo::Rect) {
        if let Some(size) = Size::from_wh(r.width() as f32, r.height() as f32) {
            surface.push_transform(&Transform::from_translate(r.x0 as f32, r.y0 as f32));
            surface.draw_image(image, size);
            surface.pop();
        }
    }
    fn semantic(&mut self, surface: &mut Surface, source: &str, b: kurbo::Rect) -> Result<()> {
        if source.is_empty() {
            return Ok(());
        }
        let data = ratex_katex_fonts::ttf_bytes("KaTeX_Main-Regular.ttf")
            .ok_or_else(|| error("缺少公式字体"))?
            .into_owned();
        let face = ttf_parser::Face::parse(&data, 0).map_err(|_| error("无效公式字体"))?;
        let font = if let Some(font) = &self.semantic_font {
            font.clone()
        } else {
            let font =
                Font::new(data.clone().into(), 0).ok_or_else(|| error("PDF 公式字体读取失败"))?;
            self.semantic_font = Some(font.clone());
            font
        };
        let size = b.height().max(0.1) as f32;
        let units = face.units_per_em() as f32;
        let fallback = face.glyph_index('x').unwrap();
        let glyphs: Vec<_> = source
            .char_indices()
            .map(|(offset, ch)| {
                let gid = face.glyph_index(ch).unwrap_or(fallback);
                KrillaGlyph::new(
                    GlyphId::new(gid.0 as u32),
                    face.glyph_hor_advance(gid).unwrap_or(500) as f32 / units,
                    0.,
                    0.,
                    0.,
                    offset..offset + ch.len_utf8(),
                    None,
                )
            })
            .collect();
        let advance = glyphs.iter().map(|g| g.x_advance).sum::<f32>() * size;
        surface.push_transform(&Transform::from_row(
            if advance > 0. {
                b.width() as f32 / advance
            } else {
                1.
            },
            0.,
            0.,
            1.,
            b.x0 as f32,
            b.y0 as f32,
        ));
        surface.set_fill(Some(Fill {
            opacity: NormalizedF32::ZERO,
            ..Default::default()
        }));
        surface.set_stroke(None);
        surface.start_tagged(ContentTag::Span(
            SpanTag::empty().with_actual_text(Some(source)),
        ));
        surface.draw_glyphs(
            Point::from_xy(0., size * 0.8),
            &glyphs,
            font,
            source,
            size,
            false,
        );
        surface.end_tagged();
        surface.pop();
        Ok(())
    }
    fn node(&mut self, surface: &mut Surface, n: &DrawNode, parent: Affine) -> Result<()> {
        let semantic = n.semantic.as_deref().filter(|_| self.semantic_depth == 0);
        if n.semantic.is_some() {
            self.semantic_depth += 1;
        }
        let global = parent * n.transform;
        surface.push_transform(&transform(n.transform));
        surface.push_opacity(norm(n.opacity));
        let tiles = if n.effects.is_empty() {
            Vec::new()
        } else {
            let [a, b, c, d, _, _] = global.as_coeffs();
            let tiles = raster::effect_tiles(
                self.list,
                n,
                self.dpi / 25.4 * a.hypot(b).max(c.hypot(d)).max(1e-9),
            )?;
            for tile in &tiles {
                self.effect_pixels += tile.pixels.width() as u64 * tile.pixels.height() as u64;
                if self.effect_pixels > raster::MAX_PIXELS {
                    return Err(error(
                        "PDF effects exceed the 100000000-pixel budget; reduce DPI or effect size",
                    ));
                }
            }
            tiles
        };
        for t in tiles.iter().filter(|t| !t.inner) {
            let image = Image::from_rgba8(
                t.pixels
                    .as_ref()
                    .clone()
                    .take_rgba8(vello_cpu::peniko::ImageAlphaType::Alpha),
                t.pixels.width() as u32,
                t.pixels.height() as u32,
            );
            Self::draw_image(surface, image, t.rect);
        }
        match &n.kind {
            DrawKind::Group {
                children,
                clip,
                mask,
            } => {
                let mut layers = 0;
                if let Some(c) = clip {
                    if let Some(p) = path(&c.path) {
                        surface.push_clip_path(
                            &p,
                            &if c.even_odd {
                                FillRule::EvenOdd
                            } else {
                                FillRule::NonZero
                            },
                        );
                        layers += 1;
                    }
                }
                if let Some(m) = mask {
                    let mut builder = surface.stream_builder();
                    let mut sub = builder.surface();
                    self.node(&mut sub, &m.node, Affine::IDENTITY)?;
                    sub.finish();
                    let stream = builder.finish();
                    surface.push_mask(Mask::new(
                        stream,
                        if m.luminance {
                            MaskType::Luminosity
                        } else {
                            MaskType::Alpha
                        },
                    ));
                    layers += 1;
                }
                for child in children {
                    self.node(surface, child, global)?;
                }
                for _ in 0..layers {
                    surface.pop();
                }
            }
            DrawKind::Path {
                path: p,
                fill,
                stroke,
                even_odd,
            } => {
                let fill = self.fill(surface, fill, *even_odd)?;
                let painted = fill.is_some() || stroke.is_some();
                surface.set_fill(fill);
                surface.set_stroke(stroke.as_ref().map(|s| Stroke {
                    paint: color(s.rgba).into(),
                    opacity: norm(s.rgba[3]),
                    width: s.width as f32,
                    miter_limit: s.miter as f32,
                    line_cap: match s.cap.as_str() {
                        "round" => LineCap::Round,
                        "square" => LineCap::Square,
                        _ => LineCap::Butt,
                    },
                    line_join: match s.join.as_str() {
                        "round" => LineJoin::Round,
                        "bevel" => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    },
                    dash: (!s.dash.is_empty()).then(|| StrokeDash {
                        array: s.dash.iter().map(|v| *v as f32).collect(),
                        offset: s.offset as f32,
                    }),
                }));
                if painted {
                    if let Some(p) = path(p) {
                        surface.draw_path(&p);
                    }
                }
            }
            DrawKind::Glyphs {
                resource,
                text,
                size,
                glyphs,
                color: c,
                ..
            } => {
                let font = if let Some(font) = self.fonts.get(resource) {
                    font.clone()
                } else {
                    let r = self
                        .list
                        .resources
                        .get(resource)
                        .ok_or_else(|| error("PDF 缺少字体"))?;
                    let font = Font::new(r.data.clone().into(), r.font_index.unwrap_or(0))
                        .ok_or_else(|| error("PDF 无效字体"))?;
                    self.fonts.insert(resource.clone(), font.clone());
                    font
                };
                let mut cursor = 0.;
                let glyphs: Vec<_> = glyphs
                    .iter()
                    .map(|g| {
                        let out = KrillaGlyph::new(
                            GlyphId::new(g.id as u32),
                            (g.advance / size) as f32,
                            ((g.x - cursor) / size) as f32,
                            (g.y / size) as f32,
                            0.,
                            g.source.clone(),
                            None,
                        );
                        cursor += g.advance;
                        out
                    })
                    .collect();
                surface.set_fill(Some(Fill {
                    paint: color(*c).into(),
                    opacity: norm(c[3]),
                    ..Default::default()
                }));
                surface.set_stroke(None);
                if self.semantic_depth > 0 {
                    surface.start_tagged(ContentTag::Span(
                        SpanTag::empty().with_actual_text(Some("")),
                    ));
                }
                surface.draw_glyphs(
                    Point::from_xy(0., 0.),
                    &glyphs,
                    font,
                    text,
                    *size as f32,
                    false,
                );
                if self.semantic_depth > 0 {
                    surface.end_tagged();
                }
            }
            DrawKind::Image {
                resource,
                rect,
                nearest,
            } => Self::draw_image(surface, self.image(resource, *nearest)?, *rect),
        }
        for t in tiles.iter().filter(|t| t.inner) {
            let image = Image::from_rgba8(
                t.pixels
                    .as_ref()
                    .clone()
                    .take_rgba8(vello_cpu::peniko::ImageAlphaType::Alpha),
                t.pixels.width() as u32,
                t.pixels.height() as u32,
            );
            Self::draw_image(surface, image, t.rect);
        }
        if let Some(source) = semantic {
            self.semantic(surface, source, n.local_bounds())?;
        }
        if n.semantic.is_some() {
            self.semantic_depth -= 1;
        }
        surface.pop();
        surface.pop();
        Ok(())
    }
}
pub(crate) fn draw(surface: &mut Surface, list: &DisplayList, dpi: f64) -> Result<()> {
    let mut writer = Writer {
        list,
        dpi,
        fonts: BTreeMap::new(),
        images: BTreeMap::new(),
        effect_pixels: 0,
        semantic_depth: 0,
        semantic_font: None,
    };
    for node in &list.nodes {
        writer.node(surface, node, Affine::IDENTITY)?;
    }
    Ok(())
}
