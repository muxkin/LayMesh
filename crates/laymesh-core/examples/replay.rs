//! Replay the captured legacy compiler inputs without running JavaScript.
use laymesh_core::{engine::Engine, model::Host};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
    let mut files = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect::<Vec<_>>();
    files.sort();
    let mut records = vec![];
    let mut cache = BTreeMap::new();
    let mut cases = 0;
    let mut accepted = 0;
    let mut errors = 0;
    let mut expected_success = 0;
    let mut expected_errors = 0;
    for path in files {
        let input = std::fs::read_to_string(&path).unwrap();
        for (line, row) in input.lines().enumerate() {
            let case: Value = serde_json::from_str(row).unwrap();
            let mut host = Host {
                native: true,
                ..Host::default()
            };
            for (name, hash) in case["files"].as_object().unwrap() {
                let hash = hash.as_str().unwrap();
                let data = cache
                    .entry(hash.to_string())
                    .or_insert_with(|| std::fs::read(root.join("blobs").join(hash)).unwrap());
                host.files.insert(name.clone(), data.clone());
            }
            let source = case["source"].as_str().unwrap();
            let file = case["file"].as_str().unwrap();
            let expected_ok = case["result"]["ok"] == true;
            if expected_ok {
                expected_success += 1;
            } else {
                expected_errors += 1;
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Engine::new(host).compile(source, file)
            }));
            let (matches, detail) = match result {
                Ok(Ok(scene)) => {
                    if expected_ok {
                        accepted += 1;
                        (
                            true,
                            json!({"nodes":scene.nodes.len(),"warnings":scene.warnings.len()}),
                        )
                    } else {
                        (
                            false,
                            json!({"unexpected_success":true,"expected":case["result"]}),
                        )
                    }
                }
                Ok(Err(error)) => {
                    let matches = !expected_ok
                        && error.code == case["result"]["code"].as_str().unwrap_or("")
                        && error.file == case["result"]["file"].as_str().unwrap_or("")
                        && Some(error.loc.line as u64) == case["result"]["loc"]["line"].as_u64()
                        && error.loc.column > 0;
                    if matches {
                        errors += 1;
                    }
                    (
                        matches,
                        json!({"error":error,"expected":if expected_ok{json!("success")}else{case["result"].clone()}}),
                    )
                }
                Err(_) => (false, json!({"panic":true})),
            };
            cases += 1;
            records.push(json!({"capture":path.file_name().unwrap().to_string_lossy(),"line":line+1,"matches":matches,"detail":detail,"source":source,"file":file}));
        }
    }
    let summary = json!({"cases":cases,"expected_success":expected_success,"expected_errors":expected_errors,"accepted_success":accepted,"matched_error":errors,"mismatches":cases-accepted-errors});
    println!("{}", summary);
    std::fs::write(
        root.parent().unwrap().join("rust-corpus-results.json"),
        serde_json::to_vec_pretty(&json!({"summary":summary,"cases":records})).unwrap(),
    )
    .unwrap();
    if cases != accepted + errors {
        std::process::exit(1);
    }
}
