#![cfg(feature = "native")]
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use laymesh_render::{ExportOptions, render_export};
use std::io::Cursor;

fn scene() -> Scene {
    compile_source("page=canvas(size=(25.4mm,12.7mm),background=\"none\")\npage.add(rect(size=(10mm,10mm),fill=\"#ff000080\",border_width=0),offset=(1mm,1mm))", "/export.lay",Host::default()).unwrap()
}
fn options() -> ExportOptions {
    ExportOptions {
        dpi: Some(144.),
        ..Default::default()
    }
}
fn decoded(bytes: &[u8], format: image::ImageFormat) -> image::RgbaImage {
    image::load_from_memory_with_format(bytes, format)
        .unwrap()
        .to_rgba8()
}

#[test]
fn raster_formats_round_trip_dimensions_and_alpha() {
    let scene = scene();
    let reference = decoded(
        &render_export(&scene, "png", &options()).unwrap(),
        image::ImageFormat::Png,
    );
    assert_eq!(reference.dimensions(), (144, 72));
    assert_eq!(reference.get_pixel(25, 25).0, [255, 0, 0, 128]);
    assert_eq!(reference.get_pixel(140, 70).0, [0, 0, 0, 0]);
    for (extension, format) in [
        ("tif", image::ImageFormat::Tiff),
        ("webp", image::ImageFormat::WebP),
        ("bmp", image::ImageFormat::Bmp),
        ("ico", image::ImageFormat::Ico),
        ("tga", image::ImageFormat::Tga),
        ("pam", image::ImageFormat::Pnm),
        ("pnm", image::ImageFormat::Pnm),
    ] {
        let bytes = render_export(&scene, extension, &options()).unwrap();
        let actual = decoded(&bytes, format);
        assert_eq!(actual, reference, "{extension} must retain straight RGBA");
    }
    for extension in ["pbm", "pgm", "ppm"] {
        let bytes = render_export(&scene, extension, &options()).unwrap();
        let actual = decoded(&bytes, image::ImageFormat::Pnm);
        assert_eq!(actual.dimensions(), (144, 72), "{extension}");
        assert_eq!(actual.get_pixel(140, 70).0, [255, 255, 255, 255]);
    }
    let gif = decoded(
        &render_export(&scene, "gif", &options()).unwrap(),
        image::ImageFormat::Gif,
    );
    assert_eq!(gif.get_pixel(140, 70).0[3], 0);
    assert_eq!(gif.get_pixel(25, 25).0, [255, 127, 127, 255]);
}

#[test]
fn physical_dpi_metadata_and_lossless_compression() {
    let scene = scene();
    for (name, tag) in [("none", 1), ("lzw", 5), ("deflate", 8), ("packbits", 32773)] {
        let opts = ExportOptions {
            compression: Some(name.into()),
            ..options()
        };
        let bytes = render_export(&scene, "tiff", &opts).unwrap();
        let mut decoder = tiff::decoder::Decoder::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(
            decoder
                .get_tag_unsigned::<u16>(tiff::tags::Tag::Compression)
                .unwrap(),
            tag
        );
        assert_eq!(
            decoder
                .get_tag_unsigned::<u16>(tiff::tags::Tag::ResolutionUnit)
                .unwrap(),
            2
        );
        assert_eq!(
            decoder
                .get_tag(tiff::tags::Tag::XResolution)
                .unwrap()
                .into_u32_vec()
                .unwrap(),
            vec![1440000, 10000]
        );
        assert_eq!(
            decoder
                .get_tag_u16_vec(tiff::tags::Tag::ExtraSamples)
                .unwrap(),
            vec![2]
        );
        assert_eq!(
            decoded(&bytes, image::ImageFormat::Tiff)
                .get_pixel(25, 25)
                .0,
            [255, 0, 0, 128]
        );
    }
    for name in ["fast", "default", "best"] {
        let bytes = render_export(
            &scene,
            "png",
            &ExportOptions {
                compression: Some(name.into()),
                ..options()
            },
        )
        .unwrap();
        let reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
        assert_eq!(reader.info().pixel_dims.unwrap().xppu, 5669);
        assert_eq!(
            decoded(&bytes, image::ImageFormat::Png).dimensions(),
            (144, 72)
        );
    }
    let bytes = render_export(&scene, "bmp", &options()).unwrap();
    assert_eq!(i32::from_le_bytes(bytes[38..42].try_into().unwrap()), 5669);
    let bytes = render_export(&scene, "jpg", &options()).unwrap();
    assert_eq!(&bytes[6..11], b"JFIF\0");
    assert_eq!(bytes[13], 1);
    assert_eq!(u16::from_be_bytes(bytes[14..16].try_into().unwrap()), 144);
}

