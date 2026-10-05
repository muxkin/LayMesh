//! Strict local image decoding and the documented SVG subset. This module never fetches URLs.
use crate::{Diagnostic, Loc, Result, model::escape};
use base64::Engine;
use image::{ImageDecoder, ImageFormat};
use serde_json::{Value as Json, json};
use std::io::Cursor;

fn error(code: &str, message: impl Into<String>, file: &str, loc: Loc) -> Diagnostic {
    Diagnostic::new(code, message, file, loc)
}
fn allowed_value(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if ["javascript:", "data:", "http:", "https:", "file:"]
        .iter()
        .any(|p| lower.contains(p))
    {
        return false;
    }
    let re = regex::Regex::new(r"(?i)url\(([^)]*)\)").unwrap();
    let fragment = regex::Regex::new(r"^#[A-Za-z_][\w.-]*$").unwrap();
    re.captures_iter(s)
        .all(|m| fragment.is_match(m[1].trim().trim_matches(['\'', '"'])))
}
pub fn sanitize_svg(xml: &str, file: &str, loc: Loc) -> Result<String> {
    let lower = xml.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") || lower.contains("<?") {
        return Err(error("E_SVG", "SVG 不允许 DTD、实体或处理指令", file, loc));
    }
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| error("E_SVG", format!("无效 SVG：{e}"), file, loc))?;
    if doc.root_element().tag_name().name() != "svg" {
        return Err(error("E_SVG", "SVG 缺少 svg 根元素", file, loc));
    }
    let elements = [
        "svg",
        "g",
        "defs",
        "path",
        "rect",
        "circle",
        "ellipse",
        "line",
        "polyline",
        "polygon",
        "clipPath",
        "mask",
        "linearGradient",
        "radialGradient",
        "stop",
        "text",
        "tspan",
        "use",
        "title",
        "desc",
        "metadata",
    ];
    let attrs = [
        "id",
        "viewBox",
        "preserveAspectRatio",
        "version",
        "width",
        "height",
        "x",
        "y",
        "x1",
        "y1",
        "x2",
        "y2",
        "cx",
        "cy",
        "r",
        "rx",
        "ry",
        "d",
        "points",
        "transform",
        "fill",
        "fill-rule",
        "fill-opacity",
        "stroke",
        "stroke-width",
        "stroke-opacity",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-dasharray",
        "stroke-dashoffset",
        "opacity",
        "font-family",
        "font-size",
        "font-weight",
        "font-style",
        "text-anchor",
        "letter-spacing",
        "word-spacing",
        "dx",
        "dy",
        "clip-path",
        "mask",
        "gradientUnits",
        "gradientTransform",
        "offset",
        "stop-color",
        "stop-opacity",
        "href",
        "style",
        "space",
    ];
    let styles = [
        "fill",
        "fill-rule",
        "fill-opacity",
        "stroke",
        "stroke-width",
        "stroke-opacity",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-dasharray",
        "stroke-dashoffset",
        "opacity",
        "font-family",
        "font-size",
        "font-weight",
        "font-style",
        "text-anchor",
        "letter-spacing",
        "word-spacing",
        "clip-path",
    ];
    fn visit(
        node: roxmltree::Node,
        elems: &[&str],
        attrs: &[&str],
        styles: &[&str],
        file: &str,
        loc: Loc,
        out: &mut String,
    ) -> Result<()> {
        if node.is_comment() {
            return Ok(());
        }
        if node.is_text() {
            out.push_str(&escape(node.text().unwrap_or("")));
            return Ok(());
        }
        if !node.is_element() {
            return Err(error("E_SVG", "SVG 包含不允许的节点", file, loc));
        }
        let name = node.tag_name().name();
        if !elems.contains(&name) {
            return Err(error(
                "E_SVG",
                format!("SVG 元素 {name} 不受支持"),
                file,
                loc,
            ));
        }
        out.push('<');
        out.push_str(name);
        if node.parent().is_some_and(|p| p.is_root()) {
            out.push_str(" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"");
        }
        for a in node.attributes() {
            if !attrs.contains(&a.name()) || !allowed_value(a.value()) {
                return Err(error(
                    "E_SVG",
                    format!("SVG 属性 {} 不安全或不受支持", a.name()),
                    file,
                    loc,
                ));
            }
            if a.name() == "href"
                && !regex::Regex::new(r"^#[A-Za-z_][\w.-]*$")
                    .unwrap()
                    .is_match(a.value())
            {
                return Err(error("E_SVG", "SVG 仅允许内部片段引用", file, loc));
            }
            if a.name() == "style" {
                for item in a.value().split(';').filter(|v| !v.trim().is_empty()) {
                    let Some((name, _)) = item.split_once(':') else {
                        return Err(error("E_SVG", "SVG 样式语法错误", file, loc));
                    };
                    if !styles.contains(&name.trim()) {
                        return Err(error(
                            "E_SVG",
                            format!("SVG 样式 {name} 不受支持"),
                            file,
                            loc,
                        ));
                    }
                }
            }
            out.push(' ');
            if a.namespace() == Some("http://www.w3.org/1999/xlink") {
                out.push_str("xlink:");
            } else if a.namespace() == Some("http://www.w3.org/XML/1998/namespace") {
                out.push_str("xml:");
            } else if a.namespace().is_some() {
                return Err(error("E_SVG", "SVG 属性命名空间不受支持", file, loc));
            }
            out.push_str(a.name());
            out.push_str("=\"");
            out.push_str(&escape(a.value()));
            out.push('"');
        }
        out.push('>');
        for child in node.children() {
            visit(child, elems, attrs, styles, file, loc, out)?;
        }
        out.push_str("</");
        out.push_str(name);
        out.push('>');
        Ok(())
    }
    let mut out = String::new();
    visit(
        doc.root_element(),
        &elements,
        &attrs,
        &styles,
        file,
        loc,
        &mut out,
    )?;
    Ok(out)
}
fn svg_length(s: &str) -> Option<f64> {
    let n: svgtypes::Length = s.parse().ok()?;
    Some(
        n.number
            * match n.unit {
                svgtypes::LengthUnit::None | svgtypes::LengthUnit::Px => 1.,
                svgtypes::LengthUnit::Mm => 96. / 25.4,
                svgtypes::LengthUnit::Cm => 96. / 2.54,
                svgtypes::LengthUnit::In => 96.,
                svgtypes::LengthUnit::Pt => 96. / 72.,
                svgtypes::LengthUnit::Pc => 16.,
                _ => return None,
            },
    )
}
fn tiff_page_count(bytes: &[u8]) -> Option<usize> {
    let le = bytes.get(..2)? == b"II";
    let u16_at = |p: usize| {
        let s = bytes.get(p..p + 2)?;
        Some(if le {
            u16::from_le_bytes([s[0], s[1]])
        } else {
            u16::from_be_bytes([s[0], s[1]])
        })
    };
    let u32_at = |p: usize| {
        let s = bytes.get(p..p + 4)?;
        Some(if le {
            u32::from_le_bytes([s[0], s[1], s[2], s[3]])
        } else {
            u32::from_be_bytes([s[0], s[1], s[2], s[3]])
        })
    };
    if u16_at(2)? != 42 {
        return None;
    }
    let mut p = u32_at(4)? as usize;
    let mut count = 0;
    let mut seen = std::collections::BTreeSet::new();
    while p != 0 {
        if !seen.insert(p) {
            return None;
        }
        count += 1;
        if count > 1 {
            return Some(count);
        }
        let n = u16_at(p)? as usize;
        p = u32_at(p + 2 + n * 12)? as usize;
    }
    Some(count)
}
// tiff 0.11 exposes gray+alpha as Multiband, which image's TIFF adapter rejects.
// Decode those two channels directly, retaining the same pixel/allocation limits.
fn gray_alpha_tiff(
    bytes: &[u8],
    mut decoder: tiff::decoder::Decoder<Cursor<&[u8]>>,
    white_is_zero: bool,
    associated_alpha: bool,
    file: &str,
    loc: Loc,
) -> Result<Json> {
    use tiff::tags::Tag;
    let fail = |e| error("E_TIFF", format!("TIFF 无法读取：{e}"), file, loc);
    let (width, height) = decoder.dimensions().map_err(fail)?;
    if u64::from(width) * u64::from(height) > 100_000_000 {
        return Err(error("E_ASSET", "图片像素数超过上限", file, loc));
    }
    let orientation = decoder
        .find_tag(Tag::Orientation)
        .map_err(fail)?
        .and_then(|v| image::metadata::Orientation::from_exif(v.into_u16().ok()?.min(255) as u8))
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let profile = decoder
        .find_tag(Tag::IccProfile)
        .map_err(fail)?
        .map(|v| v.into_u8_vec().map_err(fail))
        .transpose()?;
    // The TIFF decoder cannot invert Multiband. Normalize only the photometric
    // tag in our private copy, then invert gray samples (never alpha) ourselves.
    let mut normalized = vec![];
    if white_is_zero {
        normalized.extend_from_slice(bytes);
        let le = &bytes[..2] == b"II";
        let read16 = |p| {
            if le {
                u16::from_le_bytes([bytes[p], bytes[p + 1]])
            } else {
                u16::from_be_bytes([bytes[p], bytes[p + 1]])
            }
        };
        let offset = if le {
            u32::from_le_bytes(bytes[4..8].try_into().unwrap())
        } else {
            u32::from_be_bytes(bytes[4..8].try_into().unwrap())
        } as usize;
        for p in (offset + 2..offset + 2 + usize::from(read16(offset)) * 12).step_by(12) {
            if read16(p) == 262 {
                if read16(p + 2) != 3
                    || &bytes[p + 4..p + 8]
                        != &if le {
                            1u32.to_le_bytes()
                        } else {
                            1u32.to_be_bytes()
                        }
                {
                    return Err(error("E_TIFF", "TIFF 灰度类型标签无效", file, loc));
                }
                normalized[p + 8..p + 10].copy_from_slice(&if le {
                    1u16.to_le_bytes()
                } else {
                    1u16.to_be_bytes()
                });
                break;
            }
        }
    }
    let mut decoder = if white_is_zero {
        tiff::decoder::Decoder::new(Cursor::new(normalized.as_slice())).map_err(fail)?
    } else {
        decoder
    };
    let mut buffer = tiff::decoder::DecodingResult::U8(vec![]);
    let layout = decoder.read_image_to_buffer(&mut buffer).map_err(fail)?;
    let sample_bytes = match &buffer {
        tiff::decoder::DecodingResult::U8(_) => 1,
        tiff::decoder::DecodingResult::U16(_) => 2,
        _ => return Err(error("E_TIFF", "TIFF 仅支持无符号 8/16 位样本", file, loc)),
    };
    let raw = buffer.as_buffer(0);
    let raw = raw.as_bytes();
    if raw.len() < layout.complete_len {
        return Err(error("E_ASSET", "图片解码超过内存上限", file, loc));
    }
    let mut interleaved = vec![];
    let raw = if layout.planes > 1 {
        let stride = layout.plane_stride.unwrap().get();
        interleaved.reserve(raw.len());
        for i in (0..stride).step_by(sample_bytes) {
            interleaved.extend_from_slice(&raw[i..i + sample_bytes]);
            interleaved.extend_from_slice(&raw[stride + i..stride + i + sample_bytes]);
        }
        interleaved.as_slice()
    } else {
        raw
    };
    let mut image = match sample_bytes {
        1 => {
            let mut pixels = raw.to_vec();
            if white_is_zero {
                for p in pixels.chunks_exact_mut(2) {
                    p[0] = 255 - p[0];
                }
            }
            image::DynamicImage::ImageLumaA8(
                image::ImageBuffer::from_raw(width, height, pixels)
                    .ok_or_else(|| error("E_TIFF", "TIFF 灰度 Alpha 数据长度错误", file, loc))?,
            )
        }
        2 => {
            let mut pixels = raw
                .chunks_exact(2)
                .map(|v| u16::from_ne_bytes([v[0], v[1]]))
                .collect::<Vec<_>>();
            if white_is_zero {
                for p in pixels.chunks_exact_mut(2) {
                    p[0] = 65535 - p[0];
                }
            }
            image::DynamicImage::ImageLumaA16(
                image::ImageBuffer::from_raw(width, height, pixels)
                    .ok_or_else(|| error("E_TIFF", "TIFF 灰度 Alpha 数据长度错误", file, loc))?,
            )
        }
        _ => return Err(error("E_TIFF", "TIFF 仅支持无符号 8/16 位样本", file, loc)),
    };
    if associated_alpha {
        straighten_alpha(&mut image);
    }
    if let Some(profile) = profile {
        let profile = moxcms::ColorProfile::new_from_slice(&profile)
            .map_err(|e| error("E_ASSET", format!("ICC 无效：{e}"), file, loc))?;
        image = convert_icc(image, &profile, file, loc)?;
    }
    image.apply_orientation(orientation);
    raster_asset(image, file, loc)
}
fn raster_asset(image: image::DynamicImage, file: &str, loc: Loc) -> Result<Json> {
    let key = crate::asset_cache::insert(image);
    crate::asset_cache::asset(&key).map_err(|e| error("E_ASSET", e.message, file, loc))
}
fn convert_icc(
    image: image::DynamicImage,
    profile: &moxcms::ColorProfile,
    file: &str,
    loc: Loc,
) -> Result<image::DynamicImage> {
    use moxcms::{DataColorSpace, Layout};
    let (width, height) = (image.width(), image.height());
    if matches!(
        image.color(),
        image::ColorType::L16
            | image::ColorType::La16
            | image::ColorType::Rgb16
            | image::ColorType::Rgba16
    ) {
        let (layout, pixels) = match profile.color_space {
            DataColorSpace::Rgb => (Layout::Rgba, image.into_rgba16().into_raw()),
            DataColorSpace::Gray => (Layout::GrayAlpha, image.into_luma_alpha16().into_raw()),
            _ => return Err(error("E_ASSET", "ICC 色彩空间与解码像素不匹配", file, loc)),
        };
        let transform = profile
            .create_transform_16bit(
                layout,
                &moxcms::ColorProfile::new_srgb(),
                Layout::Rgba,
                moxcms::TransformOptions::default(),
            )
            .map_err(|e| error("E_ASSET", format!("ICC 转换无法创建：{e}"), file, loc))?;
        let mut converted = vec![0u16; (width as usize) * (height as usize) * 4];
        transform
            .transform(&pixels, &mut converted)
            .map_err(|e| error("E_ASSET", format!("ICC 转换失败：{e}"), file, loc))?;
        return Ok(image::DynamicImage::ImageRgba16(
            image::ImageBuffer::from_raw(width, height, converted).unwrap(),
        ));
    }
    let (layout, pixels) = match profile.color_space {
        DataColorSpace::Rgb => (Layout::Rgba, image.into_rgba8().into_raw()),
        DataColorSpace::Gray => (Layout::GrayAlpha, image.into_luma_alpha8().into_raw()),
        _ => return Err(error("E_ASSET", "ICC 色彩空间与解码像素不匹配", file, loc)),
    };
    let converted = transform_icc(&pixels, layout, Layout::Rgba, profile, file, loc)?;
    Ok(image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(width, height, converted).unwrap(),
    ))
}
/// TIFF associated alpha is stored premultiplied; PNG and ICC expect straight samples.
fn straighten_alpha(image: &mut image::DynamicImage) {
    fn straighten<T: Copy + Into<u64> + TryFrom<u64>>(pixels: &mut [T], channels: usize, max: u64) {
        for pixel in pixels.chunks_exact_mut(channels) {
            let alpha = pixel[channels - 1].into();
            for value in &mut pixel[..channels - 1] {
                let result = if alpha == 0 {
                    0
                } else {
                    ((*value).into() * max + alpha / 2) / alpha
                };
                *value = T::try_from(result.min(max)).ok().unwrap();
            }
        }
    }
    match image {
        image::DynamicImage::ImageLumaA8(p) => straighten(p.as_mut(), 2, 255),
        image::DynamicImage::ImageRgba8(p) => straighten(p.as_mut(), 4, 255),
        image::DynamicImage::ImageLumaA16(p) => straighten(p.as_mut(), 2, 65535),
        image::DynamicImage::ImageRgba16(p) => straighten(p.as_mut(), 4, 65535),
        _ => {}
    }
}
fn transform_icc(
    pixels: &[u8],
    input: moxcms::Layout,
    output: moxcms::Layout,
    profile: &moxcms::ColorProfile,
    file: &str,
    loc: Loc,
) -> Result<Vec<u8>> {
    let transform = profile
        .create_transform_8bit(
            input,
            &moxcms::ColorProfile::new_srgb(),
            output,
            moxcms::TransformOptions::default(),
        )
        .map_err(|e| error("E_ASSET", format!("ICC 转换无法创建：{e}"), file, loc))?;
    let mut result = vec![0; pixels.len() / input.channels() * output.channels()];
    transform
        .transform(pixels, &mut result)
        .map_err(|e| error("E_ASSET", format!("ICC 转换失败：{e}"), file, loc))?;
    Ok(result)
}
// image's JPEG decoder converts CMYK to RGB before exposing the pixels. Keep
// the four ink channels here so the embedded CMYK profile receives its actual
// input. Adobe JPEG stores inverted ink values; YCCK stores YCbCr(CMY) and
// inverted K. Decode without color conversion, then normalize those samples.
fn cmyk_jpeg(
    bytes: &[u8],
    profile: &moxcms::ColorProfile,
    file: &str,
    loc: Loc,
) -> Result<image::DynamicImage> {
    use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        ZCursor::new(bytes),
        DecoderOptions::default().set_strict_mode(false),
    );
    decoder
        .decode_headers()
        .map_err(|e| error("E_ASSET", e.to_string(), file, loc))?;
    let space = decoder.input_colorspace().unwrap();
    if !matches!(space, ColorSpace::CMYK | ColorSpace::YCCK) {
        return Err(error(
            "E_ASSET",
            "CMYK ICC 需要 CMYK 或 YCCK JPEG 像素",
            file,
            loc,
        ));
    }
    let (width, height) = decoder.dimensions().unwrap();
    decoder.set_options(decoder.options().jpeg_set_out_colorspace(space));
    let mut samples = decoder
        .decode()
        .map_err(|e| error("E_ASSET", e.to_string(), file, loc))?;
    for pixel in samples.chunks_exact_mut(4) {
        if space == ColorSpace::YCCK {
            let (y, cb, cr) = (
                f64::from(pixel[0]),
                f64::from(pixel[1]) - 128.,
                f64::from(pixel[2]) - 128.,
            );
            pixel[0] = (y + 1.402 * cr).round().clamp(0., 255.) as u8;
            pixel[1] = (y - 0.344136 * cb - 0.714136 * cr).round().clamp(0., 255.) as u8;
            pixel[2] = (y + 1.772 * cb).round().clamp(0., 255.) as u8;
        } else {
            for value in &mut pixel[..3] {
                *value = 255 - *value;
            }
        }
        pixel[3] = 255 - pixel[3];
    }
    let rgb = transform_icc(
        &samples,
        moxcms::Layout::Rgba,
        moxcms::Layout::Rgb,
        profile,
        file,
        loc,
    )?;
    Ok(image::DynamicImage::ImageRgb8(
        image::RgbImage::from_raw(width as u32, height as u32, rgb).unwrap(),
    ))
}
pub fn load(bytes: &[u8], path: &str, file: &str, loc: Loc) -> Result<Json> {
    let key = crate::asset_cache::input_key(bytes, path);
    if let Some(asset) = crate::asset_cache::get_input(&key) { return Ok(asset); }
    let asset = load_uncached(bytes, path, file, loc)?;
    crate::asset_cache::put_input(key, &asset);
    Ok(asset)
}
fn load_uncached(bytes: &[u8], path: &str, file: &str, loc: Loc) -> Result<Json> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if ext == "svg" {
        let xml = std::str::from_utf8(bytes)
            .map_err(|_| error("E_SVG", "SVG 不是有效 UTF-8", file, loc))?;
        let safe = sanitize_svg(xml, file, loc)?;
        let doc = roxmltree::Document::parse(&safe).unwrap();
        let root = doc.root_element();
        let vb = root
            .attribute("viewBox")
            .and_then(|s| s.parse::<svgtypes::ViewBox>().ok());
        let width = root
            .attribute("width")
            .and_then(svg_length)
            .or(vb.map(|v| v.w));
        let height = root
            .attribute("height")
            .and_then(svg_length)
            .or(vb.map(|v| v.h));
        let (w, h) = match (width, height) {
            (Some(w), Some(h)) if w.is_finite() && h.is_finite() && w > 0. && h > 0. => (w, h),
            _ => {
                return Err(error(
                    "E_SVG",
                    format!("SVG 尺寸无法读取：{path}"),
                    file,
                    loc,
                ));
            }
        };
        return Ok(
            json!({"mime":"image/svg+xml","data":base64::engine::general_purpose::STANDARD.encode(safe.as_bytes()),"width":w,"height":h}),
        );
    }
    let format = match ext.as_str() {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "tif" | "tiff" => ImageFormat::Tiff,
        "bmp" => ImageFormat::Bmp,
        "webp" => ImageFormat::WebP,
        "gif" => ImageFormat::Gif,
        "ico" => ImageFormat::Ico,
        "pnm" | "pbm" | "pgm" | "ppm" | "pam" => ImageFormat::Pnm,
        "tga" => ImageFormat::Tga,
        _ => {
            return Err(error(
                "E_ASSET",
                format!("图片格式不受支持：{ext}"),
                file,
                loc,
            ));
        }
    };
    let mut associated_alpha = false;
    let mut decoder: Box<dyn ImageDecoder + '_> = if format == ImageFormat::Tiff {
        if tiff_page_count(bytes) != Some(1) {
            return Err(error("E_TIFF", "仅支持单页 TIFF", file, loc));
        }
        let mut metadata = tiff::decoder::Decoder::new(Cursor::new(bytes))
            .map_err(|e| error("E_TIFF", format!("TIFF 无法读取：{e}"), file, loc))?;
        let samples = metadata
            .find_tag_unsigned_vec::<u16>(tiff::tags::Tag::SampleFormat)
            .map_err(|e| error("E_TIFF", format!("TIFF 样本类型无法读取：{e}"), file, loc))?;
        if samples.is_some_and(|samples| samples.iter().any(|&sample| sample != 1)) {
            return Err(error("E_TIFF", "TIFF 仅支持无符号 8/16 位样本", file, loc));
        }
        let alpha = metadata
            .find_tag_unsigned_vec::<u16>(tiff::tags::Tag::ExtraSamples)
            .map_err(|e| error("E_TIFF", format!("TIFF Alpha 无法读取：{e}"), file, loc))?;
        associated_alpha = alpha
            .as_ref()
            .is_some_and(|samples| samples.first() == Some(&1));
        let color = metadata
            .colortype()
            .map_err(|e| error("E_TIFF", format!("TIFF 色彩类型无法读取：{e}"), file, loc))?;
        if matches!(
            color,
            tiff::ColorType::Multiband {
                bit_depth: 8 | 16,
                num_samples: 2
            }
        ) && alpha
            .as_ref()
            .is_some_and(|samples| matches!(samples.as_slice(), [1] | [2]))
        {
            let photometric = metadata
                .get_tag_unsigned::<u16>(tiff::tags::Tag::PhotometricInterpretation)
                .map_err(|e| error("E_TIFF", format!("TIFF 灰度类型无法读取：{e}"), file, loc))?;
            if matches!(photometric, 0 | 1) {
                return gray_alpha_tiff(
                    bytes,
                    metadata,
                    photometric == 0,
                    associated_alpha,
                    file,
                    loc,
                );
            }
        }
        if !matches!(
            color,
            tiff::ColorType::Gray(8 | 16)
                | tiff::ColorType::RGB(8 | 16)
                | tiff::ColorType::RGBA(8 | 16)
        ) {
            return Err(error(
                "E_TIFF",
                "TIFF 仅支持 8/16 位灰度或 RGB，可带 Alpha",
                file,
                loc,
            ));
        }
        let decoder = image::codecs::tiff::TiffDecoder::new(Cursor::new(bytes))
            .map_err(|e| error("E_TIFF", format!("TIFF 无法读取：{e}"), file, loc))?;
        if !matches!(
            decoder.color_type(),
            image::ColorType::L8
                | image::ColorType::La8
                | image::ColorType::Rgb8
                | image::ColorType::Rgba8
                | image::ColorType::L16
                | image::ColorType::La16
                | image::ColorType::Rgb16
                | image::ColorType::Rgba16
        ) {
            return Err(error(
                "E_TIFF",
                "TIFF 仅支持 8/16 位灰度或 RGB，可带 Alpha",
                file,
                loc,
            ));
        }
        // Read TIFF tags before image 0.25's set_limits adjustment, which can
        // otherwise make a valid ICC tag unavailable on small images.
        Box::new(decoder)
    } else {
        Box::new(
            image::ImageReader::with_format(Cursor::new(bytes), format)
                .into_decoder()
                .map_err(|e| error("E_ASSET", format!("图片无法解码：{path}: {e}"), file, loc))?,
        )
    };
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let profile = decoder
        .icc_profile()
        .map_err(|e| error("E_ASSET", format!("ICC 无法读取：{e}"), file, loc))?
        .map(|bytes| {
            moxcms::ColorProfile::new_from_slice(&bytes)
                .map_err(|e| error("E_ASSET", format!("ICC 无效：{e}"), file, loc))
        })
        .transpose()?;
    let (w, h) = decoder.dimensions();
    if u64::from(w) * u64::from(h) > 100_000_000 {
        return Err(error("E_ASSET", "图片像素数超过上限", file, loc));
    }
    if format == ImageFormat::Tiff {
        decoder
            .set_limits(image::Limits::default())
            .map_err(|e| error("E_ASSET", format!("图片解码限制：{e}"), file, loc))?;
    }
    let mut image = if format == ImageFormat::Jpeg
        && profile
            .as_ref()
            .is_some_and(|p| p.color_space == moxcms::DataColorSpace::Cmyk)
    {
        cmyk_jpeg(bytes, profile.as_ref().unwrap(), file, loc)?
    } else {
        let mut image = image::DynamicImage::from_decoder(decoder)
            .map_err(|e| error("E_ASSET", format!("图片无法解码：{path}: {e}"), file, loc))?;
        if associated_alpha {
            straighten_alpha(&mut image);
        }
        if let Some(profile) = &profile {
            convert_icc(image, profile, file, loc)?
        } else {
            image
        }
    };
    let passthrough = format == ImageFormat::Jpeg && profile.is_none()
        && orientation == image::metadata::Orientation::NoTransforms
        && matches!(image.color(), image::ColorType::L8 | image::ColorType::Rgb8);
    image.apply_orientation(orientation);
    let mut asset = raster_asset(image, file, loc)?;
    if passthrough { crate::asset_cache::register_jpeg(asset["rasterKey"].as_str().unwrap(), bytes); asset["sourceJpegHash"]=json!(crate::asset_cache::digest(bytes)); }
    Ok(asset)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_svg_references_are_rejected() {
        for input in [
            "<svg><script/></svg>",
            "<svg><use href='https://example.com/x'/></svg>",
            "<svg><path fill='url(//example.com/x)'/></svg>",
            "<!DOCTYPE svg><svg/>",
            "<svg onload='alert(1)'/>",
        ] {
            assert_eq!(
                sanitize_svg(input, "x.lay", Loc::default())
                    .unwrap_err()
                    .code,
                "E_SVG"
            );
        }
    }
    #[test]
    fn svg_dimensions_and_internal_use() {
        let input=b"<svg width='25.4mm' height='12.7mm'><defs><path id='a' d='M0 0L1 1'/></defs><use href='#a'/></svg>";
        let a = load(input, "x.svg", "x.lay", Loc::default()).unwrap();
        assert_eq!(a["width"], 96.);
        assert_eq!(a["height"], 48.);
    }
    #[test]
    fn tiff_boundary() {
        for bytes in [
            include_bytes!("../../../tests/assets/gray8.tiff").as_slice(),
            include_bytes!("../../../tests/assets/rgb8.tiff").as_slice(),
            include_bytes!("../../../tests/assets/gray16.tiff").as_slice(),
        ] {
            assert!(load(bytes, "image.tiff", "x", Loc::default()).is_ok());
        }
        for bytes in [include_bytes!("../../../tests/assets/multipage.tiff").as_slice()] {
            assert_eq!(
                load(bytes, "image.tiff", "x", Loc::default())
                    .unwrap_err()
                    .code,
                "E_TIFF"
            );
        }
    }
}

