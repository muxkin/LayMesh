//! Headless Skia CPU surfaces; no full-scene copies while sampling animation.
use crate::{
    display_list::*,
    display_update::DisplayUpdate,
    error,
    retained::NodeFrame,
    skia::{self, Painter},
};
use kurbo::{Affine, Rect};
use laymesh_core::Result;
use skia_safe as sk;
use std::{collections::BTreeMap, sync::Arc};
pub const MAX_PIXELS: u64 = 100_000_000;
#[derive(Clone, Debug)]
pub struct RasterFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub(crate) struct EffectTile {
    pub rect: Rect,
    pub pixels: Arc<image::RgbaImage>,
    pub inner: bool,
}
pub struct PreparedRaster {
    list: DisplayList,
    images: BTreeMap<String, sk::Image>,
}
impl PreparedRaster {
    pub fn new(list: DisplayList) -> Result<Self> {
        if list.schema_version != DISPLAY_LIST_VERSION {
            return Err(error("Unsupported drawing version"));
        }
        let images = Painter::new(&list)?.images;
        Ok(Self { list, images })
    }
    pub fn set_media_frame(
        &mut self,
        resource: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.images
            .insert(resource.into(), skia::rgba_image(width, height, rgba)?);
        Ok(())
    }
    pub fn display_list(&self) -> &DisplayList {
        &self.list
    }
    pub fn update(&mut self, update: DisplayUpdate) -> Result<()> {
        update.validate(&self.list)?;
        let mut next = self.list.clone();
        update.apply(&mut next)?;
        let mut images = BTreeMap::new();
        for (id, r) in &next.resources {
            if r.mime.starts_with("image/") {
                let image = if self.list.resources.get(id) == Some(r) {
                    self.images
                        .get(id)
                        .cloned()
                        .map(Ok)
                        .unwrap_or_else(|| skia::decode(r))?
                } else {
                    skia::decode(r)?
                };
                images.insert(id.clone(), image);
            }
        }
        self.images = images;
        self.list = next;
        Ok(())
    }
    pub fn set_cache_budget(&mut self, _mib: usize) {}
    pub fn render_region(&mut self, r: Rect, w: u32, h: u32) -> Result<RasterFrame> {
        self.render_states(r, w, h, &BTreeMap::new())
    }
    pub fn render_retained_region(&mut self, r: Rect, w: u32, h: u32) -> Result<RasterFrame> {
        self.render_region(r, w, h)
    }
    pub fn render_states(
        &self,
        r: Rect,
        w: u32,
        h: u32,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<RasterFrame> {
        validate(r, w, h)?;
        // Skia clips saveLayer inputs to the device. Keep filter support outside
        // a cropped viewport so panning does not change a shadow at its edges.
        fn support(nodes: &[DrawNode], parent: Affine) -> f64 {
            nodes
                .iter()
                .map(|n| {
                    let transform = parent * n.transform;
                    let [a, b, c, d, _, _] = transform.as_coeffs();
                    let scale = (a.abs() + c.abs()).max(b.abs() + d.abs());
                    let own = n
                        .effects
                        .iter()
                        .map(|e| {
                            (4. * e.blur + e.spread + e.offset[0].abs().max(e.offset[1].abs()))
                                * scale
                        })
                        .fold(0., f64::max);
                    own + match &n.kind {
                        DrawKind::Group { children, .. } => support(children, transform),
                        _ => 0.,
                    }
                })
                .fold(0., f64::max)
        }
        let density = (w as f64 / r.width()).max(h as f64 / r.height());
        let pad = (support(&self.list.nodes, Affine::IDENTITY) * density).ceil() as u32;
        let (sw, sh) = (
            w.saturating_add(pad.saturating_mul(2)),
            h.saturating_add(pad.saturating_mul(2)),
        );
        let mut surface = surface(sw, sh)?;
        let c = surface.canvas();
        c.translate((pad as f32, pad as f32));
        c.scale((w as f32 / r.width() as f32, h as f32 / r.height() as f32));
        c.translate((-r.x0 as f32, -r.y0 as f32));
        Painter::with_images(&self.list, self.images.clone()).draw(c, states)?;
        read_at(&mut surface, w, h, pad)
    }
}
pub fn set_image_cache_budget(_mib: usize) {}
fn validate(r: Rect, w: u32, h: u32) -> Result<()> {
    if w == 0 || h == 0 || !r.is_finite() || r.width() <= 0. || r.height() <= 0. {
        return Err(error("Invalid viewport"));
    }
    Ok(())
}
fn surface(w: u32, h: u32) -> Result<sk::Surface> {
    let width =
        i32::try_from(w).map_err(|_| error("Surface width exceeds Skia's integer range"))?;
    let height =
        i32::try_from(h).map_err(|_| error("Surface height exceeds Skia's integer range"))?;
    sk::surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| error("Skia surface allocation failed"))
}
fn read(s: &mut sk::Surface, w: u32, h: u32) -> Result<RasterFrame> {
    read_at(s, w, h, 0)
}
fn read_at(s: &mut sk::Surface, w: u32, h: u32, pad: u32) -> Result<RasterFrame> {
    let mut rgba = vec![0; w as usize * h as usize * 4];
    let info = sk::ImageInfo::new(
        (w as i32, h as i32),
        sk::ColorType::RGBA8888,
        sk::AlphaType::Unpremul,
        sk::ColorSpace::new_srgb(),
    );
    if !s.read_pixels(&info, &mut rgba, w as usize * 4, (pad as i32, pad as i32)) {
        return Err(error("Skia pixel read failed"));
    }
    Ok(RasterFrame {
        width: w,
        height: h,
        rgba,
    })
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
    let (w, h) = dimensions(list.width, list.height, dpi)?;
    render_region(list, Rect::new(0., 0., list.width, list.height), w, h)
}
pub fn render_region(list: &DisplayList, r: Rect, w: u32, h: u32) -> Result<RasterFrame> {
    PreparedRaster::new(list.clone())?.render_region(r, w, h)
}
pub(crate) fn effect_tiles(
    list: &DisplayList,
    node: &DrawNode,
    density: f64,
) -> Result<Vec<EffectTile>> {
    if !density.is_finite() || density <= 0. {
        return Err(error("Invalid effect density"));
    }
    let bounds = node.local_bounds();
    let mut out = vec![];
    let mut total = 0u64;
    let mut painter = Painter::new(list)?;
    for e in &node.effects {
        if ![e.blur, e.spread, e.offset[0], e.offset[1]]
            .iter()
            .all(|v| v.is_finite())
            || e.blur < 0.
        {
            return Err(error("Invalid effect parameters"));
        }
        let pad = 4. * e.blur + e.spread.abs();
        let b = if e.inner {
            bounds
        } else {
            bounds.inflate(pad, pad) + kurbo::Vec2::new(e.offset[0], e.offset[1])
        };
        let ww = (b.width() * density).ceil().max(1.) as u32;
        let hh = (b.height() * density).ceil().max(1.) as u32;
        total += ww as u64 * hh as u64;
        if total > MAX_PIXELS {
            return Err(error(
                "PDF effects exceed the 100000000-pixel budget; reduce DPI or effect size",
            ));
        }
        let overlap =
            ((pad + e.offset[0].abs().max(e.offset[1].abs())) * density).ceil() as u32 + 2;
        if (1024u64 + 2 * overlap as u64).pow(2) > 16_000_000 {
            return Err(error(
                "PDF blur tile exceeds the 64MB buffer budget; reduce blur or DPI",
            ));
        }
        for y in (0..hh).step_by(1024) {
            for x in (0..ww).step_by(1024) {
                let tw = (ww - x).min(1024);
                let th = (hh - y).min(1024);
                let pw = tw + 2 * overlap;
                let ph = th + 2 * overlap;
                let ox = b.x0 + (x as f64 - overlap as f64) / density;
                let oy = b.y0 + (y as f64 - overlap as f64) / density;
                let mut s = surface(pw, ph)?;
                s.canvas().concat(&skia::matrix(
                    Affine::scale(density) * Affine::translate((-ox, -oy)),
                ));
                painter.effect(s.canvas(), node, "effect", &BTreeMap::new(), e)?;
                let pixels = read(&mut s, pw, ph)?;
                let mut rgba = vec![0u8; tw as usize * th as usize * 4];
                for yy in 0..th {
                    let source = (((yy + overlap) * pw + overlap) * 4) as usize;
                    let target = (yy * tw * 4) as usize;
                    rgba[target..target + tw as usize * 4]
                        .copy_from_slice(&pixels.rgba[source..source + tw as usize * 4]);
                }
                out.push(EffectTile {
                    rect: Rect::new(
                        b.x0 + x as f64 / density,
                        b.y0 + y as f64 / density,
                        b.x0 + (x + tw) as f64 / density,
                        b.y0 + (y + th) as f64 / density,
                    ),
                    pixels: Arc::new(image::RgbaImage::from_raw(tw, th, rgba).unwrap()),
                    inner: e.inner,
                });
            }
        }
    }
    Ok(out)
}
