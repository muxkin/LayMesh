#![cfg(feature = "native")]
use laymesh_core::{asset_cache, engine::compile_source, model::Host};
use laymesh_render::{ExportOptions, config, preview_assets, raster_policy as policy};
use serde_json::json;
fn node(image: image::DynamicImage) -> serde_json::Value {
    let width = image.width();
    let height = image.height();
    let key = asset_cache::insert(image);
    json!({"kind":"image","mime":"image/png","rasterKey":key,"width":25.4,"height":12.7,"intrinsicWidth":width,"intrinsicHeight":height,"fit":"contain","x":0,"y":0})
}
fn scene(n: serde_json::Value) -> laymesh_core::model::Scene {
    let mut s = compile_source(
        "page=canvas(size=(25.4mm,12.7mm))",
        "/policy.lay",
        Host::default(),
    )
    .unwrap();
    s.nodes = vec![n];
    s
}
fn photo() -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(1024, 512, |x, y| {
        image::Rgb([
            (x * 31 + y * 73) as u8,
            (x * 51 + y * 29) as u8,
            (x * 97 + y * 13) as u8,
        ])
    }))
}
#[test]
fn preview_uses_full_resolution_jpeg_and_only_actual_alpha_uses_webp() {
    let opaque16 = image::DynamicImage::ImageRgba16(image::ImageBuffer::from_pixel(
        12,
        8,
        image::Rgba([12000u16, 35000, 55000, 65535]),
    ));
    assert_eq!(
        policy::preview_bytes(&opaque16, 90, 90, 0).unwrap().0,
        "image/jpeg"
    );
    let mut transparent = opaque16.to_rgba16();
    transparent.get_pixel_mut(0, 0)[3] = 65534;
    let (_, bytes) =
        policy::preview_bytes(&image::DynamicImage::ImageRgba16(transparent), 90, 90, 0).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    let (_, bytes) = policy::preview_bytes(&photo(), 90, 90, 0).unwrap();
    assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 1024);
    let dir = std::env::temp_dir().join(format!("laymesh-policy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = scene(node(photo()));
    s.nodes.push(s.nodes[0].clone());
    let settings = config::PreviewSettings {
        processing_memory_mb: 1,
        image_threads: 2,
        ..Default::default()
    };
    let (svg, r) = preview_assets::frame(&s, &dir, &settings).unwrap();
    assert!(!svg.contains("data:image"));
    assert_eq!(
        r.iter().filter(|v| v["mime"] == "image/jpeg").count(),
        1,
        "Repeated instances share one full-resolution resource"
    );
    let image = r.iter().find(|v| v["mime"] == "image/jpeg").unwrap();
    assert_eq!(image["width"], 1024);
    assert_eq!(image["height"], 512);
    let path = std::path::Path::new(image["path"].as_str().unwrap());
    let modified = path.metadata().unwrap().modified().unwrap();
    preview_assets::frame(&s, &dir, &settings).unwrap();
    assert_eq!(modified, path.metadata().unwrap().modified().unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn pdf_content_choice_depth_alpha_direct_jpeg_and_downsample() {
    let small = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(12, 12, |x, y| {
        image::Rgb([
            (x * 31 + y * 73) as u8,
            (x * 51 + y * 29) as u8,
            (x * 97 + y * 13) as u8,
        ])
    }));
    assert!(
        !policy::line_art(&small, 32, 0.9),
        "Small photos must not be upsampled for flatness classification"
    );
    let mut s = scene(node(photo()));
    let opts = ExportOptions {
        dpi: Some(100.),
        ..Default::default()
    };
    let optimized = policy::pdf_scene(&s, &opts).unwrap();
    assert_eq!(optimized.nodes[0]["mime"], "image/jpeg");
    assert_eq!(optimized.nodes[0]["intrinsicWidth"], 100);
    let opts = ExportOptions {
        pdf_downsample: Some(false),
        ..opts
    };
    assert_eq!(
        policy::pdf_scene(&s, &opts).unwrap().nodes[0]["intrinsicWidth"],
        1024
    );
    let pdf = laymesh_render::render_export(&s, "pdf", &opts).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/DCTDecode"));
    assert!(!pdf.windows(4).any(|v| v == b"WEBP"));
    let white = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        80,
        40,
        image::Rgb([255, 255, 255]),
    ));
    let lines = scene(node(white));
    assert_eq!(
        policy::pdf_scene(&lines, &opts).unwrap().nodes[0]["mime"],
        "image/png"
    );
    let high = scene(node(image::DynamicImage::ImageLuma16(
        image::ImageBuffer::from_pixel(80, 40, image::Luma([32000u16])),
    )));
    let forced = ExportOptions {
        pdf_image_compression: Some("jpeg".into()),
        ..opts.clone()
    };
    assert_eq!(
        policy::pdf_scene(&high, &forced).unwrap().nodes[0]["mime"],
        "image/png"
    );
    let pdf = laymesh_render::render_export(&high, "pdf", &forced).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/BitsPerComponent 16"));
    assert_eq!(
        policy::pdf_scene(
            &high,
            &ExportOptions {
                pdf_preserve_16bit: Some(false),
                ..forced.clone()
            }
        )
        .unwrap()
        .nodes[0]["mime"],
        "image/jpeg"
    );
    let alpha = scene(node(image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(80, 40, image::Rgba([255, 0, 0, 128])),
    )));
    let pdf = laymesh_render::render_export(&alpha, "pdf", &forced).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/SMask"));
    let flat = ExportOptions {
        pdf_preserve_alpha: Some(false),
        pdf_alpha_background: Some("#0000ff".into()),
        ..forced.clone()
    };
    let pdf = laymesh_render::render_export(&alpha, "pdf", &flat).unwrap();
    assert!(!String::from_utf8_lossy(&pdf).contains("/SMask"));
    assert!(String::from_utf8_lossy(&pdf).contains("/DCTDecode"));
    let gray = scene(node(image::DynamicImage::ImageLuma8(photo().to_luma8())));
    let optimized = policy::pdf_scene(&gray, &opts).unwrap();
    assert_eq!(optimized.nodes[0]["mime"], "image/jpeg");
    let encoded = base64::engine::general_purpose::STANDARD
        .decode(optimized.nodes[0]["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        image::load_from_memory(&encoded).unwrap().color(),
        image::ColorType::L8
    );
    let jpeg = policy::jpeg(&photo(), 77).unwrap();
    asset_cache::register_jpeg(s.nodes[0]["rasterKey"].as_str().unwrap(), &jpeg);
    s.nodes[0]["sourceJpegHash"] = json!(asset_cache::digest(&jpeg));
    let optimized = policy::pdf_scene(&s, &forced).unwrap();
    use base64::Engine;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(optimized.nodes[0]["data"].as_str().unwrap())
            .unwrap(),
        jpeg
    );
    s.nodes[0]["sourceJpegHash"] = json!("other-source");
    assert_ne!(
        base64::engine::general_purpose::STANDARD
            .decode(
                policy::pdf_scene(&s, &forced).unwrap().nodes[0]["data"]
                    .as_str()
                    .unwrap()
            )
            .unwrap(),
        jpeg
    );
    let mut grouped = s.clone();
    grouped.nodes = vec![
        json!({"kind":"group","width":50.8,"height":25.4,"contentWidth":25.4,"contentHeight":12.7,"children":s.nodes}),
    ];
    let optimized = policy::pdf_scene(
        &grouped,
        &ExportOptions {
            dpi: Some(100.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(optimized.nodes[0]["children"][0]["intrinsicWidth"], 200);
}
#[test]
fn configuration_precedence_and_changed_defaults() {
    let dir = std::env::temp_dir().join(format!("laymesh-config-{}", std::process::id()));
    let nested = dir.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("figure.lay");
    let configfile = dir.join(".laymesh.json");
    std::fs::write(
        &configfile,
        r#"{"export":{"dpi":800,"pdf":{"pdf_jpeg_quality":70}},"preview":{"jpeg_quality":91}}"#,
    )
    .unwrap();
    let user = json!({"export":{"dpi":400}});
    let workspace = json!({"export":{"pdf":{"pdf_jpeg_quality":95}}});
    let (cfg, paths) = config::resolve(&file, None, &user, &workspace).unwrap();
    assert!(paths.contains(&configfile.to_string_lossy().into_owned()));
    let o = config::export_options(&cfg, "pdf", &Default::default()).unwrap();
    assert_eq!(o.dpi, Some(800.));
    assert_eq!(o.pdf_jpeg_quality, Some(95));
    let o = config::export_options(
        &cfg,
        "pdf",
        &ExportOptions {
            dpi: Some(1200.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(o.dpi, Some(1200.));
    assert_eq!(
        config::preview_settings(&cfg, &json!({}))
            .unwrap()
            .jpeg_quality,
        91
    );
    std::fs::write(&configfile, r#"{"export":{"dpi":600}}"#).unwrap();
    assert_eq!(
        config::export_options(
            &config::resolve(&file, None, &user, &workspace).unwrap().0,
            "pdf",
            &Default::default()
        )
        .unwrap()
        .dpi,
        Some(600.)
    );
    assert_eq!(scene(node(photo())).export_dpi, 1200.);
    assert!(config::preview_settings(&json!({}), &json!({"processing_memory_mb":0})).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
