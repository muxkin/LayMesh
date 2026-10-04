#![cfg(feature = "native")]
mod plot_support;
use plot_support::*;
use serde_json::{Value as J, json};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-8
}
fn legend_contract(n: &J, gap: f64, padding: f64, sample_gap: f64) -> bool {
    let children = n["children"].as_array().unwrap();
    let labels = texts(n);
    if labels.len() != 2 {
        return false;
    }
    let marker = children
        .iter()
        .filter(|n| n["kind"] == "points")
        .collect::<Vec<_>>();
    if marker.len() != 1 {
        return false;
    }
    let m = marker[0];
    if m["marker"] != "diamond"
        || !close(num(m, "markerSize"), 8.)
        || m["markerFill"] != "none"
        || m["markerStroke"] != "#0000ff"
        || !close(num(m, "markerStrokeWidth"), 0.4)
        || !close(num(m, "pointOpacity"), 0.5)
    {
        return false;
    }
    if !close(m["positions"][0].as_f64().unwrap(), padding + 4.)
        || !close(m["positions"][1].as_f64().unwrap(), padding + 4.)
    {
        return false;
    }
    for (i, label) in [text(n, "Line"), text(n, "Bars")].into_iter().enumerate() {
        if !close(num(label, "x"), padding + 8. + sample_gap)
            || !close(
                num(label, "y") + num(label, "height") / 2.,
                padding + 4. + i as f64 * (8. + gap),
            )
        {
            return false;
        }
    }
    let label_width = labels.iter().map(|n| num(n, "width")).fold(0., f64::max);
    if !close(
        num(n, "width"),
        padding * 2. + 8. + sample_gap + label_width,
    ) || !close(num(n, "height"), padding * 2. + 16. + gap)
    {
        return false;
    }
    let fills = children
        .iter()
        .filter(|n| n["kind"] == "path" && n["fill"] == "#aabbcc")
        .collect::<Vec<_>>();
    if fills.len() != 1 || !close(num(fills[0], "opacity"), 0.25) {
        return false;
    }
    let fill_vertices = vertices(fills[0]);
    if fill_vertices
        != vec![
            [padding, padding + 11. + gap],
            [padding + 8., padding + 11. + gap],
            [padding + 8., padding + 13. + gap],
            [padding, padding + 13. + gap],
        ]
    {
        return false;
    }
    let hatch = children
        .iter()
        .filter(|n| n["kind"] == "group" && n["clip"].is_object())
        .collect::<Vec<_>>();
    if hatch.len() != 1 {
        return false;
    }
    let hatch = hatch[0];
    if !close(num(hatch, "x"), padding)
        || !close(num(hatch, "y"), padding + 11. + gap)
        || !close(num(&hatch["clip"], "width"), 8.)
        || !close(num(&hatch["clip"], "height"), 2.)
    {
        return false;
    }
    let ink = paths(hatch);
    if ink.is_empty()
        || !ink
            .iter()
            .all(|n| close(num(n, "opacity"), 0.25) && n["strokeStyle"]["color"] == "#222222")
    {
        return false;
    }
    let mut slopes = std::collections::BTreeSet::new();
    for p in ink {
        for q in vertices(p).chunks_exact(2) {
            if q.iter()
                .any(|p| p[0] < -1e-8 || p[0] > 8. + 1e-8 || p[1] < -1e-8 || p[1] > 2. + 1e-8)
            {
                return false;
            }
            let dx = q[1][0] - q[0][0];
            let dy = q[1][1] - q[0][1];
            if dx.abs() > 1e-9 {
                if !close(dx.abs(), dy.abs()) {
                    return false;
                }
                slopes.insert(if dy / dx > 0. { 1 } else { -1 });
            }
        }
    }
    slopes.len() == 2
}
fn scenario(options: &str) -> laymesh_core::model::Scene {
    source(&format!(
        r##"p=plot(size=(120 mm,95 mm),plot_area=box(offset=(15 mm,10 mm),size=(70 mm,50 mm)),style=s,x=axis(range=(0,2)),y=axis(range=(0,3)))
p.line(x=[0,1,2],y=[1,2,1],label="Line",marker="diamond",marker_size=8 mm,marker_fill="none",marker_border_color="#0000ff",marker_border_width=0.4 mm,opacity=0.5)
p.bar(positions=[1],values=[2],label="Bars",fill="#aabbcc",border_color="#222222",hatch="cross",hatch_spacing=1.5 mm,opacity=0.25)
p.legend(position=(0 mm,55 mm){options})
page.add(p)"##
    ))
}
#[test]
fn legend_marker_hatch_and_default_layout_have_physical_contracts() {
    for (options, gap, padding, sample_gap) in [
        ("", 1.2, 1.2, 3.),
        (",gap=3 mm", 3., 3., 3.),
        (",gap=3 mm,padding=2 mm,sample_gap=5 mm", 3., 2., 5.),
    ] {
        let s = scenario(options);
        assert!(
            legend_contract(legend(&s.nodes[0]), gap, padding, sample_gap),
            "{options}"
        );
        exports(&s);
    }
}
#[test]
fn legend_contract_rejects_missing_marker_hatch_and_old_spacing_defaults() {
    let s = scenario(",gap=3 mm");
    let original = legend(&s.nodes[0]);
    assert!(legend_contract(original, 3., 3., 3.));
    let mut missing_marker = original.clone();
    missing_marker["children"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["kind"] != "points");
    assert!(!legend_contract(&missing_marker, 3., 3., 3.));
    let mut solid_marker = original.clone();
    for n in solid_marker["children"].as_array_mut().unwrap() {
        if n["kind"] == "points" {
            n["markerFill"] = json!("#0000ff");
        }
    }
    assert!(!legend_contract(&solid_marker, 3., 3., 3.));
    let mut missing_hatch = original.clone();
    missing_hatch["children"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["kind"] != "group");
    assert!(!legend_contract(&missing_hatch, 3., 3., 3.));
    let mut old_padding = original.clone();
    for n in old_padding["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .skip(1)
    {
        n["x"] = json!(num(n, "x") - 1.8);
        n["y"] = json!(num(n, "y") - 1.8);
    }
    old_padding["width"] = json!(num(original, "width") - 3.6);
    old_padding["height"] = json!(num(original, "height") - 3.6);
    assert!(!legend_contract(&old_padding, 3., 3., 3.));
    let mut old_sample_gap = original.clone();
    for n in old_sample_gap["children"].as_array_mut().unwrap() {
        if n["kind"] == "text" {
            n["x"] = json!(num(n, "x") - 1.8);
        }
    }
    assert!(!legend_contract(&old_sample_gap, 3., 3., 3.));
    let mut old_row_gap = original.clone();
    for n in old_row_gap["children"].as_array_mut().unwrap() {
        if n["kind"] == "text" && n["content"] == "Bars" {
            n["y"] = json!(num(n, "y") - 1.8);
        }
    }
    assert!(!legend_contract(&old_row_gap, 3., 3., 3.));
}
fn axis_colors(p: &J, wanted: &str) -> bool {
    let labels = texts(p)
        .into_iter()
        .filter(|n| n["content"] != "")
        .collect::<Vec<_>>();
    labels.len() == 6
        && labels.iter().all(|n| {
            let glyphs = n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["kind"] == "glyph")
                .collect::<Vec<_>>();
            !glyphs.is_empty() && glyphs.iter().all(|r| r["color"] == wanted)
        })
}
#[test]
fn axis_color_contract_rejects_black_text_and_line_color_leaking_into_explicit_text() {
    for (spec, wanted, wrong) in [
        ("line_color=\"#245447\"", "#245447", "#000000"),
        (
            "color=\"#004488\",line_color=\"#555555\"",
            "#004488",
            "#555555",
        ),
    ] {
        let s = source(&format!(
            r#"p=plot(size=(100 mm,80 mm),style=s,x=axis(range=(0,1),ticks=[0,1],label="X",{spec}),y=axis(range=(0,1),ticks=[0,1],label="Y",{spec}))
p.line(x=[0,1],y=[0,1])
page.add(p)"#
        ));
        let p = &s.nodes[0];
        assert!(axis_colors(p, wanted));
        fn recolor(n: &mut J, color: &str) {
            for r in n["runs"].as_array_mut().into_iter().flatten() {
                if r["kind"] == "glyph" {
                    r["color"] = json!(color);
                }
            }
            for c in n["children"].as_array_mut().into_iter().flatten() {
                recolor(c, color)
            }
        }
        let mut bad = p.clone();
        recolor(&mut bad, wrong);
        assert!(!axis_colors(&bad, wanted));
    }
}
