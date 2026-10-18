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
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
pub fn serve() -> io::Result<()> {
    let mut presentation_plan: Option<(
        String,
        laymesh_core::document::DocumentPlan,
        BTreeSet<String>,
    )> = None;
    let mut compressed_cache: Option<(String, Value)> = None;
    // One bounded baseline. A different panel, restart, or unacknowledged
    // generation receives a full CPU display list instead of an unsafe patch.
    let mut retained_list: Option<(String, String, laymesh_render::DisplayList)> = None;
    let mut retained_scene: Option<Arc<laymesh_core::model::Scene>> = None;
    let session = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut revision = 0u64;
    let mut compiler = laymesh_core::session::CompileSession::new(true);
    let latest = Arc::new(AtomicU64::new(0));
    let (input, requests) = mpsc::channel();
    let cancellation = latest.clone();
    std::thread::spawn(move || {
        let mut sequence = 0u64;
        for line in io::stdin().lock().lines() {
            sequence += 1;
            if let Ok(line) = &line {
                if let Ok(value) = serde_json::from_str::<Value>(line) {
                    if value["type"] == "cancel" {
                        cancellation.store(sequence, Ordering::Relaxed);
                        continue;
                    }
                    if !matches!(
                        value["type"].as_str(),
                        Some("export" | "animate" | "defaults")
                    ) {
                        cancellation.store(sequence, Ordering::Relaxed);
                    }
                }
            }
            if input.send((sequence, line)).is_err() {
                break;
            }
        }
    });
    let (sender, receiver) = mpsc::channel::<Value>();
    let writer = std::thread::spawn(move || {
        let mut stdout = io::stdout().lock();
        for message in receiver {
            // Serialize once, then write the complete JSON line. Formatting a
            // large display list directly into line-buffered stdout fragments
            // it into thousands of small writes.
            let mut bytes = serde_json::to_vec(&message).unwrap();
            bytes.push(b'\n');
            if stdout
                .write_all(&bytes)
                .and_then(|_| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    });
    sender
        .send(json!({"type":"ready","protocol":1,"capabilities":["media-v1","web-export-v1","animation-v1","bundled-stylesheets-v1","ffmpeg-manager-v1","resources-v2","display-list-v3","display-list-gzip-v3","display-list-updates-v3","incremental-session-v1","incremental-document-v1","defaults"]}))
        .unwrap();
    for (input_revision, line) in requests {
        let request: Value = match serde_json::from_str(&line?) {
            Ok(v) => v,
            Err(e) => {
                let _=sender.send(json!({"id":null,"error":{"code":"E_PREVIEW_PROTOCOL","message":e.to_string()},"dependencies":[]}));
                continue;
            }
        };
        let file = request["file"].as_str().unwrap_or("");
        let incremental = request["incremental_session"] == "incremental-session-v1"
            && !matches!(
                request["type"].as_str(),
                Some("export" | "animate" | "defaults")
            );
        compiler.set_cancellation(latest.clone(), input_revision);
        let dependencies = Rc::new(RefCell::new(BTreeSet::new()));
        let mut document_metadata = None;
        let mut document_dependencies = BTreeSet::new();
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            if !Path::new(file).is_absolute()
                || !(file.ends_with(".lay") || file.ends_with(".laydoc"))
            {
                return Err("Preview requires an absolute .lay or .laydoc path".into());
            }
            let (defaults, paths) = config::resolve(
                Path::new(file),
                request["config"].as_str().map(Path::new),
                &request["editor_user"],
                &request["editor_workspace"],
            )?;
            dependencies.borrow_mut().extend(paths);
            let mut settings = config::preview_settings(&defaults, &request["preview"])?;
            settings.parallelism = request["parallelism"].as_u64().unwrap_or(0) as usize;
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
            let source_owned;
            let source = if incremental {
                if request["reset_session"] == true {
                    compiler = laymesh_core::session::CompileSession::new(true);
                    compiler.set_cancellation(latest.clone(), input_revision);
                }
                let version = request["sourceVersion"]
                    .as_u64()
                    .ok_or("Missing source version")?;
                let changes: std::collections::BTreeMap<String, Option<String>> =
                    serde_json::from_value(request["changes"].clone())?;
                compiler.set_dependencies(Some(dependencies.clone()));
                compiler.apply_changes(
                    version,
                    changes
                        .into_iter()
                        .map(|(p, v)| (p, v.map(String::into_bytes))),
                )?;
                source_owned = compiler.source(file)?;
                source_owned.as_str()
            } else {
                request["source"]
                    .as_str()
                    .ok_or("Preview requires source text")?
            };
            let protocol = request["protocol"].as_u64().unwrap_or(3);
            if ![1, 2, 3].contains(&protocol) {
                return Err("Unsupported preview protocol".into());
            }
            let resources = protocol >= 2 && request["type"] != "export";
            laymesh_core::asset_cache::preview_mode(
                resources
                    || (request["type"] == "export"
                        && request["output"]
                            .as_str()
                            .is_some_and(|p| p.to_ascii_lowercase().ends_with(".pdf"))),
            );
            let mut host = Host {
                native: true,
                dependencies: Some(dependencies.clone()),
                ..super::documents::configured_host(defaults.clone(), Path::new(file).to_path_buf())
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
            compiler.set_media_host(&host);
            if request["type"] == "animate" {
                let output = Path::new(request["output"].as_str().ok_or("Missing output path")?);
                if !output.is_absolute() {
                    return Err("Animation output path must be absolute".into());
                }
                return super::animation::export(
                    file,
                    source,
                    host,
                    output,
                    &defaults,
                    &request["options"],
                    None,
                    request["cancel_file"].as_str().map(Path::new),
                );
            }
            if request["type"] == "export"
                && request["output"].as_str().is_some_and(|s| {
                    s.to_ascii_lowercase().ends_with(".html")
                        || s.to_ascii_lowercase().ends_with(".zip")
                })
            {
                let output = Path::new(request["output"].as_str().unwrap());
                if !output.is_absolute() {
                    return Err("Output must be absolute".into());
                }
                let explicit: laymesh_render::ExportOptions = if request["options"].is_null() {
                    Default::default()
                } else {
                    serde_json::from_value(request["options"].clone())?
                };
                let options = config::export_options(
                    &defaults,
                    output
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("html"),
                    &explicit,
                )?;
                let page = if request["document_all"].as_bool().unwrap_or(true) {
                    None
                } else if let Some(id) = request["page_id"].as_str() {
                    let p = laymesh_core::document::plan_source(source, file, host.clone())?;
                    Some(
                        p.pages
                            .iter()
                            .position(|p| p.id == id)
                            .ok_or("Unknown page")?
                            + 1,
                    )
                } else {
                    request["page"].as_u64().map(|n| n as usize)
                };
                return super::web_export::export(
                    file, source, host, output, &options, page, "hide",
                );
            }
            let mut drawing_key = file.to_owned();
            let scene = if file.ends_with(".laydoc") {
                let plan_key = format!("{}\n{}", file, request["presentation_revision"]);
                let plan = if incremental {
                    compiler.plan(source, file)?
                } else if request["presentation_revision"].is_string() {
                    if let Some((_, plan, deps)) = presentation_plan
                        .as_ref()
                        .filter(|(key, _, _)| key == &plan_key)
                    {
                        dependencies.borrow_mut().extend(deps.iter().cloned());
                        plan.clone()
                    } else {
                        let plan = laymesh_core::document::plan_source(source, file, host.clone())?;
                        presentation_plan =
                            Some((plan_key, plan.clone(), dependencies.borrow().clone()));
                        plan
                    }
                } else {
                    laymesh_core::document::plan_source(source, file, host.clone())?
                };
                document_metadata = Some(json!({"name":plan.name,"pages":plan.pages}));
                document_dependencies = dependencies.borrow().clone();
                if request["type"] == "plan" {
                    return Ok(
                        json!({"protocol":3,"previewSettings":settings,"configFingerprint":config::fingerprint(&defaults)}),
                    );
                }
                let number = if let Some(id) = request["page_id"].as_str() {
                    plan.pages
                        .iter()
                        .position(|p| p.id == id)
                        .ok_or("Selected page ID no longer exists")?
                        + 1
                } else {
                    match request.get("page") {
                        None => 1,
                        Some(v) => {
                            usize::try_from(v.as_u64().ok_or("Page must be a positive integer")?)?
                        }
                    }
                };
                let page = plan.page(number)?;
                document_metadata = Some(
                    json!({"name":plan.name,"pages":plan.pages,"pageId":page.id,"pageNumber":number}),
                );
                if request["type"] == "export" {
                    let selected = request.get("page").is_some() || request["page_id"].is_string();
                    let pdf = request["output"].as_str().is_some_and(|p| {
                        [".pdf", ".pptx"]
                            .iter()
                            .any(|ext| p.to_ascii_lowercase().ends_with(ext))
                    });
                    if request["document_all"]
                        .as_bool()
                        .unwrap_or(pdf && !selected)
                    {
                        return export_document(&plan, host, &request, &defaults);
                    }
                    if !selected {
                        return Err(
                            "Document export requires an explicit page or document_all PDF/PPTX"
                                .into(),
                        );
                    }
                }
                drawing_key = format!("{file}\n{}", page.id);
                dependencies.borrow_mut().clear();
                if incremental {
                    compiler.evaluate_page(page)?
                } else {
                    Arc::new(laymesh_core::document::compile_preview_page(page, host)?)
                }
            } else {
                document_dependencies = dependencies.borrow().clone();
                if request["type"] == "plan" {
                    document_metadata = Some(
                        json!({"pages":[{"id":"page","name":Path::new(file).file_name().unwrap_or_default().to_string_lossy(),"src":file}]}),
                    );
                    return Ok(
                        json!({"protocol":3,"previewSettings":settings,"configFingerprint":config::fingerprint(&defaults)}),
                    );
                }
                dependencies.borrow_mut().clear();
                if incremental {
                    compiler.evaluate(source, file, None)?
                } else {
                    Arc::new(compile_source(source, file, host)?)
                }
            };
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
                if protocol == 3 {
                    if incremental {
                        drawing_key.push_str(&format!(
                            "\n{}\n{}",
                            directory.display(),
                            serde_json::to_string(&settings)?
                        ));
                    }
                    let updates = request["display_list_updates"] == true;
                    if incremental
                        && updates
                        && let Some((previous_file, base, previous)) = &retained_list
                        && let Some(previous_scene) = &retained_scene
                        && previous_file == &drawing_key
                        && request["known_display_list"].as_str() == Some(base)
                    {
                        let (update, mut resources) = preview_assets::display_update_frame(
                            &scene,
                            previous_scene,
                            previous,
                            directory,
                            &settings,
                        )?;
                        if let Some(known) = request["known_resources"].as_array() {
                            resources.retain(|item| !known.contains(&item["id"]));
                        }
                        revision += 1;
                        let list_id = format!("{session}-{revision}");
                        let mut result = json!({"protocol":3,"displayListId":list_id,"displayListUpdate":{"base":base,"update":update},"resources":resources,"generation":request["generation"],"previewSettings":settings,"inspection":laymesh_language::inspect::inspect_scene(&scene)});
                        let (_, _, mut list) = retained_list.take().unwrap();
                        update.apply(&mut list)?;
                        result["drawingResources"] = serde_json::to_value(&list.resources)?;
                        retained_scene = Some(scene.clone());
                        retained_list = Some((drawing_key.clone(), list_id, list));
                        compressed_cache = None;
                        return Ok(result);
                    }
                    let (list, mut resources) =
                        preview_assets::display_frame(&scene, directory, &settings)?;
                    if let Some(known) = request["known_resources"].as_array() {
                        resources.retain(|item| !known.contains(&item["id"]));
                    }
                    if updates {
                        retained_scene = incremental.then(|| scene.clone());
                    }
                    revision += 1;
                    let list_id = format!("{session}-{revision}");
                    if updates
                        && let Some((previous_file, base, previous)) = &retained_list
                        && previous_file == &drawing_key
                        && request["known_display_list"].as_str() == Some(base)
                    {
                        let update = if incremental {
                            laymesh_render::display_update::DisplayUpdate::between_incremental(
                                previous, &list,
                            )
                        } else {
                            laymesh_render::display_update::DisplayUpdate::between(previous, &list)
                        };
                        let result = json!({"protocol":3,"displayListId":list_id,"displayListUpdate":{"base":base,"update":update},"drawingResources":list.resources,"resources":resources,"generation":request["generation"],"previewSettings":settings,"inspection":laymesh_language::inspect::inspect_scene(&scene)});
                        retained_list = Some((drawing_key.clone(), list_id, list));
                        compressed_cache = None;
                        return Ok(result);
                    }
                    if request["compressed_display_list"] == true {
                        use base64::Engine;
                        let bytes = serde_json::to_vec(&list)?;
                        let id = laymesh_core::asset_cache::digest(&bytes);
                        let compressed = if let Some((key, value)) = &compressed_cache
                            && key == &id
                        {
                            value.clone()
                        } else {
                            let mut encoder = flate2::write::GzEncoder::new(
                                Vec::new(),
                                flate2::Compression::fast(),
                            );
                            encoder.write_all(&bytes)?;
                            let value = json!({"id":id,"codec":"gzip","uncompressedBytes":bytes.len(),"data":base64::engine::general_purpose::STANDARD.encode(encoder.finish()?)});
                            compressed_cache = Some((id, value.clone()));
                            value
                        };
                        let mut result = json!({"protocol":3,"displayListCompressed":compressed,"drawingResources":list.resources,"resources":resources,"generation":request["generation"],"previewSettings":settings,"inspection":laymesh_language::inspect::inspect_scene(&scene)});
                        if updates {
                            result["displayListId"] = json!(list_id);
                            retained_list = Some((drawing_key.clone(), list_id, list));
                        }
                        return Ok(result);
                    }
                    let mut result = json!({"protocol":3,"displayList":list,"resources":resources,"generation":request["generation"],"previewSettings":settings,"inspection":laymesh_language::inspect::inspect_scene(&scene)});
                    if updates {
                        result["displayListId"] = json!(list_id);
                        retained_list = Some((drawing_key.clone(), list_id, list));
                    }
                    return Ok(result);
                }
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
        if incremental {
            result["appliedSourceVersion"] = json!(compiler.revision());
            result["sourceVersion"] = request["sourceVersion"].clone();
            result["compileMetrics"] = serde_json::to_value(compiler.metrics()).unwrap();
        }
        result["id"] = request["id"].clone();
        if let Some(document) = document_metadata {
            result["document"] = document;
        }
        result["pageDependencies"] = json!(*dependencies.borrow());
        result["documentDependencies"] = json!(document_dependencies);
        dependencies.borrow_mut().extend(document_dependencies);
        result["dependencies"] = json!(*dependencies.borrow());
        let _ = sender.send(result);
    }
    drop(sender);
    let _ = writer.join();
    Ok(())
}
fn export_document(
    plan: &laymesh_core::document::DocumentPlan,
    host: Host,
    request: &Value,
    defaults: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Path::new(
        request["output"]
            .as_str()
            .ok_or("Export requires an output path")?,
    );
    if !output.is_absolute()
        || !output
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf") || e.eq_ignore_ascii_case("pptx"))
    {
        return Err("Whole document export currently requires an absolute PDF/PPTX path".into());
    }
    let explicit = serde_json::from_value(request.get("options").cloned().unwrap_or(json!({})))?;
    let format = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf")
        .to_ascii_lowercase();
    let options = config::export_options(defaults, &format, &explicit)?;
    if format == "pptx" {
        let mut deck = laymesh_render::pptx::PptxDocument::new();
        let mut warnings = Vec::new();
        for page in &plan.pages {
            let scene = laymesh_core::document::compile_page(page, host.clone())?;
            warnings.extend(scene.warnings.clone());
            warnings.extend(deck.append(&scene, &options)?);
        }
        let data = deck.finish()?;
        super::export::write_atomic(output, &data)?;
        return Ok(
            json!({"exported":output.to_string_lossy(),"bytes":data.len(),"warnings":warnings,"pageCount":plan.pages.len()}),
        );
    }
    let mut pdf = laymesh_render::PdfDocument::new();
    let mut warnings = Vec::new();
    for page in &plan.pages {
        let scene = laymesh_core::document::compile_page(page, host.clone())?;
        warnings.extend(scene.warnings.clone());
        warnings.extend(laymesh_render::raster_policy::pdf_warnings(
            &scene, &options,
        )?);
        pdf.append(&scene, &options)?;
    }
    let data = pdf.finish()?;
    super::export::write_atomic(output, &data)?;
    Ok(
        json!({"exported":output.to_string_lossy(),"bytes":data.len(),"warnings":warnings,"pageCount":plan.pages.len()}),
    )
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
    let (data, pptx_warnings) = if format.eq_ignore_ascii_case("pptx") {
        laymesh_render::pptx::render_pptx(scene, &options)?
    } else {
        (
            laymesh_render::render_export(scene, format, &options)?,
            vec![],
        )
    };
    super::export::write_atomic(output, &data)?;
    let mut warnings = scene.warnings.clone();
    warnings.extend(pptx_warnings);
    if format.eq_ignore_ascii_case("pdf") {
        warnings.extend(laymesh_render::raster_policy::pdf_warnings(
            scene, &options,
        )?);
    }
    Ok(json!({"exported":output.to_string_lossy(),"bytes":data.len(),"warnings":warnings}))
}
