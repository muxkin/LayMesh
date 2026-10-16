use super::*;
use geometry::Geometry;
pub(super) struct ImageData {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}
fn relative_rect(tag: &str, r: [f64; 4]) -> String {
    format!(
        "<a:{tag} l=\"{}\" t=\"{}\" r=\"{}\" b=\"{}\"/>",
        (r[0] * 100_000.).round() as i64,
        (r[1] * 100_000.).round() as i64,
        (r[2] * 100_000.).round() as i64,
        (r[3] * 100_000.).round() as i64
    )
}
impl Writer<'_> {
    pub(super) fn image_relationship(&mut self, data: Vec<u8>, extension: &'static str) -> usize {
        if let Some(index) = self
            .media
            .iter()
            .position(|m| m.extension == extension && m.data == data)
        {
            return index + 2;
        }
        self.media.push(Media { data, extension });
        self.media.len() + 1
    }
    pub(super) fn blip_fill(
        &mut self,
        data: ImageData,
        opacity: f64,
        crop: [f64; 4],
        inset: [f64; 4],
    ) -> String {
        let rid = self.image_relationship(data.bytes, data.extension);
        format!(
            "<a:blipFill><a:blip r:embed=\"rId{rid}\"><a:alphaModFix amt=\"{}\"/></a:blip>{}<a:stretch>{}</a:stretch></a:blipFill>",
            (opacity.clamp(0., 1.) * 100_000.).round() as i64,
            relative_rect("srcRect", crop),
            relative_rect("fillRect", inset)
        )
    }
    pub(super) fn emit_shape(&mut self, name: &str, g: Geometry, fill: &str, line: &str) -> String {
        let id = self.id();
        self.bounds.push(g.bounds);
        // LibreOffice ignores fillRect insets on p:pic and stretches contain
        // fills across the geometry. A shape's a:blipFill preserves the same
        // original image and editable outline, and honors those insets.
        if fill.starts_with("<a:blipFill>") && fill.contains(&relative_rect("fillRect", [0.; 4])) {
            let fill = fill.replace("a:blipFill", "p:blipFill");
            format!(
                "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>{fill}<p:spPr>{}{}{line}</p:spPr></p:pic>",
                escape(name),
                g.transform,
                g.xml
            )
        } else {
            format!(
                "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{}{}{fill}{line}</p:spPr></p:sp>",
                escape(name),
                g.transform,
                g.xml
            )
        }
    }

    pub(super) fn image_data(&self, id: &str) -> Result<ImageData> {
        let resource = self
            .list
            .resources
            .get(id)
            .ok_or_else(|| error("PPTX 缺少图片资源"))?;
        let mut bytes = resource.data.as_ref().clone();
        let im = image::load_from_memory(&bytes).map_err(|e| error(e.to_string()))?;
        let extension = if resource.mime == "image/jpeg" {
            "jpeg"
        } else {
            if resource.mime != "image/png" {
                let mut out = Cursor::new(Vec::new());
                im.write_to(&mut out, image::ImageFormat::Png)
                    .map_err(|e| error(e.to_string()))?;
                bytes = out.into_inner();
            }
            "png"
        };
        Ok(ImageData { bytes, extension })
    }
    pub(super) fn fill(
        &mut self,
        p: &Paint,
        opacity: f64,
        local: Rect,
        g: &Geometry,
    ) -> Result<Option<String>> {
        match p {
            Paint::None => Ok(Some("<a:noFill/>".into())),
            Paint::Solid { rgba } => Ok(Some(solid(*rgba, opacity)?)),
            Paint::Linear {
                start,
                end,
                stops,
                transform,
                spread,
            } if *transform == Affine::IDENTITY && spread == "pad" => {
                linear_gradient(*start, *end, stops, opacity, local, g)
            }
            Paint::Pattern {
                width,
                height,
                transform,
                nodes,
            } if nodes.len() == 1 && geometry::orthogonal(g.paint_transform).is_some() => {
                let mut node = &nodes[0];
                let mut tr = *transform;
                let mut alpha = opacity;
                loop {
                    tr *= node.transform;
                    alpha *= node.opacity;
                    if !node.effects.is_empty() {
                        return Ok(None);
                    }
                    match &node.kind {
                        DrawKind::Group {
                            children,
                            clip,
                            mask,
                        } if children.len() == 1 && mask.is_none() => {
                            if let Some(c) = clip {
                                let r = c.path.bounding_box();
                                if !rectangular(&c.path) || r != Rect::new(0., 0., *width, *height)
                                {
                                    return Ok(None);
                                }
                            }
                            node = &children[0];
                        }
                        _ => break,
                    }
                }
                let DrawKind::Image { resource, rect, .. } = &node.kind else {
                    return Ok(None);
                };
                let [a, b, c, d, _, _] = tr.as_coeffs();
                if b.abs() > 1e-10 || c.abs() > 1e-10 || a <= 0. || d <= 0. {
                    return Ok(None);
                }
                let image = tr.transform_rect_bbox(*rect);
                let (crop, inset) = crop_inset(image, local);
                let data = self.image_data(resource)?;
                Ok(Some(self.blip_fill(data, alpha, crop, inset)))
            }
            _ => Ok(None),
        }
    }
    pub(super) fn clipped_image(
        &mut self,
        children: &[DrawNode],
        path: &BezPath,
        t: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<Option<String>> {
        if !rectangular(path) || children.len() != 1 {
            return Ok(None);
        }
        let mut node = &children[0];
        let mut tr = Affine::IDENTITY;
        let mut alpha = opacity;
        loop {
            tr *= node.transform;
            alpha *= node.opacity;
            if !node.effects.is_empty() {
                return Ok(None);
            }
            match &node.kind {
                DrawKind::Group {
                    children,
                    clip: None,
                    mask: None,
                } if children.len() == 1 => node = &children[0],
                _ => break,
            }
        }
        let DrawKind::Image { resource, rect, .. } = &node.kind else {
            return Ok(None);
        };
        let [a, b, c, d, _, _] = tr.as_coeffs();
        if b.abs() > 1e-10 || c.abs() > 1e-10 || a <= 0. || d <= 0. {
            return Ok(None);
        }
        let viewport = path.bounding_box();
        let image = tr.transform_rect_bbox(*rect);
        let (crop, inset) = crop_inset(image, viewport);
        if image.intersect(viewport).area() <= 0. {
            return Ok(Some(String::new()));
        }
        let data = self.image_data(resource)?;
        let fill = self.blip_fill(data, alpha, crop, inset);
        let g = geometry::geometry(path, t, false, Some(&json!({"kind":"rect"})));
        Ok(Some(self.emit_shape(
            name,
            g,
            &fill,
            "<a:ln><a:noFill/></a:ln>",
        )))
    }
}
fn rectangular(path: &BezPath) -> bool {
    let b = path.bounding_box();
    path.elements().len() <= 5
        && path.iter().all(|el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                (p.x == b.x0 || p.x == b.x1) && (p.y == b.y0 || p.y == b.y1)
            }
            PathEl::ClosePath => true,
            _ => false,
        })
}
fn crop_inset(image: Rect, viewport: Rect) -> ([f64; 4], [f64; 4]) {
    let v = image.intersect(viewport);
    (
        [
            (v.x0 - image.x0) / image.width(),
            (v.y0 - image.y0) / image.height(),
            (image.x1 - v.x1) / image.width(),
            (image.y1 - v.y1) / image.height(),
        ],
        [
            (v.x0 - viewport.x0) / viewport.width(),
            (v.y0 - viewport.y0) / viewport.height(),
            (viewport.x1 - v.x1) / viewport.width(),
            (viewport.y1 - v.y1) / viewport.height(),
        ],
    )
}
fn linear_gradient(
    start: [f64; 2],
    end: [f64; 2],
    source: &[display_list::Stop],
    opacity: f64,
    local: Rect,
    g: &Geometry,
) -> Result<Option<String>> {
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    let norm = dx * dx + dy * dy;
    if norm <= 1e-20 || local.width() <= 0. || local.height() <= 0. {
        return Ok(None);
    }
    let (qx, qy, c) = (
        dx / norm,
        dy / norm,
        -(start[0] * dx + start[1] * dy) / norm,
    );
    let mut previous = 0.;
    let stops: Vec<_> = source
        .iter()
        .map(|s| {
            let at = s.offset.clamp(previous, 1.);
            previous = at;
            (at, s.rgba)
        })
        .collect();
    let inv = g.paint_transform.inverse();
    let [a, b, cx, d, e, f] = inv.as_coeffs();
    let (qx, qy, c) = (a * qx + b * qy, cx * qx + d * qy, c + e * qx + f * qy);
    let lo = qx * g.rect.x0
        + qy * g.rect.y0
        + c
        + qx.min(0.) * g.rect.width()
        + qy.min(0.) * g.rect.height();
    let span = qx.abs() * g.rect.width() + qy.abs() * g.rect.height();
    if span <= 1e-12 || !lo.is_finite() || !span.is_finite() {
        return Ok(None);
    }
    if stops.len() < 2 || stops.iter().any(|(_, c)| c[3] != stops[0].1[3]) {
        return Ok(None);
    }
    let sample = |at: f64| -> [f64; 4] {
        if at < stops[0].0 {
            return stops[0].1;
        }
        for pair in stops.windows(2) {
            if at < pair[1].0 {
                let t = ((at - pair[0].0) / (pair[1].0 - pair[0].0)).clamp(0., 1.);
                let lerp = |a: f64, b: f64| a + (b - a) * t;
                return std::array::from_fn(|i| lerp(pair[0].1[i], pair[1].1[i]));
            }
        }
        stops.last().unwrap().1
    };
    let mut mapped = vec![(0., sample(lo))];
    mapped.extend(stops.iter().filter_map(|(at, c)| {
        let p = (at - lo) / span;
        (p >= 0. && p <= 1.).then_some((p, *c))
    }));
    mapped.push((1., sample(lo + span)));
    let mut gs = String::new();
    for (pos, c) in mapped {
        let fill = solid(c, opacity)?;
        let color = fill
            .strip_prefix("<a:solidFill>")
            .unwrap()
            .strip_suffix("</a:solidFill>")
            .unwrap();
        gs += &format!(
            "<a:gs pos=\"{}\">{color}</a:gs>",
            (pos * 100_000.).round() as i64
        );
    }
    let angle = (qy.atan2(qx).to_degrees().rem_euclid(360.) * 60_000.).round() as i64;
    Ok(Some(format!(
        "<a:gradFill rotWithShape=\"1\"><a:gsLst>{gs}</a:gsLst><a:lin ang=\"{angle}\" scaled=\"0\"/><a:tileRect/></a:gradFill>"
    )))
}
