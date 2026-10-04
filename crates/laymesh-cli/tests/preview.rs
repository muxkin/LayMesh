use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn preview(requests: &[Value]) -> Vec<Value> {
    let mut process = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .args(["preview", "--stdio"])
        .env("LAYMESH_NO_SYSTEM_FONTS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn preview_units_overlays_failed_dependencies_and_recovery() {
    let requests: Vec<Value> = ["mm","cm","in","inch","pt","px"].iter().enumerate().map(|(id,unit)|json!({"id":id,"file":"/fixture/main.lay","source":format!("page=canvas(size=(100,80),unit=\"{unit}\",layout_dpi=144)\nimport {{width}} from \"./values.lay\"\npage.add(rect(size=(width,2),fill=\"#ff0000\"))"),"overlays":{"/fixture/values.lay":"export width=3"}})).collect();
    let mut requests = requests;
    requests.push(json!({"id":6,"file":"/fixture/main.lay","source":"page=canvas(size=(10,10))\nimport {x} from \"./missing.lay\""}));
    requests.push(json!({"id":7,"file":"/fixture/main.lay","source":"page=canvas(size=(10,10))\nimport {x} from \"./missing.lay\"","overlays":{"/fixture/missing.lay":"export x=1"}}));
    let results = preview(&requests);
    assert_eq!(results[0]["protocol"], 1);
    for (i, unit) in ["mm", "cm", "in", "inch", "pt", "px"].iter().enumerate() {
        let r = &results[i + 1];
        let factor = [1., 10., 25.4, 25.4, 25.4 / 72., 25.4 / 144.][i];
        assert_eq!(r["id"], i);
        assert_eq!(r["inspection"]["page"]["unit"], *unit);
        assert_eq!(r["inspection"]["page"]["layout_dpi"], 144.);
        assert_eq!(r["inspection"]["units"], "mm");
        assert!((r["inspection"]["page"]["width"].as_f64().unwrap() - 100. * factor).abs() < 1e-8);
        assert!((r["inspection"]["page"]["height"].as_f64().unwrap() - 80. * factor).abs() < 1e-8);
        assert!(r["svg"].as_str().unwrap().contains("#ff0000"));
        assert!(
            r["dependencies"]
                .as_array()
                .unwrap()
                .contains(&json!("/fixture/values.lay"))
        );
    }
    assert_eq!(results[7]["error"]["code"], "E_ASSET");
    assert!(
        results[7]["dependencies"]
            .as_array()
            .unwrap()
            .contains(&json!("/fixture/missing.lay"))
    );
    assert!(results[8]["svg"].is_string());
}

#[test]
fn mixed_lengths_and_layout_pixels_keep_physical_inspection_dimensions() {
    for dpi in [72., 96., 144., 300.] {
        let source = format!(
            "page=canvas(size=(10cm,80mm),unit=\"px\",layout_dpi={dpi})\npage.add(rect(size=(1in,72pt)),offset=(2cm,10mm))"
        );
        let scene =
            laymesh_core::engine::compile_source(&source, "/fixture/main.lay", Default::default())
                .unwrap();
        let page = &laymesh_language::inspect::inspect_scene(&scene)["page"];
        assert_eq!(page["unit"], "px");
        assert_eq!(page["layout_dpi"], dpi);
        assert_eq!(page["width"], 100.);
        assert_eq!(page["height"], 80.);
        assert_eq!(scene.nodes[0]["x"], 20.);
        assert_eq!(scene.nodes[0]["y"], 10.);
        assert!((scene.nodes[0]["width"].as_f64().unwrap() - 25.4).abs() < 1e-8);
        assert!((scene.nodes[0]["height"].as_f64().unwrap() - 25.4).abs() < 1e-8);
    }
}

#[test]
fn scene_unit_is_additive_and_old_scenes_default_to_mm() {
    let scene = laymesh_core::engine::compile_source(
        "page=canvas(size=(4,3),unit=\"cm\")",
        "/fixture/main.lay",
        Default::default(),
    )
    .unwrap();
    let mut json = serde_json::to_value(scene).unwrap();
    assert_eq!(json["canvasUnit"], "cm");
    json.as_object_mut().unwrap().remove("canvasUnit");
    let old: laymesh_core::model::Scene = serde_json::from_value(json).unwrap();
    assert_eq!(old.canvas_unit, "mm");
}

#[test]
fn native_resources_and_unsaved_data_are_journaled() {
    let root =
        std::env::temp_dir().join(format!("laymesh-preview-resources-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    std::fs::write(root.join("style.lcss"), "rect {fill: #ee2211;}").unwrap();
    std::fs::write(
        root.join("photo.png"),
        include_bytes!("../../../examples/assets/photo.png"),
    )
    .unwrap();
    std::fs::write(
        root.join("font.ttf"),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf"),
    )
    .unwrap();
    std::fs::write(root.join("data.json"), "[1,2]").unwrap();
    std::fs::write(root.join("data.csv"), "x,y\n1,2\n").unwrap();
    let file = root.join("main.lay").to_string_lossy().replace('\\', "/");
    let source = "page=canvas(size=(40,30),stylesheet=\"style.lcss\",font_family=\"font.ttf\")\na=array(src=\"data.json\")\nt=table(src=\"data.csv\")\npage.add(image(src=\"photo.png\"),size=(10,10))\npage.add(rect(size=(a[0],2)),offset=(12,0))\npage.add(text(\"Preview\"),offset=(0,15))";
    let results = preview(&[
        json!({"id":1,"file":file,"source":source,"overlays":{root.join("data.json").to_string_lossy().replace('\\',"/"):"[7,8]",root.join("data.csv").to_string_lossy().replace('\\',"/"):"x,y\n3,4\n"}}),
    ]);
    let result = &results[1];
    assert!(result["svg"].is_string(), "{result}");
    for name in [
        "style.lcss",
        "photo.png",
        "font.ttf",
        "data.json",
        "data.csv",
    ] {
        assert!(
            result["dependencies"]
                .as_array()
                .unwrap()
                .contains(&json!(root.join(name).to_string_lossy().replace('\\', "/"))),
            "{name}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(root.join("data.json")).unwrap(),
        "[1,2]"
    );
}
