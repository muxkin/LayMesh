//! Skia command adapter, shared by CPU surfaces and SkPDF pages.
use crate::{display_list::*, error, retained::NodeFrame};
use kurbo::{Affine, BezPath, PathEl, Rect};
use laymesh_core::Result;
use skia_safe as sk;
use std::collections::BTreeMap;
#[path = "skia_surface.rs"]
mod surface;

pub(crate) fn matrix(a: Affine) -> sk::Matrix {
    let [a, b, c, d, e, f] = a.as_coeffs();
    sk::Matrix::new_all(a as _, c as _, e as _, b as _, d as _, f as _, 0., 0., 1.)
}
pub(crate) fn rect(r: Rect) -> sk::Rect {
    sk::Rect::new(r.x0 as _, r.y0 as _, r.x1 as _, r.y1 as _)
}
fn point(p: kurbo::Point) -> sk::Point {
    sk::Point::new(p.x as _, p.y as _)
}
pub(crate) fn color(c: [f64; 4]) -> sk::Color4f {
    sk::Color4f::new(c[0] as _, c[1] as _, c[2] as _, c[3] as _)
}
fn path(p: &BezPath, even_odd: bool) -> sk::Path {
    let mut out = sk::PathBuilder::new();
    for e in p.iter() {
        match e {
            PathEl::MoveTo(p) => {
                out.move_to(point(p));
            }
            PathEl::LineTo(p) => {
                out.line_to(point(p));
            }
            PathEl::QuadTo(a, b) => {
                out.quad_to(point(a), point(b));
            }
            PathEl::CurveTo(a, b, c) => {
                out.cubic_to(point(a), point(b), point(c));
            }
            PathEl::ClosePath => {
                out.close();
            }
        }
    }
    out.set_fill_type(if even_odd {
        sk::PathFillType::EvenOdd
    } else {
        sk::PathFillType::Winding
    });
    out.detach()
}
fn solid(c: [f64; 4]) -> sk::Paint {
    let mut p = sk::Paint::default();
    p.set_anti_alias(true).set_color4f(color(c), None);
    p
}
pub(crate) fn rgba_image(width: u32, height: u32, rgba: &[u8]) -> Result<sk::Image> {
    let info = sk::ImageInfo::new(
        (width as i32, height as i32),
        sk::ColorType::RGBA8888,
        sk::AlphaType::Unpremul,
        sk::ColorSpace::new_srgb(),
    );
    sk::images::raster_from_data(&info, sk::Data::new_copy(rgba), width as usize * 4)
        .ok_or_else(|| error("Skia image allocation failed"))
}
pub(crate) fn decode(resource: &Resource) -> Result<sk::Image> {
    if resource.mime == "image/x-laymesh-rgba" {
        if resource.data.len() < 8 {
            return Err(error("Invalid RGBA image"));
        }
        let w = u32::from_le_bytes(resource.data[..4].try_into().unwrap());
        let h = u32::from_le_bytes(resource.data[4..8].try_into().unwrap());
        if w as u64 * h as u64 > crate::raster::MAX_PIXELS
            || resource.data.len() != 8 + w as usize * h as usize * 4
        {
            return Err(error("Invalid RGBA image dimensions"));
        }
        return rgba_image(w, h, &resource.data[8..]);
    }
    // Keep encoded JPEG bytes available to SkPDF for lossless passthrough.
    if resource.mime == "image/jpeg" {
        if let Some(i) = sk::Image::from_encoded(sk::Data::new_copy(&resource.data)) {
            return Ok(i);
        }
    }
    let image = image::load_from_memory(&resource.data)
        .map_err(|e| error(e.to_string()))?
        .into_rgba8();
    if image.width() as u64 * image.height() as u64 > crate::raster::MAX_PIXELS {
        return Err(error("Image exceeds pixel budget"));
    }
    rgba_image(image.width(), image.height(), image.as_raw())
}

