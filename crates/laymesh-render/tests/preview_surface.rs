#![cfg(feature = "native")]
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{raster::PreparedRaster, to_display_list};

#[test]
fn preview_surface_uses_backend_allocation_instead_of_export_pixel_limit() {
    let scene = compile_source(
        "p=canvas(size=(10,10),background=\"#112233\")",
        "/surface.lay",
        Host::default(),
    )
    .unwrap();
    let raster = PreparedRaster::new(to_display_list(&scene).unwrap()).unwrap();
    let bounds = kurbo::Rect::new(0., 0., 10., 10.);
    let frame = raster
        .render_states(bounds, 10001, 10000, &Default::default())
        .unwrap();
    assert_eq!((frame.width, frame.height), (10001, 10000));
    assert_eq!(frame.rgba.len(), 400_040_000);
    assert_eq!(&frame.rgba[..4], &[17, 34, 51, 255]);
    assert_eq!(&frame.rgba[frame.rgba.len() - 4..], &[17, 34, 51, 255]);
    drop(frame);
    // Skia accepts signed dimensions. Check that overflow is rejected instead
    // of silently casting to a negative size or imposing a smaller quota.
    assert!(
        raster
            .render_states(bounds, u32::MAX, 1, &Default::default())
            .is_err()
    );
}
