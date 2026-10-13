//! Portable precompiled playback and optional offline editing, one ZIP contract.
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use laymesh_render::{
    DisplayList, ExportOptions,
    display_list::{DrawKind, DrawNode, Paint},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RUNTIME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web-runtime.zip"));
fn asset(files: &mut BTreeMap<String, Vec<u8>>, bytes: Vec<u8>) -> String {
    let name = format!("assets/{}", laymesh_core::asset_cache::digest(&bytes));
    files.entry(name.clone()).or_insert(bytes);
    name
}
fn image(bytes: Vec<u8>, options: &ExportOptions) -> (Vec<u8>, bool) {
    if options.html_image_compression.as_deref() == Some("original") {
        return (bytes, false);
    }
    let Ok(decoded) = image::load_from_memory(&bytes) else {
        return (bytes, false);
    };
    let rgba = decoded.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w > 16383 || h > 16383 {
        return (bytes, false);
    }
    let encoder = webp::Encoder::from_rgba(&rgba, w, h);
    let encoded = if options.html_image_compression.as_deref() == Some("lossless") {
        let Ok(mut config) = webp::WebPConfig::new() else {
            return (bytes, false);
        };
        config.lossless = 1;
        config.exact = 1; // Retain RGB values even under fully transparent pixels.
        encoder.encode_advanced(&config)
    } else {
        encoder.encode_simple(false, options.html_image_quality.unwrap_or(85.))
    };
    let Ok(encoded) = encoded else {
        return (bytes, false);
    };
    if encoded.len() < bytes.len() {
        (encoded.to_vec(), true)
    } else {
        (bytes, false)
    }
}
fn pixel_key(bytes: &[u8]) -> Option<String> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(format!(
        "{}x{}:{}",
        image.width(),
        image.height(),
        laymesh_core::asset_cache::digest(image.as_raw())
    ))
}
fn original_images(
    host: &Host,
    dependencies: &BTreeSet<String>,
) -> BTreeMap<String, (Vec<u8>, String)> {
    let mut images: BTreeMap<String, (Vec<u8>, String)> = BTreeMap::new();
    for path in dependencies {
        if !["png", "jpg", "jpeg", "webp"].contains(
            &Path::new(path)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
        ) {
            continue;
        }
        let Some(data) = host
            .files
            .get(path)
            .cloned()
            .or_else(|| std::fs::read(path).ok())
        else {
            continue;
        };
        let mime = match image::guess_format(&data) {
            Ok(image::ImageFormat::Png) => "image/png",
            Ok(image::ImageFormat::Jpeg) => "image/jpeg",
            Ok(image::ImageFormat::WebP) => "image/webp",
            _ => continue,
        };
        let Some(key) = pixel_key(&data) else {
            continue;
        };
        if images
            .get(&key)
            .is_none_or(|(previous, _)| data.len() < previous.len())
        {
            images.insert(key, (data, mime.into()));
        }
    }
    images
}
fn outlines(nodes: &mut [DrawNode]) {
    for n in nodes {
        if matches!(&n.kind, DrawKind::Glyphs { .. }) {
            let mut text = serde_json::to_value(&n.kind).unwrap();
            for key in ["kind", "resource", "glyphs", "outlines"] {
                text.as_object_mut().unwrap().remove(key);
            }
            if !n.hint.is_object() {
                n.hint = json!({});
            }
            n.hint["exportText"] = text;
        }
        match &mut n.kind {
            DrawKind::Glyphs {
                outlines, color, ..
            } => {
                let children = outlines
                    .iter()
                    .map(|path| {
                        DrawNode::new(DrawKind::Path {
                            path: path.clone(),
                            fill: Paint::Solid { rgba: *color },
                            stroke: None,
                            even_odd: false,
                        })
                    })
                    .collect();
                n.kind = DrawKind::Group {
                    children,
                    clip: None,
                    mask: None,
                };
            }
            DrawKind::Group { children, mask, .. } => {
                outlines(children);
                if let Some(m) = mask {
                    outlines(std::slice::from_mut(m.node.as_mut()));
                }
            }
            DrawKind::Path {
                fill: Paint::Pattern { nodes, .. },
                ..
            } => outlines(nodes),
            _ => {}
        }
    }
}
fn page(
    scene: &Scene,
    id: &str,
    name: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    resources: &mut BTreeMap<String, String>,
    options: &ExportOptions,
    originals: &BTreeMap<String, (Vec<u8>, String)>,
) -> Result<Value> {
    let mut list: DisplayList = laymesh_render::to_display_list(scene)?;
    outlines(&mut list.nodes);
    list.resources.retain(|_, r| r.mime.starts_with("image/") || r.mime.starts_with("video/") || r.mime.starts_with("audio/"));
    for (id, r) in &mut list.resources {
        // Match normalized pixels so orientation, ICC conversion and crops
        // still use their prepared derivative, while ordinary images retain
        // the original encoding before comparing WebP sizes.
        if let Some((data, mime)) = pixel_key(&r.data).and_then(|key| originals.get(&key)) {
            r.data = data.clone();
            r.mime = mime.clone();
        }
        let (data, converted) = image(std::mem::take(&mut r.data), options);
        if converted {
            r.mime = "image/webp".into();
        }
        resources.insert(id.clone(), asset(files, data));
    }
    Ok(
        json!({"id":id,"name":name,"list":list,"inspection":laymesh_language::inspect::inspect_scene(scene)}),
    )
}
fn runtime(editable: bool) -> Result<BTreeMap<String, Vec<u8>>> {
    let bytes = if let Some(path) = std::env::var_os("LAYMESH_WEB_RUNTIME") {
        std::fs::read(path)?
    } else if RUNTIME.is_empty() {
        let path = std::env::var_os("LAYMESH_WEB_RUNTIME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/web-export/runtime.zip")
            });
        std::fs::read(path).map_err(|_|"Web runtime missing: run python scripts/build-web-export.py before building the native distribution")?
    } else {
        RUNTIME.to_vec()
    };
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut files = BTreeMap::new();
    let index: Value = serde_json::from_reader(zip.by_name("runtime-files.json")?)?;
    let allowed: BTreeSet<String> = index[if editable { "editable" } else { "playback" }]
        .as_array()
        .ok_or("Invalid runtime index")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let name = f.name().to_owned();
        if !name.starts_with("licenses/")
            && !allowed.contains(&name)
            && !(editable && name.starts_with("runtime/vendor/") && name.ends_with("/LICENSE"))
        {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        if name == "licenses/manifest.json" {
            let mut inventory: Value = serde_json::from_slice(&data)?;
            let profile = if editable { "editable" } else { "playback" };
            if inventory["schemaVersion"] == 1 {
                let records = inventory["packages"]
                    .as_array_mut()
                    .ok_or("Invalid web license inventory")?;
                records.retain(|p| {
                    p["profiles"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|v| v == profile))
                });
                let used: BTreeSet<String> = records
                    .iter()
                    .flat_map(|p| p["texts"].as_array().into_iter().flatten())
                    .filter_map(|r| r["sha256"].as_str().map(str::to_owned))
                    .collect();
                inventory["texts"]
                    .as_object_mut()
                    .ok_or("Invalid web license texts")?
                    .retain(|hash, _| used.contains(hash));
                data = serde_json::to_vec(&inventory)?;
            }
        }
        files.insert(name, data);
    }
    Ok(files)
}
fn zip(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        let mut deflater =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        deflater.write_all(data)?;
        let smaller = deflater.finish()?.len() < data.len();
        writer.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(if smaller {
                    zip::CompressionMethod::Deflated
                } else {
                    zip::CompressionMethod::Stored
                })
                .compression_level(if smaller { Some(9) } else { None }),
        )?;
        writer.write_all(data)?;
    }
    Ok(writer.finish()?.into_inner())
}
fn universal(zip: &[u8], bootstrap: &[u8]) -> Result<Vec<u8>> {
    let mut cr = Vec::new();
    let mut i = 0;
    let mut previous = 0;
    while i < zip.len() {
        if zip[i] == 13 {
            let pair = zip.get(i + 1) == Some(&10);
            let mut v = ((i - previous) as u64) * 2 + u64::from(pair);
            previous = i;
            while v >= 128 {
                cr.push(((v & 127) as u8) | 128);
                v >>= 7;
            }
            cr.push(v as u8);
            if pair {
                i += 1
            }
        }
        i += 1;
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&cr)?;
    use base64::Engine;
    let records = serde_json::to_string(
        &base64::engine::general_purpose::STANDARD.encode(encoder.finish()?),
    )?;
    let script = std::str::from_utf8(bootstrap)?.replace("</script", "<\\/script");
    let header = format!(
        "<!doctype html><meta charset=\"windows-1252\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>LayMesh</title><style>plaintext{{display:none}}body{{margin:0;background:#f6f7fa;color:#263247;font:16px system-ui}}</style><div id=\"app\">Preparing LayMesh...</div><script type=\"module\">{script}\ntry{{const zipBytes=location.protocol.startsWith(\"http\")?webPayload(new Uint8Array(await (await fetch(location.href)).arrayBuffer())):webRecover(document.getElementById(\"laymesh-zip\").textContent,{},{});document.getElementById(\"laymesh-zip\").remove();await webBoot(webUnzip(zipBytes));}}catch(error){{document.getElementById(\"app\").textContent=String(error);console.error(error);}}</script><plaintext hidden id=\"laymesh-zip\">",
        zip.len(),
        records
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(zip);
    Ok(out)
}
pub fn export(
    file: &str,
    source: &str,
    mut host: Host,
    output: &Path,
    options: &ExportOptions,
    selected: Option<usize>,
    warnings: &str,
) -> Result<Value> {
    let started = std::time::Instant::now();
    let editable = options.html_editable.unwrap_or(false);
    let journal = Rc::new(RefCell::new(BTreeSet::new()));
    host.dependencies = Some(journal.clone());
    host.files.insert(file.into(), source.as_bytes().to_vec());
    let mut files = runtime(editable)?;
    let mut resources = BTreeMap::new();
    let mut pages = Vec::new();
    let mut transitions = BTreeSet::new();
    let mut fonts = BTreeMap::new();
    let mut requested_fonts = BTreeMap::new();
    let mut name = Path::new(file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut append = |scene: Scene, id: &str, title: &str| -> Result<()> {
        super::documents::scene_warnings(&scene, warnings);
        if editable {
            for request in &scene.font_requests {
                requested_fonts.insert(request.to_string(), request.clone());
            }
            for f in scene.fonts.values() {
                fonts
                    .entry(laymesh_core::asset_cache::digest(&f.data))
                    .or_insert(f.clone());
            }
        }
        let originals = original_images(&host, &journal.borrow());
        let body = page(
            &scene,
            id,
            title,
            &mut files,
            &mut resources,
            options,
            &originals,
        )?;
        let path = format!("pages/{}.json", pages.len());
        if let Some(name) = body["list"]["pageTransition"]["name"].as_str() {
            transitions.insert(name.to_owned());
        }
        files.insert(path.clone(), serde_json::to_vec(&body)?);
        pages.push(json!({"id":id,"name":title,"file":path}));
        Ok(())
    };
    if file.ends_with(".laydoc") {
        let plan = laymesh_core::document::plan_source(source, file, host.clone())?;
        name = plan.name.clone();
        for (i, p) in plan.pages.iter().enumerate() {
            if selected.is_none_or(|n| n == i + 1) {
                append(
                    laymesh_core::document::compile_page(p, host.clone())?,
                    &p.id,
                    &p.name,
                )?;
            }
        }
        if pages.is_empty() {
            return Err("Invalid page selection".into());
        }
    } else {
        append(compile_source(source, file, host.clone())?, "page", &name)?;
    }
    if !editable {
        // Playback cannot add effects. Keep every used page boundary, including
        // those traversed backwards, but omit all other upstream shaders.
        let key = "runtime/presentation-transitions-data.mjs";
        let source = std::str::from_utf8(files.get(key).ok_or("Missing transition catalog")?)?;
        let (_, value) = source
            .split_once("export const transitions = ")
            .ok_or("Invalid transition catalog")?;
        let mut catalog: BTreeMap<String, Value> =
            serde_json::from_str(value.trim().trim_end_matches(';'))?;
        if transitions.iter().any(|name| !catalog.contains_key(name)) {
            return Err("Missing used transition in runtime catalog".into());
        }
        catalog.retain(|name, _| transitions.contains(name));
        files.insert(
            key.into(),
            format!(
                "export const transitions = {};\n",
                serde_json::to_string(&catalog)?
            )
            .into_bytes(),
        );
    }
    let mut font_meta = Vec::new();
    let mut project = json!(null);
    let mut resource_info = Vec::new();
    if editable {
        let mut dependencies = journal.borrow().clone();
        dependencies.insert(file.into());
        let paths: Vec<_> = dependencies
            .iter()
            .filter(|p| host.files.contains_key(*p) || Path::new(p).is_file())
            .collect();
        let mut base = Path::new(file)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        while paths.iter().any(|p| !Path::new(p).starts_with(&base)) {
            if !base.pop() {
                break;
            }
        }
        let logical = |p: &str| {
            format!(
                "/project/{}",
                Path::new(p)
                    .strip_prefix(&base)
                    .unwrap_or(Path::new(p))
                    .to_string_lossy()
                    .replace('\\', "/")
            )
        };
        let mut entries = BTreeMap::new();
        let mut project_faces = BTreeMap::new();
        for p in paths {
            let bytes = host
                .files
                .get(p)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| std::fs::read(p))?;
            // Source paths retain their original encoding for recompilation.
            // The prepared playback resources are optimized independently.
            let path = logical(p);
            if ["ttf", "otf", "ttc", "otc"].contains(
                &Path::new(p)
                    .extension()
                    .and_then(|v| v.to_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .as_str(),
            ) {
                for face in laymesh_core::text::font_metadata(&bytes) {
                    project_faces.insert(
                        face["postscriptName"].as_str().unwrap_or("").to_owned(),
                        (path.clone(), face),
                    );
                }
            }
            let storage = asset(&mut files, bytes);
            resource_info.push(json!({"path":path,"file":storage,"name":Path::new(p).file_name().unwrap_or_default().to_string_lossy(),"readonly":false,"type":match Path::new(p).extension().and_then(|v|v.to_str()).unwrap_or(""){ "lay"|"laydoc"=>"source","lcss"|"css"=>"style","ttf"|"otf"|"ttc"|"otc"=>"font","png"|"jpg"|"jpeg"|"webp"|"tif"|"tiff"|"svg"|"bmp"|"gif"=>"image",_=>"data"}}));
            entries.insert(path, storage);
        }
        project = json!({"entry":logical(file),"page":selected,"files":entries});
        for (id, font) in fonts {
            let mut face = laymesh_core::text::font_metadata(&font.data)
                .into_iter()
                .next()
                .unwrap_or_else(|| json!({"family":font.family,"fontIndex":font.index}));
            let project_face = project_faces.get(face["postscriptName"].as_str().unwrap_or(""));
            let path = project_face
                .map(|(p, _)| p.clone())
                .unwrap_or_else(|| format!("/@exported/{id}.ttf"));
            face["path"] = json!(path);
            face["source"] = json!(if project_face.is_some() {
                "project"
            } else {
                "system"
            });
            if let Some((path, _)) = project_face {
                face["file"] = project["files"][path].clone();
            } else if options.html_embed_fonts.unwrap_or(false) {
                face["file"] = json!(asset(&mut files, font.data));
            }
            font_meta.push(face);
        }
        for library in laymesh_core::style::bundled_stylesheets() {
            let path = format!("laymesh-library:/{}.css", library.name);
            let storage = asset(
                &mut files,
                laymesh_core::style::bundled_stylesheet(&library.path())
                    .unwrap()
                    .as_bytes()
                    .to_vec(),
            );
            resource_info.push(json!({"path":path,"file":storage,"name":library.name.to_owned()+".css","readonly":true,"type":"library"}));
        }
    }
    let standard = output
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("zip"));
    let has_media=files.iter().any(|(k,v)|k.starts_with("pages/")&&serde_json::from_slice::<Value>(v).ok().is_some_and(|p|p["list"]["media"].is_object()));
    let manifest = json!({"schemaVersion":if has_media{3}else{2},"capabilities":if has_media{json!(["media-v1"])}else{json!([])},"resourceInfo":resource_info,"embedFonts":options.html_embed_fonts.unwrap_or(false),"fontRequirements":font_meta,"requestedFonts":requested_fonts.into_values().collect::<Vec<_>>(),"name":name,"editable":editable,"container":if standard{"zip"}else{"universal"},"pages":pages,"resources":resources,"fonts":font_meta,"project":project,"imageCompression":options.html_image_compression.as_deref().unwrap_or("auto"),"imageQuality":options.html_image_quality.unwrap_or(85.),"laymeshVersion":env!("CARGO_PKG_VERSION")});
    files.insert("manifest.json".into(), serde_json::to_vec(&manifest)?);
    files.insert("index.html".into(),br#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LayMesh</title><div id="app">Preparing LayMesh...</div><script type="module">import {webBoot} from './runtime/web-archive.mjs';try{const manifest=await(await fetch('manifest.json')).json();const response=await(await fetch('entries.json')).json();const files={};await Promise.all(response.map(async p=>{const r=await fetch(p);if(!r.ok)throw Error('Missing entry: '+p);files[p]=new Uint8Array(await r.arrayBuffer());}));await webBoot(files);}catch(e){document.getElementById('app').textContent=location.protocol==='file:'?'Open this folder through a static HTTP server, or use the Universal .zip.html file.':String(e);}</script>"#.to_vec());
    files.insert("README.txt".into(),b"LayMesh portable web presentation\nOpen the Universal .zip.html file directly in a desktop browser.\nFor the extracted standard ZIP, serve this directory over HTTP and open index.html.\nEditing and playback run locally. No CDN or remote compilation.\n".to_vec());
    let names: Vec<_> = files
        .keys()
        .cloned()
        .chain(["entries.json".into()])
        .collect();
    files.insert("entries.json".into(), serde_json::to_vec(&names)?);
    let raw: usize = files.values().map(Vec::len).sum();
    let packed = zip(&files)?;
    let data = if standard {
        packed.clone()
    } else {
        universal(&packed, files.get("runtime/bootstrap.js").unwrap())?
    };
    super::export::write_atomic(output, &data)?;
    Ok(
        json!({"exported":output,"output":output,"pages":pages.len(),"editable":editable,"uncompressedBytes":raw,"zipBytes":packed.len(),"bytes":data.len(),"containerOverheadBytes":data.len()-packed.len(),"prepareMs":started.elapsed().as_secs_f64()*1000.}),
    )
}
