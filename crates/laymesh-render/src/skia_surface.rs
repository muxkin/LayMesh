use super::*;
use skia_safe::QuickReject;
impl Painter<'_> {
    pub(super) fn soft_subject_pdf(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        radius: f64,
    ) -> Result<()> {
        let m = c.local_to_device_as_3x3();
        let density = crate::portable_svg::max_scale(Affine::new([
            m.scale_x() as f64,
            m.skew_y() as f64,
            m.skew_x() as f64,
            m.scale_y() as f64,
            0.,
            0.,
        ]));
        let b = node.local_bounds();
        let w = (b.width() * density).ceil().max(1.) as u32;
        let h = (b.height() * density).ceil().max(1.) as u32;
        self.effect_pixels += w as u64 * h as u64;
        if self.effect_pixels > 100_000_000 {
            return Err(error("PDF local effects exceed the 100000000-pixel budget"));
        }
        let halo = (radius * 2.5 * density).ceil() as u32 + 2;
        if (1024u64 + 2 * halo as u64).pow(2) > 16_777_216 {
            return Err(error("Soft edge halo exceeds the 64 MiB tile budget"));
        }
        let mut local = node.clone();
        local.transform = Affine::IDENTITY;
        local.opacity = 1.;
        local.effects.retain(|e| e.soft_edge.is_some());
        local.semantic = None;
        let mut list = self.list.clone();
        list.nodes = vec![local];
        let mut painter = Painter::new(&list)?;
        for y in (0..h).step_by(1024) {
            for x in (0..w).step_by(1024) {
                let tw = (w - x).min(1024);
                let th = (h - y).min(1024);
                let pw = tw + 2 * halo;
                let ph = th + 2 * halo;
                let mut s = sk::surfaces::raster_n32_premul((pw as i32, ph as i32))
                    .ok_or_else(|| error("Soft edge allocation failed"))?;
                let tc = s.canvas();
                tc.scale((density as f32, density as f32));
                tc.translate((
                    -(b.x0 + (x as f64 - halo as f64) / density) as f32,
                    -(b.y0 + (y as f64 - halo as f64) / density) as f32,
                ));
                painter.content(tc, &list.nodes[0], "soft-edge", &BTreeMap::new())?;
                let info = sk::ImageInfo::new(
                    (tw as i32, th as i32),
                    sk::ColorType::RGBA8888,
                    sk::AlphaType::Unpremul,
                    sk::ColorSpace::new_srgb(),
                );
                let mut rgba = vec![0; tw as usize * th as usize * 4];
                if !s.read_pixels(
                    &info,
                    &mut rgba,
                    tw as usize * 4,
                    (halo as i32, halo as i32),
                ) {
                    return Err(error("Soft edge read failed"));
                }
                let image = rgba_image(tw, th, &rgba)?;
                let r = Rect::new(
                    b.x0 + x as f64 / density,
                    b.y0 + y as f64 / density,
                    b.x0 + (x + tw) as f64 / density,
                    b.y0 + (y + th) as f64 / density,
                );
                c.draw_image_rect_with_sampling_options(
                    image,
                    None,
                    rect(r),
                    sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None),
                    &sk::Paint::default(),
                );
            }
        }
        Ok(())
    }
    pub(super) fn surface3d(
        &mut self,
        c: &sk::Canvas,
        node: &DrawNode,
        id: &str,
        states: &BTreeMap<String, NodeFrame>,
    ) -> Result<()> {
        let m = c.local_to_device_as_3x3();
        let density = crate::portable_svg::max_scale(Affine::new([
            m.scale_x() as f64,
            m.skew_y() as f64,
            m.skew_x() as f64,
            m.scale_y() as f64,
            0.,
            0.,
        ]));
        if density <= 0. || !density.is_finite() {
            return Err(error("Invalid three_d rendering scale"));
        }
        let density = (density * 64.).ceil() / 64.;
        let clip = c
            .local_clip_bounds()
            .map(|r| [r.left as f64, r.top as f64, r.right as f64, r.bottom as f64]);
        let clip_key = clip.map(|r| r.map(|v| (v * density * 1024.).round() as i64));
        let key = format!(
            "{}:{density}:{clip_key:?}",
            serde_json::to_string(node).map_err(|e| error(e.to_string()))?
        );
        if !self.surface_cache.contains_key(&key) {
            let base = crate::surface3d::mesh(node).ok_or_else(|| error("Invalid three_d mesh"))?;
            let mesh = crate::surface3d::refine(&base, density)?;
            let source_density = density * mesh.texture_scale;
            let b = crate::surface3d::rect(mesh.bounds);
            if b.width() <= 0. || b.height() <= 0. {
                return Ok(());
            }
            let x0 = (b.x0 * density).floor();
            let y0 = (b.y0 * density).floor();
            let w = (b.x1 * density).ceil() - x0;
            let h = (b.y1 * density).ceil() - y0;
            if w * h > 100_000_000. {
                return Err(error("three_d exceeds the 100000000-pixel budget"));
            }
            let cache = w * h * 4. <= self.surface_cache_limit as f64;
            let mut tiles = vec![];
            let mut pending = vec![];
            for y in (0..h as u32).step_by(512) {
                for x in (0..w as u32).step_by(512) {
                    pending.push((x, y, (w as u32 - x).min(512), (h as u32 - y).min(512)));
                }
            }
            while let Some((x, y, w, h)) = pending.pop() {
                let r = [
                    (x0 + x as f64) / density,
                    (y0 + y as f64) / density,
                    (x0 + x as f64 + w as f64) / density,
                    (y0 + y as f64 + h as f64) / density,
                ];
                if c.quick_reject(&rect(crate::surface3d::rect(r))) {
                    continue;
                }
                let Some(source) = crate::surface3d::texture_region(&mesh, r, source_density)
                else {
                    continue;
                };
                let source = crate::surface3d::rect(source);
                let tw = (source.width() * source_density).round().max(1.) as u32;
                let th = (source.height() * source_density).round().max(1.) as u32;
                if tw as u64 * th as u64 * 16 + w as u64 * h as u64 * 132 > 64 * 1024 * 1024 {
                    if w > 1 || h > 1 {
                        if w >= h {
                            let a = w / 2;
                            pending.extend([(x, y, a, h), (x + a, y, w - a, h)]);
                        } else {
                            let a = h / 2;
                            pending.extend([(x, y, w, a), (x, y + a, w, h - a)]);
                        }
                        continue;
                    }
                    return Err(error("three_d texture exceeds the 64 MiB working budget"));
                }
                let mut texture = sk::surfaces::raster_n32_premul((tw as i32, th as i32))
                    .ok_or_else(|| error("three_d texture allocation failed"))?;
                let tc = texture.canvas();
                tc.scale((source_density as f32, source_density as f32));
                tc.translate((-source.x0 as f32, -source.y0 as f32));
                let pdf = self.pdf;
                self.pdf = false;
                let result = self.raw_content(tc, node, id, states);
                self.pdf = pdf;
                result?;
                let info = sk::ImageInfo::new(
                    (tw as i32, th as i32),
                    sk::ColorType::RGBA8888,
                    sk::AlphaType::Unpremul,
                    sk::ColorSpace::new_srgb(),
                );
                let mut rgba = vec![0; tw as usize * th as usize * 4];
                if !texture.read_pixels(&info, &mut rgba, tw as usize * 4, (0, 0)) {
                    return Err(error("three_d texture read failed"));
                }
                if let Some(mask) = &mesh.coverage {
                    let tc = texture.canvas();
                    tc.clear(sk::Color::TRANSPARENT);
                    self.raw_content(tc, mask, id, &BTreeMap::new())?;
                    let mut coverage = vec![0; rgba.len()];
                    if !texture.read_pixels(&info, &mut coverage, tw as usize * 4, (0, 0)) {
                        return Err(error("three_d coverage read failed"));
                    }
                    rgba = crate::surface3d::material_texture(
                        &rgba,
                        &coverage,
                        tw,
                        th,
                        mesh.material_alpha,
                    )?;
                }
                let mut tile_mesh = mesh.clone();
                tile_mesh.source = [source.x0, source.y0, source.x1, source.y1];
                let pixels = crate::surface3d::raster(&tile_mesh, &rgba, tw, th, r, w, h)?;
                let tile = (crate::surface3d::rect(r), rgba_image(w, h, &pixels)?);
                if cache {
                    tiles.push(tile);
                } else {
                    c.draw_image_rect_with_sampling_options(
                        &tile.1,
                        None,
                        rect(tile.0),
                        sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None),
                        &sk::Paint::default(),
                    );
                }
            }
            if !cache {
                return Ok(());
            }
            let cache_bytes = self
                .surface_cache
                .values()
                .flatten()
                .map(|(_, i)| i.width() as u64 * i.height() as u64 * 4)
                .sum::<u64>();
            if self.surface_cache.len() >= 8
                || cache_bytes
                    + tiles
                        .iter()
                        .map(|(_, i)| i.width() as u64 * i.height() as u64 * 4)
                        .sum::<u64>()
                    > self.surface_cache_limit
            {
                self.surface_cache.clear();
            }
            self.surface_cache.insert(key.clone(), tiles);
        }
        for (b, image) in &self.surface_cache[&key] {
            c.draw_image_rect_with_sampling_options(
                image,
                None,
                rect(*b),
                sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None),
                &sk::Paint::default(),
            );
        }
        Ok(())
    }
}
