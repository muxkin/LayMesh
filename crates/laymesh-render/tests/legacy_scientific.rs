#![cfg(feature = "native")]
//! Scientific plot contracts ported from all fourteen original tests.
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene, jnum},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::{Value, json};
use std::{
    io::Cursor,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
fn capture(line: usize) -> Value {
    serde_json::from_str(
        include_str!("../../../migration/corpus/cases-1044586.jsonl")
            .lines()
            .nth(line - 1)
            .unwrap(),
    )
    .unwrap()
}
fn compile(line: usize) -> laymesh_core::Result<Scene> {
    let c = capture(line);
    let mut host = Host::default();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus/blobs");
    for (name, hash) in c["files"].as_object().unwrap() {
        host.files.insert(
            name.clone(),
            std::fs::read(root.join(hash.as_str().unwrap())).unwrap(),
        );
    }
    compile_source(
        &laymesh_core::migration::migrate_arrows(c["source"].as_str().unwrap()),
        c["file"].as_str().unwrap(),
        host,
    )
}
fn scene(line: usize) -> Scene {
    let s = compile(line).unwrap_or_else(|e| panic!("capture {line}: {e:?}"));

    s
}
fn children(n: &Value) -> &Vec<Value> {
    n["children"].as_array().unwrap()
}
fn data(n: &Value) -> &Value {
    children(n).iter().find(|n| n["id"] == "plot-data").unwrap()
}
fn descendants(n: &Value) -> Vec<&Value> {
    let mut out = vec![n];
    for c in n["children"].as_array().into_iter().flatten() {
        out.extend(descendants(c));
    }
    out
}
fn legend(n: &Value) -> &Value {
    children(n)
        .iter()
        .find(|n| {
            n["kind"] == "group"
                && descendants(n).iter().any(|c| c["kind"] == "text")
                && n["id"] != "plot-data"
        })
        .unwrap()
}
fn bar(n: &Value) -> Value {
    fn find(n: &Value, x: f64, y: f64) -> Option<Value> {
        let x = x + jnum(n, "x", 0.);
        let y = y + jnum(n, "y", 0.);
        if n["kind"] == "path" && n["fill"].is_object() {
            let mut c = n.clone();
            c["x"] = json!(x);
            c["y"] = json!(y);
            return Some(c);
        }
        for c in n["children"].as_array().into_iter().flatten() {
            if let Some(b) = find(c, x, y) {
                return Some(b);
            }
        }
        None
    }
    find(n, 0., 0.).expect("colorbar gradient path")
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn formulas(n: &Value) -> Vec<&str> {
    let mut out = vec![];
    if n["kind"] == "formula" {
        out.push(n["source"].as_str().unwrap());
    }
    for r in n["runs"].as_array().into_iter().flatten() {
        if r["kind"] == "formula" {
            out.push(r["source"].as_str().unwrap());
        }
    }
    for c in n["children"].as_array().into_iter().flatten() {
        out.extend(formulas(c));
    }
    out
}
fn texts(n: &Value) -> Vec<&str> {
    descendants(n)
        .into_iter()
        .filter(|n| n["kind"] == "text")
        .map(|n| n["content"].as_str().unwrap())
        .collect()
}
fn exports(s: &Scene) -> String {
    let svg = render_svg(s).unwrap();
    assert!(!svg.contains("NaN") && !svg.contains("Infinity"));
    assert!(render_pdf(s).unwrap().len() > 1000);
    assert!(render_png(s, 96.).unwrap().len() > 100);
    svg
}
struct Raster {
    bytes: Vec<u8>,
    width: usize,
    channels: usize,
}
impl Raster {
    fn read(bytes: Vec<u8>) -> Self {
        let mut r = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
        let mut bytes = vec![0; r.output_buffer_size().unwrap()];
        let info = r.next_frame(&mut bytes).unwrap();
        Self {
            bytes,
            width: info.width as usize,
            channels: info.color_type.samples(),
        }
    }
    fn pixel(&self, x: f64, y: f64) -> &[u8] {
        let i =
            ((y * 10.).round() as usize * self.width + (x * 10.).round() as usize) * self.channels;
        &self.bytes[i..i + 3]
    }
}
static SERIAL: AtomicUsize = AtomicUsize::new(0);
fn pdf_image(s: &Scene) -> Raster {
    let dir = std::env::temp_dir().join(format!(
        "laymesh-science-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = dir.join("out.pdf");
    std::fs::write(&pdf, render_pdf(s).unwrap()).unwrap();
    let run = Command::new("pdftoppm")
        .args(["-r", "254", "-singlefile", "-png"])
        .arg(pdf)
        .arg(dir.join("out"))
        .output()
        .expect("Poppler pdftoppm is required for PDF pixel contract tests");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let r = Raster::read(std::fs::read(dir.join("out.png")).unwrap());
    std::fs::remove_dir_all(dir).unwrap();
    r
}
fn pdf_text(s: &Scene) -> String {
    let dir = std::env::temp_dir().join(format!(
        "laymesh-scientific-text-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("out.pdf");
    std::fs::write(&p, render_pdf(s).unwrap()).unwrap();
    let run = Command::new("pdftotext")
        .arg(p)
        .arg("-")
        .output()
        .expect("Poppler pdftotext is required");
    assert!(run.status.success());
    std::fs::remove_dir_all(dir).unwrap();
    String::from_utf8(run.stdout).unwrap()
}
#[test]
fn hollow_points_retain_outer_size_and_individual_opacity_in_png_pdf() {
    let s = scene(1);
    assert_eq!(s.schema_version, 8);
    for r in [Raster::read(render_png(&s, 254.).unwrap()), pdf_image(&s)] {
        assert_eq!(r.pixel(40., 40.), [255, 0, 0]);
        assert!(r.pixel(40., 35.5).iter().all(|&c| (126..=129).contains(&c)));
        assert!(r.pixel(60., 35.5).iter().all(|&c| (62..=65).contains(&c)));
        assert_eq!(r.pixel(40., 34.8), [255, 255, 255]);
    }
}
#[test]
fn line_errorbar_legend_samples_share_marker_paint_and_order() {
    let s = scene(2);
    let p = &s.nodes[0];
    let batches: Vec<_> = descendants(data(p))
        .into_iter()
        .filter(|n| n["kind"] == "points")
        .collect();
    let samples: Vec<_> = children(legend(p))
        .iter()
        .filter(|n| n["kind"] == "points")
        .collect();
    assert_eq!(batches.len(), 3);
    assert_eq!(samples.len(), 3);
    assert_eq!(
        batches
            .iter()
            .map(|n| n["marker"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["diamond", "triangle_down", "square"]
    );
    for i in 0..3 {
        for key in [
            "marker",
            "markerSize",
            "markerFill",
            "markerStroke",
            "markerStrokeWidth",
            "pointOpacity",
        ] {
            assert!(batches[i].get(key).is_some(), "missing {key}");
            assert_eq!(batches[i][key], samples[i][key], "{key}");
        }
    }
    assert_eq!(texts(p).iter().filter(|&&t| t == "Same").count(), 2);
    let index = children(legend(p))
        .iter()
        .position(|n| std::ptr::eq(n, samples[1]))
        .unwrap();
    assert_eq!(
        children(legend(p))[index - 1]["d"]
            .as_str()
            .unwrap()
            .matches('M')
            .count(),
        6
    );
    assert!(exports(&s).contains("stroke-linejoin=\"round\""));
}
#[test]
fn polygon_marker_ink_stays_within_size_and_errorbar_directions() {
    let s = scene(3);
    let p = &s.nodes[0];
    assert_eq!(
        children(legend(p))
            .iter()
            .filter(|n| n["kind"] == "path" && n["d"].as_str().unwrap().matches('M').count() == 3)
            .count(),
        2
    );
    assert_eq!(
        children(p).iter().filter(|n| n["kind"] == "points").count(),
        0
    );
    for r in [Raster::read(render_png(&s, 254.).unwrap()), pdf_image(&s)] {
        for i in 1..=4 {
            let cx = (20 + i * 22) * 10;
            let cy = 450;
            let mut points = vec![];
            for y in cy - 55..=cy + 55 {
                for x in cx - 55..=cx + 55 {
                    if r.bytes[(y as usize * r.width + x as usize) * r.channels] < 180 {
                        points.push([x - cx, y - cy]);
                    }
                }
            }
            assert!(points.len() > 100);
            for axis in 0..2 {
                assert!(points.iter().map(|q| q[axis]).min().unwrap() >= -50);
                assert!(points.iter().map(|q| q[axis]).max().unwrap() <= 50);
            }
            assert_eq!(r.pixel(cx as f64 / 10., 45.)[0], 255);
        }
    }
}
fn legacy_points(n: &mut Value) {
    if n["kind"] == "points" {
        for k in ["markerFill", "markerStroke", "markerStrokeWidth"] {
            n.as_object_mut().unwrap().remove(k);
        }
    }
    for c in n["children"].as_array_mut().into_iter().flatten() {
        legacy_points(c);
    }
}
#[test]
fn legacy_solid_point_defaults_match_all_three_exports() {
    let current = scene(4);
    let mut legacy = current.clone();
    legacy.schema_version = 4;
    for n in &mut legacy.nodes {
        legacy_points(n);
    }
    assert_eq!(render_svg(&current).unwrap(), render_svg(&legacy).unwrap());
    assert_eq!(
        render_png(&current, 120.).unwrap(),
        render_png(&legacy, 120.).unwrap()
    );
    assert_eq!(pdf_image(&current).bytes, pdf_image(&legacy).bytes);
}
#[test]
fn rich_labels_keep_module_fonts_formula_sizes_and_pdf_sources() {
    let s = scene(5);
    let a = &s.nodes[0];
    let b = &s.nodes[1];
    assert_eq!(a["plotBounds"], b["plotBounds"]);
    assert_eq!(data(a), data(b));
    let rich: Vec<_> = descendants(a)
        .into_iter()
        .filter(|n| n["kind"] == "text" && n["content"].as_str().unwrap().contains("Signal"))
        .collect();
    assert_eq!(rich.len(), 3);
    for n in rich {
        near(n["runs"][0]["fontSize"].as_f64().unwrap(), 11. * 25.4 / 72.);
        assert!(formulas(n).contains(&"\\alpha_2"));
        let f = n["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "formula")
            .unwrap();
        let expected = compile_source(
            r#"page=canvas(size=(30,20))
page.add(formula(source=r"\alpha_2",font_size=12pt))"#,
            "/formula-size.lay",
            Host::default(),
        )
        .unwrap();
        near(
            f["width"].as_f64().unwrap(),
            expected.nodes[0]["width"].as_f64().unwrap(),
        );
        near(
            f["height"].as_f64().unwrap(),
            expected.nodes[0]["height"].as_f64().unwrap(),
        );
    }
    assert!(formulas(a).contains(&"\\beta^2"));
    assert!(exports(&s).contains("data-latex-source"));
    let txt = pdf_text(&s);
    for q in ["Signal", "\\alpha_2", "\\beta^2"] {
        assert!(txt.contains(q), "{q}: {txt}");
    }
}
#[test]
fn scientific_offset_labels_preserve_domains_and_physical_anchors() {
    let scenes = [scene(6), scene(7)];
    for s in &scenes {
        let p = &s.nodes[0];
        let content = texts(p);
        for label in ["−1.23×10^-3", "0", "1.00×10^-2", "1.0", "2.0"] {
            assert!(content.contains(&label), "{label}: {content:?}");
        }
        assert!(formulas(p).contains(&"\\times 10^{6}"));
        assert_eq!(p["plotAxes"]["y"]["domain"], json!([0., 2000000.]));
        near(jnum(&s.nodes[1], "y", 0.) + 0.5, 45.);
        near(jnum(&s.nodes[1], "x", 0.) + 0.5, 30. + 80. * 0.002 / 0.013);
        exports(s);
    }
    assert_eq!(
        scenes[0].nodes[0]["plotBounds"],
        scenes[1].nodes[0]["plotBounds"]
    );
    assert_eq!(scenes[0].nodes[1], scenes[1].nodes[1]);
    let f = |s: &Scene| {
        children(&s.nodes[0])
            .iter()
            .find(|n| n["kind"] == "formula")
            .unwrap()
            .clone()
    };
    let a = f(&scenes[0]);
    let b = f(&scenes[1]);
    near(jnum(&b, "x", 0.) - jnum(&a, "x", 0.), 3.);
    near(jnum(&b, "y", 0.) - jnum(&a, "y", 0.), -2.);
}
#[test]
fn extreme_finite_scientific_ticks_and_explicit_exponents() {
    for (line, exponent) in [(8, -300), (9, 300)] {
        let s = scene(line);
        assert!(formulas(&s.nodes[0]).contains(&format!("\\times 10^{{{exponent}}}").as_str()));
        exports(&s);
    }
}
#[test]
fn scientific_format_preserves_accounting_and_exponent_padding() {
    let s = scene(10);
    let p = &s.nodes[0];
    let content = texts(p);
    assert!(content.contains(&"(1.00×10^0)"));
    assert!(content.contains(&"1.00×10^000000"));
    assert!(formulas(p).iter().all(|&s| s == "\\times 10^{0}"));
}
#[test]
fn every_colorbar_side_keeps_fixed_data_and_normalization() {
    let mut reference = None;
    for line in 11..=16 {
        let s = scene(line);
        let a = &s.nodes[0];
        let b = &s.nodes[1];
        assert_eq!(
            a["plotBounds"],
            json!({"x":40.,"y":30.,"width":60.,"height":50.})
        );
        assert_eq!(a["plotBounds"], b["plotBounds"]);
        if let Some(ref r) = reference {
            assert_eq!(data(a), r);
        } else {
            reference = Some(data(a).clone());
        }
        let c = bar(a);
        let h = [13, 14, 16].contains(&line);
        near(jnum(&c, "width", 0.), if h { 30. } else { 4. });
        near(jnum(&c, "height", 0.), if h { 4. } else { 30. });
        assert_eq!(
            c["fill"]["stops"][0]["color"],
            if h { "#000000" } else { "#ffffff" }
        );
        assert_eq!(
            c["fill"]["stops"].as_array().unwrap().last().unwrap()["color"],
            if h { "#ffffff" } else { "#000000" }
        );
        assert_eq!(c, bar(b));
        let x = jnum(&c, "x", 0.);
        let y = jnum(&c, "y", 0.);
        match line {
            11 => near(x, 102.4),
            12 => assert!(x + 4. < 40.),
            13 => assert!(y + 4. < 30.),
            14 => assert!(y > 80.),
            15 => {
                near(x, 110.);
                near(y, 30.);
            }
            16 => {
                near(x, 40.);
                near(y, 95.);
            }
            _ => unreachable!(),
        }
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);
        exports(&s);
    }
}
#[test]
fn side_colorbars_reserve_margins_manual_bars_warn_per_instance() {
    let unadorned = scene(17).nodes[0]["plotBounds"].clone();
    for line in 18..=22 {
        let s = scene(line);
        let p = &s.nodes[0];
        if line == 22 {
            assert_eq!(p["plotBounds"], unadorned);
            let w: Vec<_> = s
                .warnings
                .iter()
                .filter(|w| w.message.contains("手动色标超出"))
                .collect();
            assert_eq!(w.len(), 2);
            assert_ne!(w[0].loc.line, w[1].loc.line);
        } else {
            assert!(s.warnings.is_empty(), "{:?}", s.warnings);
            for n in children(p).iter().filter(|n| n["kind"] == "text") {
                assert!(
                    jnum(n, "x", 0.) + jnum(n, "width", 0.) / 2. >= 0.
                        && jnum(n, "y", 0.) + jnum(n, "height", 0.) / 2. >= 0.
                );
            }
            if line == 18 {
                assert!(jnum(&p["plotBounds"], "x", 0.) > jnum(&unadorned, "x", 0.));
            }
            if line == 20 {
                assert!(jnum(&p["plotBounds"], "height", 0.) < jnum(&unadorned, "height", 0.));
            }
        }
    }
}
#[test]
fn multiline_colorbar_labels_and_pixels_match_heatmap_normalization() {
    for line in [23, 24] {
        let s = scene(line);
        let p = &s.nodes[0];
        let b = bar(p);
        assert_eq!(
            p["plotBounds"],
            json!({"x":30.,"y":20.,"width":70.,"height":50.})
        );
        assert!(formulas(p).contains(&"\\times 10^{6}"));
        assert!(texts(p).contains(&"−1.0"));
        let labels: Vec<_> = descendants(p)
            .into_iter()
            .filter(|n| n["kind"] == "text" && n["content"].as_str().unwrap().contains("Signal"))
            .collect();
        assert_eq!(labels.len(), 2);
        for n in labels {
            near(jnum(n, "width", 0.), 18.);
        }
        for r in [Raster::read(render_png(&s, 254.).unwrap()), pdf_image(&s)] {
            let a = r.pixel(
                jnum(&b, "x", 0.) + jnum(&b, "width", 0.) / 2.,
                jnum(&b, "y", 0.) + jnum(&b, "height", 0.) / 2.,
            )[0];
            let b = r.pixel(82.5, 32.5)[0];
            assert!((a as i32 - b as i32).abs() <= 3, "{a} != {b}");
        }
    }
}
#[test]
fn plot_labels_and_manual_colorbar_follow_nested_rotation_and_scale() {
    let s = scene(25);
    let g = &s.nodes[0];
    let p = &children(g)[0];
    let dot = &children(g)[1];
    assert_eq!(
        p["plotBounds"],
        json!({"x":20.,"y":20.,"width":60.,"height":40.})
    );
    near(
        jnum(dot, "x", 0.) + 0.5,
        jnum(p, "x", 0.) + jnum(p, "width", 0.) / 2.,
    );
    near(
        jnum(dot, "y", 0.) + 0.5,
        jnum(p, "y", 0.) + jnum(p, "height", 0.) / 2.,
    );
    near(jnum(g, "width", 0.) / jnum(g, "contentWidth", 0.), 1.75);
    near(jnum(g, "height", 0.) / jnum(g, "contentHeight", 0.), 1.75);
    near(jnum(g, "rotation", 0.), 10.);
    assert!(exports(&s).contains("rotate(90)"));
}
#[test]
fn decorations_leave_fixed_adjacent_panels_and_anchors_unchanged() {
    let a = scene(26);
    let b = scene(27);
    for i in 0..2 {
        for key in [
            "x",
            "y",
            "width",
            "height",
            "rotation",
            "plotBounds",
            "plotAxes",
        ] {
            assert!(a.nodes[i].get(key).is_some());
            assert_eq!(a.nodes[i][key], b.nodes[i][key], "{key}");
        }
    }
    assert_eq!(a.nodes[2..], b.nodes[2..]);
}
#[test]
fn scientific_and_colorbar_errors_are_located_and_overflow_keeps_geometry() {
    for line in 28..=45 {
        let c = capture(line);
        let e = compile(line).unwrap_err();
        assert_eq!(e.file, c["file"]);
        assert_eq!(e.loc.line, if line <= 34 { 3 } else { 5 });
    }
    let s = scene(46);
    assert_eq!(
        s.nodes[0]["plotBounds"],
        json!({"x":10.,"y":10.,"width":50.,"height":35.})
    );
    assert!(s.warnings.iter().any(|w| w.code == "W_PLOT_LAYOUT"));
    near(jnum(&bar(&s.nodes[0]), "width", 0.), 150.);
}
