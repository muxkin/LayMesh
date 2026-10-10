//! Pure Rust CPU renderer, shared without native features by CLI and WASM.
use super::*;
use display_list::*;
use kurbo::{Affine, Rect};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};
use vello_cpu::{Image, ImageSource, PixelMetadata, Pixmap, RenderContext, Resources, peniko};

pub const MAX_PIXELS: u64 = 100_000_000;
const TILE: u32 = 1024;
const MAX_EFFECT_TILE: u64 = 16_000_000;
#[derive(Clone, Debug)]
pub struct RasterFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub(crate) struct EffectTile {
    pub rect: Rect,
    pub pixels: Arc<Pixmap>,
    pub inner: bool,
}
struct ImageTile {
    x: u32,
    y: u32,
    inner: Rect,
    pixels: Arc<Pixmap>,
}
struct DecodedImage {
    width: u32,
    height: u32,
    tiles: Vec<ImageTile>,
}
type Images = BTreeMap<String, Arc<DecodedImage>>;
struct CachedEffects {
    tiles: Arc<Vec<EffectTile>>,
    pixels: u64,
    costs: BTreeMap<(usize, u64), u64>,
    used: u64,
}
struct EffectCache {
    entries: BTreeMap<(usize, u64), CachedEffects>,
    bytes: u64,
    budget: u64,
    tick: u64,
}
impl Default for EffectCache {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            bytes: 0,
            budget: 64 * 1024 * 1024,
            tick: 0,
        }
    }
}
impl EffectCache {
    fn trim(&mut self, incoming: u64) {
        while self.bytes.saturating_add(incoming) > self.budget || self.entries.len() >= 128 {
            let Some(key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            self.bytes -= self.entries.remove(&key).unwrap().pixels * 4;
        }
    }
}
/// An immutable drawing and its decoded resources/effect tiles. Moving this
/// object cannot change node addresses: the list is boxed and never mutated.
pub struct PreparedRaster {
    list: Box<DisplayList>,
    images: Images,
    effects: EffectCache,
}
impl PreparedRaster {
    pub fn new(list: DisplayList) -> Result<Self> {
        if list.schema_version != DISPLAY_LIST_VERSION {
            return Err(error("不支持的绘制指令版本"));
        }
        let images = decode(&list.resources)?;
        Ok(Self {
            list: Box::new(list),
            images,
            effects: EffectCache::default(),
        })
    }
    pub fn display_list(&self) -> &DisplayList {
        &self.list
    }
    /// Effects retain at most 64 MiB; current frames keep independent pixel
    /// references while an old cache entry is evicted.
    pub fn set_cache_budget(&mut self, mib: usize) {
        self.effects.budget = mib.min(64) as u64 * 1024 * 1024;
        self.effects.trim(0);
    }
    pub fn render_region(&mut self, rect: Rect, w: u32, h: u32) -> Result<RasterFrame> {
        validate_region(&self.list, rect, w, h)?;
        render_prepared(&self.list, rect, w, h, &self.images, &mut self.effects)
    }
}
struct ImageCache {
    entries: HashMap<String, (Arc<DecodedImage>, usize, u64)>,
    bytes: usize,
    tick: u64,
    budget: usize,
}
thread_local! {
    static IMAGE_CACHE: RefCell<ImageCache> = RefCell::new(ImageCache {
        entries: HashMap::new(), bytes: 0, tick: 0, budget: 256 * 1024 * 1024,
    });
}
/// Bound reusable decoded image storage; current renders own their pixels
/// independently, so eviction never invalidates an in-flight drawing job.
pub fn set_image_cache_budget(mib: usize) {
    IMAGE_CACHE.with_borrow_mut(|cache| {
        cache.budget = mib.saturating_mul(1024 * 1024);
        trim_images(cache, 0);
    });
}
fn trim_images(cache: &mut ImageCache, incoming: usize) {
    while cache.bytes.saturating_add(incoming) > cache.budget || cache.entries.len() >= 128 {
        let Some(oldest) = cache
            .entries
            .iter()
            .min_by_key(|(_, e)| e.2)
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        cache.bytes -= cache.entries.remove(&oldest).unwrap().1;
    }
}
fn color(r: [f64; 4]) -> peniko::Color {
    peniko::Color::new(r.map(|c| c as f32))
}
fn spread(s: &str) -> peniko::Extend {
    match s {
        "repeat" => peniko::Extend::Repeat,
        "reflect" => peniko::Extend::Reflect,
        _ => peniko::Extend::Pad,
    }
}
fn scale(t: Affine) -> f64 {
    let [a, b, c, d, _, _] = t.as_coeffs();
    a.hypot(b).max(c.hypot(d))
}
fn pixmap(bytes: Vec<u8>, w: u32, h: u32) -> Pixmap {
    Pixmap::from_parts(
        bytes,
        w as u16,
        h as u16,
        PixelMetadata::new(peniko::ImageAlphaType::Alpha, true),
    )
}
fn decode(resources: &BTreeMap<String, Resource>) -> Result<Images> {
    let mut out = Images::new();
    for (id, r) in resources {
        if !r.mime.starts_with("image/") {
            continue;
        }
        // Caller IDs are not trusted as content keys: replacing bytes under
        // the same ID must replace the decoded pixels, on native and WASM.
        let key = laymesh_core::asset_cache::digest(&r.data);
        if let Some(image) = IMAGE_CACHE.with_borrow_mut(|cache| {
            cache.tick += 1;
            let tick = cache.tick;
            cache.entries.get_mut(&key).map(|(image, _, used)| {
                *used = tick;
                image.clone()
            })
        }) {
            out.insert(id.clone(), image);
            continue;
        }
        let im = image::load_from_memory(&r.data)
            .map_err(|e| error(format!("图片解码失败：{e}")))?
            .into_rgba8();
        let (w, h) = im.dimensions();
        if w as u64 * h as u64 > MAX_PIXELS {
            return Err(error("图片像素数超过 100000000 上限"));
        }
        let mut tiles = Vec::new();
        if w <= u16::MAX as u32 && h <= u16::MAX as u32 {
            tiles.push(ImageTile {
                x: 0,
                y: 0,
                inner: Rect::new(0., 0., w as f64, h as f64),
                pixels: Arc::new(pixmap(im.into_raw(), w, h)),
            });
        } else {
            for y in (0..h).step_by(4096) {
                for x in (0..w).step_by(4096) {
                    let tw = (w - x).min(4096);
                    let th = (h - y).min(4096);
                    let rx = x.saturating_sub(2);
                    let ry = y.saturating_sub(2);
                    let right = (x + tw + 2).min(w);
                    let bottom = (y + th + 2).min(h);
                    let bytes = image::imageops::crop_imm(&im, rx, ry, right - rx, bottom - ry)
                        .to_image()
                        .into_raw();
                    tiles.push(ImageTile {
                        x: rx,
                        y: ry,
                        inner: Rect::new(x as f64, y as f64, (x + tw) as f64, (y + th) as f64),
                        pixels: Arc::new(pixmap(bytes, right - rx, bottom - ry)),
                    });
                }
            }
        }
        let image = Arc::new(DecodedImage {
            width: w,
            height: h,
            tiles,
        });
        let bytes = image
            .tiles
            .iter()
            .map(|tile| tile.pixels.width() as usize * tile.pixels.height() as usize * 4)
            .sum();
        IMAGE_CACHE.with_borrow_mut(|cache| {
            if bytes <= cache.budget {
                trim_images(cache, bytes);
                cache.tick += 1;
                let tick = cache.tick;
                cache.entries.insert(key, (image.clone(), bytes, tick));
                cache.bytes += bytes;
            }
        });
        out.insert(id.clone(), image);
    }
    Ok(out)
}
pub fn dimensions(width: f64, height: f64, dpi: f64) -> Result<(u32, u32)> {
    if !dpi.is_finite() || dpi <= 0. || dpi > 25_400. {
        return Err(error("DPI 必须为 0–25400 之间的正数"));
    }
    if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
        return Err(error("画布尺寸必须为正"));
    }
    let w = (width * dpi / 25.4).round().max(1.);
    let h = (height * dpi / 25.4).round().max(1.);
    if w * h > MAX_PIXELS as f64 || w > u32::MAX as f64 || h > u32::MAX as f64 {
        return Err(error("导出像素数超过 100000000 上限"));
    }
    Ok((w as u32, h as u32))
}
pub fn render_rgba(list: &DisplayList, dpi: f64) -> Result<RasterFrame> {
    if list.schema_version != DISPLAY_LIST_VERSION {
        return Err(error("不支持的绘制指令版本"));
    }
    let (w, h) = dimensions(list.width, list.height, dpi)?;
    render_region(list, Rect::new(0., 0., list.width, list.height), w, h)
}
/// Render a viewport in physical coordinates, allowing previews to avoid huge offscreen canvases.
pub fn render_region(list: &DisplayList, rect: Rect, w: u32, h: u32) -> Result<RasterFrame> {
    validate_region(list, rect, w, h)?;
    let images = decode(&list.resources)?;
    render_prepared(list, rect, w, h, &images, &mut EffectCache::default())
}
fn validate_region(list: &DisplayList, rect: Rect, w: u32, h: u32) -> Result<()> {
    if list.schema_version != DISPLAY_LIST_VERSION
        || w == 0
        || h == 0
        || w as u64 * h as u64 > MAX_PIXELS
        || !rect.width().is_finite()
        || !rect.height().is_finite()
        || rect.width() <= 0.
        || rect.height() <= 0.
    {
        return Err(error("无效渲染区域或超过 100000000 像素上限"));
    }
    Ok(())
}
fn render_prepared(
    list: &DisplayList,
    rect: Rect,
    w: u32,
    h: u32,
    images: &Images,
    effects: &mut EffectCache,
) -> Result<RasterFrame> {
    let mut renderer = Renderer {
        images,
        effects,
        effect_pixels: 0,
        used_effects: BTreeSet::new(),
        frame_effects: BTreeMap::new(),
        effect_costs: Vec::new(),
    };
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(w as usize * h as usize * 4)
        .map_err(|_| error("无法分配导出像素缓冲区"))?;
    rgba.resize(w as usize * h as usize * 4, 0);
    let global = Affine::scale_non_uniform(w as f64 / rect.width(), h as f64 / rect.height())
        * Affine::translate((-rect.x0, -rect.y0));
    for y in (0..h).step_by(TILE as usize) {
        for x in (0..w).step_by(TILE as usize) {
            let tw = (w - x).min(TILE);
            let th = (h - y).min(TILE);
            let tr = Affine::translate((-(x as f64), -(y as f64))) * global;
            let pix = renderer.tile(&list.nodes, tr, tw, th)?;
            let data = pix.take_rgba8(peniko::ImageAlphaType::Alpha);
            for row in 0..th as usize {
                let to = ((y as usize + row) * w as usize + x as usize) * 4;
                rgba[to..to + tw as usize * 4]
                    .copy_from_slice(&data[row * tw as usize * 4..(row + 1) * tw as usize * 4]);
            }
        }
    }
    Ok(RasterFrame {
        width: w,
        height: h,
        rgba,
    })
}
struct Renderer<'a> {
    images: &'a Images,
    effects: &'a mut EffectCache,
    effect_pixels: u64,
    used_effects: BTreeSet<(usize, u64)>,
    frame_effects: BTreeMap<(usize, u64), (Arc<Vec<EffectTile>>, BTreeMap<(usize, u64), u64>)>,
    // Each retained effect includes its nested effects' logical pixel costs.
    // Cache hits must enforce the same frame budget as fresh calculations.
    effect_costs: Vec<((usize, u64), BTreeMap<(usize, u64), u64>)>,
}
impl Renderer<'_> {
    fn tile(&mut self, nodes: &[DrawNode], tr: Affine, w: u32, h: u32) -> Result<Pixmap> {
        self.tile_content(nodes, tr, w, h, false)
    }
    fn tile_content(
        &mut self,
        nodes: &[DrawNode],
        tr: Affine,
        w: u32,
        h: u32,
        source_root: bool,
    ) -> Result<Pixmap> {
        if w == 0
            || h == 0
            || w > u16::MAX as u32
            || h > u16::MAX as u32
            || w as u64 * h as u64 > MAX_EFFECT_TILE
        {
            return Err(error("渲染分块超过 64MB 缓冲区预算"));
        }
        let mut context = RenderContext::new(w as u16, h as u16);
        for n in nodes {
            self.node_content(&mut context, n, tr, w, h, source_root)?;
        }
        let mut target = Pixmap::new(w as u16, h as u16);
        context.flush();
        context.render(&mut target, &mut Resources::new());
        Ok(target)
    }
    fn image(
        &self,
        ctx: &mut RenderContext,
        pixels: Arc<Pixmap>,
        rect: Rect,
        tr: Affine,
        nearest: bool,
    ) {
        let (iw, ih) = (pixels.width() as f64, pixels.height() as f64);
        ctx.set_transform(tr);
        ctx.set_paint_transform(
            Affine::translate((rect.x0, rect.y0))
                * Affine::scale_non_uniform(rect.width() / iw, rect.height() / ih),
        );
        ctx.set_paint(Image {
            image: ImageSource::Pixmap(pixels),
            sampler: peniko::ImageSampler::default().with_quality(if nearest {
                peniko::ImageQuality::Low
            } else {
                peniko::ImageQuality::Medium
            }),
        });
        ctx.fill_rect(&rect);
        ctx.reset_paint_transform();
    }
    fn paint(&mut self, ctx: &mut RenderContext, p: &Paint, tr: Affine) -> Result<bool> {
        ctx.reset_paint_transform();
        match p {
            Paint::None => return Ok(false),
            Paint::Solid { rgba } => ctx.set_paint(color(*rgba)),
            Paint::Linear {
                start,
                end,
                stops,
                transform,
                spread: s,
            } => {
                ctx.set_paint(
                    peniko::Gradient::new_linear((start[0], start[1]), (end[0], end[1]))
                        .with_stops(
                            stops
                                .iter()
                                .map(|s| peniko::ColorStop::from((s.offset as f32, color(s.rgba))))
                                .collect::<Vec<_>>()
                                .as_slice(),
                        )
                        .with_extend(spread(s)),
                );
                ctx.set_paint_transform(*transform);
            }
            Paint::Radial {
                center,
                focal,
                radius,
                stops,
                transform,
                spread: s,
            } => {
                ctx.set_paint(
                    peniko::Gradient::new_two_point_radial(
                        (focal[0], focal[1]),
                        0.,
                        (center[0], center[1]),
                        *radius as f32,
                    )
                    .with_stops(
                        stops
                            .iter()
                            .map(|s| peniko::ColorStop::from((s.offset as f32, color(s.rgba))))
                            .collect::<Vec<_>>()
                            .as_slice(),
                    )
                    .with_extend(spread(s)),
                );
                ctx.set_paint_transform(*transform);
            }
            Paint::Pattern {
                width,
                height,
                transform,
                nodes,
            } => {
                let density = scale(tr * *transform).max(1.);
                let pw = (*width * density).ceil().max(1.) as u32;
                let ph = (*height * density).ceil().max(1.) as u32;
                let pixels = Arc::new(self.tile(
                    nodes,
                    Affine::scale_non_uniform(pw as f64 / width, ph as f64 / height),
                    pw,
                    ph,
                )?);
                ctx.set_paint(Image {
                    image: ImageSource::Pixmap(pixels),
                    sampler: peniko::ImageSampler::default().with_extend(peniko::Extend::Repeat),
                });
                ctx.set_paint_transform(
                    *transform * Affine::scale_non_uniform(width / pw as f64, height / ph as f64),
                );
            }
        }
        Ok(true)
    }
    fn node(
        &mut self,
        ctx: &mut RenderContext,
        n: &DrawNode,
        parent: Affine,
        w: u32,
        h: u32,
    ) -> Result<()> {
        self.node_content(ctx, n, parent, w, h, false)
    }
    fn node_content(
        &mut self,
        ctx: &mut RenderContext,
        n: &DrawNode,
        parent: Affine,
        w: u32,
        h: u32,
        source_root: bool,
    ) -> Result<()> {
        let tr = if source_root {
            parent
        } else {
            parent * n.transform
        };
        let b = tr.transform_rect_bbox(n.local_bounds());
        let effect_pad = if source_root {
            0.
        } else {
            n.effects
                .iter()
                .map(|e| 4. * e.blur + e.spread + e.offset[0].abs().max(e.offset[1].abs()))
                .fold(0f64, f64::max)
                * scale(tr)
        };
        if b.inflate(effect_pad, effect_pad)
            .intersect(Rect::new(0., 0., w as f64, h as f64))
            .area()
            <= 0.
        {
            return Ok(());
        }
        if !tr.as_coeffs().iter().all(|v| v.is_finite()) || !n.opacity.is_finite() {
            return Err(error("绘制变换或透明度无效"));
        }
        ctx.set_transform(tr);
        ctx.push_opacity_layer(if source_root {
            1.
        } else {
            n.opacity.clamp(0., 1.) as f32
        });
        let key = (n as *const DrawNode as usize, scale(tr).to_bits());
        let effect_tiles = if !source_root && !n.effects.is_empty() {
            Some(self.cached_effects(n, key)?)
        } else {
            None
        };
        if let Some(tiles) = &effect_tiles {
            for t in tiles.iter() {
                if !t.inner {
                    self.image(ctx, t.pixels.clone(), t.rect, tr, false);
                }
            }
        }
        match &n.kind {
            DrawKind::Group {
                children,
                clip,
                mask,
            } => {
                let mut layers = 0;
                if let Some(c) = clip {
                    ctx.set_transform(tr);
                    ctx.set_fill_rule(if c.even_odd {
                        peniko::Fill::EvenOdd
                    } else {
                        peniko::Fill::NonZero
                    });
                    ctx.push_clip_layer(&c.path);
                    layers += 1;
                }
                if let Some(m) = mask {
                    let pix = self.tile(std::slice::from_ref(m.node.as_ref()), tr, w, h)?;
                    ctx.push_mask_layer(if m.luminance {
                        vello_cpu::Mask::new_luminance(&pix)
                    } else {
                        vello_cpu::Mask::new_alpha(&pix)
                    });
                    layers += 1;
                }
                for child in children {
                    self.node(ctx, child, tr, w, h)?;
                }
                for _ in 0..layers {
                    ctx.pop_layer();
                }
            }
            DrawKind::Path {
                path,
                fill,
                stroke,
                even_odd,
            } => {
                ctx.set_transform(tr);
                ctx.set_fill_rule(if *even_odd {
                    peniko::Fill::EvenOdd
                } else {
                    peniko::Fill::NonZero
                });
                if let Paint::Pattern {
                    nodes, transform, ..
                } = fill
                {
                    if nodes.len() == 1 && is_image_pattern(&nodes[0]) {
                        ctx.push_clip_layer(path);
                        self.node(ctx, &nodes[0], tr * *transform, w, h)?;
                        ctx.pop_layer();
                    } else if self.paint(ctx, fill, tr)? {
                        ctx.fill_path(path);
                    }
                } else if self.paint(ctx, fill, tr)? {
                    ctx.fill_path(path);
                }
                ctx.set_transform(tr);

                if let Some(s) = stroke {
                    ctx.reset_paint_transform();
                    ctx.set_paint(color(s.rgba));
                    let style = kurbo::Stroke::new(s.width)
                        .with_caps(match s.cap.as_str() {
                            "round" => kurbo::Cap::Round,
                            "square" => kurbo::Cap::Square,
                            _ => kurbo::Cap::Butt,
                        })
                        .with_join(match s.join.as_str() {
                            "round" => kurbo::Join::Round,
                            "bevel" => kurbo::Join::Bevel,
                            _ => kurbo::Join::Miter,
                        })
                        .with_miter_limit(s.miter)
                        .with_dashes(s.offset, s.dash.iter().copied());
                    ctx.set_stroke(style);
                    ctx.stroke_path(path);
                }
            }
            DrawKind::Glyphs {
                outlines, color: c, ..
            } => {
                ctx.set_transform(tr);
                ctx.reset_paint_transform();
                ctx.set_fill_rule(peniko::Fill::NonZero);
                ctx.set_paint(color(*c));
                for path in outlines {
                    ctx.fill_path(path);
                }
            }
            DrawKind::Image {
                resource,
                rect,
                nearest,
            } => {
                let im = self
                    .images
                    .get(resource)
                    .ok_or_else(|| error(format!("缺少图片资源：{resource}")))?;
                let sx = rect.width() / im.width as f64;
                let sy = rect.height() / im.height as f64;
                let previous = ctx.blend_mode();
                // Sum complementary edge coverage inside this isolated image layer.
                // Source-over would reduce alpha where two fractional tile edges meet.
                if im.tiles.len() > 1 {
                    ctx.set_blend_mode(peniko::Compose::Plus.into());
                }
                for t in &im.tiles {
                    let r = Rect::new(
                        rect.x0 + t.x as f64 * sx,
                        rect.y0 + t.y as f64 * sy,
                        rect.x0 + (t.x as f64 + t.pixels.width() as f64) * sx,
                        rect.y0 + (t.y as f64 + t.pixels.height() as f64) * sy,
                    );
                    let inner = Rect::new(
                        rect.x0 + t.inner.x0 * sx,
                        rect.y0 + t.inner.y0 * sy,
                        rect.x0 + t.inner.x1 * sx,
                        rect.y0 + t.inner.y1 * sy,
                    );
                    ctx.set_transform(tr);
                    ctx.set_paint_transform(
                        Affine::translate((r.x0, r.y0)) * Affine::scale_non_uniform(sx, sy),
                    );
                    ctx.set_paint(Image {
                        image: ImageSource::Pixmap(t.pixels.clone()),
                        sampler: peniko::ImageSampler::default().with_quality(if *nearest {
                            peniko::ImageQuality::Low
                        } else {
                            peniko::ImageQuality::Medium
                        }),
                    });
                    ctx.fill_rect(&inner);
                    ctx.reset_paint_transform();
                }
                ctx.set_blend_mode(previous);
            }
        }
        if let Some(tiles) = &effect_tiles {
            for t in tiles.iter() {
                if t.inner {
                    self.image(ctx, t.pixels.clone(), t.rect, tr, false);
                }
            }
        }
        ctx.pop_layer();
        Ok(())
    }
    fn cached_effects(&mut self, n: &DrawNode, key: (usize, u64)) -> Result<Arc<Vec<EffectTile>>> {
        if let Some((tiles, costs)) = self.frame_effects.get(&key) {
            if let Some((_, current)) = self.effect_costs.last_mut() {
                current.extend(costs);
            }
            return Ok(tiles.clone());
        }
        self.effects.tick += 1;
        if let Some(entry) = self.effects.entries.get_mut(&key) {
            entry.used = self.effects.tick;
            for (&cost_key, &pixels) in &entry.costs {
                if self.used_effects.insert(cost_key) {
                    self.effect_pixels = self
                        .effect_pixels
                        .checked_add(pixels)
                        .ok_or_else(|| error("效果像素预算溢出"))?;
                    if self.effect_pixels > MAX_PIXELS {
                        return Err(error("效果像素数超过 100000000 上限"));
                    }
                }
            }
            if let Some((_, current)) = self.effect_costs.last_mut() {
                current.extend(&entry.costs);
            }
            let tiles = entry.tiles.clone();
            self.frame_effects
                .insert(key, (tiles.clone(), entry.costs.clone()));
            return Ok(tiles);
        }
        self.used_effects.insert(key);
        self.effect_costs.push((key, BTreeMap::new()));
        let tiles = Arc::new(self.effect_tiles(n, f64::from_bits(key.1).max(1e-9))?);
        let (_, costs) = self.effect_costs.pop().unwrap();
        if let Some((_, current)) = self.effect_costs.last_mut() {
            current.extend(&costs);
        }
        // A frame must still reuse large effects when the retention budget is
        // zero or too small, preserving the export budget and tiled behavior.
        self.frame_effects
            .insert(key, (tiles.clone(), costs.clone()));
        let pixels = tiles
            .iter()
            .map(|t| t.pixels.width() as u64 * t.pixels.height() as u64)
            .sum::<u64>();
        if pixels * 4 <= self.effects.budget {
            self.effects.trim(pixels * 4);
            self.effects.tick += 1;
            self.effects.entries.insert(
                key,
                CachedEffects {
                    tiles: tiles.clone(),
                    pixels,
                    costs,
                    used: self.effects.tick,
                },
            );
            self.effects.bytes += pixels * 4;
        }
        Ok(tiles)
    }
    fn effect_tiles(&mut self, n: &DrawNode, density: f64) -> Result<Vec<EffectTile>> {
        // Draw the original children, suppressing only this root's effects,
        // opacity and transform. A temporary cloned tree would make persistent
        // node-address cache keys invalid after this call.
        let bounds = n.local_bounds();
        let mut out = Vec::new();
        for e in &n.effects {
            if ![e.blur, e.spread, e.offset[0], e.offset[1]]
                .iter()
                .all(|v| v.is_finite())
                || e.blur < 0.
                || e.spread < 0.
            {
                return Err(error("无效效果参数"));
            }
            let pad = 4. * e.blur + e.spread;
            let b = if e.inner {
                bounds
            } else {
                bounds.inflate(pad, pad) + kurbo::Vec2::new(e.offset[0], e.offset[1])
            };
            let ww = (b.width() * density).ceil().max(1.) as u32;
            let hh = (b.height() * density).ceil().max(1.) as u32;
            if let Some((key, costs)) = self.effect_costs.last_mut() {
                *costs.entry(*key).or_default() += ww as u64 * hh as u64;
            }
            self.effect_pixels = self
                .effect_pixels
                .checked_add(ww as u64 * hh as u64)
                .ok_or_else(|| error("效果像素预算溢出"))?;
            if self.effect_pixels > MAX_PIXELS {
                return Err(error(
                    "PDF effects exceed the 100000000-pixel budget; reduce DPI or effect size",
                ));
            }
            let overlap =
                ((pad + e.offset[0].abs().max(e.offset[1].abs())) * density).ceil() as u32 + 2;
            if (TILE as u64 + 2 * overlap as u64).pow(2) > MAX_EFFECT_TILE {
                return Err(error(
                    "PDF blur tile exceeds the 64MB buffer budget; reduce blur or DPI",
                ));
            }
            for y in (0..hh).step_by(TILE as usize) {
                for x in (0..ww).step_by(TILE as usize) {
                    let tw = (ww - x).min(TILE);
                    let th = (hh - y).min(TILE);
                    let pw = tw + 2 * overlap;
                    let ph = th + 2 * overlap;
                    let ox = b.x0 + (x as f64 - overlap as f64) / density;
                    let oy = b.y0 + (y as f64 - overlap as f64) / density;
                    let tr = Affine::scale(density) * Affine::translate((-ox, -oy));
                    let pixels = self.tile_content(std::slice::from_ref(n), tr, pw, ph, true)?;
                    let alpha: Vec<u8> = pixels
                        .data()
                        .iter()
                        .map(|p| if e.inner { 255 - p.a } else { p.a })
                        .collect();
                    let mut alpha = image::GrayImage::from_raw(pw, ph, alpha)
                        .ok_or_else(|| error("无法分配效果缓冲区"))?;
                    if e.spread > 0. {
                        alpha = dilate(&alpha, (e.spread * density).ceil() as usize);
                    }
                    if e.blur > 0. {
                        alpha = image::imageops::blur(&alpha, (e.blur * density) as f32);
                    }
                    let dx = e.offset[0] * density;
                    let dy = e.offset[1] * density;
                    let mut rgba = vec![0u8; tw as usize * th as usize * 4];
                    for yy in 0..th {
                        for xx in 0..tw {
                            let px = xx + overlap;
                            let py = yy + overlap;
                            let mut a = sample_alpha(&alpha, px as f64 - dx, py as f64 - dy) / 255.;
                            if e.inner {
                                a *= pixels.data()[(py * pw + px) as usize].a as f64 / 255.;
                            }
                            let offset = ((yy * tw + xx) * 4) as usize;
                            for c in 0..3 {
                                rgba[offset + c] = (e.rgba[c] * 255.).round() as u8;
                            }
                            rgba[offset + 3] = (a * e.rgba[3] * 255.).round().clamp(0., 255.) as u8;
                        }
                    }
                    out.push(EffectTile {
                        rect: Rect::new(
                            b.x0 + x as f64 / density,
                            b.y0 + y as f64 / density,
                            b.x0 + (x + tw) as f64 / density,
                            b.y0 + (y + th) as f64 / density,
                        ),
                        pixels: Arc::new(pixmap(rgba, tw, th)),
                        inner: e.inner,
                    });
                }
            }
        }
        Ok(out)
    }
}
fn is_image_pattern(n: &DrawNode) -> bool {
    match &n.kind {
        DrawKind::Image { .. } => true,
        DrawKind::Group { children, .. } => children.len() == 1 && is_image_pattern(&children[0]),
        _ => false,
    }
}
fn sample_alpha(im: &image::GrayImage, x: f64, y: f64) -> f64 {
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let sample = |x: i64, y: i64| {
        if x >= 0 && y >= 0 && x < im.width() as i64 && y < im.height() as i64 {
            im.get_pixel(x as u32, y as u32).0[0] as f64
        } else {
            0.
        }
    };
    let fx = x - ix as f64;
    let fy = y - iy as f64;
    (sample(ix, iy) * (1. - fx) + sample(ix + 1, iy) * fx) * (1. - fy)
        + (sample(ix, iy + 1) * (1. - fx) + sample(ix + 1, iy + 1) * fx) * fy
}
// Sliding maxima give bounded linear work even for large shadow spread values.
fn dilate(im: &image::GrayImage, radius: usize) -> image::GrayImage {
    use std::collections::VecDeque;
    let (w, h) = (im.width() as usize, im.height() as usize);
    let input = im.as_raw();
    let mut tmp = vec![0u8; input.len()];
    let mut out = vec![0u8; input.len()];
    let pass = |src: &[u8],
                dst: &mut [u8],
                length: usize,
                lines: usize,
                stride: usize,
                line_stride: usize| {
        for line in 0..lines {
            let mut queue = VecDeque::<usize>::new();
            let mut next = 0;
            for i in 0..length {
                while next < length && next <= i.saturating_add(radius) {
                    while queue.back().is_some_and(|&j| {
                        src[line * line_stride + j * stride]
                            <= src[line * line_stride + next * stride]
                    }) {
                        queue.pop_back();
                    }
                    queue.push_back(next);
                    next += 1;
                }
                while queue.front().is_some_and(|&j| j < i.saturating_sub(radius)) {
                    queue.pop_front();
                }
                dst[line * line_stride + i * stride] = src[line * line_stride + queue[0] * stride];
            }
        }
    };
    pass(input, &mut tmp, w, h, 1, w);
    pass(&tmp, &mut out, h, w, w, 1);
    image::GrayImage::from_raw(w as u32, h as u32, out).unwrap()
}
#[cfg(feature = "native")]
pub(crate) fn effect_tiles(
    list: &DisplayList,
    n: &DrawNode,
    density: f64,
) -> Result<Vec<EffectTile>> {
    let images = decode(&list.resources)?;
    Renderer {
        images: &images,
        effects: &mut EffectCache::default(),
        effect_pixels: 0,
        used_effects: BTreeSet::new(),
        frame_effects: BTreeMap::new(),
        effect_costs: Vec::new(),
    }
    .effect_tiles(n, density)
}

