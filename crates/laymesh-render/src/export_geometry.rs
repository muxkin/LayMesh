//! Allocation-free export validation, also used by the preview-only WASM build.
use super::{Result, error};
pub fn export_format(extension: &str) -> Result<&'static str> {
    Ok(match extension.to_ascii_lowercase().as_str() {
        "html" | "zip" => "html",
        "svg" => "svg",
        "pdf" => "pdf",
        "pptx" => "pptx",
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
                "不支持的导出格式；可用 SVG/PDF/PPTX/PNG/JPEG/TIFF/WebP/BMP/GIF/ICO/PNM/TGA",
            ));
        }
    })
}

/// Validate physical export dimensions before any pixel allocation.
pub fn raster_dimensions(width: f64, height: f64, dpi: f64, extension: &str) -> Result<(u32, u32)> {
    let format = export_format(extension)?;
    if ![width, height, dpi]
        .iter()
        .all(|v| v.is_finite() && *v > 0.)
    {
        return Err(error("Export size and DPI must be positive"));
    }
    let dimensions = (
        (width * dpi / 25.4).round().max(1.),
        (height * dpi / 25.4).round().max(1.),
    );
    let limit = match format {
        "ico" => 256.,
        "webp" => 16383.,
        "jpeg" | "gif" | "tga" => 65535.,
        _ => u32::MAX as f64,
    };
    if dimensions.0 > limit || dimensions.1 > limit {
        return Err(error(format!(
            "{format} 宽高不能超过 {limit} 像素；请在导出设置中减小 DPI 或画布尺寸"
        )));
    }
    if dimensions.0 * dimensions.1 > 100_000_000. {
        return Err(error(
            "导出超过 100,000,000 像素；请在导出设置中减小 DPI 或画布尺寸",
        ));
    }
    Ok((dimensions.0 as u32, dimensions.1 as u32))
}
