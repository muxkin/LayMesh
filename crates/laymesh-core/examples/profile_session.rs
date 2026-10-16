//! Native session/oracle comparison for an extracted universal HTML archive.
use laymesh_core::{engine::compile_preview_source, model::Host, session::CompileSession};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
fn encoded(scene: &laymesh_core::model::Scene) -> Vec<u8> {
    fn strip(v: &mut Value) {
        match v {
            Value::Object(m) => {
                m.remove("_identity");
                for v in m.values_mut() {
                    strip(v)
                }
            }
            Value::Array(a) => {
                for v in a {
                    strip(v)
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(scene).unwrap();
    strip(&mut v);
    serde_json::to_vec(&v).unwrap()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let asset = |key: &str| {
        std::fs::read(format!(
            "{}/{}",
            args[2],
            key.strip_prefix("assets/").unwrap()
        ))
        .unwrap()
    };
    let mut host = Host::default();
    for (path, key) in manifest["project"]["files"].as_object().unwrap() {
        host.shared_files
            .insert(path.clone(), Arc::new(asset(key.as_str().unwrap())));
    }
    for font in manifest["fontRequirements"].as_array().unwrap() {
        if let (Some(path), Some(key)) = (font["path"].as_str(), font["file"].as_str()) {
            host.shared_files.insert(path.into(), Arc::new(asset(key)));
        }
    }
    let files: Vec<_> = host
        .shared_files
        .keys()
        .filter(|p| p.ends_with(".lay"))
        .cloned()
        .collect();
    let mut rows = vec![];
    let mut session = CompileSession::new(false);
    session.sync_host(&host, 1).unwrap();
    session.set_profiling(true);
    let mut revision = 1;
    for file in files {
        if !file.contains("/pages/") {
            continue;
        }
        if args.get(3).is_some_and(|filter| !file.ends_with(filter)) {
            continue;
        }
        let original = String::from_utf8(host.bytes(&file).unwrap().to_vec()).unwrap();
        let mut hashes = BTreeMap::new();
        for iteration in 0..4 {
            let source = if iteration == 2 {
                format!("# location shift\n{original}")
            } else {
                original.clone()
            };
            let start = Instant::now();
            let full = compile_preview_source(&source, &file, host.clone()).unwrap();
            let full_ms = start.elapsed().as_secs_f64() * 1000.;
            let full_hash = format!("{:x}", Sha256::digest(encoded(&full)));
            drop(full);
            revision += 1;
            session
                .apply_changes(revision, [(file.clone(), Some(source.as_bytes().to_vec()))])
                .unwrap();
            let start = Instant::now();
            let result = session.evaluate(&source, &file, None).unwrap();
            let session_ms = start.elapsed().as_secs_f64() * 1000.;
            let hash = format!("{:x}", Sha256::digest(encoded(&result)));
            assert_eq!(hash, full_hash, "{file} iteration {iteration}");
            hashes.insert(iteration, hash);
            rows.push(json!({"file":file,"iteration":iteration,"fullMs":full_ms,"sessionMs":session_ms,"metrics":session.metrics()}));
            eprintln!("{file} {iteration}: full {full_ms:.1} ms, session {session_ms:.1} ms");
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
