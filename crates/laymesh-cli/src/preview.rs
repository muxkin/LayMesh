//! Independent preview transport. LSP analysis never invokes the renderer.
use laymesh_core::{engine::compile_source, model::Host};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    io::{self, BufRead, Write},
    path::Path,
    rc::Rc,
};

pub fn serve() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", json!({"type":"ready","protocol":1}))?;
    stdout.flush()?;
    for line in stdin.lock().lines() {
        let line = line?;
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                writeln!(
                    stdout,
                    "{}",
                    json!({"id":null,"error":{"code":"E_PREVIEW_PROTOCOL","message":error.to_string()},"dependencies":[]})
                )?;
                stdout.flush()?;
                continue;
            }
        };
        let dependencies = Rc::new(RefCell::new(BTreeSet::new()));
        let mut host = Host {
            native: true,
            dependencies: Some(dependencies.clone()),
            ..Host::default()
        };
        let file = request["file"].as_str().unwrap_or("");
        let source = request["source"].as_str();
        let result = if !Path::new(file).is_absolute()
            || !file.ends_with(".lay")
            || source.is_none()
        {
            json!({"error":{"code":"E_PREVIEW_PROTOCOL","message":"Preview requires an absolute .lay path and source text"}})
        } else {
            dependencies.borrow_mut().insert(file.into());
            if let Some(overlays) = request["overlays"].as_object() {
                for (path, text) in overlays {
                    if let Some(text) = text.as_str() {
                        host.files.insert(
                            laymesh_core::model::normalize_path(Path::new(path))
                                .to_string_lossy()
                                .replace('\\', "/"),
                            text.as_bytes().to_vec(),
                        );
                    }
                }
            }
            host.files
                .insert(file.into(), source.unwrap().as_bytes().to_vec());
            match compile_source(source.unwrap(), file, host) {
                Ok(scene) if request["type"] == "export" => export_scene(&scene, &request),
                Ok(scene) => match laymesh_render::render_svg(&scene) {
                    Ok(svg) => {
                        json!({"svg":svg,"inspection":laymesh_language::inspect::inspect_scene(&scene)})
                    }
                    Err(error) => json!({"error":error}),
                },
                Err(error) => json!({"error":error}),
            }
        };
        let mut result = result;
        result["id"] = request["id"].clone();
        result["dependencies"] = json!(*dependencies.borrow());
        writeln!(stdout, "{result}")?;
        stdout.flush()?;
    }
    Ok(())
}

fn export_scene(scene: &laymesh_core::model::Scene, request: &Value) -> Value {
    let result = (|| -> Result<Value, String> {
        let output = Path::new(
            request["output"]
                .as_str()
                .ok_or("Export requires an output path")?,
        );
        if !output.is_absolute() {
            return Err("Export requires an absolute output path".into());
        }
        let options: laymesh_render::ExportOptions =
            serde_json::from_value(request.get("options").cloned().unwrap_or(json!({})))
                .map_err(|e| e.to_string())?;
        let format = output.extension().and_then(|s| s.to_str()).unwrap_or("");
        let data =
            laymesh_render::render_export(scene, format, &options).map_err(|e| e.to_string())?;
        super::export::write_atomic(output, &data).map_err(|e| e.to_string())?;
        Ok(
            json!({"exported":output.to_string_lossy(),"bytes":data.len(),"warnings":scene.warnings}),
        )
    })();
    result.unwrap_or_else(|message| json!({"error":{"code":"E_EXPORT","message":message}}))
}
