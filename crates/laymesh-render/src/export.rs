//! One export contract shared by the CLI, Python and the native editor transport.
use super::*;
use image::{ExtendedColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// Explicit options are validated even when a format would otherwise ignore them.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExportOptions {
    pub dpi: Option<f64>,
    pub quality: Option<f32>,
    pub compression: Option<String>,
    pub background: Option<String>,
    pub webp_lossless: Option<bool>,
    pub webp_method: Option<u8>,
    pub webp_alpha_quality: Option<u8>,
    pub webp_near_lossless: Option<u8>,
    pub pdf_image_compression: Option<String>,
    pub pdf_jpeg_quality: Option<u8>,
    pub pdf_downsample: Option<bool>,
    pub pdf_recompress_jpeg: Option<bool>,
    pub pdf_preserve_16bit: Option<bool>,
    pub pdf_preserve_alpha: Option<bool>,
    pub pdf_alpha_background: Option<String>,
    pub pdf_auto_palette_limit: Option<usize>,
    pub pdf_auto_flatness_threshold: Option<f64>,
}

pub fn export_format(extension: &str) -> Result<&'static str> {
    Ok(match extension.to_ascii_lowercase().as_str() {
        "svg" => "svg",
        "pdf" => "pdf",
        "png" => "png",
        "jpg" | "jpeg" => "jpeg",
        "tif" | "tiff" => "tiff",
        "webp" => "webp",
        "bmp" => "bmp",
        "gif" => "gif",
        "ico" => "ico",
        "tga" => "tga",
        "pnm" | "pam" => "pam",
        "ppm" => "ppm",
        "pgm" => "pgm",
        "pbm" => "pbm",
        _ => {
            return Err(error(
                "不支持的导出格式；可用 SVG/PDF/PNG/JPEG/TIFF/WebP/BMP/GIF/ICO/PNM/TGA",
            ));
        }
    })
}

impl ExportOptions {
    pub fn validate(&self, extension: &str) -> Result<()> {
        let format = export_format(extension)?;
        let raster = !matches!(format, "svg" | "pdf");
        if let Some(dpi) = self.dpi {
            if !(raster || format == "pdf") || !dpi.is_finite() || dpi <= 0. || dpi > 25_400. {
                return Err(error("DPI 仅用于位图/PDF 输出，须为 0–25400 之间的正数"));
            }
        }
        if let Some(quality) = self.quality {
            let min = if format == "jpeg" { 1. } else { 0. };
            if !matches!(format, "jpeg" | "webp")
                || !quality.is_finite()
                || !(min..=100.).contains(&quality)
            {
                return Err(error("quality 仅用于 JPEG（1–100）或 WebP（0–100）"));
            }
        }
        if let Some(compression) = &self.compression {
            let valid = match format {
                "png" => matches!(compression.as_str(), "fast" | "default" | "best"),
                "tiff" => matches!(
                    compression.as_str(),
                    "none" | "lzw" | "deflate" | "packbits"
                ),
                _ => false,
            };
            if !valid {
                return Err(error(
                    "compression：PNG 使用 fast/default/best；TIFF 使用 none/lzw/deflate/packbits",
                ));
            }
        }
        if let Some(background) = &self.background {
            if !matches!(format, "jpeg" | "gif" | "ppm" | "pgm" | "pbm") {
                return Err(error("background 仅用于 JPEG/GIF/PPM/PGM/PBM 的不透明底色"));
            }
            parse_background(background)?;
        }
        if format != "webp"
            && (self.webp_lossless.is_some()
                || self.webp_method.is_some()
                || self.webp_alpha_quality.is_some()
                || self.webp_near_lossless.is_some())
        {
            return Err(error("webp_* 参数仅用于 WebP 输出"));
        }
        if self.webp_method.is_some_and(|v| v > 6)
            || self.webp_alpha_quality.is_some_and(|v| v > 100)
            || self.webp_near_lossless.is_some_and(|v| v > 100)
        {
            return Err(error(
                "WebP method 须为 0–6，alpha_quality 和 near_lossless 须为 0–100",
            ));
        }
        if self.webp_lossless == Some(false) && self.webp_near_lossless.is_some() {
            return Err(error("webp_near_lossless 仅用于无损 WebP 模式"));
        }
        if self.webp_lossless.unwrap_or(true) && self.webp_alpha_quality.is_some_and(|v| v != 100) {
            return Err(error("降低 webp_alpha_quality 须选择有损 WebP 模式"));
        }
        if let Some(background) = &self.pdf_alpha_background {
            parse_background(background)?;
        }
        let pdf_explicit = self.pdf_image_compression.is_some()
            || self.pdf_jpeg_quality.is_some()
            || self.pdf_downsample.is_some()
            || self.pdf_recompress_jpeg.is_some()
            || self.pdf_preserve_16bit.is_some()
            || self.pdf_preserve_alpha.is_some()
            || self.pdf_alpha_background.is_some()
            || self.pdf_auto_palette_limit.is_some()
            || self.pdf_auto_flatness_threshold.is_some();
        if pdf_explicit && format != "pdf" {
            return Err(error("pdf_* 参数仅用于 PDF 输出"));
        }
        if self
            .pdf_image_compression
            .as_deref()
            .is_some_and(|s| !matches!(s, "auto" | "lossless" | "jpeg"))
            || self
                .pdf_jpeg_quality
                .is_some_and(|v| !(1..=100).contains(&v))
            || self.pdf_auto_palette_limit.is_some_and(|v| v > 16384)
            || [self.pdf_auto_flatness_threshold]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err(error(
                "无效 PDF 参数：compression=auto/lossless/jpeg，quality=1–100，阈值=0–1，palette_limit=0–16384",
            ));
        }
        Ok(())
    }
}

