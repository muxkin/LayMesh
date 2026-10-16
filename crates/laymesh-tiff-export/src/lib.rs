//! Small offline TIFF encoder. Browser PNG/JPEG encoders and the shared
//! CanvasKit renderer handle the other raster work; no duplicate image decoder.
use std::io::Cursor;
use tiff::{
    encoder::{Compression, DeflateLevel, Rational, TiffEncoder, colortype::RGB8},
    tags::{ExtraSamples, ResolutionUnit},
};
use wasm_bindgen::prelude::*;
fn encode(w: u32, h: u32, rgba: &[u8], dpi: f64, compression: &str) -> Result<Vec<u8>, String> {
    if !dpi.is_finite()
        || dpi <= 0.
        || dpi > 25400.
        || w == 0
        || h == 0
        || w as u64 * h as u64 > 100_000_000
        || rgba.len() as u64 != w as u64 * h as u64 * 4
    {
        return Err("Invalid TIFF size, resolution or pixel buffer".into());
    }
    let compression = match compression {
        "none" => Compression::Uncompressed,
        "lzw" => Compression::Lzw,
        "deflate" => Compression::Deflate(DeflateLevel::Balanced),
        "packbits" => Compression::Packbits,
        _ => return Err("Unsupported TIFF compression".into()),
    };
    let mut output = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut output)
        .map_err(|e| e.to_string())?
        .with_compression(compression);
    let mut image = encoder.new_image::<RGB8>(w, h).map_err(|e| e.to_string())?;
    image
        .extra_samples(&[ExtraSamples::UnassociatedAlpha])
        .map_err(|e| e.to_string())?;
    image.resolution(
        ResolutionUnit::Inch,
        Rational {
            n: (dpi * 1000.).round() as u32,
            d: 1000,
        },
    );
    image.write_data(rgba).map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}
#[wasm_bindgen]
pub fn encode_tiff(
    w: u32,
    h: u32,
    rgba: &[u8],
    dpi: f64,
    compression: &str,
) -> Result<Vec<u8>, JsValue> {
    encode(w, h, rgba, dpi, compression).map_err(|e| JsValue::from_str(&e))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_straight_alpha_resolution_and_all_compressions() {
        for (compression, tag) in [("none", 1), ("lzw", 5), ("deflate", 8), ("packbits", 32773)] {
            let pixels = [255, 0, 0, 128, 30, 60, 90, 0];
            let bytes = encode(2, 1, &pixels, 1200., compression).unwrap();
            let mut decoder = tiff::decoder::Decoder::new(Cursor::new(bytes)).unwrap();
            assert_eq!(decoder.dimensions().unwrap(), (2, 1));
            assert_eq!(
                decoder.get_tag_u32(tiff::tags::Tag::Compression).unwrap(),
                tag
            );
            assert_eq!(
                decoder
                    .get_tag_u32(tiff::tags::Tag::ResolutionUnit)
                    .unwrap(),
                2
            );
            let tiff::decoder::ifd::Value::Rational(numerator, denominator) =
                decoder.get_tag(tiff::tags::Tag::XResolution).unwrap()
            else {
                panic!("Missing rational density")
            };
            assert_eq!(f64::from(numerator) / f64::from(denominator), 1200.);
            let tiff::decoder::DecodingResult::U8(actual) = decoder.read_image().unwrap() else {
                panic!()
            };
            assert_eq!(actual, pixels);
        }
    }
    #[test]
    fn rejects_invalid_dimensions_before_encoding() {
        for (w, h, dpi) in [
            (0, 1, 1200.),
            (1, 0, 1200.),
            (10001, 10000, 1200.),
            (1, 1, f64::NAN),
            (1, 1, -1.),
        ] {
            assert!(encode(w, h, &[0; 4], dpi, "lzw").is_err());
        }
        assert!(encode(1, 1, &[], 1200., "lzw").is_err());
        assert!(encode(1, 1, &[0; 4], 1200., "unknown").is_err());
    }
}
