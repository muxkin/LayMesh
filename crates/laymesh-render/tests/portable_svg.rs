#![cfg(feature = "native")]
use kurbo::{Affine, Rect, Shape};
use laymesh_render::{display_list::*, portable_svg};
use std::{collections::BTreeMap, io::Cursor};

fn list(nodes: Vec<DrawNode>) -> DisplayList {
    DisplayList {
        page_transition: None,
        animation: None,
        schema_version: 1,
        width: 40.,
        height: 30.,
        nodes,
        resources: BTreeMap::new(),
    }
}
fn path() -> DrawNode {
    DrawNode::new(DrawKind::Path {
        path: Rect::new(1., 2., 6., 7.).to_path(0.01),
        fill: Paint::Solid {
            rgba: [1., 0., 0., 0.5],
        },
        stroke: None,
        even_odd: true,
    })
}
fn png() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([30, 50, 80, 128]),
    ))
    .write_to(&mut out, image::ImageFormat::Png)
    .unwrap();
    out.into_inner()
}
#[test]
fn retained_images_are_embedded_once_and_missing_data_is_an_error() {
    let image = DrawNode::new(DrawKind::Image {
        resource: "shared".into(),
        rect: Rect::new(0., 0., 10., 10.),
        nearest: false,
    });
    let mut list = list(vec![image.clone(), image]);
    assert!(portable_svg::render(&list, 1200., None).is_err());
    list.resources.insert(
        "shared".into(),
        Resource {
            mime: "image/png".into(),
            data: png(),
            font_index: None,
            family: None,
        },
    );
    let svg = portable_svg::render(&list, 1200., None).unwrap();
    assert_eq!(svg.matches("data:image/png;base64,").count(), 1);
    assert_eq!(svg.matches("<use ").count(), 2);
    assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
}
#[test]
fn pdf_rasterizes_only_effect_layers_at_transformed_density() {
    let mut node = path();
    node.transform = Affine::scale_non_uniform(2., 3.);
    node.opacity = 0.6;
    for inner in [false, true] {
        node.effects.push(Effect {
            inner,
            blur: 0.2,
            spread: 0.,
            offset: [0.2, 0.3],
            rgba: [0., 0., 0., 0.5],
            source_path: None,
            knockout: false,
        });
    }
    let list = list(vec![node]);
    let mut calls = Vec::new();
    let svg = portable_svg::render(
        &list,
        254.,
        Some(&mut |node, rect, w, h, inner| {
            assert_eq!(node.transform, Affine::IDENTITY);
            assert_eq!(node.opacity, 1.);
            assert!((w as f64 / rect.width() - 30.).abs() < 0.01);
            assert!((h as f64 / rect.height() - 30.).abs() < 0.01);
            calls.push(inner);
            Ok(png())
        }),
    )
    .unwrap();
    assert_eq!(calls, vec![Some(false), Some(true)]);
    assert_eq!(
        svg.matches("<path ").count(),
        1,
        "The subject must remain a vector"
    );
    assert_eq!(svg.matches("<image ").count(), 2);
    assert!(!svg.contains("<filter"));
    assert!(svg.contains("opacity='0.6'"));
    let standalone = portable_svg::render(&list, 254., None).unwrap();
    assert_eq!(standalone.matches("<filter ").count(), 2);
    assert!(usvg::Tree::from_str(&standalone, &usvg::Options::default()).is_ok());
}
#[test]
fn enormous_effects_fail_before_requesting_any_pixels() {
    let mut node = path();
    node.transform = Affine::scale(1000.);
    node.effects.push(Effect {
        inner: false,
        blur: 1.,
        spread: 0.,
        offset: [0., 0.],
        rgba: [0., 0., 0., 1.],
        source_path: None,
        knockout: false,
    });
    let result = portable_svg::render(
        &list(vec![node]),
        1200.,
        Some(&mut |_, _, _, _, _| panic!("must reject before allocation")),
    );
    assert!(result.is_err());
}

#[test]
fn large_pdf_effects_are_independent_bounded_tiles_at_1200_dpi() {
    let mut node = path();
    if let DrawKind::Path { path, .. } = &mut node.kind {
        *path = Rect::new(0., 0., 190., 130.).to_path(0.01);
    }
    for blur in [0.5, 2.] {
        node.effects.push(Effect {
            inner: false,
            blur,
            spread: 0.,
            offset: [0., 1.],
            rgba: [0., 0., 0., 0.2],
            source_path: None,
            knockout: false,
        });
    }
    let mut pixels = [0u64; 2];
    let mut calls = 0;
    let svg = portable_svg::render(
        &list(vec![node]),
        1200.,
        Some(&mut |node, rect, w, h, inner| {
            assert_eq!(
                node.effects.len(),
                1,
                "Effects must not inflate each other's bounds"
            );
            assert_eq!(inner, Some(false));
            let e = &node.effects[0];
            let density = 1200. / 25.4;
            let halo = ((4. * e.blur + 1.) * density).ceil() as u64 + 2;
            assert!((w as u64 + 2 * halo) * (h as u64 + 2 * halo) * 4 <= 64 * 1024 * 1024);
            assert!((w as f64 / rect.width() - density).abs() < 1e-6);
            assert!((h as f64 / rect.height() - density).abs() < 1e-6);
            pixels[usize::from(e.blur > 1.)] += w as u64 * h as u64;
            calls += 1;
            Ok(png())
        }),
    )
    .unwrap();
    assert!(calls > 2);
    assert!(
        pixels[1] > pixels[0],
        "Blur radius must define each effect's own bounds"
    );
    assert_eq!(svg.matches("<path ").count(), 1);
}
#[test]
fn masks_clips_and_gradient_spread_survive_serialization() {
    let mut n = path();
    if let DrawKind::Path { fill, .. } = &mut n.kind {
        *fill = Paint::Linear {
            start: [0., 0.],
            end: [2., 0.],
            stops: vec![
                Stop {
                    offset: 0.,
                    rgba: [1., 0., 0., 0.5],
                },
                Stop {
                    offset: 1.,
                    rgba: [0., 0., 1., 1.],
                },
            ],
            transform: Affine::IDENTITY,
            spread: "reflect".into(),
        };
    }
    let group = DrawNode::new(DrawKind::Group {
        children: vec![n],
        clip: Some(Clip {
            path: Rect::new(0., 0., 8., 8.).to_path(0.01),
            even_odd: true,
        }),
        mask: Some(Mask {
            node: Box::new(path()),
            luminance: false,
        }),
    });
    let list = list(vec![group]);
    let svg = portable_svg::render(&list, 1200., None).unwrap();
    assert!(svg.contains("mask-type='alpha'"));
    assert!(svg.contains("clip-rule='evenodd'"));
    assert!(svg.contains("spreadMethod='reflect'"));
    assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
    let mut fallback = false;
    let pdf_svg = portable_svg::render(
        &list,
        1200.,
        Some(&mut |_, _, _, _, inner| {
            assert_eq!(inner, None);
            fallback = true;
            Ok(png())
        }),
    )
    .unwrap();
    assert!(fallback);
    assert!(!pdf_svg.contains("linearGradient"));
}
