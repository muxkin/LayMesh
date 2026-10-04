#![cfg(feature = "native")]
mod plot_support;
use laymesh_render::render_svg;
use plot_support::*;
use serde_json::{Value as J, json};
fn title(p: &J) -> &J {
    p["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| {
            texts(n)
                .iter()
                .any(|t| t["content"].as_str().is_some_and(|s| s.starts_with("Time")))
                || n["source"] == "t_0^2"
        })
        .unwrap()
}
fn factor<'a>(p: &'a J, source: &str) -> &'a J {
    p["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "formula" && n["source"] == source)
        .unwrap()
}
fn parts(p: &J) -> Vec<&J> {
    vec![
        title(p),
        text(p, "Energy (J)"),
        factor(p, "\\times 10^{-6}"),
        factor(p, "\\times 10^{5}"),
    ]
}
#[test]
fn title_multiplier_rows_use_measured_boxes_and_physical_gap() {
    for line in 1..=3 {
        let s = scene(LABELS, line);
        let p = &s.nodes[0];
        let t = title(p);
        let f = factor(p, "\\times 10^{-6}");
        near(num(t, "x") + num(t, "width") / 2., 70.);
        near(num(f, "x") + num(f, "width"), 110.);
        if line == 2 {
            near(num(t, "y") - num(f, "y") - num(f, "height"), 1.2);
            near(num(t, "width"), 70.);
            assert!(num(t, "height") > 6.);
        } else {
            near(num(t, "y"), num(f, "y"));
        }
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    }
}
#[test]
fn offsets_move_only_the_named_title_or_multiplier() {
    for base_line in [4, 9] {
        let base = scene(LABELS, base_line);
        for (index, (dx, dy)) in [(2., -0.5), (-2., 3.), (2., -3.), (-1., 2.)]
            .into_iter()
            .enumerate()
        {
            let s = scene(LABELS, base_line + index + 1);
            let p = &s.nodes[0];
            let b = &base.nodes[0];
            assert_eq!(p["plotBounds"], b["plotBounds"]);
            assert_eq!(data(p), data(b));
            for (i, (new, old)) in parts(p).iter().zip(parts(b)).enumerate() {
                if i == index {
                    near(num(new, "x") - num(old, "x"), dx);
                    near(num(new, "y") - num(old, "y"), dy);
                } else {
                    assert_eq!(*new, old);
                }
            }
        }
    }
}
#[test]
fn overlapping_offsets_warn_per_placement_and_preserve_positions() {
    let base = scene(LABELS, 14);
    let b = &base.nodes[0];
    for axis in ["x", "y"] {
        let mut c = capture(LABELS, 14);
        let src = c["source"].as_str().unwrap();
        let t = if axis == "x" {
            title(b)
        } else {
            text(b, "Energy (J)")
        };
        let f = factor(
            b,
            if axis == "x" {
                "\\times 10^{-6}"
            } else {
                "\\times 10^{5}"
            },
        );
        let option = if axis == "x" {
            format!(
                "exponent=-6,exponent_offset=({} mm,0 mm)",
                num(t, "x") - num(f, "x")
            )
        } else {
            format!(
                "exponent=5,label_offset=({} mm,{} mm)",
                num(f, "x") + num(f, "width") / 2. - num(t, "x") - num(t, "width") / 2.,
                num(f, "y") + num(f, "height") / 2. - num(t, "y") - num(t, "height") / 2.
            )
        };
        let src = src.replace(
            if axis == "x" {
                "exponent=-6"
            } else {
                "exponent=5"
            },
            &option,
        );
        c["source"] = json!(format!("{src}\npage.add(p)"));
        let s = compile_capture(&c).unwrap();
        let warnings = s
            .warnings
            .iter()
            .filter(|w| w.message.contains(&format!("{axis} 轴标题与倍率重叠")))
            .collect::<Vec<_>>();
        assert_eq!(warnings.len(), 2);
        assert_ne!(warnings[0].loc.line, warnings[1].loc.line);
        if axis == "x" {
            assert_eq!(title(&s.nodes[0]), title(b));
            near(
                num(factor(&s.nodes[0], "\\times 10^{-6}"), "x"),
                num(title(b), "x"),
            );
            near(
                num(factor(&s.nodes[0], "\\times 10^{-6}"), "y"),
                num(f, "y"),
            );
        }
        assert_eq!(s.nodes[0]["children"], s.nodes[1]["children"]);
        assert_eq!(s.nodes[0]["plotBounds"], b["plotBounds"]);
    }
}
fn geometry(n: &J) -> J {
    let mut out = n.clone();
    if out["kind"] == "group" {
        out.as_object_mut().unwrap().remove("plotDecorations");
        let plot = out["plotBounds"].is_object();
        out["children"] = json!(
            n["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| !plot || c["kind"] != "text" && c["kind"] != "formula")
                .map(geometry)
                .collect::<Vec<_>>()
        );
    }
    out
}
#[test]
fn decorated_neighboring_panels_anchors_rotations_and_group_scaling_stay_fixed() {
    let a = scene(LABELS, 17);
    let b = scene(LABELS, 18);
    for s in [&a, &b] {
        let (l, r) = (&s.nodes[0], &s.nodes[1]);
        near(num(l, "width") / num(r, "width"), 2.);
        near(num(l, "height") / num(r, "height"), 2.);
        near(num(l, "rotation"), -10.);
        near(num(r, "rotation"), 20.);
        assert_eq!(l["children"], r["children"]);
        assert!(descendants(l).iter().any(|n| num(n, "rotation") == 15.));
        exports(s);
    }
    assert_eq!(
        a.nodes.iter().map(geometry).collect::<Vec<_>>(),
        b.nodes.iter().map(geometry).collect::<Vec<_>>()
    );
}
#[test]
fn automatic_rows_grow_margins_but_fixed_areas_warn_without_resizing() {
    let a = scene(LABELS, 19);
    let b = scene(LABELS, 20);
    assert!(a.warnings.is_empty() && b.warnings.is_empty());
    assert!(num(&a.nodes[0]["plotBounds"], "height") > num(&b.nodes[0]["plotBounds"], "height"));
    for line in [21, 22] {
        let s = scene(LABELS, line);
        assert_eq!(
            s.nodes[0]["plotBounds"],
            json!({"x":30.,"y":20.,"width":80.,"height":50.})
        );
        assert!(
            s.warnings
                .iter()
                .any(|w| w.code == "W_PLOT_LAYOUT" && w.message.contains("不足"))
        );
    }
    assert!(
        scene(LABELS, 23)
            .warnings
            .iter()
            .any(|w| w.message.contains("标题偏移后超出图表外框"))
    );
}
#[test]
fn log_axes_title_offsets_preserve_data_and_reject_malformed_values() {
    let a = scene(LABELS, 24);
    let b = scene(LABELS, 25);
    assert_eq!(data(&a.nodes[0]), data(&b.nodes[0]));
    assert_eq!(a.nodes[1], b.nodes[1]);
    near(
        num(title(&b.nodes[0]), "x") - num(title(&a.nodes[0]), "x"),
        -2.,
    );
    near(
        num(title(&b.nodes[0]), "y") - num(title(&a.nodes[0]), "y"),
        1.,
    );
    for line in 26..=30 {
        failure(LABELS, line);
    }
}
fn ink_bounds(r: &Raster) -> [usize; 4] {
    let mut out = [usize::MAX, 0, usize::MAX, 0];
    let mut count = 0;
    let scale = r.dpi / 25.4;
    for y in (76. * scale).ceil() as usize..(89. * scale) as usize {
        for x in (35. * scale).ceil() as usize..(112. * scale) as usize {
            if r.bytes[(y * r.width + x) * r.channels] < 150 {
                count += 1;
                out[0] = out[0].min(x);
                out[1] = out[1].max(x);
                out[2] = out[2].min(y);
                out[3] = out[3].max(y);
            }
        }
    }
    assert!(count > 100);
    out
}
#[test]
fn rich_title_and_multiplier_export_semantics_and_bounds_match_pdf() {
    let s = scene(LABELS, 31);
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("data-latex-source=\"t_0^2\""));
    assert!(svg.contains("data-latex-source=\"\\times 10^{-6}\""));
    let text = pdf_text(&s);
    assert!(text.contains("Time") && text.contains("t_0^2"));
    let a = ink_bounds(&raster(&s, 180.));
    let b = ink_bounds(&pdf_raster(&s, 180.));
    for (a, b) in a.into_iter().zip(b) {
        assert!(a.abs_diff(b) <= 2, "{a} != {b}");
    }
}
