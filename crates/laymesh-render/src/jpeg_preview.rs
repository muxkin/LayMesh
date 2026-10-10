//! Native Notebook previews. Export settings and encoders remain independent.
use super::*;
use jpeg_encoder::{ColorType, Encoder, PixelDensity, SamplingFactor};

#[derive(Clone, Debug)]
pub struct JpegPreviewOptions {
    pub dpi: f64,
    pub quality: u8,
    pub background: String,
}

impl Default for JpegPreviewOptions {
    fn default() -> Self {
        Self {
            dpi: 192.,
            quality: 95,
            background: "#ffffff".into(),
        }
    }
}

impl JpegPreviewOptions {
    pub fn validate(&self) -> Result<()> {
        ExportOptions {
            dpi: Some(self.dpi),
            quality: Some(self.quality as f32),
            background: Some(self.background.clone()),
            ..Default::default()
        }
        .validate("jpeg")
    }

    /// An already generated JPEG can serve as the preview at identical settings.
    pub fn matches_export(&self, scene: &Scene, options: &ExportOptions) -> Result<bool> {
        self.validate()?;
        Ok(self.dpi == options.dpi.unwrap_or(scene.export_dpi)
            && self.quality == options.quality.unwrap_or(90.).round() as u8
            && export::parse_background(&self.background)?
                == export::parse_background(options.background.as_deref().unwrap_or("#ffffff"))?)
    }
}

pub fn render_jpeg_preview(scene: &Scene, options: &JpegPreviewOptions) -> Result<Vec<u8>> {
    options.validate()?;
    let (w, h) = raster::dimensions(scene.width, scene.height, options.dpi)?;
    if w > u16::MAX as u32 || h > u16::MAX as u32 {
        return Err(error(
            "JPEG 预览边长不能超过 65535 像素；请降低 --preview-dpi",
        ));
    }
    let (_, _, rgba) = native::rasterize(scene, options.dpi)?;
    let rgb = export::matte(&rgba, export::parse_background(&options.background)?);
    let mut data = Vec::new();
    // Runtime AVX2 detection in jpeg-encoder retains its scalar fallback.
    let mut encoder = Encoder::new(&mut data, options.quality);
    encoder.set_sampling_factor(SamplingFactor::F_1_1);
    encoder.set_density(PixelDensity::dpi(options.dpi.round().max(1.) as u16));
    encoder
        .encode(&rgb, w as u16, h as u16, ColorType::Rgb)
        .map_err(|e| error(&format!("JPEG 预览编码失败：{e}")))?;
    Ok(data)
}
