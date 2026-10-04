//! Compare physical layout of non-plot legacy scenes, recording font metric changes.
use laymesh_core::{engine::Engine, model::Host};
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, path::PathBuf};
fn typed(n: &J, t: &str) -> bool {
    n["kind"] == t
        || n["children"]
            .as_array()
            .is_some_and(|a| a.iter().any(|n| typed(n, t)))
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
    let mut files = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect::<Vec<_>>();
    files.sort();
    let mut rows = vec![];
    let mut cache = BTreeMap::new();
    let (mut count, mut mismatches) = (0, 0);
    for file in files {
        for (line, row) in std::fs::read_to_string(&file).unwrap().lines().enumerate() {
            let c: J = serde_json::from_str(row).unwrap();
            if let Some(filter) = std::env::args().nth(1) {
                if !format!(
                    "{}:{}",
                    file.file_name().unwrap().to_string_lossy(),
                    line + 1
                )
                .contains(&filter)
                {
                    continue;
                }
            }
            if c["result"]["ok"] != true || c["source"].as_str().unwrap().contains("plot(") {
                continue;
            }
            let mut host = Host {
                native: true,
                ..Host::default()
            };
            for (k, v) in c["files"].as_object().unwrap() {
                let hash = v.as_str().unwrap();
                let data = cache
                    .entry(hash.to_string())
                    .or_insert_with(|| std::fs::read(root.join("blobs").join(hash)).unwrap());
                host.files.insert(k.clone(), data.clone());
            }
            let scene = Engine::new(host)
                .compile(c["source"].as_str().unwrap(), c["file"].as_str().unwrap())
                .unwrap();
            let old = c["result"]["scene"]["nodes"].as_array().unwrap();
            let mut differences = vec![];
            let text_dependent = old.iter().any(|n| typed(n, "text") || typed(n, "formula"));
            if old.len() != scene.nodes.len() {
                differences
                    .push(json!({"node_count_old":old.len(),"node_count_new":scene.nodes.len()}));
            }
            for (i, (a, b)) in old.iter().zip(&scene.nodes).enumerate() {
                for key in ["x", "y", "width", "height", "rotation", "opacity"] {
                    let (x, y) = (a[key].as_f64().unwrap_or(0.), b[key].as_f64().unwrap_or(0.));
                    if (x - y).abs() > 0.001 {
                        differences
                            .push(json!({"node":i,"id":a["id"],"field":key,"old":x,"new":y}));
                    }
                }
            }
            count += 1;
            if !differences.is_empty() {
                mismatches += 1;
            }
            rows.push(json!({"capture":file.file_name().unwrap().to_string_lossy(),"line":line+1,"file":c["file"],"text_dependent":text_dependent,"differences":differences}));
        }
    }
    let summary =
        json!({"cases":count,"cases_with_layout_changes":mismatches,"tolerance_mm":0.001});
    println!("{summary}");
    std::fs::write(
        root.parent().unwrap().join(if std::env::args().len() > 1 {
            "rust-geometry-filter.json"
        } else {
            "rust-geometry-comparison.json"
        }),
        serde_json::to_vec_pretty(&json!({"summary":summary,"cases":rows})).unwrap(),
    )
    .unwrap();
}
