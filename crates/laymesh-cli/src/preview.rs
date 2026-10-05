//! Persistent foreground compiler with versioned external preview resources.
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{config, preview_assets};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    io::{self, BufRead, Write},
    path::Path,
    rc::Rc,
    sync::mpsc,
};
pub fn serve() -> io::Result<()> {
    let (sender, receiver) = mpsc::channel::<Value>();
    let writer = std::thread::spawn(move || {
        let mut stdout = io::stdout().lock();
        for message in receiver {
            if writeln!(stdout, "{message}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    });
    sender
        .send(json!({"type":"ready","protocol":1,"capabilities":["resources-v2","defaults"]}))
        .unwrap();
    for line in io::stdin().lock().lines() {
        let request: Value = match serde_json::from_str(&line?) {
            Ok(v) => v,
            Err(e) => {
                let _=sender.send(json!({"id":null,"error":{"code":"E_PREVIEW_PROTOCOL","message":e.to_string()},"dependencies":[]}));
                continue;
            }
        };
        let file = request["file"].as_str().unwrap_or("");
        let dependencies = Rc::new(RefCell::new(BTreeSet::new()));
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            if !Path::new(file).is_absolute() || !file.ends_with(".lay") {
                return Err("Preview requires an absolute .lay path".into());
            }
            let (defaults, paths) = config::resolve(
                Path::new(file),
                request["config"].as_str().map(Path::new),
                &request["editor_user"],
                &request["editor_workspace"],
            )?;
            dependencies.borrow_mut().extend(paths);
            let settings = config::preview_settings(&defaults, &request["preview"])?;
            if request["type"] == "defaults" {
                let options = config::export_options(
                    &defaults,
                    request["format"].as_str().unwrap_or("pdf"),
                    &Default::default(),
                )?;
                return Ok(
                    json!({"options":options,"previewSettings":settings,"fingerprint":config::fingerprint(&defaults)}),
                );
            }
            let source = request["source"]
                .as_str()
                .ok_or("Preview requires source text")?;
            let resources = request["protocol"] == 2 && request["type"] != "export";
            laymesh_core::asset_cache::preview_mode(
                resources
                    || (request["type"] == "export"
                        && request["output"]
                            .as_str()
                            .is_some_and(|p| p.to_ascii_lowercase().ends_with(".pdf"))),
            );
            laymesh_core::asset_cache::set_budget(settings.cache_mb);
            let mut host = Host {
                native: true,
                dependencies: Some(dependencies.clone()),
                ..Host::default()
            };
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
            host.files.insert(file.into(), source.as_bytes().to_vec());
            let scene = compile_source(source, file, host)?;
            if request["type"] == "export" {
                return export_scene(&scene, &request, &defaults);
            }
            if resources {
                let directory = Path::new(
                    request["resource_dir"]
                        .as_str()
                        .ok_or("Missing preview resource directory")?,
                );
                if !directory.is_absolute() {
                    return Err("Resource directory must be absolute".into());
                }
                std::fs::create_dir_all(directory)?;
                let (svg, mut resources) = preview_assets::frame(&scene, directory, &settings)?;
                if let Some(known) = request["known_resources"].as_array() {
                    resources.retain(|item| !known.contains(&item["id"]));
                }
                return Ok(
                    json!({"protocol":2,"svg":svg,"resources":resources,"generation":request["generation"],"previewSettings":settings,"inspection":laymesh_language::inspect::inspect_scene(&scene)}),
                );
            }
            Ok(
                json!({"svg":laymesh_render::render_svg(&scene)?,"inspection":laymesh_language::inspect::inspect_scene(&scene)}),
            )
        })();
        laymesh_core::asset_cache::preview_mode(false);
        let mut result = result.unwrap_or_else(|e| {
            let error = if let Some(d) = e
                .downcast_ref::<laymesh_core::Diagnostic>()
                .filter(|d| !d.file.is_empty())
            {
                serde_json::to_value(d).unwrap()
            } else if request["type"] == "export" {
                json!({"code":"E_EXPORT","message":e.to_string()})
            } else if let Some(d) = e.downcast_ref::<laymesh_core::Diagnostic>() {
                serde_json::to_value(d).unwrap()
            } else {
                json!({"code":"E_PREVIEW","message":e.to_string()})
            };
            json!({"error":error})
        });
        result["id"] = request["id"].clone();
        result["dependencies"] = json!(*dependencies.borrow());
        let _ = sender.send(result);
    }
    drop(sender);
    let _ = writer.join();
    Ok(())
}
fn export_scene(
    scene: &laymesh_core::model::Scene,
    request: &Value,
    defaults: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Path::new(
        request["output"]
            .as_str()
            .ok_or("Export requires an output path")?,
    );
    if !output.is_absolute() {
        return Err("Export requires an absolute output path".into());
    }
    let explicit: laymesh_render::ExportOptions =
        serde_json::from_value(request.get("options").cloned().unwrap_or(json!({})))?;
    let format = output.extension().and_then(|s| s.to_str()).unwrap_or("");
    let options = config::export_options(defaults, format, &explicit)?;
    let data = laymesh_render::render_export(scene, format, &options)?;
    super::export::write_atomic(output, &data)?;
    let mut warnings = scene.warnings.clone();
    if format.eq_ignore_ascii_case("pdf") {
        warnings.extend(laymesh_render::raster_policy::pdf_warnings(
            scene, &options,
        )?);
    }
    Ok(json!({"exported":output.to_string_lossy(),"bytes":data.len(),"warnings":warnings}))
}