#[cfg(test)]
mod retained_tests {
    use super::*;
    use kurbo::Shape;
    fn drawing() -> DisplayList {
        let mut child = DrawNode::new(DrawKind::Path {
            path: Rect::new(8., 8., 50., 35.).to_path(0.0001),
            fill: Paint::Solid {
                rgba: [0.2, 0.5, 0.8, 0.75],
            },
            stroke: None,
            even_odd: false,
        });
        child.effects.push(Effect {
            inner: true,
            blur: 1.2,
            spread: 0.,
            offset: [0.6, 0.8],
            rgba: [0., 0., 0., 0.6],
        });
        let mut group = DrawNode::group(vec![child]);
        group.transform = Affine::translate((4., 2.));
        group.opacity = 0.8;
        group.effects.push(Effect {
            inner: false,
            blur: 2.,
            spread: 0.5,
            offset: [2., 3.],
            rgba: [0.7, 0.1, 0.3, 0.5],
        });
        DisplayList {
            schema_version: DISPLAY_LIST_VERSION,
            width: 80.,
            height: 50.,
            nodes: vec![group],
            resources: BTreeMap::new(),
        }
    }
    #[test]
    fn prepared_frames_preserve_nested_effects_across_pan_scale_move_and_eviction() {
        let list = drawing();
        let mut prepared = PreparedRaster::new(list.clone()).unwrap();
        for (rect, w, h) in [
            (Rect::new(0., 0., 80., 50.), 320, 200),
            (Rect::new(3., 2., 83., 52.), 320, 200),
            (Rect::new(0., 0., 80., 50.), 640, 400),
        ] {
            assert_eq!(
                render_region(&list, rect, w, h).unwrap().rgba,
                prepared.render_region(rect, w, h).unwrap().rgba
            );
        }
        assert!(prepared.effects.bytes > 0);
        assert!(prepared.effects.bytes <= prepared.effects.budget);
        fn pixel_cost(prepared: &PreparedRaster, effects: &mut EffectCache) -> u64 {
            let mut renderer = Renderer {
                images: &prepared.images,
                effects,
                effect_pixels: 0,
                used_effects: BTreeSet::new(),
                frame_effects: BTreeMap::new(),
                effect_costs: Vec::new(),
            };
            renderer
                .tile(&prepared.list.nodes, Affine::scale(4.), 320, 200)
                .unwrap();
            renderer.effect_pixels
        }
        let mut cache = EffectCache::default();
        let cold_cost = pixel_cost(&prepared, &mut cache);
        assert!(cold_cost > 0);
        assert_eq!(cold_cost, pixel_cost(&prepared, &mut cache));
        assert!(cache.entries.values().any(|entry| entry.costs.len() > 1));
        let count = prepared.effects.entries.len();
        let mut moved = Box::new(prepared);
        let rect = Rect::new(0., 0., 80., 50.);
        assert_eq!(
            render_region(&list, rect, 640, 400).unwrap().rgba,
            moved.render_region(rect, 640, 400).unwrap().rgba
        );
        assert_eq!(count, moved.effects.entries.len());
        moved.set_cache_budget(0);
        assert_eq!(moved.effects.bytes, 0);
        // Multiple ordinary output tiles still share effects within a frame
        // when the persistent cache is disabled.
        assert_eq!(
            render_region(&list, rect, 1500, 938).unwrap().rgba,
            moved.render_region(rect, 1500, 938).unwrap().rgba
        );
        assert_eq!(moved.effects.bytes, 0);
    }
}
