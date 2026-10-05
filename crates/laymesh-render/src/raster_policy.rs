//! Image encoding policy shared by PDF and display-only preview derivatives.
use super::*;
use image::{
    ColorType, DynamicImage, ImageFormat, codecs::jpeg::JpegEncoder, imageops::FilterType,
};
use std::{collections::HashSet, io::Cursor, sync::Arc};

pub fn protected(image: &DynamicImage) -> bool {
    matches!(
        image.color(),
        ColorType::L16
            | ColorType::La16
            | ColorType::Rgb16
            | ColorType::Rgba16
            | ColorType::Rgb32F
            | ColorType::Rgba32F
    ) || has_transparency(image)
}
pub fn line_art(image: &DynamicImage, palette: usize, flatness: f64) -> bool {
    let sample = image
        .resize_exact(
            image.width().min(128),
            image.height().min(128),
            FilterType::Nearest,
        )
        .to_rgb8();
    let mut colors = HashSet::new();
    let mut flat = 0usize;
    let mut pairs = 0usize;
    for (x, y, p) in sample.enumerate_pixels() {
        colors.insert(p.0);
        for (a, b) in [(x.wrapping_sub(1), y), (x, y.wrapping_sub(1))] {
            if a < sample.width() && b < sample.height() {
                pairs += 1;
                if p.0
                    .iter()
                    .zip(sample.get_pixel(a, b).0)
                    .all(|(v, w)| v.abs_diff(w) <= 2)
                {
                    flat += 1;
                }
            }
        }
    }
    colors.len() <= palette || (pairs > 0 && flat as f64 / pairs as f64 >= flatness)
}
pub fn jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut e = JpegEncoder::new_with_quality(&mut bytes, quality);
    if matches!(
        image.color(),
        ColorType::L8 | ColorType::La8 | ColorType::L16 | ColorType::La16
    ) {
        e.encode_image(&image.to_luma8())
    } else {
        e.encode_image(&image.to_rgb8())
    }
    .map_err(|e| error(e.to_string()))?;
    Ok(bytes)
}
pub fn png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|e| error(e.to_string()))?;
    Ok(out.into_inner())
}
pub fn pixels(node: &Json) -> Result<Arc<DynamicImage>> {
    if let Some(image) = node["rasterKey"]
        .as_str()
        .and_then(laymesh_core::asset_cache::pixels)
    {
        return Ok(image);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(jstr(node, "data", ""))
        .map_err(|e| error(e.to_string()))?;
    Ok(Arc::new(
        image::load_from_memory(&bytes).map_err(|e| error(e.to_string()))?,
    ))
}
pub fn scale(node: &Json, transform: kurbo::Affine) -> f64 {
    let iw = jnum(node, "intrinsicWidth", jnum(node, "width", 1.)).max(1.);
    let ih = jnum(node, "intrinsicHeight", jnum(node, "height", 1.)).max(1.);
    let cw = jnum(&node["crop"], "width", iw);
    let ch = jnum(&node["crop"], "height", ih);
    let sx = jnum(node, "width", 1.) / cw;
    let sy = jnum(node, "height", 1.) / ch;
    let (sx, sy) = match jstr(node, "fit", "contain") {
        "stretch" => (sx, sy),
        "cover" => (sx.max(sy), sx.max(sy)),
        _ => (sx.min(sy), sx.min(sy)),
    };
    let [a, b, c, d, _, _] = (transform * kurbo::Affine::scale_non_uniform(sx, sy)).as_coeffs();
    let sum = a * a + b * b + c * c + d * d;
    let determinant = (a * d - b * c).powi(2);
    ((sum + (sum * sum - 4. * determinant).max(0.).sqrt()) / 2.).sqrt()
}
fn encode_pdf(node: &mut Json, transform: kurbo::Affine, options: &ExportOptions) -> Result<()> {
    let original = pixels(node)?;
    let factor = if options.pdf_downsample.unwrap_or(true) {
        (scale(node, transform) * options.dpi.unwrap_or(1200.) / 25.4).min(1.)
    } else {
        1.
    };
    let resized;
    let image = if factor < 0.999999 {
        resized = original.resize(
            ((original.width() as f64 * factor).ceil() as u32).max(1),
            ((original.height() as f64 * factor).ceil() as u32).max(1),
            FilterType::Lanczos3,
        );
        &resized
    } else {
        original.as_ref()
    };
    let prepared = prepare_pdf_pixels(image, options)?;
    let image = &prepared;
    let mode = options.pdf_image_compression.as_deref().unwrap_or("auto");
    let is_protected = protected(image);
    let candidate = if mode == "lossless" || is_protected {
        None
    } else {
        let source = if factor >= 0.999999
            && !options.pdf_recompress_jpeg.unwrap_or(false)
            && !node["crop"].is_object()
        {
            node["rasterKey"].as_str().and_then(|key| {
                node["sourceJpegHash"]
                    .as_str()
                    .and_then(|hash| laymesh_core::asset_cache::matching_jpeg(key, hash))
            })
        } else {
            None
        };
        if let Some(source) = source {
            Some(source.as_ref().clone())
        } else if mode == "jpeg"
            || !line_art(
                image,
                options.pdf_auto_palette_limit.unwrap_or(32),
                options.pdf_auto_flatness_threshold.unwrap_or(0.9),
            )
        {
            Some(jpeg(image, options.pdf_jpeg_quality.unwrap_or(90))?)
        } else {
            None
        }
    };
    let (mime, bytes) = if let Some(bytes) = candidate {
        ("image/jpeg", bytes)
    } else {
        ("image/png", png(image)?)
    };
    node["mime"] = json!(mime);
    node["data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
    let ratio_x = image.width() as f64 / original.width() as f64;
    let ratio_y = image.height() as f64 / original.height() as f64;
    if node["kind"] == "image" {
        node["intrinsicWidth"] = json!(image.width());
        node["intrinsicHeight"] = json!(image.height());
    } else {
        node["width"] = json!(image.width());
        node["height"] = json!(image.height());
    }
    if let Some(crop) = node["crop"].as_object_mut() {
        for (key, ratio) in [
            ("x", ratio_x),
            ("width", ratio_x),
            ("y", ratio_y),
            ("height", ratio_y),
        ] {
            if let Some(value) = crop.get(key).and_then(Json::as_f64) {
                crop.insert(key.into(), json!(value * ratio));
            }
        }
    }
    Ok(())
}
pub fn pdf_scene(scene: &Scene, options: &ExportOptions) -> Result<Scene> {
    fn visit(n: &mut Json, parent: kurbo::Affine, o: &ExportOptions) -> Result<()> {
        let transform = parent * laymesh_core::geometry::node_transform(n);
        if n["kind"] == "image" && n["mime"] != "image/svg+xml" {
            encode_pdf(n, transform, o)?;
        }
        for key in ["fill", "stroke"] {
            if n[key]["mime"].is_string() && n[key]["mime"] != "image/svg+xml" {
                let mut t = n[key].clone();
                t["intrinsicWidth"] = t["width"].clone();
                t["intrinsicHeight"] = t["height"].clone();
                t["width"] = n["width"].clone();
                t["height"] = n["height"].clone();
                encode_pdf(&mut t, transform, o)?;
                n[key] = t;
            }
        }
        let transform = if n["kind"] == "group" {
            transform
                * kurbo::Affine::scale_non_uniform(
                    jnum(n, "width", 1.) / jnum(n, "contentWidth", 1.).max(1e-12),
                    jnum(n, "height", 1.) / jnum(n, "contentHeight", 1.).max(1e-12),
                )
        } else {
            transform
        };
        if let Some(children) = n["children"].as_array_mut() {
            for child in children {
                visit(child, transform, o)?;
            }
        }
        Ok(())
    }
    options.validate("pdf")?;
    let mut effective = options.clone();
    if effective.dpi.is_none() {
        effective.dpi = Some(scene.export_dpi);
    }
    let options = &effective;
    let mut optimized = scene.clone();
    for node in &mut optimized.nodes {
        visit(node, kurbo::Affine::IDENTITY, options)?;
    }
    if optimized.background["mime"].is_string() && optimized.background["mime"] != "image/svg+xml" {
        let mut paint = optimized.background.clone();
        paint["intrinsicWidth"] = paint["width"].clone();
        paint["intrinsicHeight"] = paint["height"].clone();
        paint["width"] = json!(scene.width);
        paint["height"] = json!(scene.height);
        encode_pdf(&mut paint, kurbo::Affine::IDENTITY, options)?;
        optimized.background = paint;
    }
    Ok(optimized)
}
pub fn has_transparency(image: &DynamicImage) -> bool {
    match image {
        DynamicImage::ImageLumaA8(data) => data.pixels().any(|p| p[1] != 255),
        DynamicImage::ImageRgba8(data) => data.pixels().any(|p| p[3] != 255),
        DynamicImage::ImageLumaA16(data) => data.pixels().any(|p| p[1] != 65535),
        DynamicImage::ImageRgba16(data) => data.pixels().any(|p| p[3] != 65535),
        DynamicImage::ImageRgba32F(data) => data.pixels().any(|p| p[3] != 1.0),
        _ => false,
    }
}
pub fn preview_bytes(
    image: &DynamicImage,
    quality: u8,
    webp_quality: u8,
    webp_method: u8,
) -> Result<(String, Vec<u8>)> {
    if has_transparency(image) {
        let rgba = image.to_rgba8();
        let mut config =
            webp::WebPConfig::new().map_err(|_| error("WebP initialization failed"))?;
        config.lossless = 0;
        config.quality = webp_quality as f32;
        config.method = webp_method as i32;
        config.alpha_quality = 100;
        let bytes = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
            .encode_advanced(&config)
            .map_err(|e| error(format!("WebP preview encoding failed: {e:?}")))?;
        return Ok(("image/webp".into(), bytes.to_vec()));
    }
    Ok(("image/jpeg".into(), jpeg(image, quality)?))
}
fn prepare_pdf_pixels(image: &DynamicImage, options: &ExportOptions) -> Result<DynamicImage> {
    let mut image = image.clone();
    if !options.pdf_preserve_16bit.unwrap_or(true)
        && matches!(
            image.color(),
            ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16
        )
    {
        image = if image.color().has_alpha() {
            DynamicImage::ImageRgba8(image.to_rgba8())
        } else if matches!(image.color(), ColorType::L16) {
            DynamicImage::ImageLuma8(image.to_luma8())
        } else {
            DynamicImage::ImageRgb8(image.to_rgb8())
        };
    }
    if !options.pdf_preserve_alpha.unwrap_or(true) && image.color().has_alpha() {
        let digits = options
            .pdf_alpha_background
            .as_deref()
            .unwrap_or("#ffffff")
            .trim_start_matches('#');
        let bg = [0, 2, 4].map(|i| u8::from_str_radix(&digits[i..i + 2], 16).unwrap());
        if matches!(image.color(), ColorType::La16 | ColorType::Rgba16) {
            let rgba = image.to_rgba16();
            let data = rgba
                .pixels()
                .flat_map(|p| {
                    (0..3).map(move |i| {
                        ((p[i] as u64 * p[3] as u64
                            + bg[i] as u64 * 257 * (65535 - p[3]) as u64
                            + 32767)
                            / 65535) as u16
                    })
                })
                .collect();
            image = DynamicImage::ImageRgb16(
                image::ImageBuffer::from_raw(rgba.width(), rgba.height(), data).unwrap(),
            );
        } else {
            let rgba = image.to_rgba8();
            let data = rgba
                .pixels()
                .flat_map(|p| {
                    (0..3).map(move |i| {
                        ((p[i] as u32 * p[3] as u32 + bg[i] as u32 * (255 - p[3]) as u32 + 127)
                            / 255) as u8
                    })
                })
                .collect();
            image = DynamicImage::ImageRgb8(
                image::ImageBuffer::from_raw(rgba.width(), rgba.height(), data).unwrap(),
            );
        }
    }
    Ok(image)
}

/// Explain why explicitly requested JPEG is overridden by preservation settings.
pub fn pdf_warnings(
    scene: &Scene,
    options: &ExportOptions,
) -> Result<Vec<laymesh_core::Diagnostic>> {
    let mut warnings = Vec::new();
    if options.pdf_image_compression.as_deref() != Some("jpeg") {
        return Ok(warnings);
    }
    fn visit(n: &Json, o: &ExportOptions, w: &mut Vec<laymesh_core::Diagnostic>) -> Result<()> {
        if (n["kind"] == "image" || n["kind"] == "imagePaint") && n["mime"] != "image/svg+xml" {
            let image = pixels(n)?;
            let high = matches!(
                image.color(),
                ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16
            );
            let reason = if high && o.pdf_preserve_16bit.unwrap_or(true) {
                Some("16-bit precision is preserved; disable pdf_preserve_16bit to allow JPEG")
            } else if has_transparency(&image) && o.pdf_preserve_alpha.unwrap_or(true) {
                Some(
                    "Transparency is preserved; disable pdf_preserve_alpha to flatten and allow JPEG",
                )
            } else {
                None
            };
            if let Some(reason) = reason {
                w.push(laymesh_core::Diagnostic::new(
                    "W_PDF_LOSSLESS",
                    reason,
                    jstr(n, "source", ""),
                    Default::default(),
                ));
            }
        }
        for key in ["fill", "stroke"] {
            if n[key]["mime"].is_string() {
                let mut p = n[key].clone();
                p["kind"] = json!("imagePaint");
                visit(&p, o, w)?;
            }
        }
        for child in n["children"].as_array().into_iter().flatten() {
            visit(child, o, w)?;
        }
        Ok(())
    }
    for n in &scene.nodes {
        visit(n, options, &mut warnings)?;
    }
    if scene.background["mime"].is_string() {
        let mut p = scene.background.clone();
        p["kind"] = json!("imagePaint");
        visit(&p, options, &mut warnings)?;
    }
    Ok(warnings)
}
