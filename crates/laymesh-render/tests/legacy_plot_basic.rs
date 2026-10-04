#![cfg(feature = "native")]
//! Every behavioral assertion in tests/plot.test.mjs at 78db22d, using its original inputs.
mod plot_support;
use base64::Engine;
use laymesh_render::{render_pdf, render_svg};
use plot_support::*;
use serde_json::{Value as J, json};
fn s(row: usize) -> laymesh_core::model::Scene {
    scene(PLOT, row)
}
fn children(n: &J) -> &Vec<J> {
    n["children"].as_array().unwrap()
}
fn bounds(n: &J, values: [f64; 4]) {
    assert_eq!(
        n["plotBounds"],
        json!({"x":values[0],"y":values[1],"width":values[2],"height":values[3]})
    );
}
fn is_rect(n: &J) -> bool {
    n["kind"] == "rect" || n["geometryRecipe"]["kind"] == "rect"
}
fn geometry_equal(a: &J, b: &J) {
    assert_eq!(a, b);
}
fn axis_lines(p: &J) -> Vec<[[f64; 2]; 2]> {
    let mut out = vec![];
    for n in children(p).iter().filter(|n| n["kind"] == "path") {
        let mut prior = None;
        for el in kurbo::BezPath::from_svg(n["d"].as_str().unwrap())
            .unwrap()
            .elements()
        {
            match el {
                kurbo::PathEl::MoveTo(q) => prior = Some([q.x, q.y]),
                kurbo::PathEl::LineTo(q) => {
                    let now = [q.x, q.y];
                    if let Some(p) = prior {
                        if p != now {
                            out.push([p, now]);
                        }
                    }
                    prior = Some(now)
                }
                _ => {}
            }
        }
    }
    out
}
fn has_line(lines: &[[[f64; 2]; 2]], wanted: [[f64; 2]; 2]) {
    assert!(
        lines
            .iter()
            .any(|v| [wanted, [wanted[1], wanted[0]]].iter().any(|w| v
                .iter()
                .flatten()
                .zip(w.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 0.0001))),
        "missing {wanted:?} in {lines:?}"
    );
}
fn legend_text(p: &J, name: &str) -> (f64, f64, f64) {
    let l = legend(p);
    let t = text(l, name);
    (
        num(l, "x") + num(t, "x"),
        num(l, "y") + num(t, "y"),
        font_size(t),
    )
}
fn font_size(n: &J) -> f64 {
    let runs = n["runs"].as_array().unwrap();
    let sizes: Vec<_> = runs
        .iter()
        .filter(|r| r["kind"] == "glyph")
        .map(|r| num(r, "fontSize"))
        .collect();
    assert!(!sizes.is_empty());
    assert!(sizes.iter().all(|size| (*size - sizes[0]).abs() < 1e-12));
    sizes[0]
}
fn axis_domains(n: &J) -> J {
    let mut a = n["plotAxes"].clone();
    for v in a.as_object_mut().unwrap().values_mut() {
        v.as_object_mut().unwrap().remove("segments");
    }
    a
}
#[test]
fn basic_ticks_grids_and_log_coordinates_follow_physical_directions() {
    let sc = s(1);
    let p = &sc.nodes[0];
    bounds(p, [20., 10., 60., 40.]);
    let nodes = children(p);
    let grid = nodes.iter().find(|n| n["id"] == "plot-grid").unwrap();
    assert_eq!(
        grid["clip"],
        json!({"x":0.,"y":0.,"width":60.,"height":40.})
    );
    assert!(nodes.iter().position(|n| n == grid) < nodes.iter().position(|n| n == data(p)));
    let mut counts = [0, 0];
    for n in paths(grid) {
        let minor = num(n, "opacity") == 0.5;
        counts[usize::from(!minor)] += n["d"].as_str().unwrap().matches('M').count();
        assert!(minor || num(n, "opacity") == 1.);
        if !minor {
            assert_eq!(n["strokeStyle"]["dash"], json!([1., 2.]));
            near(num(&n["strokeStyle"], "width"), 0.2);
        }
    }
    assert_eq!(counts, [4, 6]);
    let lines = axis_lines(p);
    for wanted in [
        [[20., 10.], [80., 10.]],
        [[80., 10.], [80., 50.]],
        [[20., 48.], [20., 50.]],
        [[50., 48.], [50., 50.]],
        [[35., 49.], [35., 50.]],
        [[22., 50.], [18., 50.]],
        [[22., 30.], [18., 30.]],
        [[21., 43.9794], [19., 43.9794]],
    ] {
        has_line(&lines, wanted);
    }
    assert!(sc.warnings.is_empty());
    assert!(
        render_svg(&sc)
            .unwrap()
            .contains("stroke-dasharray=\"1 2\"")
    );
    assert!(render_pdf(&sc).unwrap().len() > 1000);
}
#[test]
fn basic_scientific_style_preserves_both_panels_and_annotation_geometry() {
    let scenarios = [s(2), s(3)];
    for (i, sc) in scenarios.iter().enumerate() {
        let [a, b, da, db] = sc.nodes.as_slice() else {
            panic!()
        };
        for p in [a, b] {
            bounds(p, [20., 10., 60., 40.]);
        }
        near(num(a, "x") + 20., 30.);
        near(num(b, "x") + 20., 105.);
        for (n, x, y) in [(da, 60., 45.), (db, 135., 45.)] {
            near(num(n, "x") + 0.5, x);
            near(num(n, "y") + 0.5, y);
        }
        geometry_equal(data(a), data(b));
        if i == 1 {
            assert_eq!(
                texts(a)
                    .iter()
                    .filter(|n| ["0", "1", "2"].iter().any(|v| n["content"] == *v))
                    .count(),
                3
            );
            near(font_size(text(a, "1")), 7. * 25.4 / 72.);
            near(font_size(text(a, "Long signal label")), 11. * 25.4 / 72.);
            let l = legend(a);
            near(num(l, "x"), 20.);
            near(num(l, "y"), 1.);
            let frame = &children(l)[0];
            assert_eq!(frame["fill"], "none");
            let (measured, model) = (legend_text(a, "Measured"), legend_text(a, "Model"));
            near(measured.2, 6. * 25.4 / 72.);
            near(measured.1, model.1);
            assert!(model.0 > measured.0);
            for dpi in [96., 300.] {
                assert_eq!(
                    raster(sc, dpi).width,
                    (180. * dpi / 25.4_f64).round() as usize
                );
            }
        }
    }
    for i in 0..2 {
        geometry_equal(data(&scenarios[0].nodes[i]), data(&scenarios[1].nodes[i]));
        assert_eq!(
            scenarios[0].nodes[i]["plotAxes"],
            scenarios[1].nodes[i]["plotAxes"]
        );
    }
    assert_eq!(scenarios[0].nodes[2..], scenarios[1].nodes[2..]);
}
#[test]
fn basic_manual_legend_is_row_major_and_warns_for_every_placement() {
    let plain = s(4);
    let sc = s(5);
    let warnings: Vec<_> = sc
        .warnings
        .iter()
        .filter(|w| w.message.contains("手动图例超出图表外框"))
        .collect();
    assert_eq!(warnings.len(), 2);
    assert_ne!(warnings[0].loc.line, warnings[1].loc.line);
    let p = &sc.nodes[0];
    let (a, b, c) = (
        legend_text(p, "A"),
        legend_text(p, "B"),
        legend_text(p, "C"),
    );
    near(a.0, c.0);
    near(a.1, b.1);
    assert!(b.0 > a.0 && c.1 > a.1);
    let l = legend(p);
    near(num(l, "x"), num(&p["plotBounds"], "x") - 30.);
    near(num(l, "y"), num(&p["plotBounds"], "y") - 20.);
    assert_eq!(plain.nodes[0]["plotBounds"], p["plotBounds"]);
    geometry_equal(data(&plain.nodes[0]), data(p));
}
#[test]
fn basic_empty_ticks_hidden_labels_zero_lengths_and_outward_minor_ticks() {
    for row in 6..=8 {
        let sc = s(row);
        let p = &sc.nodes[0];
        assert!(texts(p).is_empty());
        bounds(p, [20., 10., 60., 40.]);
        let lines = axis_lines(p);
        assert_eq!(lines.len(), if row == 8 { 3 } else { 2 });
        for edge in [[[20., 10.], [20., 50.]], [[20., 50.], [80., 50.]]] {
            has_line(&lines, edge);
        }
        if row == 8 {
            has_line(&lines, [[50., 50.], [50., 52.]]);
        }
    }
}
#[test]
fn basic_unpainted_shapes_and_transparent_legend_do_not_leak_pdf_paths() {
    let mut actual = s(9);
    let mut expected = actual.clone();
    let l = legend(&actual.nodes[0]);
    let invisible = children(l)[0].clone();
    fn remove(n: &mut J) {
        if let Some(c) = n["children"].as_array_mut() {
            c.retain(|n| {
                !(n["kind"] == "path" && n["fill"] == "none" && n["strokeStyle"]["color"] == "none")
            });
            for n in c {
                remove(n)
            }
        }
    }
    remove(&mut expected.nodes[0]);
    let mut path = paths(data(&actual.nodes[0]))[0].clone();
    path["fill"] = json!("none");
    path["strokeStyle"]["color"] = json!("none");
    let mut rect = invisible.clone();
    rect["kind"] = json!("rect");
    rect["x"] = json!(3.);
    rect["y"] = json!(3.);
    let mut ellipse = rect.clone();
    ellipse["kind"] = json!("ellipse");
    ellipse["x"] = json!(5.);
    ellipse["y"] = json!(5.);
    actual.nodes.splice(0..0, [rect, ellipse, path]);
    assert_eq!(
        pdf_raster(&actual, 144.).bytes,
        pdf_raster(&expected, 144.).bytes
    );
}
#[test]
fn basic_invalid_scientific_options_keep_every_parameter_case_located() {
    for row in 10..=30 {
        let c = capture(PLOT, row);
        let e = compile_capture(&c).unwrap_err();
        assert_eq!(e.file, c["file"].as_str().unwrap());
        assert!(e.loc.line > 1);
        if row >= 23 {
            assert_eq!(e.loc.line, 5);
        }
    }
}
#[test]
fn basic_locked_area_labels_colorbar_and_style_survive_outer_frame_changes() {
    let sc = s(31);
    let [a, b, c] = sc.nodes.as_slice() else {
        panic!()
    };
    assert_eq!(c["height"], 80.);
    for p in [a, b, c] {
        bounds(p, [25., 8., 50., 40.]);
    }
    geometry_equal(data(a), data(b));
    let labels = |p: &J| {
        texts(p)
            .iter()
            .map(|t| json!([t["content"], t["x"], t["y"], t["rotation"], font_size(t)]))
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(a), labels(b));
    let gradients = |p: &J| {
        paths(p)
            .iter()
            .filter(|n| n["fill"].is_object())
            .map(|n| (*n).clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(gradients(a), gradients(b));
    assert!(!gradients(a).is_empty());
    let before = a["plotBounds"].clone();
    for dpi in [96., 300.] {
        assert_eq!(
            raster(&sc, dpi).width,
            (180. * dpi / 25.4_f64).round() as usize
        );
    }
    assert_eq!(a["plotBounds"], before);
    let sc = s(32);
    assert_eq!(sc.nodes[0]["plotBounds"], sc.nodes[1]["plotBounds"]);
    assert_eq!(
        paths(data(&sc.nodes[0]))[0]["d"],
        paths(data(&sc.nodes[1]))[0]["d"]
    );
}
#[test]
fn basic_auto_log_and_changed_ranges_attach_annotations_to_the_same_data() {
    for row in [33, 34] {
        let sc = s(row);
        let [a, b, da, db] = sc.nodes.as_slice() else {
            panic!()
        };
        for (p, d) in [(a, da), (b, db)] {
            let data = data(p);
            let pos = pos(data);
            near(num(d, "x") + 0.5, num(p, "x") + num(data, "x") + pos[1][0]);
            near(num(d, "y") + 0.5, num(p, "y") + num(data, "y") + pos[1][1]);
        }
        assert_eq!(axis_domains(a), axis_domains(b));
        if row == 34 {
            near(
                num(da, "x") + 0.5,
                num(a, "x") + num(&a["plotBounds"], "x") + num(&a["plotBounds"], "width") / 2.,
            );
            assert_eq!(a["plotAxes"]["x"]["domain"], json!([1., 100.]));
        } else {
            assert_eq!(a["plotAxes"]["x"]["domain"], json!([-3.95, 104.95]));
        }
    }
    let mut positions = vec![];
    for (row, max) in [(35, 2.), (36, 4.)] {
        let sc = s(row);
        positions.push(num(&sc.nodes[1], "x") + 0.5);
        near(num(&sc.nodes[1], "y") + 0.5, 30.);
        assert_eq!(sc.nodes[0]["plotAxes"]["x"]["domain"], json!([0., max]));
    }
    assert_eq!(positions, [50., 35.]);
}
#[test]
fn basic_rotations_group_scaling_and_arrow_tip_attachments_preserve_anchors() {
    let sc = s(37);
    let [a, tl, br, center, tip, marker, line] = sc.nodes.as_slice() else {
        panic!()
    };
    near(num(a, "x"), -15.);
    near(num(a, "y"), 35.);
    for (n, x, y) in [
        (tl, 60., 40.),
        (br, 20., 100.),
        (center, 40., 70.),
        (marker, 40., 70.),
    ] {
        near(num(n, "x") + 0.5, x);
        near(num(n, "y") + 0.5, y);
    }
    let angle = std::f64::consts::PI / 6.;
    let actual = [
        num(line, "x") + 5. + 5. * std::f64::consts::FRAC_1_SQRT_2,
        num(line, "y") + 5. * std::f64::consts::FRAC_1_SQRT_2,
    ];
    let endpoint = tip["endpoints"][0].as_array().unwrap();
    let x = endpoint[0].as_f64().unwrap() - num(tip, "width") / 2.;
    let y = endpoint[1].as_f64().unwrap() - num(tip, "height") / 2.;
    let target = [
        num(tip, "x") + num(tip, "width") / 2. + x * angle.cos() - y * angle.sin(),
        num(tip, "y") + num(tip, "height") / 2. + x * angle.sin() + y * angle.cos(),
    ];
    for (a, b) in actual.into_iter().zip(target) {
        near(a, b);
    }
    near(actual[0], 40. - 10. * angle.cos() - 5. * angle.sin());
    near(actual[1], 70. - 10. * angle.sin() + 5. * angle.cos());
    let sc = s(38);
    let g = &sc.nodes[0];
    let (p, d) = (&g["children"][0], &g["children"][1]);
    near(num(g, "width") / num(g, "contentWidth"), 1.2);
    near(num(g, "height") / num(g, "contentHeight"), 1.2);
    near(num(d, "x") + 1., num(p, "x") + 50.);
    near(num(d, "y") + 1., num(p, "y") + 30.);
    assert!(render_svg(&sc).unwrap().contains("rotate(15"));
    assert!(render_pdf(&sc).unwrap().len() > 1000);
}
#[test]
fn basic_invalid_areas_and_annotation_targets_keep_all_diagnostic_messages() {
    let mut failures = vec![];
    let messages = [
        "不能同时",
        "非负",
        "大于零",
        "长度",
        "宽度",
        "宽高必须大于零",
        "必须大于零",
        "超出轴范围",
        "超出轴范围",
        "无单位",
        "缺少",
        "位置参数",
        "已放置",
        "同一画布或组",
        "原生图表",
        "线段或箭头",
    ];
    for (row, message) in (39..=54).zip(messages) {
        let c = capture(PLOT, row);
        let e = compile_capture(&c).unwrap_err();
        assert_eq!(e.file, c["file"].as_str().unwrap());
        assert!(e.loc.line > 1);
        if !e.message.contains(message) {
            failures.push(format!("row{row} expected{message}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn basic_cached_placements_warn_independently_without_changing_data() {
    let sc = s(55);
    let warnings: Vec<_> = sc
        .warnings
        .iter()
        .filter(|w| w.code == "W_PLOT_LAYOUT")
        .collect();
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings.iter().map(|w| w.loc.line).collect::<Vec<_>>(),
        [6, 7]
    );
    for w in warnings {
        assert_eq!(w.file, capture(PLOT, 55)["file"].as_str().unwrap());
        let amount = w
            .message
            .split("下侧缺少约 ")
            .nth(1)
            .unwrap()
            .split(" mm")
            .next()
            .unwrap();
        assert!(amount.chars().all(|c| c.is_ascii_digit() || c == '.'));
        assert!(amount.parse::<f64>().unwrap() > 0.);
    }
    for p in &sc.nodes {
        bounds(p, [20., 10., 60., 40.]);
    }
    geometry_equal(data(&sc.nodes[0]), data(&sc.nodes[2]));
}
#[test]
fn basic_overflow_tiny_and_crowded_areas_keep_complete_labels_and_geometry() {
    for (row, b, message) in [
        (56, [20., 10., 90., 40.], "超出图表外框"),
        (57, [20., 10., 60., 40.], "超出图表外框"),
        (58, [0., 0., 100., 70.], "固定留白不足"),
        (59, [20., 10., 1., 1.], "不超过 2 mm"),
    ] {
        let sc = s(row);
        bounds(&sc.nodes[0], b);
        assert!(
            sc.warnings
                .iter()
                .any(|w| w.code == "W_PLOT_LAYOUT" && w.message.contains(message)),
            "row{row}"
        );
    }
    let sc = s(60);
    for phrase in [
        "x 轴刻度标签重叠",
        "y 轴刻度标签重叠",
        "轴标签长于绘图区",
        "色标刻度标签重叠",
        "图例超出绘图区",
    ] {
        assert_eq!(
            sc.warnings
                .iter()
                .filter(|w| w.code == "W_PLOT_LAYOUT" && w.message.contains(phrase))
                .count(),
            1,
            "{phrase}"
        );
    }
    for label in [
        "Long horizontal label",
        "Long vertical label",
        "Long colorbar label",
        "Legend remains complete",
        "0.25",
        "1.75",
    ] {
        assert!(
            texts(&sc.nodes[0]).iter().any(|n| n["content"] == label),
            "{label}"
        );
    }
    bounds(&sc.nodes[0], [25., 10., 15., 10.]);
    let sc = s(61);
    assert!(!sc.warnings.iter().any(|w| w.code == "W_PLOT_LAYOUT"));
    assert!(texts(&sc.nodes[0]).len() < 12);
}
#[test]
fn basic_native_reflow_keeps_units_clips_marker_size_and_export_dimensions() {
    let sc = s(62);
    assert_eq!(sc.schema_version, 8);
    let (a, b) = (&sc.nodes[0], &sc.nodes[1]);
    assert_eq!(b["height"], 60.);
    assert_eq!(b["width"], 100.);
    let (da, db) = (data(a), data(b));
    assert!(num(db, "width") > num(da, "width"));
    assert_eq!(
        da["children"][0]["clip"],
        json!({"x":0.,"y":0.,"width":da["width"],"height":da["height"]})
    );
    assert_eq!(points(da)[0]["markerSize"], points(db)[0]["markerSize"]);
    assert_eq!(
        pos(da),
        [
            [0., num(da, "height")],
            [num(da, "width") / 2., num(da, "height") / 2.],
            [num(da, "width"), 0.]
        ]
    );
    assert_eq!(
        paths(da)[0]["strokeStyle"]["width"],
        paths(db)[0]["strokeStyle"]["width"]
    );
    let sizes = |n: &J| {
        let mut s = texts(n).iter().map(|n| font_size(n)).collect::<Vec<_>>();
        s.sort_by(f64::total_cmp);
        s.dedup();
        s
    };
    assert_eq!(sizes(a), sizes(b));
    let svg = render_svg(&sc).unwrap();
    assert!(svg.contains("<clipPath") && svg.contains("<text "));
    assert_eq!(
        roxmltree::Document::parse(&svg)
            .unwrap()
            .descendants()
            .filter(|n| n.has_tag_name("circle"))
            .count(),
        6
    );
    assert!(
        !String::from_utf8_lossy(&render_pdf(&sc).unwrap())
            .split_whitespace()
            .collect::<String>()
            .contains("/Subtype/Image")
    );
    let before = a["plotBounds"].clone();
    for dpi in [96., 300.] {
        assert_eq!(
            raster(&sc, dpi).width,
            (180. * dpi / 25.4_f64).round() as usize
        );
    }
    assert_eq!(a["plotBounds"], before);
}
#[test]
fn basic_missing_data_preserves_gaps_order_and_log_error_coordinates() {
    let sc = s(63);
    assert_eq!(
        sc.warnings
            .iter()
            .filter(|w| w.code == "W_PLOT_MISSING")
            .count(),
        3
    );
    let d = data(&sc.nodes[0]);
    assert_eq!(
        paths(&d["children"][0])
            .iter()
            .map(|n| n["d"].as_str().unwrap().matches('M').count())
            .sum::<usize>(),
        2
    );
    assert_eq!(
        paths(&d["children"][1])
            .iter()
            .map(|n| n["d"].as_str().unwrap().matches('Z').count())
            .sum::<usize>(),
        2
    );
    let p = pos(d);
    assert_eq!(p.len(), 3);
    assert!(p[0][0] > p[1][0] && p[1][0] > p[2][0]);
    let sc = s(64);
    let d = data(&sc.nodes[0]);
    near(pos(d)[1][0], num(d, "width") / 2.);
    assert!(
        paths(&d["children"][1])
            .iter()
            .any(|n| n["d"].as_str().unwrap().contains('M'))
    );
    assert!(texts(&sc.nodes[0]).iter().any(|n| n["content"] == "10.0"));
}
#[test]
fn basic_heatmap_pixels_orientation_transparency_and_colorbar_remain_exact() {
    let sc = s(65);
    let (a, b) = (data(&sc.nodes[0]), data(&sc.nodes[1]));
    assert_eq!(a["children"][0]["children"][0]["kind"], "image");
    assert_eq!(a["children"][0]["children"][0]["interpolation"], "nearest");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(a["children"][0]["children"][0]["data"].as_str().unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    assert_eq!(
        &bytes[..info.buffer_size()],
        &[
            170, 170, 170, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 0
        ]
    );
    let cells = paths(b);
    assert_eq!(cells.len(), 3);
    use kurbo::Shape;
    let cell = kurbo::BezPath::from_svg(cells[0]["d"].as_str().unwrap())
        .unwrap()
        .bounding_box();
    near(cell.x0, 0.);
    near(cell.y0, 0.);
    near(cell.width(), num(b, "width") / 2.);
    near(cell.height(), num(b, "height") / 2.);
    assert!(
        render_svg(&sc)
            .unwrap()
            .contains("image-rendering='optimizeSpeed'")
    );
    assert!(
        String::from_utf8_lossy(&render_pdf(&sc).unwrap())
            .split_whitespace()
            .collect::<String>()
            .contains("/Subtype/Image")
    );
    assert!(pdf_text(&sc).contains("Value"));
    let image = raster(&sc, 254.);
    let area = &sc.nodes[0]["plotBounds"];
    for (fx, fy, expected) in [
        (0.12, 0.12, 170),
        (0.38, 0.38, 170),
        (0.62, 0.12, 255),
        (0.88, 0.38, 255),
        (0.12, 0.62, 0),
        (0.38, 0.88, 0),
    ] {
        assert_eq!(
            image.pixel(
                num(area, "x") + num(a, "width") * fx,
                num(area, "y") + num(a, "height") * fy
            )[0],
            expected
        );
    }
}
#[test]
fn basic_snapshots_frozen_definitions_shadowing_modules_and_large_paths() {
    let sc = s(66);
    assert_eq!(sc.nodes.len(), 3);
    assert_eq!(
        paths(data(&sc.nodes[0]))[0]["d"]
            .as_str()
            .unwrap()
            .matches('L')
            .count(),
        1
    );
    assert!(
        compile_case(PLOT, 67)
            .unwrap_err()
            .message
            .contains("已经放置的图表")
    );
    let sc = s(68);
    assert!(is_rect(&sc.nodes[0]));
    assert_eq!(s(69).nodes.len(), 1);
    assert_eq!(
        paths(data(&s(70).nodes[0]))[0]["d"]
            .as_str()
            .unwrap()
            .matches('L')
            .count(),
        10000
    );
}
#[test]
fn basic_invalid_data_layout_and_resources_keep_all_original_error_phrases() {
    let mut failures = vec![];
    for (row, message) in (71..=82).zip([
        "长度不匹配",
        "误差不能为负",
        "lower",
        "矩形",
        "marker",
        "不支持参数",
        "对数轴",
        "空间不足",
        "表头",
        "没有有效数据",
        "不存在数据列",
        "无效 JSON",
    ]) {
        let c = capture(PLOT, row);
        let e = compile_capture(&c).unwrap_err();
        if row <= 76 {
            assert_eq!(e.file, c["file"].as_str().unwrap());
            assert!(e.loc.line > 1);
        }
        if !e.message.contains(message) {
            failures.push(format!("row{row} expected{message}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn basic_clipping_and_coincident_scatter_alpha_match_png_and_pdf() {
    let sc = s(83);
    let area = &sc.nodes[0]["plotBounds"];
    for r in [raster(&sc, 254.), pdf_raster(&sc, 254.)] {
        assert_eq!(
            r.pixel(
                10. + num(area, "x") + num(area, "width") + 3.,
                10. + num(area, "y") + num(area, "height") / 2.
            ),
            [255, 255, 255]
        );
        let mid = r.pixel(
            10. + num(area, "x") + num(area, "width") / 2.,
            10. + num(area, "y") + num(area, "height") / 2.,
        );
        assert!((62..=65).contains(&mid[0]), "{mid:?}");
        assert_eq!(mid[1], 0);
        assert_eq!(mid[2], 0);
    }
}
