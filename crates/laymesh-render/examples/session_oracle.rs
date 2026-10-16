//! Compare retained compilation and drawing updates for an extracted HTML project.
//! Usage: session_oracle MANIFEST ASSET_DIR ENTRY REPLACEMENTS_JSON OUTPUT_DIR
use laymesh_core::{
    engine::compile_preview_source,
    model::{Host, Scene},
    session::CompileSession,
};
use laymesh_render::display_list::{display_update_reusing, outline_nodes, to_display_list};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
fn milliseconds() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
        * 1000.
}
fn evaluate(session: &mut CompileSession, source: &str, file: &str) -> Arc<Scene> {
    let mut slice = std::env::var("LAYMESH_ORACLE_SLICE_MS")
        .ok()
        .map(|s| s.parse::<f64>().unwrap());
    loop {
        session.set_deadline(slice.map(|s| milliseconds() + s));
        match session.evaluate(source, file, None) {
            Ok(scene) => {
                session.set_deadline(None);
                return scene;
            }
            Err(e) if e.code == "E_YIELD" => {
                slice = slice.map(|s| s * 2.);
            }
            Err(e) => panic!("{e}"),
        }
    }
}
fn canonical<T: serde::Serialize>(value: &T) -> Value {
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
    let mut v = serde_json::to_value(value).unwrap();
    strip(&mut v);
    v
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 6);
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let dir = Path::new(&args[2]);
    let read = |key: &str| {
        Arc::new(std::fs::read(dir.join(key.strip_prefix("assets/").unwrap())).unwrap())
    };
    let mut host = Host::default();
    for (path, key) in manifest["project"]["files"].as_object().unwrap() {
        host.shared_files
            .insert(path.clone(), read(key.as_str().unwrap()));
    }
    for font in manifest["fontRequirements"].as_array().unwrap() {
        if let (Some(path), Some(key)) = (font["path"].as_str(), font["file"].as_str()) {
            host.shared_files.insert(path.into(), read(key));
        }
    }
    let mut session = CompileSession::new(false);
    session.set_profile_clock(milliseconds);
    session.set_focus_file(&args[3]);
    session.sync_host(&host, 1).unwrap();
    let files: Vec<_> = host
        .shared_files
        .keys()
        .filter(|p| p.contains("/pages/") && p.ends_with(".lay"))
        .cloned()
        .collect();
    for file in files {
        let source = session.source(&file).unwrap();
        evaluate(&mut session, &source, &file);
    }
    let entry = &args[3];
    let edit_file = std::env::var("LAYMESH_ORACLE_EDIT_FILE").unwrap_or_else(|_| entry.clone());
    let original = String::from_utf8(host.bytes(&edit_file).unwrap().to_vec()).unwrap();
    let replacements: Vec<Vec<String>> =
        serde_json::from_str(&std::fs::read_to_string(&args[4]).unwrap()).unwrap();
    let out = Path::new(&args[5]);
    std::fs::create_dir_all(out).unwrap();
    let mut baseline: Option<(Arc<Scene>, laymesh_render::display_list::DisplayList)> = None;
    for (i, pair) in std::iter::once(vec![]).chain(replacements).enumerate() {
        let source = if pair.is_empty() {
            original.clone()
        } else {
            original.replace(&pair[0], &pair[1])
        };
        session
            .apply_changes(
                i as u64 + 2,
                [(edit_file.clone(), Some(source.as_bytes().to_vec()))],
            )
            .unwrap();
        host.shared_files
            .insert(edit_file.clone(), Arc::new(source.as_bytes().to_vec()));
        let source = session.source(entry).unwrap();
        let started = milliseconds();
        let actual = evaluate(&mut session, &source, entry);
        let elapsed = milliseconds() - started;
        let expected = compile_preview_source(&source, entry, host.clone()).unwrap();
        let a = canonical(actual.as_ref());
        let b = canonical(&expected);
        let core_equal = a == b;
        if !core_equal {
            std::fs::write(out.join(format!("{i}-actual-scene.json")), a.to_string()).unwrap();
            std::fs::write(out.join(format!("{i}-expected-scene.json")), b.to_string()).unwrap();
        }
        let mut full = to_display_list(&expected).unwrap();
        outline_nodes(&mut full.nodes);
        let retained = if let Some((previous, list)) = &baseline {
            let update = display_update_reusing(&actual, previous, list, true).unwrap();
            let mut next = list.clone();
            update.apply(&mut next).unwrap();
            next
        } else {
            let mut list = to_display_list(&actual).unwrap();
            outline_nodes(&mut list.nodes);
            list
        };
        let a = canonical(&retained);
        let b = canonical(&full);
        let drawing_equal = a == b;
        if !drawing_equal {
            std::fs::write(out.join(format!("{i}-actual-list.json")), a.to_string()).unwrap();
            std::fs::write(out.join(format!("{i}-expected-list.json")), b.to_string()).unwrap();
        }
        println!(
            "{}",
            json!({"iteration":i,"coreEqual":core_equal,"drawingEqual":drawing_equal,"evaluateMs":elapsed,"metrics":session.metrics()})
        );
        assert!(
            core_equal && drawing_equal,
            "Incremental oracle failed at {i}"
        );
        baseline = Some((actual, retained));
    }
}