#[test]
fn jpeg_quality_and_matte_and_webp_encoder_controls_are_effective() {
    let mut noise = image::RgbaImage::new(144, 72);
    for (x, y, p) in noise.enumerate_pixels_mut() {
        *p = image::Rgba([
            ((x * 53 + y * 17) % 256) as u8,
            ((x * 29 + y * 73) % 256) as u8,
            ((x * 91 + y * 37) % 256) as u8,
            128,
        ]);
    }
    let mut bytes = Cursor::new(Vec::new());
    noise.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let mut host = Host::default();
    host.files.insert("/noise.png".into(), bytes.into_inner());
    let busy=compile_source("page=canvas(size=(25.4mm,12.7mm),background=\"none\")\npage.add(image(src=\"/noise.png\"),size=(25.4mm,12.7mm))", "/quality.lay",host).unwrap();
    let jpeg_low = render_export(
        &busy,
        "jpeg",
        &ExportOptions {
            quality: Some(10.),
            ..options()
        },
    )
    .unwrap();
    let jpeg_high = render_export(
        &busy,
        "jpeg",
        &ExportOptions {
            quality: Some(95.),
            ..options()
        },
    )
    .unwrap();
    assert!(jpeg_high.len() > jpeg_low.len() * 2);
    let matte = render_export(
        &scene(),
        "jpg",
        &ExportOptions {
            background: Some("#0000ff".into()),
            quality: Some(100.),
            ..options()
        },
    )
    .unwrap();
    let actual = decoded(&matte, image::ImageFormat::Jpeg);
    assert!(actual.get_pixel(140, 70).0[2] > 250);
    assert!(actual.get_pixel(25, 25).0[0].abs_diff(128) < 4);
    assert!(actual.get_pixel(25, 25).0[2].abs_diff(127) < 4);
    let low = render_export(
        &busy,
        "webp",
        &ExportOptions {
            webp_lossless: Some(false),
            quality: Some(10.),
            webp_method: Some(0),
            webp_alpha_quality: Some(60),
            ..options()
        },
    )
    .unwrap();
    let high = render_export(
        &busy,
        "webp",
        &ExportOptions {
            webp_lossless: Some(false),
            quality: Some(95.),
            webp_method: Some(6),
            ..options()
        },
    )
    .unwrap();
    assert!(high.len() > low.len());
    assert!(high.windows(4).any(|chunk| chunk == b"VP8 "));
    assert_eq!(
        decoded(&high, image::ImageFormat::WebP).get_pixel(25, 25).0[3],
        128
    );
    let near = render_export(
        &busy,
        "webp",
        &ExportOptions {
            webp_near_lossless: Some(60),
            webp_method: Some(6),
            ..options()
        },
    )
    .unwrap();
    assert!(near.windows(4).any(|chunk| chunk == b"VP8L"));
    assert_eq!(
        decoded(&near, image::ImageFormat::WebP).dimensions(),
        (144, 72)
    );
}

#[test]
fn invalid_options_and_codec_dimensions_fail_before_encoding() {
    let invalid = [
        (
            "png",
            ExportOptions {
                quality: Some(90.),
                ..options()
            },
        ),
        (
            "jpg",
            ExportOptions {
                quality: Some(0.),
                ..options()
            },
        ),
        (
            "tif",
            ExportOptions {
                compression: Some("jpeg".into()),
                ..options()
            },
        ),
        ("svg", options()),
        (
            "png",
            ExportOptions {
                webp_method: Some(4),
                ..options()
            },
        ),
        (
            "webp",
            ExportOptions {
                webp_method: Some(7),
                ..options()
            },
        ),
        (
            "webp",
            ExportOptions {
                webp_lossless: Some(false),
                webp_near_lossless: Some(50),
                ..options()
            },
        ),
        (
            "jpg",
            ExportOptions {
                background: Some("#ffffffff".into()),
                ..options()
            },
        ),
    ];
    for (extension, opts) in invalid {
        assert!(
            render_export(&scene(), extension, &opts).is_err(),
            "{extension}: {opts:?}"
        );
    }
    assert!(
        render_export(
            &scene(),
            "ico",
            &ExportOptions {
                dpi: Some(300.),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        render_export(
            &scene(),
            "webp",
            &ExportOptions {
                dpi: Some(25400.),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        render_export(
            &scene(),
            "png",
            &ExportOptions {
                dpi: Some(f64::NAN),
                ..Default::default()
            }
        )
        .is_err()
    );
}
