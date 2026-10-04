#![cfg(feature = "native")]
mod plot_support;
use laymesh_core::model::Scene;
use laymesh_render::render_pdf;
use plot_support::*;
use serde_json::{Value as J, json};
fn anchor_probes(line: usize, probes: &[&str]) -> Scene {
    let mut c = capture(COMPLETE, line);
    let mut s = c["source"].as_str().unwrap().to_owned();
    s.push_str("\nprobe=page.add(p)\n");
    for probe in probes {
        s.push_str(&format!(
            "page.add(rect(size=(1 mm,1 mm)),anchor=center,target=probe.{probe})\n"
        ));
    }
    c["source"] = json!(s);
    compile_capture(&c).unwrap()
}
#[test]
fn independent_axis_segments_retain_millimetres_offsets_and_anchors() {
    let s = scene(COMPLETE, 1);
    let p = &s.nodes[0];
    assert_eq!(
        p["plotBounds"],
        json!({"x":20.,"y":10.,"width":60.,"height":52.})
    );
    assert_eq!(p["plotBounds"], s.nodes[3]["plotBounds"]);
    assert_eq!(p["plotAxes"], s.nodes[3]["plotAxes"]);
    near(num(&s.nodes[1], "x") + 0.5, 42.);
    near(num(&s.nodes[1], "y") + 0.5, 34.);
    near(num(&s.nodes[2], "x") + 0.5, 88.);
    near(num(&s.nodes[2], "y") + 0.5, 62.);
    let segments = p["plotAxes"]["y"]["segments"].as_array().unwrap();
    assert_eq!(
        segments
            .iter()
            .map(|s| (s["range"][1].as_f64().unwrap() - s["range"][0].as_f64().unwrap()).abs())
            .collect::<Vec<_>>(),
        [20., 30.]
    );
    let inspected = laymesh_language::inspect::inspect_scene(&s);
    let info = &inspected["plots"][0];
    assert_eq!(info["axes"]["c"]["gaps"], json!([]));
    near(num(&info["axes"]["b"]["gaps"][0], "width"), 4.);
    near(num(&info["axes"]["x"]["gaps"][0], "width"), 2.);
    assert!(!info["decorations"].as_array().unwrap().is_empty());
    let heights = descendants(p)
        .iter()
        .filter(|n| n["clip"].is_object())
        .map(|n| num(&n["clip"], "height"))
        .collect::<Vec<_>>();
    for h in [52., 24., 20.] {
        assert!(heights.contains(&h));
    }
}
#[test]
fn broken_axis_render_keeps_other_axes_and_true_error_endcaps() {
    let s = scene(COMPLETE, 2);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        assert_eq!(r.pixel(32., 41.), [255, 255, 255]);
        assert_eq!(r.pixel(68., 41.), [0, 0, 255]);
        assert_eq!(r.pixel(32., 38.), [255, 0, 0]);
    }
    let s = scene(COMPLETE, 3);
    let paths = paths(data(&s.nodes[0]));
    assert_eq!(paths.len(), 2);
    for p in paths {
        assert_eq!(p["d"].as_str().unwrap().matches('M').count(), 2);
    }
}
#[test]
fn reversed_symlog_custom_ticks_and_named_autodomains() {
    let s = scene(COMPLETE, 4);
    let p = &s.nodes[0];
    assert!(
        descendants(p)
            .iter()
            .any(|n| n["kind"] == "formula" && num(n, "rotation") == 30.)
    );
    let t = anchor_probes(
        4,
        &[
            "data(x=0,y=0)",
            "axis(name=\"x\",anchor=\"start\")",
            "axis(name=\"x\",anchor=\"end\")",
        ],
    );
    for (n, [x, y]) in t
        .nodes
        .iter()
        .rev()
        .take(3)
        .rev()
        .zip([[90., 40.], [90., 65.], [20., 65.]])
    {
        near(num(n, "x") + 0.5, x);
        near(num(n, "y") + 0.5, y);
    }
    exports(&s);
    let a = scene(COMPLETE, 5);
    let b = scene(COMPLETE, 6);
    assert_eq!(a.nodes[0]["plotAxes"], b.nodes[0]["plotAxes"]);
    assert_eq!(a.nodes[0]["plotBounds"], b.nodes[0]["plotBounds"]);
    assert_eq!(a.nodes[0]["plotAxes"]["y"]["domain"], json!([-0.1, 2.1]));
    assert_eq!(
        a.nodes[0]["plotAxes"]["temp"]["domain"],
        json!([160., 1040.])
    );
}
#[test]
fn invalid_breaks_bindings_statistics_colors_have_located_errors() {
    for line in (7..=17).chain(35..=45) {
        failure(COMPLETE, line);
    }
    let s = scene(COMPLETE, 46);
    assert!(s.nodes[0]["plotAxes"].get("__proto__").is_some());
}
#[test]
fn repeated_plots_and_group_scaling_preserve_anchor_transforms() {
    let s = scene(COMPLETE, 18);
    let g = &s.nodes[0];
    let p = &g["children"][0];
    let dot = &g["children"][1];
    near(num(dot, "x") + 1., num(p, "x") + 55. - (34. - 45.));
    near(num(dot, "y") + 1., num(p, "y") + 45. + (42. - 55.));
    near(num(g, "width") / num(g, "contentWidth"), 2.);
    near(num(g, "height") / num(g, "contentHeight"), 2.);
    let inspected = laymesh_language::inspect::inspect_scene(&s);
    assert_eq!(inspected["plots"].as_array().unwrap().len(), 2);
    let m = inspected["plots"][0]["page_transform"].as_array().unwrap();
    near(m[0].as_f64().unwrap().hypot(m[1].as_f64().unwrap()), 2.);
    near(m[2].as_f64().unwrap().hypot(m[3].as_f64().unwrap()), 2.);
    assert_eq!(num(p, "rotation"), 90.);
    exports(&s);
}
#[test]
fn all_statistical_layers_keep_missing_degenerate_and_excluded_warnings() {
    let s = scene(COMPLETE, 19);
    for word in ["缺失", "中位数", "分箱"] {
        assert!(
            s.warnings.iter().any(|w| w.message.contains(word)),
            "{word}"
        );
    }
    let svg = exports(&s);
    assert!(svg.contains("clipPath") && svg.contains("evenodd"));
    assert!(render_pdf(&s).unwrap().len() > 2000);
}
#[test]
fn shared_scales_preserve_per_point_paints_sizes_opacity_and_standalone_nodes() {
    let s = scene(COMPLETE, 20);
    let p = &s.nodes[0];
    let point = points(data(p))[0];
    assert_eq!(point["markerSizes"], json!([2., 3., 4.]));
    assert_eq!(
        point["markerFills"],
        json!(["#000000", "#808080", "#ffffff"])
    );
    assert_eq!(point["pointOpacities"], json!([0.25, 0.5, 1.]));
    assert_eq!(
        p["plotBounds"],
        json!({"x":20.,"y":15.,"width":70.,"height":50.})
    );
    assert_eq!(s.nodes.len(), 3);
    exports(&s);
}
#[test]
fn nonuniform_contour_coordinates_and_missing_cell_masks() {
    let s = scene(COMPLETE, 21);
    let pp = paths(data(&s.nodes[0]));
    assert!(!pp.is_empty());
    for p in pp {
        for q in vertices(p) {
            near(q[0], 10.);
        }
    }
    assert!(render_pdf(&s).unwrap().len() > 1000);
    let s = scene(COMPLETE, 22);
    assert!(s.warnings.iter().any(|w| w.message.contains("缺失")));
    assert!(data(&s.nodes[0])["children"].as_array().unwrap().is_empty());
}
#[test]
fn filled_contour_alpha_does_not_accumulate() {
    let s = scene(COMPLETE, 23);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        for [x, y] in [[30., 30.], [70., 30.]] {
            let rgb = r.pixel(x, y);
            assert!(rgb.iter().all(|v| (126..=129).contains(v)), "{rgb:?}");
        }
    }
}
#[test]
fn legacy_schema_solid_points_export() {
    let s = scene(COMPLETE, 24);
    for version in [4, 5, 6] {
        let mut s = s.clone();
        s.schema_version = version;
        fn strip(n: &mut J) {
            if n["kind"] == "points" {
                for k in ["markerFill", "markerStroke", "markerStrokeWidth"] {
                    n.as_object_mut().unwrap().remove(k);
                }
            }
            for c in n["children"].as_array_mut().into_iter().flatten() {
                strip(c)
            }
        }
        for n in &mut s.nodes {
            strip(n)
        }
        exports(&s);
    }
}
#[test]
fn per_point_alpha_size_and_stroke_fit_final_bounds_in_png_pdf() {
    let s = scene(COMPLETE, 26);
    for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
        assert_eq!(r.pixel(40., 30.), [255, 255, 255]);
        assert_eq!(r.pixel(60., 30.), [255, 255, 255]);
        assert_eq!(r.pixel(60., 25.8), [255, 255, 255]);
        for (x, y, lo, hi) in [(40., 28.5, 126, 129), (60., 26.5, 190, 193)] {
            let rgb = r.pixel(x, y);
            assert!(rgb.iter().all(|v| (*v >= lo) && (*v <= hi)), "{rgb:?}");
        }
    }
}
#[test]
fn filled_layers_and_errors_split_only_at_bound_axes() {
    let s = scene(COMPLETE, 25);
    let groups = data(&s.nodes[0])["children"].as_array().unwrap();
    assert_eq!(groups.len(), 26);
    assert_eq!(
        groups[18..22]
            .iter()
            .map(|n| [num(&n["clip"], "width"), num(&n["clip"], "height")])
            .collect::<Vec<_>>(),
        vec![[20., 20.], [20., 30.], [38., 20.], [38., 30.]]
    );
    exports(&s);
}
#[test]
fn transformed_break_lengths_reversed_axis_min_max_semantics() {
    let s = scene(COMPLETE, 27);
    let r = &s.nodes[0]["plotAxes"]["x"]["segments"][0]["range"];
    near((r[1].as_f64().unwrap() - r[0].as_f64().unwrap()).abs(), 29.);
    let s = anchor_probes(
        27,
        &[
            "data(x=10,y=0.3)",
            "axis(name=\"x\",anchor=\"start\")",
            "axis(name=\"x\",anchor=\"end\")",
        ],
    );
    let n = s.nodes.len();
    near(num(&s.nodes[n - 3], "x") + 0.5, 51.);
    for (i, [x, y]) in [[80., 62.], [20., 62.]].iter().enumerate() {
        near(num(&s.nodes[n - 2 + i], "x") + 0.5, *x);
        near(num(&s.nodes[n - 2 + i], "y") + 0.5, *y);
    }
}
#[test]
fn automatic_margins_enclose_added_axes_and_rotated_rich_ticks() {
    let s = scene(COMPLETE, 28);
    let p = &s.nodes[0];
    assert!(num(&p["plotBounds"], "y") > 10.);
    assert!(num(p, "height") - num(&p["plotBounds"], "y") - num(&p["plotBounds"], "height") > 10.);
    assert!(!s.warnings.iter().any(|w| w.message.contains("空间不足")));
    within(p);
}
#[test]
fn all_color_norms_and_colorbar_sides_export() {
    for line in 29..=33 {
        exports(&scene(COMPLETE, line));
    }
}
#[test]
fn four_named_sides_offsets_tick_color_and_anchor_positions() {
    let s = anchor_probes(
        34,
        &[
            "axis(name=\"left\",anchor=\"center\")",
            "axis(name=\"top\",anchor=\"start\")",
            "axis(name=\"bottom\",anchor=\"end\")",
            "axis(name=\"right\",anchor=\"center\")",
        ],
    );
    for (n, [x, y]) in
        s.nodes
            .iter()
            .rev()
            .take(4)
            .rev()
            .zip([[45., 60.], [105., 29.], [105., 97.], [115., 60.]])
    {
        near(num(n, "x") + 0.5, x);
        near(num(n, "y") + 0.5, y);
    }
    assert!(texts(&s.nodes[0]).iter().any(|n| {
        n["runs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "glyph" && r["color"] == "#ff0000")
    }));
    exports(&s);
}
#[test]
fn standalone_decorations_preserve_pdf_text_and_formula_source() {
    let s = scene(COMPLETE, 47);
    let t = pdf_text(&s);
    for expected in ["Observations", "Intensity", "E_0"] {
        assert!(t.contains(expected), "{expected}: {t}");
    }
}
#[test]
fn histogram_auto_bins_exclude_missing_weights() {
    let s = scene(COMPLETE, 48);
    assert_eq!(s.nodes[0]["plotAxes"]["x"]["domain"], json!([0., 1.]));
    assert!(s.warnings.iter().any(|w| w.message.contains("缺失权重")));
}

#[test]
fn default_ticks_and_text_paints_match_original_axis_and_lcss_contracts() {
    let s = scene(COMPLETE, 1);
    assert_eq!(texts(&s.nodes[0]).len(), 24);
    for value in ["5", "85", "95"] {
        assert!(texts(&s.nodes[0]).iter().any(|n| n["content"] == value));
    }
    let s = scene(COMPLETE, 27);
    assert!(texts(&s.nodes[0]).iter().any(|n| n["content"] == "1k"));
    let s = scene(COMPLETE, 20);
    for n in texts(&s.nodes[2]) {
        assert!(
            n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["kind"] == "glyph")
                .all(|r| r["color"] == "#222222")
        );
    }
    for (line, expected) in [(10, "#245447"), (100, "#444444")] {
        let s = scene("cases-1044744.jsonl", line);
        assert!(texts(&s.nodes[0]).iter().any(|n| {
            n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["kind"] == "glyph" && r["color"] == expected)
        }));
    }
    let s = scene("cases-1044744.jsonl", 89);
    let p = &s.nodes[1];
    for n in texts(p).iter().filter(|n| n["content"] != "Experiment") {
        assert!(
            n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["kind"] == "glyph")
                .all(|r| r["color"] == "#245447")
        );
    }
    assert_eq!(text(p, "Experiment")["runs"][0]["color"], "#222222");
    let s = scene("cases-1044744.jsonl", 83);
    assert!(texts(&s.nodes[0]).iter().any(|n| n["content"] == "1.2"));
}
