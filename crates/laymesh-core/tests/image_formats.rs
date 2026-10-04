use base64::Engine as _;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use laymesh_core::{
    Loc,
    assets::{crop_raster, load},
};
use std::io::Cursor;

fn decoded(asset: &serde_json::Value) -> DynamicImage {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(asset["data"].as_str().unwrap())
        .unwrap();
    image::load_from_memory_with_format(&bytes, ImageFormat::Png).unwrap()
}
fn encoded(image: &DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(vec![]);
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}
#[test]
fn common_formats_load_and_crop_with_case_insensitive_extensions() {
    let source =
        DynamicImage::ImageRgb8(ImageBuffer::from_raw(2, 1, vec![255, 0, 0, 0, 255, 0]).unwrap());
    for (ext, format) in [
        ("BMP", ImageFormat::Bmp),
        ("WebP", ImageFormat::WebP),
        ("gif", ImageFormat::Gif),
        ("ico", ImageFormat::Ico),
        ("ppm", ImageFormat::Pnm),
        ("tga", ImageFormat::Tga),
    ] {
        let input = if format == ImageFormat::Ico {
            DynamicImage::ImageRgba8(source.to_rgba8())
        } else {
            source.clone()
        };
        let asset = load(
            &encoded(&input, format),
            &format!("中文 # 图片.{ext}"),
            "figure.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(asset["mime"], "image/png");
        assert_eq!(
            (asset["width"].as_u64(), asset["height"].as_u64()),
            (Some(2), Some(1))
        );
        let crop = crop_raster(
            asset["data"].as_str().unwrap(),
            [1., 0., 1., 1.],
            "figure.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(decoded(&crop).into_rgb8()[(0, 0)].0, [0, 255, 0], "{ext}");
    }
    for (ext,bytes) in [("pbm", b"P1\n2 1\n0 1\n".as_slice()),("pgm", b"P2\n2 1\n255\n0 255\n".as_slice()),("pnm", b"P6\n1 1\n255\n\xff\x00\x00".as_slice()),("pam", b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n\xff\x00\x00\x80".as_slice())] {
        let asset = load(bytes,&format!("image.{ext}"),"figure.lay",Loc::default()).unwrap();
        assert!(asset["width"].as_u64().unwrap()>0);
        if ext=="pam" { assert_eq!(decoded(&asset).into_rgba8()[(0,0)].0,[255,0,0,128]); }
    }
}
#[test]
fn alpha_is_preserved_through_import_and_crop() {
    let source = DynamicImage::ImageRgba8(
        ImageBuffer::from_raw(3, 1, vec![255, 0, 0, 0, 0, 255, 0, 128, 0, 0, 255, 255]).unwrap(),
    );
    for (ext, format) in [
        ("png", ImageFormat::Png),
        ("webp", ImageFormat::WebP),
        ("ico", ImageFormat::Ico),
        ("tga", ImageFormat::Tga),
    ] {
        let asset = load(
            &encoded(&source, format),
            &format!("image.{ext}"),
            "figure.lay",
            Loc::default(),
        )
        .unwrap();
        let pixels = decoded(&asset).into_rgba8();
        assert_eq!(pixels[(0, 0)][3], 0, "{ext}");
        assert_eq!(pixels[(1, 0)].0, [0, 255, 0, 128], "{ext}");
        assert_eq!(pixels[(2, 0)].0, [0, 0, 255, 255], "{ext}");
        let crop = crop_raster(
            asset["data"].as_str().unwrap(),
            [1., 0., 1., 1.],
            "figure.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(decoded(&crop).into_rgba8()[(0, 0)].0, [0, 255, 0, 128]);
    }
}
#[test]
fn animations_use_the_first_frame_including_transparency() {
    let mut bytes = vec![];
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        for color in [Rgba([255, 0, 0, 255]), Rgba([0, 0, 255, 255])] {
            let mut frame = ImageBuffer::from_pixel(2, 1, color);
            frame[(1, 0)] = Rgba([0, 0, 0, 0]);
            encoder.encode_frame(image::Frame::new(frame)).unwrap();
        }
    }
    let pixels =
        decoded(&load(&bytes, "animated.gif", "figure.lay", Loc::default()).unwrap()).into_rgba8();
    assert_eq!(pixels[(0, 0)].0, [255, 0, 0, 255]);
    assert_eq!(pixels[(1, 0)][3], 0);
    let bytes = include_bytes!("../../../tests/assets/animated-first-frame.webp");
    let pixels =
        decoded(&load(bytes, "animated.webp", "figure.lay", Loc::default()).unwrap()).into_rgba8();
    assert_eq!(pixels[(0, 0)].0, [255, 0, 0, 128]);
}
#[test]
fn tiff16_preserves_samples_and_crop_without_contrast_stretching() {
    let pixels: Vec<u16> = vec![0, 257, 1024, 32768, 65535];
    let source = DynamicImage::ImageLuma16(ImageBuffer::from_raw(5, 1, pixels.clone()).unwrap());
    let asset = load(
        &encoded(&source, ImageFormat::Tiff),
        "gray.tiff",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decoded(&asset).color(), image::ColorType::L16);
    assert_eq!(decoded(&asset).into_luma16().into_raw(), pixels);
    let crop = crop_raster(
        asset["data"].as_str().unwrap(),
        [1., 0., 2., 1.],
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decoded(&crop).into_luma16().into_raw(), [257, 1024]);
    let source =
        DynamicImage::ImageRgb16(ImageBuffer::from_raw(1, 1, vec![1111, 2222, 3333]).unwrap());
    let asset = load(
        &encoded(&source, ImageFormat::Tiff),
        "rgb.tif",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decoded(&asset).into_rgb16().into_raw(), [1111, 2222, 3333]);
}
#[test]
fn tiff_straight_and_associated_alpha_keep_16bit_precision() {
    for associated in [false, true] {
        let mut bytes = Cursor::new(vec![]);
        {
            let mut encoder = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
            let mut image = encoder
                .new_image::<tiff::encoder::colortype::RGBA16>(2, 1)
                .unwrap();
            image
                .encoder()
                .write_tag(
                    tiff::tags::Tag::ExtraSamples,
                    &[if associated { 1u16 } else { 2u16 }][..],
                )
                .unwrap();
            image
                .write_data(&[
                    if associated { 32768 } else { 65535 },
                    0,
                    0,
                    32768,
                    0,
                    0,
                    0,
                    0,
                ])
                .unwrap();
        }
        let asset = load(bytes.get_ref(), "alpha.tif", "figure.lay", Loc::default()).unwrap();
        assert_eq!(decoded(&asset).color(), image::ColorType::Rgba16);
        let pixels = decoded(&asset).into_rgba16();
        assert_eq!(pixels[(0, 0)].0, [65535, 0, 0, 32768]);
        assert_eq!(pixels[(1, 0)].0, [0, 0, 0, 0]);
    }
}

#[test]
fn gray_alpha_tiff_supports_both_8bit_and_16bit_samples() {
    struct GrayAlpha8;
    impl tiff::encoder::colortype::ColorType for GrayAlpha8 {
        type Inner = u8;
        const TIFF_VALUE: tiff::tags::PhotometricInterpretation =
            tiff::tags::PhotometricInterpretation::BlackIsZero;
        const BITS_PER_SAMPLE: &'static [u16] = &[8, 8];
        const SAMPLE_FORMAT: &'static [tiff::tags::SampleFormat] =
            &[tiff::tags::SampleFormat::Uint; 2];
        fn horizontal_predict(row: &[u8], out: &mut Vec<u8>) {
            out.extend_from_slice(&row[..2]);
            out.extend(row[2..].iter().zip(row).map(|(a, b)| a.wrapping_sub(*b)));
        }
    }
    struct GrayAlpha16;
    impl tiff::encoder::colortype::ColorType for GrayAlpha16 {
        type Inner = u16;
        const TIFF_VALUE: tiff::tags::PhotometricInterpretation =
            tiff::tags::PhotometricInterpretation::BlackIsZero;
        const BITS_PER_SAMPLE: &'static [u16] = &[16, 16];
        const SAMPLE_FORMAT: &'static [tiff::tags::SampleFormat] =
            &[tiff::tags::SampleFormat::Uint; 2];
        fn horizontal_predict(row: &[u16], out: &mut Vec<u16>) {
            out.extend_from_slice(&row[..2]);
            out.extend(row[2..].iter().zip(row).map(|(a, b)| a.wrapping_sub(*b)));
        }
    }
    let mut bytes = Cursor::new(vec![]);
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
        let mut image = encoder.new_image::<GrayAlpha16>(2, 1).unwrap();
        image
            .encoder()
            .write_tag(tiff::tags::Tag::ExtraSamples, &[2u16][..])
            .unwrap();
        image.write_data(&[1111, 12345, 3333, 65535]).unwrap();
    }
    let asset = load(
        bytes.get_ref(),
        "gray-alpha.tif",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decoded(&asset).color(), image::ColorType::La16);
    assert_eq!(
        decoded(&asset).into_luma_alpha16().into_raw(),
        [1111, 12345, 3333, 65535]
    );
    let mut bytes = Cursor::new(vec![]);
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
        let mut image = encoder.new_image::<GrayAlpha8>(2, 1).unwrap();
        image
            .encoder()
            .write_tag(tiff::tags::Tag::ExtraSamples, &[2u16][..])
            .unwrap();
        image.write_data(&[11, 64, 33, 255]).unwrap();
    }
    let asset = load(
        bytes.get_ref(),
        "gray-alpha8.tif",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(
        decoded(&asset).into_luma_alpha8().into_raw(),
        [11, 64, 33, 255]
    );
    // WhiteIsZero inverts only gray, and associated alpha must be straightened.
    let mut bytes = Cursor::new(vec![]);
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
        let mut image = encoder.new_image::<GrayAlpha16>(1, 1).unwrap();
        image
            .encoder()
            .write_tag(tiff::tags::Tag::ExtraSamples, &[1u16][..])
            .unwrap();
        image
            .encoder()
            .write_tag(tiff::tags::Tag::PhotometricInterpretation, 0u16)
            .unwrap();
        image.write_data(&[32767, 32768]).unwrap();
    }
    let asset = load(
        bytes.get_ref(),
        "white-alpha.tif",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(
        decoded(&asset).into_luma_alpha16().into_raw(),
        [65535, 32768]
    );
    let crop = crop_raster(
        asset["data"].as_str().unwrap(),
        [0., 0., 1., 1.],
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(
        decoded(&crop).into_luma_alpha16().into_raw(),
        [65535, 32768]
    );

    let mut bytes = Cursor::new(vec![]);
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
        let mut image = encoder.new_image::<GrayAlpha16>(1, 1).unwrap();
        image
            .encoder()
            .write_tag(tiff::tags::Tag::ExtraSamples, &[2u16][..])
            .unwrap();
        image
            .encoder()
            .write_tag(
                tiff::tags::Tag::IccProfile,
                moxcms::ColorProfile::new_gray_with_gamma(1.)
                    .encode()
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        image.write_data(&[32768, 12345]).unwrap();
    }
    let asset = load(
        bytes.get_ref(),
        "gray-icc16.tif",
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decoded(&asset).color(), image::ColorType::Rgba16);
    let rgba = decoded(&asset).into_rgba16()[(0, 0)].0;
    assert_eq!(rgba[3], 12345);
    for channel in &rgba[..3] {
        assert!(channel.abs_diff(48192) <= 64, "{rgba:?}");
    }
}
#[test]
fn signed_float_multipage_and_corrupt_assets_have_located_errors() {
    let loc = Loc {
        line: 4,
        column: 8,
        offset: 42,
    };
    let mut bytes = Cursor::new(vec![]);
    tiff::encoder::TiffEncoder::new(&mut bytes)
        .unwrap()
        .write_image::<tiff::encoder::colortype::GrayI16>(1, 1, &[-100])
        .unwrap();
    assert_eq!(
        load(bytes.get_ref(), "signed.tif", "figure.lay", loc)
            .unwrap_err()
            .code,
        "E_TIFF"
    );
    let mut bytes = Cursor::new(vec![]);
    tiff::encoder::TiffEncoder::new(&mut bytes)
        .unwrap()
        .write_image::<tiff::encoder::colortype::Gray32Float>(1, 1, &[0.5])
        .unwrap();
    assert_eq!(
        load(bytes.get_ref(), "float.tif", "figure.lay", loc)
            .unwrap_err()
            .code,
        "E_TIFF"
    );
    assert_eq!(
        load(
            include_bytes!("../../../tests/assets/multipage.tiff"),
            "multi.tif",
            "figure.lay",
            loc
        )
        .unwrap_err()
        .code,
        "E_TIFF"
    );
    for ext in ["bmp", "webp", "gif", "ico", "pgm", "tga"] {
        let error = load(b"invalid", &format!("bad.{ext}"), "figure.lay", loc).unwrap_err();
        assert_eq!(error.code, "E_ASSET");
        assert_eq!(error.file, "figure.lay");
        assert_eq!(error.loc.offset, 42);
    }
}