fn parse_background(value: &str) -> Result<[u8; 3]> {
    let digits = value.strip_prefix('#').unwrap_or("");
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(error("background 须为 #RRGGBB（默认 #ffffff）"));
    }
    Ok([0, 2, 4].map(|i| u8::from_str_radix(&digits[i..i + 2], 16).unwrap()))
}
fn matte(rgba: &[u8], background: [u8; 3]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| {
            (0..3).map(move |i| {
                ((p[i] as u32 * p[3] as u32 + background[i] as u32 * (255 - p[3]) as u32 + 127)
                    / 255) as u8
            })
        })
        .collect()
}

/// Raster exports use the renderer's 8-bit straight RGBA, in sRGB, at physical DPI.
/// Formats with density metadata (PNG/JPEG/TIFF/BMP) also record that resolution.
pub fn render_export(scene: &Scene, extension: &str, options: &ExportOptions) -> Result<Vec<u8>> {
    options.validate(extension)?;
    let format = export_format(extension)?;
    if format == "svg" {
        return Ok(render_svg(scene)?.into_bytes());
    }
    if format == "pdf" {
        return native::render_pdf_with_options(scene, options);
    }
    let dpi = options.dpi.unwrap_or(scene.export_dpi);
    // Fail before allocating the raster when a codec has a smaller dimension limit.
    let dimensions = (
        (scene.width * dpi / 25.4).round().max(1.),
        (scene.height * dpi / 25.4).round().max(1.),
    );
    let limit = match format {
        "ico" => 256.,
        "webp" => 16383.,
        "jpeg" | "gif" | "tga" => 65535.,
        _ => u32::MAX as f64,
    };
    if dimensions.0 > limit || dimensions.1 > limit {
        return Err(error(format!(
            "{format} 宽高不能超过 {limit} 像素；请减小 DPI 或画布尺寸"
        )));
    }
    let (w, h, rgba) = native::rasterize(scene, dpi)?;
    let mut out = Vec::new();
    let encode_error = |e: image::ImageError| error(format!("{format} 编码失败：{e}"));
    match format {
        "png" => {
            let mut encoder = png::Encoder::new(&mut out, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(match options.compression.as_deref().unwrap_or("default") {
                "fast" => png::Compression::Fast,
                "best" => png::Compression::High,
                _ => png::Compression::Balanced,
            });
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: (dpi / 0.0254).round() as u32,
                yppu: (dpi / 0.0254).round() as u32,
                unit: png::Unit::Meter,
            }));
            encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
            encoder
                .write_header()
                .and_then(|mut writer| writer.write_image_data(&rgba))
                .map_err(|e| error(e.to_string()))?;
        }
        "jpeg" => {
            let rgb = matte(
                &rgba,
                parse_background(options.background.as_deref().unwrap_or("#ffffff"))?,
            );
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut out,
                options.quality.unwrap_or(90.).round() as u8,
            );
            encoder.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(
                dpi.round().max(1.) as u16
            ));
            encoder
                .encode(&rgb, w, h, ExtendedColorType::Rgb8)
                .map_err(encode_error)?;
        }
        "tiff" => {
            use tiff::{
                encoder::{Compression, DeflateLevel, Rational, TiffEncoder, colortype::RGB8},
                tags::{ExtraSamples, ResolutionUnit},
            };
            let compression = match options.compression.as_deref().unwrap_or("lzw") {
                "none" => Compression::Uncompressed,
                "deflate" => Compression::Deflate(DeflateLevel::Balanced),
                "packbits" => Compression::Packbits,
                _ => Compression::Lzw,
            };
            let mut cursor = Cursor::new(&mut out);
            let mut encoder = TiffEncoder::new(&mut cursor)
                .map_err(|e| error(e.to_string()))?
                .with_compression(compression);
            let mut image = encoder
                .new_image::<RGB8>(w, h)
                .map_err(|e| error(e.to_string()))?;
            image
                .extra_samples(&[ExtraSamples::UnassociatedAlpha])
                .map_err(|e| error(e.to_string()))?;
            image.resolution(
                ResolutionUnit::Inch,
                Rational {
                    n: (dpi * 10000.).round() as u32,
                    d: 10000,
                },
            );
            image.write_data(&rgba).map_err(|e| error(e.to_string()))?;
        }
        "webp" => {
            let mut config =
                webp::WebPConfig::new().map_err(|_| error("无法初始化 WebP 编码器"))?;
            config.lossless = i32::from(options.webp_lossless.unwrap_or(true));
            config.quality =
                options
                    .quality
                    .unwrap_or(if config.lossless == 1 { 100. } else { 90. });
            config.method = options.webp_method.unwrap_or(4) as i32;
            config.alpha_quality = options.webp_alpha_quality.unwrap_or(100) as i32;
            config.near_lossless = options.webp_near_lossless.unwrap_or(100) as i32;
            config.exact = 1;
            out = webp::Encoder::from_rgba(&rgba, w, h)
                .encode_advanced(&config)
                .map_err(|e| error(format!("WebP 编码失败：{e:?}")))?
                .to_vec();
        }
        "bmp" => {
            image::codecs::bmp::BmpEncoder::new(&mut out)
                .write_image(&rgba, w, h, ExtendedColorType::Rgba8)
                .map_err(encode_error)?;
            // BITMAPINFO/V4 header stores pixels per metre in these fixed fields.
            let ppm = (dpi / 0.0254).round() as i32;
            out[38..42].copy_from_slice(&ppm.to_le_bytes());
            out[42..46].copy_from_slice(&ppm.to_le_bytes());
        }
        "gif" => {
            let rgb = matte(
                &rgba,
                parse_background(options.background.as_deref().unwrap_or("#ffffff"))?,
            );
            let binary_alpha: Vec<_> = rgb
                .chunks_exact(3)
                .zip(rgba.chunks_exact(4))
                .flat_map(|(p, a)| [p[0], p[1], p[2], if a[3] == 0 { 0 } else { 255 }])
                .collect();
            let image = image::RgbaImage::from_raw(w, h, binary_alpha).unwrap();
            image::codecs::gif::GifEncoder::new(&mut out)
                .encode_frame(image::Frame::new(image))
                .map_err(encode_error)?;
        }
        "ico" => {
            image::codecs::ico::IcoEncoder::new(&mut out)
                .write_image(&rgba, w, h, ExtendedColorType::Rgba8)
                .map_err(encode_error)?;
        }
        "tga" => {
            // Some readers reject RLE packets that cross scanlines. Raw TGA is
            // interoperable and preserves the same RGBA samples.
            image::codecs::tga::TgaEncoder::new(&mut out)
                .disable_rle()
                .write_image(&rgba, w, h, ExtendedColorType::Rgba8)
                .map_err(encode_error)?;
        }
        "pam" => {
            image::codecs::pnm::PnmEncoder::new(&mut out)
                .encode(rgba.as_slice(), w, h, ExtendedColorType::Rgba8)
                .map_err(encode_error)?;
        }
        "ppm" | "pgm" | "pbm" => {
            use image::codecs::pnm::{PnmEncoder, PnmSubtype, SampleEncoding};
            let rgb = matte(
                &rgba,
                parse_background(options.background.as_deref().unwrap_or("#ffffff"))?,
            );
            let (pixels, color, subtype) = match format {
                "ppm" => (
                    rgb,
                    ExtendedColorType::Rgb8,
                    PnmSubtype::Pixmap(SampleEncoding::Binary),
                ),
                _ => {
                    let gray = rgb
                        .chunks_exact(3)
                        .map(|p| {
                            ((299 * p[0] as u32 + 587 * p[1] as u32 + 114 * p[2] as u32 + 500)
                                / 1000) as u8
                        })
                        .map(|v| {
                            if format == "pbm" {
                                u8::from(v >= 128)
                            } else {
                                v
                            }
                        })
                        .collect();
                    (
                        gray,
                        if format == "pbm" {
                            ExtendedColorType::L1
                        } else {
                            ExtendedColorType::L8
                        },
                        if format == "pbm" {
                            PnmSubtype::Bitmap(SampleEncoding::Binary)
                        } else {
                            PnmSubtype::Graymap(SampleEncoding::Binary)
                        },
                    )
                }
            };
            PnmEncoder::new(&mut out)
                .with_subtype(subtype)
                .encode(pixels.as_slice(), w, h, color)
                .map_err(encode_error)?;
        }
        _ => unreachable!(),
    }
    Ok(out)
}
