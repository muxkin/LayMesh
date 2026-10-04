#![cfg(feature = "native")]
//! Original script and unified-language assertions, using the captured inputs.
use laymesh_core::{
    engine::compile_source,
    model::{Host, PT, Scene, jnum},
    parser::{ExprKind, StmtKind, parse},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::{Value, json};
use std::{io::Cursor, path::PathBuf};

fn capture(file: &str, line: usize) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
    serde_json::from_str(
        std::fs::read_to_string(root.join(file))
            .unwrap()
            .lines()
            .nth(line - 1)
            .unwrap(),
    )
    .unwrap()
}
fn compile(file: &str, line: usize) -> laymesh_core::Result<Scene> {
    let c = capture(file, line);
    let mut host = Host::default();
    host.files.insert(
        "/test-font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
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
const SCRIPT: &str = "cases-1044594.jsonl";
const UNIFIED: &str = "cases-1044744.jsonl";
fn unified(line: usize) -> Scene {
    compile(UNIFIED, line).unwrap()
}
fn walk<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(node);
    if let Some(children) = node["children"].as_array() {
        for child in children {
            walk(child, out)
        }
    }
}
fn all(scene: &Scene) -> Vec<&Value> {
    let mut out = vec![];
    for n in &scene.nodes {
        walk(n, &mut out)
    }
    out
}
fn text(node: &Value) -> &Value {
    if node["kind"] == "text" {
        node
    } else {
        node["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["kind"] == "text")
            .unwrap()
    }
}
fn runs(node: &Value) -> &Vec<Value> {
    text(node)["runs"].as_array().unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn error(file: &str, line: usize) {
    let c = capture(file, line);
    let e = compile(file, line).unwrap_err();
    let expected = &c["result"];
    assert_eq!(e.code, expected["code"]);
    assert_eq!(e.file, expected["file"]);
    assert_eq!(e.loc.line as u64, expected["loc"]["line"].as_u64().unwrap());
    assert!(e.loc.column > 0);
}
fn exports(scene: &Scene) -> String {
    let svg = render_svg(scene).unwrap();
    assert!(render_pdf(scene).unwrap().len() > 1000);
    assert!(render_png(scene, 96.).unwrap().len() > 100);
    svg
}
fn raster(scene: &Scene, dpi: f64) -> (Vec<u8>, png::OutputInfo) {
    let mut r = png::Decoder::new(Cursor::new(render_png(scene, dpi).unwrap()))
        .read_info()
        .unwrap();
    let mut data = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut data).unwrap();
    (data, info)
}

#[test]
fn nested_groups_scale_opacity_and_dependent_anchors() {
    let a = compile(SCRIPT, 1).unwrap();
    let b = compile(SCRIPT, 2).unwrap();
    assert_eq!(a.nodes[0]["children"][0]["children"][0]["kind"], "group");
    assert_eq!(
        a.nodes[0]["children"][0]["children"][0]["children"][0]["geometryRecipe"]["kind"],
        "rect"
    );
    close(jnum(&a.nodes[1], "x", 0.), 32.);
    close(jnum(&b.nodes[1], "x", 0.), 42.);
    close(jnum(&a.nodes[0], "opacity", 0.), 0.5);
    close(
        jnum(&a.nodes[0], "width", 0.) / jnum(&a.nodes[0], "contentWidth", 0.),
        2.,
    );
    close(
        jnum(&a.nodes[0], "height", 0.) / jnum(&a.nodes[0], "contentHeight", 0.),
        2.,
    );
    let svg = render_svg(&a).unwrap();
    assert!(svg.contains("opacity=\"0.5\""));
    assert!(String::from_utf8_lossy(&render_pdf(&a).unwrap()).contains("/ExtGState"));
    let (data, info) = raster(&a, 96.);
    let x = (15_f64 * 96. / 25.4).round() as usize;
    let y = x;
    let channels = info.color_type.samples();
    let i = (y * info.width as usize + x) * channels;
    let p = &data[i..i + 3];
    assert!(
        p[0] > 240 && p[1] > 110 && p[1] < 150 && p[2] > 110 && p[2] < 150,
        "{p:?}"
    );
}
#[test]
fn variables_units_branches_loops_functions_and_scope() {
    let s = compile(SCRIPT, 3).unwrap();
    close(jnum(&s.nodes[1], "x", 0.), 14.);
    close(jnum(&s.nodes[1], "width", 0.), 22.54);
    // The archive used `local` as an undefined loop-local name. `local` is
    // now a predefined string option; keep the historical capture untouched
    // and verify the scope error using a name outside the built-in registry.
    assert!(compile(SCRIPT, 4).unwrap().nodes.is_empty());
    let case = capture(SCRIPT, 4);
    let source = case["source"]
        .as_str()
        .unwrap()
        .replace("local", "loop_value");
    let error =
        compile_source(&source, case["file"].as_str().unwrap(), Host::default()).unwrap_err();
    assert_eq!(error.code, "E_NAME");
    assert_eq!(error.loc.line, 3);
}
#[test]
fn transitive_component_assets_and_font_origins_survive_directory_moves() {
    for line in [5, 6] {
        let s = compile(SCRIPT, line).unwrap();
        assert_eq!(s.nodes.len(), 6);
        assert_eq!(s.nodes[0]["kind"], "group");
        assert_eq!(s.nodes[1]["kind"], "group");
        let label = &s.nodes[0]["children"][0]["children"][1];
        let font_path = runs(label)[0]["fontPath"].as_str().unwrap();
        assert!(font_path.ends_with(if line == 5 {
            "/组件/素材/DejaVuSans.ttf"
        } else {
            "/组件-moved/素材/DejaVuSans.ttf"
        }));
        for (i, mime) in [
            (2, "image/png"),
            (3, "image/svg+xml"),
            (4, "image/png"),
            (5, "image/png"),
        ] {
            assert_eq!(s.nodes[i]["mime"], mime)
        }
        let svg = exports(&s);
        assert!(svg.contains("<text "));
        assert!(svg.contains("data:image/svg+xml;base64"));
        assert!(svg.contains("data:font/ttf;base64"));
        assert!(svg.contains("data:font/otf;base64"));
        let pdf = render_pdf(&s).unwrap();
        let pdf = String::from_utf8_lossy(&pdf);
        assert!(pdf.contains("/FontFile2"));
        assert!(pdf.contains("/FontFile3"));
    }
}
#[test]
fn cross_file_resource_import_unit_execution_and_group_errors_remain_located() {
    for line in 7..=19 {
        error(SCRIPT, line)
    }
}

#[test]
fn geometry_units_data_units_and_typography_are_independent() {
    let s = unified(1);
    close(s.width, 150.);
    close(s.height, 100.);
    close(jnum(&s.nodes[0], "width", 0.), 20.);
    close(jnum(&s.nodes[0]["strokeStyle"], "width", 0.), PT);
    close(jnum(&s.nodes[1], "x", 0.), 50.8);
    close(jnum(&s.nodes[1], "y", 0.), 20.);
    close(jnum(&runs(&s.nodes[1])[0], "fontSize", 0.), 12. * PT);
    let ast = parse("page=canvas(size=(2in,2inch))", "/main.lay").unwrap();
    let StmtKind::Bind(_, expression, _) = &ast[0].kind else {
        panic!("binding")
    };
    let ExprKind::Call(_, args) = &expression.kind else {
        panic!("call")
    };
    // Both accepted aliases produce exactly the same physical unit conversion.
    let _ = args;
    let s = compile_source(
        "page=canvas(size=(2in,2inch))",
        "/main.lay",
        Host::default(),
    )
    .unwrap();
    close(s.width, 50.8);
    close(s.height, 50.8);
    error(UNIFIED, 2);
}
#[test]
fn automatic_raw_and_formatted_formula_strings_keep_distinct_semantics() {
    let s = unified(3);
    assert!(runs(&s.nodes[0]).iter().any(|r| r["kind"] == "formula"));
    assert!(runs(&s.nodes[1]).iter().all(|r| r["kind"] != "formula"));
    assert!(
        text(&s.nodes[2])["content"]
            .as_str()
            .unwrap()
            .contains("97.50%")
    );
    assert!(runs(&s.nodes[3]).iter().any(|r| r["kind"] == "formula"));
    error(UNIFIED, 4);
}
#[test]
fn instance_classes_cascade_and_explicit_overrides_are_independent() {
    let s = unified(5);
    for (n, size) in s.nodes.iter().zip([12., 16., 9.]) {
        close(jnum(&runs(n)[0], "fontSize", 0.), size * PT)
    }
    assert_eq!(s.nodes[0]["kind"], "group");
    assert_eq!(s.nodes[2]["kind"], "text");
}
#[test]
fn lcss_imports_variables_and_cycle_errors() {
    let s = unified(6);
    assert_eq!(runs(&s.nodes[0])[0]["color"], "#245447");
    for line in [7, 8, 9] {
        error(UNIFIED, line)
    }
}
#[test]
fn plot_part_styles_keep_the_data_rectangle_and_vector_math() {
    let s = unified(10);
    assert_eq!(
        s.nodes[0]["plotBounds"],
        json!({"x":20.,"y":10.,"width":80.,"height":55.})
    );
    let svg = exports(&s);
    assert!(svg.contains("<linearGradient"));
    assert!(svg.contains("data-latex-source"));
}
#[test]
fn removed_names_keep_migration_diagnostics() {
    for line in 11..=14 {
        error(UNIFIED, line)
    }
}
#[test]
fn independent_group_typography_reflows_named_anchor_dependencies() {
    let s = unified(15);
    let (a, b) = (&s.nodes[0], &s.nodes[1]);
    assert!(jnum(b, "contentWidth", 0.) > jnum(a, "contentWidth", 0.) * 1.7);
    for (node, size) in [(a, 8.), (b, 16.)] {
        close(
            jnum(&runs(&node["children"][0])[0], "fontSize", 0.),
            size * PT,
        );
        close(
            jnum(&node["children"][1], "x", 0.),
            jnum(&node["children"][0], "width", 0.) + 2.,
        )
    }
}
#[test]
fn formula_fragments_inherit_style_and_display_blocks_get_their_own_line() {
    let s = unified(16);
    let f = runs(&s.nodes[0])
        .iter()
        .find(|r| r["kind"] == "formula")
        .unwrap();
    // Preserve the size and inherited paint assertions across vector backends.
    assert!(jnum(f, "height", 0.) > 5.);
    assert!(
        f["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["fill"] == "#245447" || item["color"] == "#245447")
    );
    let r = runs(&s.nodes[1]);
    let f = r.iter().find(|r| r["kind"] == "formula").unwrap();
    assert!(jnum(f, "y", 0.) > jnum(&r[0], "baseline", 0.));
}
#[test]
fn all_region_paints_remain_vector_and_keep_physical_coordinates() {
    let s = unified(17);
    assert_eq!(
        s.nodes[0]["plotBounds"],
        json!({"x":15.,"y":10.,"width":75.,"height":45.})
    );
    let svg = exports(&s);
    assert!(svg.contains("<pattern"));
    assert!(svg.contains("<radialGradient"));
}
#[test]
fn formula_error_uses_the_label_location() {
    let e = compile(UNIFIED, 18).unwrap_err();
    assert_eq!(e.code, "E_FORMULA");
    assert_eq!(e.loc.line, 3);
    assert_eq!(e.loc.column, 17);
}
#[test]
fn nested_interpolation_quotes_raw_comparisons_and_shorthand_order() {
    let s = unified(19);
    assert_eq!(text(&s.nodes[0])["content"], "nested");
    assert_eq!(s.nodes[0]["children"][0]["strokeStyle"]["color"], "#004488");
    close(
        jnum(&s.nodes[0]["children"][0]["strokeStyle"], "width", 0.),
        2. * PT,
    );
    error(UNIFIED, 20);
}
#[test]
fn axis_background_compound_series_and_text_colors_are_independent() {
    let s = unified(21);
    let nodes = all(&s);
    assert!(nodes.iter().any(|n| n["id"] == "axis-background-x"));
    assert!(nodes.iter().any(
        |n| n["strokeStyle"]["color"] == "#245447" && n["strokeStyle"]["compound"] == "double"
    ));
    assert!(nodes.iter().any(|n| {
        n["runs"]
            .as_array()
            .is_some_and(|r| r.iter().any(|r| r["color"] == "#004488"))
    }));
    let e = compile(UNIFIED, 22).unwrap_err();
    assert!(e.message.contains("angle_width"));
}
#[test]
fn fusion_fill_does_not_require_a_border() {
    let s = unified(23);
    assert_eq!(s.nodes.len(), 1);
    assert_eq!(s.nodes[0]["strokeStyle"]["color"], "none");
}
#[test]
fn cross_hatch_diagonals_cover_tile_interiors() {
    let s = unified(24);
    let (data, info) = raster(&s, 254.);
    let channels = info.color_type.samples();
    let ink = data[..info.buffer_size()]
        .chunks(channels)
        .filter(|p| p[0] < 200)
        .count();
    assert!(ink as f64 / (info.width * info.height) as f64 > 0.15);
}
#[test]
fn spans_inherit_final_parent_styles_with_class_and_explicit_overrides() {
    let s = unified(25);
    let r = runs(&s.nodes[0]);
    close(jnum(&r[0], "fontSize", 0.), 18. * PT);
    assert_eq!(r[0]["color"], "#245447");
    assert!(
        r.iter()
            .any(|r| r["content"] == "Class" && r["color"] == "#004488")
    );
    assert!(r.iter().any(|r|r["content"]=="Explicit"&&(jnum(r,"fontSize",0.)-9.*PT).abs()<1e-9));
}
#[test]
fn all_original_shipped_examples_compile_with_positive_physical_dimensions() {
    for line in 26..=99 {
        let s = unified(line);
        assert!(s.width > 0. && s.height > 0., "example capture {line}")
    }
}
#[test]
fn plot_style_line_defaults_remain_independent_of_text_color() {
    let s = unified(100);
    assert!(all(&s).iter().any(|n| {
        n["strokeStyle"]["color"] == "#444444"
            && n["strokeStyle"]["dash"]
                .as_array()
                .is_some_and(|d| d.len() == 2)
    }));
}
