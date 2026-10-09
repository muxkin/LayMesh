use super::*;
use geometry::Geometry;

pub(super) fn definition<'a, 'i>(n: Node<'a, 'i>, url: &str) -> Option<Node<'a, 'i>> {
    let id = url
        .strip_prefix("url(")?
        .strip_suffix(')')?
        .trim()
        .trim_matches(['\'', '"'])
        .strip_prefix('#')?;
    n.document()
        .descendants()
        .find(|d| d.attribute("id") == Some(id))
}
pub(super) struct ImageData {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub width: f64,
    pub height: f64,
}
pub(super) fn image_data(n: Node<'_, '_>) -> Result<Option<ImageData>> {
    let href = n
        .attribute("href")
        .or_else(|| n.attribute(("http://www.w3.org/1999/xlink", "href")))
        .unwrap_or("");
    let (extension, data) = if let Some(data) = href.strip_prefix("data:image/png;base64,") {
        ("png", data)
    } else if let Some(data) = href.strip_prefix("data:image/jpeg;base64,") {
        ("jpeg", data)
    } else {
        return Ok(None);
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| error(format!("PPTX 无效图片：{e}")))?;
    let im = image::load_from_memory(&bytes).map_err(|e| error(format!("PPTX 无效图片：{e}")))?;
    Ok(Some(ImageData {
        bytes,
        extension,
        width: im.width() as f64,
        height: im.height() as f64,
    }))
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
    pub(super) fn fill(
        &mut self,
        n: Node<'_, '_>,
        opacity: f64,
        local: Rect,
        g: &Geometry,
    ) -> Result<Option<String>> {
        let value = n.attribute("fill").unwrap_or("#000000");
        let opacity = opacity * attr(n, "fill-opacity", 1.);
        if !value.starts_with("url(") {
            return Ok(Some(solid(value, opacity)?));
        }
        let Some(def) = definition(n, value) else {
            return Ok(None);
        };
        match def.tag_name().name() {
            "linearGradient" => linear_gradient(def, opacity, local, g),
            "pattern" => {
                if geometry::orthogonal(g.paint_transform).is_none() {
                    return Ok(None);
                }
                if def.attribute("patternTransform").is_some()
                    || def.attribute("patternContentUnits") != Some("objectBoundingBox")
                    || attr(def, "width", 0.) != 1.
                    || attr(def, "height", 0.) != 1.
                {
                    return Ok(None);
                }
                let mut children = def.children().filter(Node::is_element);
                let Some(viewport) = children.next().filter(|n| n.has_tag_name("svg")) else {
                    return Ok(None);
                };
                if children.next().is_some() {
                    return Ok(None);
                }
                let mut images = viewport.children().filter(Node::is_element);
                let Some(image) = images.next().filter(|n| n.has_tag_name("image")) else {
                    return Ok(None);
                };
                if images.next().is_some() {
                    return Ok(None);
                }
                let Some(data) = image_data(image)? else {
                    return Ok(None);
                };
                let mut crop = [0.; 4];
                let mut inset = [0.; 4];
                let aspect = image
                    .attribute("preserveAspectRatio")
                    .or_else(|| viewport.attribute("preserveAspectRatio"))
                    .unwrap_or("xMidYMid meet");
                let ratio = data.width / data.height;
                let target = local.width() / local.height();
                if aspect.contains("slice") {
                    if ratio > target {
                        crop[0] = (1. - target / ratio) / 2.;
                        crop[2] = crop[0];
                    } else {
                        crop[1] = (1. - ratio / target) / 2.;
                        crop[3] = crop[1];
                    }
                } else if aspect != "none" {
                    if ratio > target {
                        inset[1] = (1. - target / ratio) / 2.;
                        inset[3] = inset[1];
                    } else {
                        inset[0] = (1. - ratio / target) / 2.;
                        inset[2] = inset[0];
                    }
                }
                Ok(Some(self.blip_fill(data, opacity, crop, inset)))
            }
            _ => Ok(None),
        }
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
    // SVG's rectangle clip around an image can be expressed by source cropping
    // plus fill insets, without decoding/re-encoding or flattening the picture.
    pub(super) fn clipped_image(
        &mut self,
        n: Node<'_, '_>,
        transform: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<Option<String>> {
        let Some(clip) = n.attribute("clip-path").and_then(|u| definition(n, u)) else {
            return Ok(None);
        };
        let mut shapes = clip.children().filter(Node::is_element);
        let Some(rect) = shapes
            .next()
            .filter(|r| r.has_tag_name("rect") && r.attribute("transform").is_none())
        else {
            return Ok(None);
        };
        if shapes.next().is_some() {
            return Ok(None);
        }
        let mut current = n;
        let mut inner = Affine::IDENTITY;
        let mut alpha = opacity;
        loop {
            let mut children = current.children().filter(Node::is_element);
            let Some(child) = children.next() else {
                return Ok(None);
            };
            if children.next().is_some()
                || child.attribute("filter").is_some()
                || child.attribute("mask").is_some()
                || child.attribute("clip-path").is_some()
            {
                return Ok(None);
            }
            inner *= affine(child)?;
            alpha *= attr(child, "opacity", 1.);
            if child.has_tag_name("image") {
                current = child;
                break;
            }
            if !child.has_tag_name("g") {
                return Ok(None);
            }
            current = child;
        }
        let [a, b, c, d, _, _] = inner.as_coeffs();
        if b.abs() > 1e-10 || c.abs() > 1e-10 || a <= 0. || d <= 0. {
            return Ok(None);
        }
        let Some(data) = image_data(current)? else {
            return Ok(None);
        };
        if current
            .attribute("preserveAspectRatio")
            .unwrap_or("xMidYMid meet")
            != "none"
        {
            return Ok(None);
        }
        let image = inner.transform_rect_bbox(Rect::new(
            attr(current, "x", 0.),
            attr(current, "y", 0.),
            attr(current, "x", 0.) + attr(current, "width", 0.),
            attr(current, "y", 0.) + attr(current, "height", 0.),
        ));
        let path = geometry::svg_path(rect)?;
        let viewport = path.bounding_box();
        let visible = image.intersect(viewport);
        if visible.width() <= 0. || visible.height() <= 0. {
            return Ok(Some(String::new()));
        }
        let crop = [
            (visible.x0 - image.x0) / image.width(),
            (visible.y0 - image.y0) / image.height(),
            (image.x1 - visible.x1) / image.width(),
            (image.y1 - visible.y1) / image.height(),
        ];
        let inset = [
            (visible.x0 - viewport.x0) / viewport.width(),
            (visible.y0 - viewport.y0) / viewport.height(),
            (viewport.x1 - visible.x1) / viewport.width(),
            (viewport.y1 - visible.y1) / viewport.height(),
        ];
        let g = geometry::geometry(rect, &path, transform, false, None);
        let fill = self.blip_fill(data, alpha, crop, inset);
        Ok(Some(self.emit_shape(
            name,
            g,
            &fill,
            "<a:ln><a:noFill/></a:ln>",
        )))
    }
}
fn coordinate(n: Node<'_, '_>, key: &str, default: f64) -> Option<f64> {
    let s = n.attribute(key);
    match s {
        None => Some(default),
        Some(s) => s
            .strip_suffix('%')
            .map(|v| v.parse::<f64>().ok().map(|v| v / 100.))
            .unwrap_or_else(|| s.parse::<f64>().ok()),
    }
}
fn linear_gradient(
    n: Node<'_, '_>,
    opacity: f64,
    local: Rect,
    g: &Geometry,
) -> Result<Option<String>> {
    if n.attribute("gradientTransform").is_some()
        || n.attribute("spreadMethod").is_some_and(|s| s != "pad")
    {
        return Ok(None);
    }
    let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
        coordinate(n, "x1", 0.),
        coordinate(n, "y1", 0.),
        coordinate(n, "x2", 1.),
        coordinate(n, "y2", 0.),
    ) else {
        return Ok(None);
    };
    let (dx, dy) = (x2 - x1, y2 - y1);
    let norm = dx * dx + dy * dy;
    if norm <= 1e-20 || local.width() <= 0. || local.height() <= 0. {
        return Ok(None);
    }
    let (qx, qy, c) = if n.attribute("gradientUnits") == Some("userSpaceOnUse") {
        (dx / norm, dy / norm, -(x1 * dx + y1 * dy) / norm)
    } else {
        let qx = dx / norm / local.width();
        let qy = dy / norm / local.height();
        (
            qx,
            qy,
            -qx * local.x0 - qy * local.y0 - (x1 * dx + y1 * dy) / norm,
        )
    };
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
    let mut stops = Vec::<(f64, svgtypes::Color)>::new();
    let mut previous = 0.;
    for s in n.children().filter(|s| s.has_tag_name("stop")) {
        let Some(at) = coordinate(s, "offset", 0.) else {
            return Ok(None);
        };
        let at = at.clamp(previous, 1.);
        previous = at;
        let mut color: svgtypes::Color = s
            .attribute("stop-color")
            .unwrap_or("#000000")
            .parse()
            .map_err(|_| error("PPTX 无效渐变颜色"))?;
        color.alpha =
            (color.alpha as f64 * attr(s, "stop-opacity", 1.).clamp(0., 1.)).round() as u8;
        stops.push((at, color));
    }
    if stops.len() < 2 || stops.iter().any(|(_, c)| c.alpha != stops[0].1.alpha) {
        return Ok(None);
    }
    let sample = |at: f64| -> svgtypes::Color {
        if at < stops[0].0 {
            return stops[0].1;
        }
        for pair in stops.windows(2) {
            if at < pair[1].0 {
                let t = ((at - pair[0].0) / (pair[1].0 - pair[0].0)).clamp(0., 1.);
                let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
                return svgtypes::Color {
                    red: lerp(pair[0].1.red, pair[1].1.red),
                    green: lerp(pair[0].1.green, pair[1].1.green),
                    blue: lerp(pair[0].1.blue, pair[1].1.blue),
                    alpha: pair[0].1.alpha,
                };
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
        let fill = solid(
            &format!("#{:02x}{:02x}{:02x}{:02x}", c.red, c.green, c.blue, c.alpha),
            opacity,
        )?;
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
