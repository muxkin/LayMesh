#![cfg(feature = "native")]
mod plot_support;
use laymesh_render::render_svg;
use plot_support::*;
use serde_json::json;
#[test]
fn four_directions_negative_anchors_and_inspect_retain_projection() {
    let s = scene(POLAR, 1);
    let p = &s.nodes[0];
    assert_eq!(s.schema_version, 8);
    assert_eq!(
        p["plotBounds"],
        json!({"x":20.,"y":15.,"width":70.,"height":60.})
    );
    assert_eq!(p["plotProjection"], s.nodes[3]["plotProjection"]);
    for (a, b) in pos(p)
        .iter()
        .flatten()
        .zip([85., 45., 55., 15., 25., 45., 55., 75.])
    {
        near(*a, b);
    }
    near(num(&s.nodes[1], "x") + 0.5, 55.);
    near(num(&s.nodes[1], "y") + 0.5, 15.);
    near(num(&s.nodes[2], "x"), num(&s.nodes[1], "x"));
    near(num(&s.nodes[2], "y"), num(&s.nodes[1], "y"));
    assert_eq!(
        s.warnings
            .iter()
            .filter(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
            .count(),
        1
    );
    let i = laymesh_language::inspect::inspect_scene(&s);
    let i = &i["plots"][0];
    assert_eq!(i["projection"]["center"], json!([55., 45.]));
    near(num(&i["projection"], "outerRadius"), 30.);
    assert!(!i["clip_boundary"]["d"].as_str().unwrap().is_empty());
    assert_eq!(i["page_transform"].as_array().unwrap().len(), 6);
}
#[test]
fn negative_radius_line_crosses_origin_with_one_warning_per_layer() {
    let s = scene(POLAR, 3);
    assert_eq!(
        s.warnings
            .iter()
            .filter(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
            .count(),
        1
    );
    let pp = paths(data(&s.nodes[0]));
    assert_eq!(pp.len(), 1);
    assert_eq!(pp[0]["d"].as_str().unwrap().matches('M').count(), 2);
    let points: Vec<_> = pp.into_iter().flat_map(vertices).collect();
    assert!(points.iter().any(|p| (p[0] - 50.).hypot(p[1] - 45.) < 1e-9));
    assert!(points.iter().any(|p| p[0] < 21.) && points.iter().any(|p| p[0] > 79.));
}
#[test]
fn annular_sector_clips_match_in_svg_png_pdf() {
    let s = scene(POLAR, 4);
    assert!(render_svg(&s).unwrap().contains("clip-rule=\"evenodd\""));
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        for (x, y, c) in [
            (50., 50., 255),
            (35., 35., 255),
            (65., 35., 128),
            (75., 25., 255),
        ] {
            assert_eq!(r.pixel(x, y), [c; 3]);
        }
    }
}
#[test]
fn projected_errors_missing_values_fills_hatches_and_bars_export() {
    for line in [5, 6] {
        let s = scene(POLAR, line);
        assert!(
            s.warnings
                .iter()
                .any(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
        );
        exports(&s);
    }
}
#[test]
fn contour_grid_radius_is_exact_with_periodic_band_and_colorbar() {
    let s = scene(POLAR, 7);
    exports(&s);
    let pp = paths(data(&s.nodes[0]))
        .into_iter()
        .filter(|n| n["strokeStyle"]["color"] == "#ff0000")
        .collect::<Vec<_>>();
    assert!(!pp.is_empty());
    for p in pp {
        for [x, y] in vertices(p) {
            assert!(((x - 55.).hypot(y - 50.) - 35. * 2. / 3.).abs() < 2e-5);
        }
    }
}
#[test]
fn radar_raw_ranges_category_order_and_anchors_are_preserved() {
    let s = scene(POLAR, 8);
    let m = &s.nodes[0]["plotProjection"];
    assert_eq!(m["categories"], json!(["Strength", "Temperature", "Cost"]));
    assert_eq!(m["ranges"], json!([[0., 100.], [-20., 20.], [10., 30.]]));
    assert!(
        !s.warnings
            .iter()
            .any(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
    );
    near(num(&s.nodes[2], "x") + 0.5, 50.);
    near(num(&s.nodes[2], "y") + 0.5, 15.);
    near(
        (num(&s.nodes[1], "x") + 0.5 - 50.).hypot(num(&s.nodes[1], "y") + 0.5 - 45.),
        7.5,
    );
    exports(&s);
}
#[test]
fn missing_radar_vertex_suppresses_closed_fill() {
    let s = scene(POLAR, 9);
    assert!(s.warnings.iter().any(|w| w.message.contains("封闭面积")));
    assert!(
        !descendants(data(&s.nodes[0]))
            .iter()
            .any(|n| n["fill"] == "#ff0000")
    );
}
#[test]
fn rotation_repetition_and_groups_preserve_polar_mapping() {
    let s = scene(POLAR, 10);
    let i = laymesh_language::inspect::inspect_scene(&s);
    let p = i["plots"].as_array().unwrap();
    assert_eq!(p.len(), 2);
    near(
        p[0]["page_transform"][0]
            .as_f64()
            .unwrap()
            .hypot(p[0]["page_transform"][1].as_f64().unwrap()),
        2.,
    );
    assert_eq!(p[0]["projection"], p[1]["projection"]);
    let g = &s.nodes[0];
    let dot = &g["children"][1];
    let plot = &g["children"][0];
    near(num(dot, "x") + 0.5, num(plot, "x") + 80.);
    near(num(dot, "y") + 0.5, num(plot, "y") + 50.);
}
#[test]
fn invalid_projections_ranges_log_data_and_anchors_keep_source_errors() {
    for line in (11..=18).chain([20, 24]) {
        failure(POLAR, line);
    }
}
#[test]
fn log_histogram_bar_area_ecdf_clip_only_synthetic_zero() {
    exports(&scene(POLAR, 19));
    exports(&scene(POLAR, 23));
}
#[test]
fn colorbar_overlap_warns_without_moving_independent_axis() {
    let s = scene(POLAR, 21);
    assert!(
        s.warnings
            .iter()
            .any(|w| w.message.contains("temperature.tick") && w.message.contains("colorbar"))
    );
    assert_eq!(
        s.nodes[0]["plotBounds"],
        json!({"x":20.,"y":15.,"width":70.,"height":50.})
    );
    near(num(&s.nodes[0]["plotAxes"]["temperature"], "offset"), 0.);
}
#[test]
fn boxplot_stroke_and_outlier_styles_retain_final_geometry() {
    let s = scene(POLAR, 22);
    assert!(all(&s).iter().any(|n| n["kind"] == "path"
        && n["strokeStyle"]["color"] == "#ff0000"
        && num(&n["strokeStyle"], "width") == 0.5));
    let p = points(&s.nodes[0])[0];
    assert_eq!(p["marker"], "diamond");
    near(num(p, "markerSize"), 3.);
    assert_eq!(p["markerFill"], "none");
    exports(&s);
}
#[test]
fn negative_grid_reflects_half_turn_and_retains_missing_transparency() {
    let s = scene(POLAR, 25);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        for (x, y, c) in [(65., 65., 128), (65., 35., 255), (35., 35., 128)] {
            assert_eq!(r.pixel(x, y), [c; 3]);
        }
    }
    assert!(
        s.warnings
            .iter()
            .any(|w| w.code == "W_POLAR_NEGATIVE_RADIUS")
    );
}
#[test]
fn periodic_contour_band_seam_has_single_alpha_in_png_pdf() {
    let s = scene(POLAR, 26);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        for (x, y) in [(65., 49.), (65., 51.), (35., 49.), (35., 51.), (50., 35.)] {
            let rgb = r.pixel(x, y);
            assert!(
                rgb.iter().all(|v| (126..=129).contains(v)),
                "{x},{y}: {rgb:?}"
            );
        }
    }
}
#[test]
fn log_radial_labels_retain_geometry_and_formula_pdf_source() {
    let a = scene(POLAR, 28);
    let b = scene(POLAR, 29);
    assert_eq!(a.nodes[0]["plotProjection"], b.nodes[0]["plotProjection"]);
    assert_eq!(a.nodes[0]["plotBounds"], b.nodes[0]["plotBounds"]);
    let t = pdf_text(&a);
    assert!(t.contains("Samples") && t.contains("I_0"));
}
#[test]
fn periodic_missing_node_masks_remove_adjacent_cells_in_all_backends() {
    let s = scene(POLAR, 30);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        assert_eq!(r.pixel(60., 40.), [255; 3]);
        assert_eq!(r.pixel(40., 60.), [0; 3]);
    }
}
#[test]
fn radar_category_ranges_have_independent_offset_multipliers() {
    let s = scene(POLAR, 31);
    let f = formulas(&s.nodes[0]);
    for exponent in [3, -3, 2] {
        assert!(f.contains(&format!("\\times 10^{{{exponent}}}").as_str()));
    }
    assert_eq!(
        s.nodes[0]["plotDecorations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["name"].as_str().unwrap().ends_with(".exponent"))
            .count(),
        3
    );
    exports(&s);
}