/// Crop normalized raster pixels before resampling so adjacent excluded pixels cannot bleed in.
/// Vector SVG crops stay vector and are handled by the renderer's viewport/clip.
pub fn crop_raster(data: &str, rect: [f64; 4], file: &str, loc: Loc) -> Result<Json> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| error("E_ASSET", "无效图片数据", file, loc))?;
    let image = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .map_err(|e| error("E_ASSET", e.to_string(), file, loc))?;
    crop_pixels(&image, rect, file, loc)
}
pub fn crop_asset(asset: &Json, rect: [f64; 4], file: &str, loc: Loc) -> Result<Json> {
    if let Some(image) = asset["rasterKey"].as_str().and_then(crate::asset_cache::pixels) { return crop_pixels(&image, rect, file, loc); }
    crop_raster(asset["data"].as_str().unwrap_or(""), rect, file, loc)
}
fn crop_pixels(image: &image::DynamicImage, rect: [f64; 4], file: &str, loc: Loc) -> Result<Json> {
    let (x, y) = (
        rect[0].round().max(0.) as u32,
        rect[1].round().max(0.) as u32,
    );
    let (w, h) = (
        (rect[2].round().max(0.) as u32).min(image.width().saturating_sub(x)),
        (rect[3].round().max(0.) as u32).min(image.height().saturating_sub(y)),
    );
    if w == 0 || h == 0 {
        return Err(error("E_IMAGE", "裁剪区域小于一个像素", file, loc));
    }
    let cropped = image.crop_imm(x, y, w, h);
    raster_asset(cropped, file, loc)
}