pub(crate) struct Painter<'a> {
    pub list: &'a DisplayList,
    pub images: BTreeMap<String, sk::Image>,
    fonts: BTreeMap<String, sk::Typeface>,
    pub pdf: bool,
    pub dpi: f64,
    pub semantic_ids: BTreeMap<usize, i32>,
    semantic_depth: usize,
    effect_pixels: u64,
    pub(crate) surface_cache: BTreeMap<String, Vec<(Rect, sk::Image)>>,
    pub(crate) surface_cache_limit: u64,
}
impl<'a> Painter<'a> {
    pub fn new(list: &'a DisplayList) -> Result<Self> {
        let images = list
            .resources
            .iter()
            .filter(|(_, r)| r.mime.starts_with("image/"))
            .map(|(id, r)| Ok((id.clone(), decode(r)?)))
            .collect::<Result<_>>()?;
        Ok(Self::with_images(list, images))
    }
    pub fn with_images(list: &'a DisplayList, images: BTreeMap<String, sk::Image>) -> Self {
        Self {
            list,
            images,
            fonts: BTreeMap::new(),
            pdf: false,
            dpi: 1200.,
            semantic_ids: BTreeMap::new(),
            semantic_depth: 0,
            effect_pixels: 0,
            surface_cache: BTreeMap::new(),
            surface_cache_limit: 256 * 1024 * 1024,
        }
    }
    #[allow(deprecated)]
    fn paint(&mut self, p: &Paint) -> Result<Option<sk::Paint>> {
        let mut out = solid([0., 0., 0., 1.]);
        let shader = match p {
            Paint::None => return Ok(None),
            Paint::Solid { rgba } => return Ok(Some(solid(*rgba))),
            Paint::Linear {
                start,
                end,
                stops,
                transform,
                spread,
            }
            | Paint::Radial {
                center: start,
                focal: end,
                stops,
                transform,
                spread,
                ..
            } => {
                let colors: Vec<_> = stops.iter().map(|s| color(s.rgba)).collect();
                let offsets: Vec<_> = stops.iter().map(|s| s.offset as f32).collect();
                let m = matrix(*transform);
                let mode = match spread.as_str() {
                    "repeat" => sk::TileMode::Repeat,
                    "reflect" => sk::TileMode::Mirror,
                    _ => sk::TileMode::Clamp,
                };
                if let Paint::Radial { radius, .. } = p {
                    sk::gradient_shader::two_point_conical(
                        (end[0] as f32, end[1] as f32),
                        0.,
                        (start[0] as f32, start[1] as f32),
                        *radius as f32,
                        colors.as_slice(),
                        offsets.as_slice(),
                        mode,
                        None,
                        &m,
                    )
                } else {
                    sk::gradient_shader::linear(
                        (
                            (start[0] as f32, start[1] as f32),
                            (end[0] as f32, end[1] as f32),
                        ),
                        colors.as_slice(),
                        offsets.as_slice(),
                        mode,
                        None,
                        &m,
                    )
                }
            }
            Paint::Conic {
                center,
                angle,
                stops,
                transform,
            } => {
                let colors: Vec<_> = stops.iter().map(|s| color(s.rgba)).collect();
                let offsets: Vec<_> = stops.iter().map(|s| s.offset as f32).collect();
                let rotation = Affine::translate((center[0], center[1]))
                    * Affine::rotate(*angle - std::f64::consts::FRAC_PI_2)
                    * Affine::translate((-center[0], -center[1]));
                sk::gradient_shader::sweep(
                    (center[0] as f32, center[1] as f32),
                    colors.as_slice(),
                    offsets.as_slice(),
                    sk::TileMode::Clamp,
                    None,
                    None,
                    &matrix(*transform * rotation),
                )
            }
            Paint::Pattern {
                width,
                height,
                transform,
                nodes,
            } => {
                let b = sk::Rect::from_wh(*width as f32, *height as f32);
                let mut recorder = sk::PictureRecorder::new();
                let c = recorder.begin_recording(b, false);
                for (i, n) in nodes.iter().enumerate() {
                    self.node(c, n, &format!("pattern/{i}"), &BTreeMap::new())?;
                }
                let picture = recorder
                    .finish_recording_as_picture(None)
                    .ok_or_else(|| error("Skia pattern recording failed"))?;
                Some(picture.to_shader(
                    (sk::TileMode::Repeat, sk::TileMode::Repeat),
                    sk::FilterMode::Linear,
                    &matrix(*transform),
                    &b,
                ))
            }
        }
        .ok_or_else(|| error("Skia shader creation failed"))?;
        out.set_shader(shader);
        Ok(Some(out))
    }
    pub fn draw(&mut self, c: &sk::Canvas, states: &BTreeMap<String, NodeFrame>) -> Result<()> {
        for (i, node) in self.list.nodes.iter().enumerate() {
            self.node(c, node, &i.to_string(), states)?;
        }
        Ok(())
    }
    pub fn node(
        &mut self,
        c: &sk::Canvas,
        original: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<()> {
        let frame = states.get(id);
        let node = frame.and_then(|s| s.content.as_ref()).unwrap_or(original);
        let opacity = frame.map_or(node.opacity, |s| s.opacity);
        if opacity <= 0. {
            return Ok(());
        }
        let saved = c.save();
        if let Some(f) = frame {
            let m = f.matrix.map(|x| x as f32);
            c.concat(&sk::Matrix::new_all(
                m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8],
            ));
        } else {
            c.concat(&matrix(node.transform));
        }
        if opacity < 1. {
            c.save_layer_alpha_f(None, opacity.clamp(0., 1.) as f32);
        }
        let semantic = self.pdf && node.semantic.is_some() && self.semantic_depth == 0;
        if node.semantic.is_some() {
            self.semantic_depth += 1;
        }
        // PDF keeps subjects vector and rasterizes only explicit effect tiles.
        let tiles = if self.pdf && !node.effects.is_empty() {
            let m = c.local_to_device_as_3x3();
            // SkPDF's device matrix already includes metadata.raster_dpi / 72.
            let scale = crate::portable_svg::max_scale(Affine::new([
                m.scale_x() as f64,
                m.skew_y() as f64,
                m.skew_x() as f64,
                m.scale_y() as f64,
                0.,
                0.,
            ]));
            let tiles = crate::raster::effect_tiles(self.list, node, scale)?;
            self.effect_pixels += tiles
                .iter()
                .map(|t| t.pixels.width() as u64 * t.pixels.height() as u64)
                .sum::<u64>();
            if self.effect_pixels > crate::raster::MAX_PIXELS {
                return Err(error("PDF effects exceed the 100000000-pixel budget"));
            }
            tiles
        } else {
            vec![]
        };
        for t in tiles.iter().filter(|t| !t.inner) {
            self.effect_tile(c, t)?;
        }
        if !self.pdf {
            for e in node.effects.iter().filter(|e| !e.inner) {
                self.effect(c, node, id, states, e)?;
            }
        }
        if node.hint["effectsOnly"] != true {
            self.content(c, node, id, states)?;
        }
        if !self.pdf {
            for e in node.effects.iter().filter(|e| e.inner) {
                self.effect(c, node, id, states, e)?;
            }
        }
        for t in tiles.iter().filter(|t| t.inner) {
            self.effect_tile(c, t)?;
        }
        if semantic {
            if let Some(&tag) = self.semantic_ids.get(&(original as *const _ as usize)) {
                // The PDF compatibility pass makes this tagged text invisible and
                // assigns ActualText. Paths above remain the sole visible subject.
                let bytes = ratex_katex_fonts::ttf_bytes("KaTeX_Main-Regular.ttf")
                    .ok_or_else(|| error("Missing semantic font"))?;
                let face = sk::FontMgr::new()
                    .new_from_data(sk::Data::new_copy(&bytes), None)
                    .ok_or_else(|| error("Invalid semantic font"))?;
                let source = node.semantic.as_deref().unwrap();
                let b = node.local_bounds();
                let mut font = sk::Font::from_typeface(face, b.height().max(0.1) as f32);
                // Preserve a real ToUnicode fallback for readers that ignore
                // ActualText (including older MuPDF structured-text output).
                let glyphs = font.str_to_glyphs_vec(source);
                let clusters: Vec<_> = source.char_indices().map(|(i, _)| i as u32).collect();
                let mut widths = vec![0.; glyphs.len()];
                font.get_widths(&glyphs, &mut widths);
                let scale = b.width() / f64::from(widths.iter().sum::<f32>().max(0.001));
                font.set_scale_x(scale as f32);
                let mut x = b.x0;
                let positions: Vec<_> = widths
                    .iter()
                    .map(|width| {
                        let point = sk::Point::new(x as f32, (b.y0 + b.height() * 0.8) as f32);
                        x += f64::from(*width) * scale;
                        point
                    })
                    .collect();
                sk::pdf::set_node_id(c, tag);
                c.draw_glyphs_utf8(
                    &glyphs,
                    &positions,
                    &clusters,
                    source,
                    (0., 0.),
                    &font,
                    &solid([0., 0., 0., 1.]),
                );
                sk::pdf::set_node_id(c, 0);
            }
        }
        if node.semantic.is_some() {
            self.semantic_depth -= 1;
        }
        c.restore_to_count(saved);
        Ok(())
    }
    fn effect_tile(&self, c: &sk::Canvas, t: &crate::raster::EffectTile) -> Result<()> {
        let image = rgba_image(t.pixels.width(), t.pixels.height(), t.pixels.as_raw())?;
        c.draw_image_rect_with_sampling_options(
            image,
            None,
            rect(t.rect),
            sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None),
            &sk::Paint::default(),
        );
        Ok(())
    }
    pub(crate) fn effect(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
        e: &Effect,
    ) -> Result<()> {
        if e.soft_edge.is_some() {
            return Ok(());
        }
        if let Some(r) = &e.reflection {
            return self.reflection(c, node, id, states, e, r);
        }
        let spread = if e.spread != 0. {
            let radius = e.spread.abs() as f32;
            if (e.spread > 0.) == e.inner {
                sk::image_filters::erode((radius, radius), None, None)
            } else {
                sk::image_filters::dilate((radius, radius), None, None)
            }
        } else {
            None
        };
        let shadow = sk::image_filters::drop_shadow_only(
            (e.offset[0] as f32, e.offset[1] as f32),
            (e.blur as f32, e.blur as f32),
            if e.inner {
                color([0., 0., 0., 1.])
            } else {
                color(e.rgba)
            },
            None,
            spread,
            None,
        );
        let filter = if e.inner {
            let cut = sk::image_filters::blend(sk::BlendMode::SrcOut, shadow, None, None);
            let tint = sk::color_filters::blend(color(e.rgba).to_color(), sk::BlendMode::SrcIn)
                .ok_or_else(|| error("Skia shadow tint failed"))?;
            sk::image_filters::color_filter(tint, cut, None)
        } else {
            shadow
        };
        let mut paint = sk::Paint::default();
        paint.set_image_filter(filter);
        let saved = c.save();
        if e.knockout {
            if let Some(source) = &e.source_path {
                c.clip_path(&path(source, false), sk::ClipOp::Difference, true);
            }
        }
        c.save_layer(&sk::canvas::SaveLayerRec::default().paint(&paint));
        if let Some(source) = &e.source_path {
            c.draw_path(&path(source, false), &solid([1., 1., 1., 1.]));
        } else {
            self.content(c, node, id, states)?;
        }
        c.restore();
        c.restore_to_count(saved);
        Ok(())
    }
    fn pdf_gradient_fill(&mut self, c: &sk::Canvas, node: &DrawNode) -> Result<()> {
        let mut local = node.clone();
        local.transform = kurbo::Affine::IDENTITY;
        local.opacity = 1.;
        local.effects.clear();
        local.semantic = None;
        if let DrawKind::Path { stroke, .. } = &mut local.kind {
            *stroke = None;
        }
        let m = c.local_to_device_as_3x3();
        let transform = kurbo::Affine::new([
            m.scale_x() as f64,
            m.skew_y() as f64,
            m.skew_x() as f64,
            m.scale_y() as f64,
            0.,
            0.,
        ]);
        let density = crate::portable_svg::max_scale(transform);
        if density <= 0. {
            return Ok(());
        }
        let b = local.local_bounds();
        let b = kurbo::Rect::new(
            (b.x0 * density + 1e-5).floor() / density,
            (b.y0 * density + 1e-5).floor() / density,
            (b.x1 * density - 1e-5).ceil() / density,
            (b.y1 * density - 1e-5).ceil() / density,
        );
        if b.width() <= 0. || b.height() <= 0. {
            return Ok(());
        }
        let (w, h) = crate::raster_dimensions(b.width(), b.height(), density * 25.4, "png")?;
        self.effect_pixels += w as u64 * h as u64;
        if self.effect_pixels > crate::raster::MAX_PIXELS {
            return Err(error(
                "PDF local fills/effects exceed the 100000000-pixel budget",
            ));
        }
        let list = DisplayList {
            nodes: vec![local],
            ..self.list.clone()
        };
        let mut raster = crate::raster::PreparedRaster::new(list)?;
        for y in (0..h).step_by(4096) {
            for x in (0..w).step_by(4096) {
                let tw = (w - x).min(4096);
                let th = (h - y).min(4096);
                let b = kurbo::Rect::new(
                    b.x0 + x as f64 / density,
                    b.y0 + y as f64 / density,
                    b.x0 + (x + tw) as f64 / density,
                    b.y0 + (y + th) as f64 / density,
                );
                let frame = raster.render_region(b, tw, th)?;
                let image = rgba_image(tw, th, &frame.rgba)?;
                c.draw_image_rect_with_sampling_options(
                    image,
                    None,
                    rect(b),
                    sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None),
                    &sk::Paint::default(),
                );
            }
        }
        Ok(())
    }
    pub fn content(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<()> {
        let radius = node
            .effects
            .iter()
            .filter_map(|e| e.soft_edge)
            .fold(0., f64::max);
        if radius <= 0. {
            return self.subject(c, node, id, states);
        }
        if self.pdf {
            return self.soft_subject_pdf(c, node, radius);
        }
        let saved = c.save();
        c.save_layer(&sk::canvas::SaveLayerRec::default());
        self.subject(c, node, id, states)?;
        let erosion =
            sk::image_filters::erode(((radius / 2.) as f32, (radius / 2.) as f32), None, None);
        let filter = sk::image_filters::blur(
            ((radius / 2.) as f32, (radius / 2.) as f32),
            sk::TileMode::Decal,
            erosion,
            None,
        );
        let mut p = sk::Paint::default();
        p.set_blend_mode(sk::BlendMode::DstIn)
            .set_image_filter(filter);
        c.save_layer(&sk::canvas::SaveLayerRec::default().paint(&p));
        self.subject(c, node, id, states)?;
        c.restore_to_count(saved);
        Ok(())
    }
    fn subject(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<()> {
        if node.hint["surface3d"].is_object() {
            return self.surface3d(c, node, id, states);
        }
        self.raw_content(c, node, id, states)
    }
    #[allow(deprecated)]
    fn reflection(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
        e: &Effect,
        r: &Reflection,
    ) -> Result<()> {
        let b = node.local_bounds();
        let height = b.height() * r.length;
        if height <= 0. || r.opacity <= 0. {
            return Ok(());
        }
        let y = b.y1 + r.distance;
        let saved = c.save();
        let mut blur = sk::Paint::default();
        blur.set_image_filter(sk::image_filters::blur(
            (e.blur as f32, e.blur as f32),
            sk::TileMode::Decal,
            None,
            None,
        ));
        c.save_layer(&sk::canvas::SaveLayerRec::default().paint(&blur));
        c.clip_rect(
            sk::Rect::new(b.x0 as f32, y as f32, b.x1 as f32, (y + height) as f32),
            None,
            false,
        );
        c.save();
        c.translate((0., (y + b.y1) as f32));
        c.scale((1., -1.));
        self.content(c, node, id, states)?;
        c.restore();
        let colors = [
            sk::Color::from_argb((r.opacity * 255.) as u8, 255, 255, 255),
            sk::Color::from_argb((r.opacity * (1. - r.fade) * 255.) as u8, 255, 255, 255),
        ];
        let shader = sk::gradient_shader::linear(
            ((b.x0 as f32, y as f32), (b.x0 as f32, (y + height) as f32)),
            &colors[..],
            None,
            sk::TileMode::Clamp,
            None,
            None,
        );
        let mut mask = sk::Paint::default();
        mask.set_shader(shader).set_blend_mode(sk::BlendMode::DstIn);
        c.draw_paint(&mask);
        c.restore_to_count(saved);
        Ok(())
    }
    fn raw_content(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<()> {
        match &node.kind {
            DrawKind::Group {
                children,
                clip,
                mask,
            } => {
                let saved = c.save();
                if let Some(clip) = clip {
                    c.clip_path(
                        &path(&clip.path, clip.even_odd),
                        sk::ClipOp::Intersect,
                        true,
                    );
                }
                if mask.is_some() {
                    c.save_layer(&sk::canvas::SaveLayerRec::default());
                }
                if node.hint["animationRevealWrapper"] == true && children.len() == 1 {
                    c.save();
                    c.concat(&matrix(children[0].transform));
                    self.content(c, &children[0], id, states)?;
                    c.restore();
                } else {
                    for (i, n) in children.iter().enumerate() {
                        self.node(c, n, &format!("{id}/{i}"), states)?;
                    }
                }
                if let Some(mask) = mask {
                    let mut p = sk::Paint::default();
                    p.set_blend_mode(sk::BlendMode::DstIn);
                    if mask.luminance {
                        p.set_color_filter(sk::luma_color_filter::new());
                    }
                    c.save_layer(&sk::canvas::SaveLayerRec::default().paint(&p));
                    self.node(c, &mask.node, &format!("{id}/mask"), &BTreeMap::new())?;
                    c.restore();
                    c.restore();
                }
                c.restore_to_count(saved);
            }
            DrawKind::Path {
                path: p,
                fill,
                stroke,
                even_odd,
            } => {
                let p = path(p, *even_odd);
                fn has_gradient(p: &Paint) -> bool {
                    match p {
                        Paint::Linear { .. } | Paint::Radial { .. } | Paint::Conic { .. } => true,
                        Paint::Pattern { nodes, .. } => nodes.iter().any(
                            |n| matches!(&n.kind,DrawKind::Path{fill,..} if has_gradient(fill)),
                        ),
                        _ => false,
                    }
                }
                let local_pdf = match fill {
                    Paint::Conic { .. } => true,
                    Paint::Linear { spread, .. } => spread != "pad",
                    Paint::Radial { spread, stops, .. } => {
                        spread != "pad"
                            || stops
                                .windows(2)
                                .any(|s| s[0].offset == s[1].offset && s[0].rgba != s[1].rgba)
                    }
                    Paint::Pattern { .. } => has_gradient(fill),
                    _ => false,
                };
                if self.pdf && local_pdf {
                    self.pdf_gradient_fill(c, node)?;
                } else if let Paint::Pattern {
                    nodes, transform, ..
                } = fill
                {
                    if nodes.len() == 1 && image_pattern(&nodes[0]) {
                        c.save();
                        c.clip_path(&p, sk::ClipOp::Intersect, true);
                        c.concat(&matrix(*transform));
                        self.node(c, &nodes[0], "image-fill", states)?;
                        c.restore();
                    } else if let Some(paint) = self.paint(fill)? {
                        c.draw_path(&p, &paint);
                    }
                } else if let Some(paint) = self.paint(fill)? {
                    c.draw_path(&p, &paint);
                }
                if let Some(s) = stroke {
                    let mut paint = solid(s.rgba);
                    paint
                        .set_style(sk::paint::Style::Stroke)
                        .set_stroke_width(s.width as _)
                        .set_stroke_miter(s.miter as _)
                        .set_stroke_cap(match s.cap.as_str() {
                            "round" => sk::paint::Cap::Round,
                            "square" => sk::paint::Cap::Square,
                            _ => sk::paint::Cap::Butt,
                        })
                        .set_stroke_join(match s.join.as_str() {
                            "round" => sk::paint::Join::Round,
                            "bevel" => sk::paint::Join::Bevel,
                            _ => sk::paint::Join::Miter,
                        });
                    if !s.dash.is_empty() {
                        paint.set_path_effect(sk::PathEffect::dash(
                            &s.dash.iter().map(|v| *v as f32).collect::<Vec<_>>(),
                            s.offset as f32,
                        ));
                    }
                    c.draw_path(&p, &paint);
                }
            }
            DrawKind::Glyphs {
                resource,
                text,
                size,
                glyphs,
                outlines,
                color: rgba,
                ..
            } => {
                let paint = solid(*rgba);
                if self.pdf && self.semantic_depth == 0 {
                    if !self.fonts.contains_key(resource) {
                        let r = self
                            .list
                            .resources
                            .get(resource)
                            .ok_or_else(|| error("Missing PDF font"))?;
                        let face = sk::FontMgr::new()
                            .new_from_data(sk::Data::new_copy(&r.data), r.font_index)
                            .ok_or_else(|| error("Invalid PDF font"))?;
                        self.fonts.insert(resource.clone(), face);
                    }
                    let font = sk::Font::from_typeface(self.fonts[resource].clone(), *size as f32);
                    c.draw_glyphs_utf8(
                        &glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
                        &glyphs
                            .iter()
                            .map(|g| sk::Point::new(g.x as f32, g.y as f32))
                            .collect::<Vec<_>>(),
                        &glyphs
                            .iter()
                            .map(|g| g.source.start as u32)
                            .collect::<Vec<_>>(),
                        text,
                        (0., 0.),
                        &font,
                        &paint,
                    );
                } else {
                    for p in outlines {
                        c.draw_path(&path(p, false), &paint);
                    }
                }
            }
            DrawKind::Image {
                resource,
                rect: r,
                nearest,
            } => {
                let image = self
                    .images
                    .get(resource)
                    .ok_or_else(|| error("Missing Skia image"))?;
                c.draw_image_rect_with_sampling_options(
                    image,
                    None,
                    rect(*r),
                    sk::SamplingOptions::new(
                        if *nearest {
                            sk::FilterMode::Nearest
                        } else {
                            sk::FilterMode::Linear
                        },
                        sk::MipmapMode::None,
                    ),
                    &sk::Paint::default(),
                );
            }
        }
        Ok(())
    }
}

fn image_pattern(n: &DrawNode) -> bool {
    match &n.kind {
        DrawKind::Image { .. } => true,
        DrawKind::Group { children, .. } => children.len() == 1 && image_pattern(&children[0]),
        _ => false,
    }
}
