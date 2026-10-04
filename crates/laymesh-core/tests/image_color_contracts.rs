use base64::Engine;
use image::{ExtendedColorType, ImageDecoder, ImageEncoder};
use laymesh_core::{
    Loc,
    assets::{crop_raster, load},
};
use moxcms::{ColorProfile, DataColorSpace, LutDataType, LutStore, LutType, LutWarehouse};
fn swap_profile() -> ColorProfile {
    let mut p = ColorProfile::new_srgb();
    std::mem::swap(&mut p.red_colorant, &mut p.green_colorant);
    p.cicp = None;
    p
}
fn decode(asset: &serde_json::Value) -> image::RgbaImage {
    image::load_from_memory(
        &base64::engine::general_purpose::STANDARD
            .decode(asset["data"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap()
    .into_rgba8()
}
#[test]
fn rgb_icc_normalization_precedes_orientation_and_crop_and_preserves_alpha() {
    let mut bytes = vec![];
    let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(swap_profile().encode().unwrap())
        .unwrap();
    // TIFF EXIF orientation=6: the first source pixel becomes the top pixel.
    encoder
        .set_exif_metadata(vec![
            b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ])
        .unwrap();
    encoder
        .write_image(
            &[255, 0, 0, 128, 0, 255, 0, 64],
            2,
            1,
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    let asset = load(&bytes, "test.png", "figure.lay", Loc::default()).unwrap();
    assert_eq!(
        (asset["width"].as_u64(), asset["height"].as_u64()),
        (Some(1), Some(2))
    );
    let pixels = decode(&asset);
    assert!(pixels[(0, 0)][1] >= 253 && pixels[(0, 0)][0] <= 2);
    assert_eq!(pixels[(0, 0)][3], 128);
    assert!(pixels[(0, 1)][0] >= 253 && pixels[(0, 1)][1] <= 2);
    assert_eq!(pixels[(0, 1)][3], 64);
    let crop = crop_raster(
        asset["data"].as_str().unwrap(),
        [0., 1., 1., 1.],
        "figure.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(decode(&crop)[(0, 0)], pixels[(0, 1)]);
}
#[test]
fn jpeg_and_tiff_rgb_profiles_are_normalized_and_gray_profile_is_applied() {
    let profile = swap_profile().encode().unwrap();
    let mut jpeg = vec![];
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100);
    encoder.set_icc_profile(profile.clone()).unwrap();
    encoder
        .write_image(&[255, 0, 0], 1, 1, ExtendedColorType::Rgb8)
        .unwrap();
    let mut tiff = std::io::Cursor::new(vec![]);
    let mut encoder = image::codecs::tiff::TiffEncoder::new(&mut tiff);
    encoder.set_icc_profile(profile).unwrap();
    encoder
        .write_image(&[255, 0, 0], 1, 1, ExtendedColorType::Rgb8)
        .unwrap();

    let mut direct =
        image::codecs::tiff::TiffDecoder::new(std::io::Cursor::new(tiff.get_ref())).unwrap();
    assert!(
        direct.icc_profile().unwrap().is_some(),
        "TIFF encoder must preserve ICC"
    );
    for (bytes, name) in [(&jpeg[..], "a.jpg"), (&tiff.get_ref()[..], "a.tiff")] {
        let output = decode(&load(bytes, name, "x", Loc::default()).unwrap());
        assert!(
            output[(0, 0)][1] > 250 && output[(0, 0)][0] < 4,
            "{name}: {:?}",
            output[(0, 0)]
        );
    }
    let mut png = vec![];
    let mut encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder
        .set_icc_profile(ColorProfile::new_gray_with_gamma(1.).encode().unwrap())
        .unwrap();
    encoder
        .write_image(&[128, 63], 1, 1, ExtendedColorType::La8)
        .unwrap();
    let output = decode(&load(&png, "gray.png", "x", Loc::default()).unwrap());
    for value in &output[(0, 0)].0[..3] {
        assert!(value.abs_diff(188) <= 1);
    }
    assert_eq!(output[(0, 0)][3], 63);
}
fn cmyk_profile() -> ColorProfile {
    let mut p = ColorProfile::new_srgb();
    p.color_space = DataColorSpace::Cmyk;
    p.cicp = None;
    let mut clut = vec![];
    for c in [0., 1.] {
        for m in [0., 1.] {
            for y in [0., 1.] {
                for k in [0., 1.] {
                    let rgb = [
                        (1. - c) * (1. - k),
                        (1. - m) * (1. - k),
                        (1. - y) * (1. - k),
                    ];
                    for xyz in [
                        p.red_colorant.x * rgb[0]
                            + p.green_colorant.x * rgb[1]
                            + p.blue_colorant.x * rgb[2],
                        p.red_colorant.y * rgb[0]
                            + p.green_colorant.y * rgb[1]
                            + p.blue_colorant.y * rgb[2],
                        p.red_colorant.z * rgb[0]
                            + p.green_colorant.z * rgb[1]
                            + p.blue_colorant.z * rgb[2],
                    ] {
                        clut.push((xyz * 32768.).round() as u16);
                    }
                }
            }
        }
    }
    p.lut_a_to_b_perceptual = Some(LutWarehouse::Lut(LutDataType {
        num_input_channels: 4,
        num_output_channels: 3,
        num_clut_grid_points: 2,
        matrix: moxcms::Matrix3d {
            v: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        },
        num_input_table_entries: 2,
        num_output_table_entries: 2,
        input_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535, 0, 65535]),
        clut_table: LutStore::Store16(clut),
        output_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535]),
        lut_type: LutType::Lut16,
    }));
    p
}
#[test]
fn cmyk_and_ycck_jpeg_profiles_receive_the_original_ink_samples() {
    let profile = cmyk_profile().encode().unwrap();
    assert!(
        ColorProfile::new_from_slice(&profile).is_ok(),
        "synthetic profile must parse"
    );
    for (source, name) in [
        (
            include_bytes!("../../../tests/assets/cmyk-samples.jpg").as_slice(),
            "cmyk.jpg",
        ),
        (
            include_bytes!("../../../tests/assets/ycck-samples.jpg").as_slice(),
            "ycck.jpg",
        ),
    ] {
        let mut jpeg = source[..2].to_vec();
        jpeg.extend_from_slice(&[0xff, 0xe2]);
        jpeg.extend_from_slice(&((profile.len() + 16) as u16).to_be_bytes());
        jpeg.extend_from_slice(b"ICC_PROFILE\0\x01\x01");
        jpeg.extend_from_slice(&profile);
        jpeg.extend_from_slice(&source[2..]);
        let out = decode(&load(&jpeg, name, "figure.lay", Loc::default()).unwrap());
        for (x, expected) in [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [0, 0, 0]]
            .iter()
            .enumerate()
        {
            for channel in 0..3 {
                assert!(
                    out[(x as u32, 0)][channel].abs_diff(expected[channel]) <= 15,
                    "{name} pixel{x}: {:?}",
                    out[(x as u32, 0)]
                );
            }
        }
    }
}
#[test]
fn malformed_icc_produces_a_located_diagnostic_instead_of_wrong_colors() {
    let mut png = vec![];
    let mut encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder.set_icc_profile(vec![1, 2, 3]).unwrap();
    encoder
        .write_image(&[255, 0, 0], 1, 1, ExtendedColorType::Rgb8)
        .unwrap();
    let loc = Loc {
        line: 3,
        column: 7,
        offset: 23,
    };
    let e = load(&png, "bad.png", "test.lay", loc).unwrap_err();
    assert_eq!(e.code, "E_ASSET");
    assert_eq!(e.loc, loc);
    assert_eq!(e.file, "test.lay");
    assert!(e.message.contains("ICC"));
}
