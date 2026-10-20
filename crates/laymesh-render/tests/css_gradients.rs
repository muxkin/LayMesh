use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{
    ExportOptions, display_update::DisplayUpdate, render_rgba, render_svg, to_display_list,
};
fn scene(css: &str) -> laymesh_core::model::Scene {
    compile_source(&format!("p=canvas(size=(63.5mm,42.333333333333mm),background=\"#ffffff\")\nstyle{{.a{{{css};border-width:0}}}}\np.add(rect(size=(240px,160px),class=\"a\",border_width=0))"),"/gradients.lay",Host::default()).unwrap()
}
#[test]
fn all_original_recipes_use_oklab_and_explicit_srgb_without_ignored_declarations() {
    for space in ["", "style{:root{--gradient-space:in srgb!important}}"] {
        for i in 1..=30 {
            let src = format!(
                "p=canvas(size=(30mm,20mm),stylesheet=\"@laymesh/open-props.css\")\n{space}\nstyle{{.a{{background:var(--gradient-{i});border-width:0}}}}\np.add(rect(size=(30mm,20mm),class=\"a\",border_width=0))"
            );
            let scene = compile_source(&src, "/original.lay", Host::default()).unwrap();
            assert!(
                !scene
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("Ignored background")),
                "{i}: {:?}",
                scene.warnings
            );
            let list = to_display_list(&scene).unwrap();
            assert_eq!(
                list.schema_version,
                laymesh_render::display_list::DISPLAY_LIST_VERSION
            );
            let frame = render_rgba(&list, 96.).unwrap();
            assert!(frame.rgba.chunks_exact(4).any(|c| c[3] != 0));
        }
    }
}
#[test]
fn independent_pixels_verify_orientation_stops_bottom_color_and_opacity() {
    let list=to_display_list(&scene("background:conic-gradient(red 0deg 90deg,lime 90deg 180deg,blue 180deg 270deg,black 270deg)" )).unwrap();
    let rgba = render_rgba(&list, 96.).unwrap().rgba;
    for (x, y, c) in [
        (180, 40, [255, 0, 0, 255]),
        (180, 120, [0, 255, 0, 255]),
        (60, 120, [0, 0, 255, 255]),
        (60, 40, [0, 0, 0, 255]),
    ] {
        assert_eq!(&rgba[(y * 240 + x) * 4..(y * 240 + x) * 4 + 4], &c);
    }
    let frame=render_rgba(&to_display_list(&scene("background:linear-gradient(rgba(255,0,0,.5),rgba(255,0,0,.5)),linear-gradient(rgba(0,0,255,.5),rgba(0,0,255,.5)) lime;opacity:.5")).unwrap(),96.).unwrap();
    // Front red over blue over lime, then object alpha over white.
    let c = &frame.rgba[(80 * 240 + 120) * 4..(80 * 240 + 120) * 4 + 4];
    for (a, b) in c.iter().zip([191, 159, 159, 255]) {
        assert!((*a as i32 - b).abs() <= 1, "{c:?}");
    }
}
#[test]
fn hint_matches_the_standard_power_curve_at_every_pixel() {
    let frame = render_rgba(
        &to_display_list(&scene("background:linear-gradient(to right,red,25%,blue)")).unwrap(),
        96.,
    )
    .unwrap();
    for x in 0..240 {
        let t = ((x as f64 + 0.5) / 240.).sqrt();
        let c = &frame.rgba[(80 * 240 + x) * 4..];
        assert!((c[0] as f64 - 255. * (1. - t)).abs() <= 1.);
        assert!((c[2] as f64 - 255. * t).abs() <= 1.);
    }
}
#[test]
fn svg_resolves_path_paints_in_physical_bounds_and_silently_rasterizes_only_conic_fill() {
    let svg = render_svg(&scene("background:linear-gradient(to right,red,blue)")).unwrap();
    assert!(svg.contains("x2='63.5'"), "{svg}");
    assert!(!svg.contains("data:image/png"));
    let svg = render_svg(&scene("background:conic-gradient(red,blue)")).unwrap();
    assert!(svg.contains("data:image/png"));
    assert!(svg.contains("<path"));
    let s = scene("background:conic-gradient(red,blue);border:1px solid black");
    let (_, warnings) = laymesh_render::pptx::render_pptx(
        &s,
        &ExportOptions {
            dpi: Some(96.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
}
#[test]
fn display_version_one_is_readable_and_migration_replaces_the_baseline() {
    let new = to_display_list(&scene("background:conic-gradient(red,blue)")).unwrap();
    let mut old = to_display_list(&scene("background:red")).unwrap();
    old.schema_version = 1;
    assert!(render_rgba(&old, 96.).is_ok());
    for incremental in [false, true] {
        let mut baseline = old.clone();
        let update = if incremental {
            DisplayUpdate::between_incremental(&baseline, &new)
        } else {
            DisplayUpdate::between(&baseline, &new)
        };
        assert!(update.roots.is_some());
        update.apply(&mut baseline).unwrap();
        assert_eq!(baseline, new);
    }
}
#[test]
fn pdf_local_fill_uses_export_dpi_once() {
    let s = scene("background:conic-gradient(red,blue)");
    for dpi in [96., 192.] {
        let bytes = laymesh_render::render_export(
            &s,
            "pdf",
            &ExportOptions {
                dpi: Some(dpi),
                ..Default::default()
            },
        )
        .unwrap();
        let pdf = lopdf::Document::load_mem(&bytes).unwrap();
        let widths: Vec<_> = pdf
            .objects
            .values()
            .filter_map(|v| v.as_stream().ok())
            .filter_map(|s| s.dict.get(b"Width").ok()?.as_i64().ok())
            .collect();
        assert!(!widths.is_empty());
        assert!(
            widths
                .iter()
                .all(|w| (*w as f64 - 240. * dpi / 96.).abs() <= 1.),
            "{dpi}: {widths:?}"
        );
    }
}
