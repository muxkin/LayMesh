#![cfg(not(feature = "native"))]
use laymesh_core::{engine::Engine, model::Host};
#[test]
fn synchronous_svg_api_rasterizes_only_the_projected_subject() {
    let mut scene=Engine::new(Host::default()).compile("p=canvas(size=(60mm,45mm))\np.add(rect(size=(20mm,15mm),fill=\"#33aabb\",three_d=three_d(depth=3mm,rotation=(20deg,-30deg,0deg)),effects=[reflection(length=0.5)]),offset=(10mm,8mm))\np.add(ellipse(size=(5mm,5mm),fill=\"#ff0000\"),offset=(45mm,30mm))","/surface.lay").unwrap();
    scene.export_dpi = 80.;
    let svg = laymesh_render::render_svg(&scene).unwrap();
    assert!(svg.contains("data:image/png;base64,"));
    assert!(svg.contains("<path"));
    assert!(svg.contains("rgb(255,0,0)"));
    assert!(svg.contains("linearGradient"));
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(240, 180).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            240. / tree.size().width(),
            180. / tree.size().height(),
        ),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
}
