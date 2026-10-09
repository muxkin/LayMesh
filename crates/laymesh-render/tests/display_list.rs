#![cfg(feature = "native")]
use kurbo::Rect;
use laymesh_core::{engine::Engine, model::Host};
use laymesh_render::{
    display_list::{DisplayList, DrawKind},
    raster, render_rgba, to_display_list,
};
fn compile(source: &str) -> DisplayList {
    to_display_list(
        &Engine::new(Host::default())
            .compile(source, "tile.lay")
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn wide_exports_keep_dimensions_and_reject_pixel_overflow() {
    let list = compile("p=canvas(size=(7000mm,0.2mm),background=\"#12345680\")");
    let frame = render_rgba(&list, 254.).unwrap();
    assert_eq!((frame.width, frame.height), (70000, 2));
    for x in [0, 1023, 1024, 2047, 2048, 65535, 69999] {
        let p = &frame.rgba[x * 4..x * 4 + 4];
        assert_eq!(p, [0x12, 0x34, 0x56, 0x80]);
    }
    assert_eq!(
        raster::dimensions(1000., 1000., 254.).unwrap(),
        (10000, 10000)
    );
    assert!(raster::dimensions(1000.1, 1000., 254.).is_err());
    let mut bad = list.clone();
    bad.schema_version += 1;
    assert!(render_rgba(&bad, 96.).is_err());
}
#[test]
fn tile_boundaries_match_a_shifted_viewport() {
    let list = compile(
        r##"p=canvas(size=(240mm,25mm))
p.add(ellipse(size=(200mm,12mm),fill=linear_gradient(stops=[(0,"#ff000080"),(1,"#0000ffff")]),effects=[shadow(blur=0.15mm,offset=(0.3mm,0.4mm))]),offset=(15mm,4mm))"##,
    );
    let full = render_rgba(&list, 254.).unwrap();
    let region = raster::render_region(&list, Rect::new(100., 0., 110., 25.), 100, 250).unwrap();
    let mut max = 0;
    for y in 0..250 {
        for x in 0..100 {
            for channel in 0..4 {
                let a = full.rgba[(y * 2400 + x + 1000) * 4 + channel];
                let b = region.rgba[(y * 100 + x) * 4 + channel];
                max = max.max(a.abs_diff(b));
            }
        }
    }
    assert!(max <= 1, "boundary delta {max}");
    let serialized = serde_json::to_string(&list).unwrap();
    let decoded: DisplayList = serde_json::from_str(&serialized).unwrap();
    assert_eq!(render_rgba(&decoded, 254.).unwrap().rgba, full.rgba);
    assert!(matches!(
        decoded.nodes[0].kind,
        DrawKind::Path { .. } | DrawKind::Group { .. }
    ));
}
#[test]
fn very_wide_translucent_images_have_no_internal_edges() {
    use laymesh_render::display_list::{DrawNode, Resource};
    use std::io::Cursor;
    let image = image::RgbaImage::from_pixel(70000, 1, image::Rgba([35, 125, 225, 128]));
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let list = DisplayList {
        schema_version: 1,
        width: 4512.3,
        height: 0.1,
        nodes: vec![DrawNode::new(DrawKind::Image {
            resource: "wide".into(),
            rect: Rect::new(0., 0., 4512.3, 0.1),
            nearest: false,
        })],
        resources: std::collections::BTreeMap::from([(
            "wide".into(),
            Resource {
                mime: "image/png".into(),
                data: bytes.into_inner(),
                font_index: None,
                family: None,
            },
        )]),
    };
    let frame = render_rgba(&list, 254.).unwrap();
    let reference = &frame.rgba[100 * 4..100 * 4 + 4];
    for (x, p) in frame.rgba.chunks_exact(4).enumerate() {
        assert_eq!(p, reference, "image seam at {x}");
    }
}
