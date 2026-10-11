//! SkPDF pages and a narrow object-level precision/semantic compatibility patch.
use crate::{
    display_list::*,
    error,
    skia::{self, Painter},
};
use laymesh_core::Result;
use lopdf::{
    Document, Object, Stream,
    content::{Content, Operation},
    dictionary,
};
use skia_safe as sk;
use std::collections::BTreeMap;
fn pdf_error(e: impl std::fmt::Display) -> laymesh_core::Diagnostic {
    error(format!("PDF: {e}"))
}
fn utf16(text: &str) -> Object {
    let mut bytes = vec![0xfe, 0xff];
    for c in text.encode_utf16() {
        bytes.extend(c.to_be_bytes());
    }
    Object::String(bytes, lopdf::StringFormat::Hexadecimal)
}
struct OriginalImage {
    marker: Vec<u8>,
    width: u32,
    height: u32,
    channels: i64,
    color: Vec<u8>,
    alpha: Option<Vec<u8>>,
}
fn high_image(resource: &Resource, resource_id: &str) -> Result<Option<OriginalImage>> {
    let image = image::load_from_memory(&resource.data).map_err(pdf_error)?;
    if !matches!(
        image.color(),
        image::ColorType::L16
            | image::ColorType::La16
            | image::ColorType::Rgb16
            | image::ColorType::Rgba16
    ) {
        return Ok(None);
    }
    let gray = matches!(
        image.color(),
        image::ColorType::L16 | image::ColorType::La16
    );
    let channels = if gray { 1 } else { 3 };
    let mut color = vec![];
    let mut alpha = image.color().has_alpha().then(Vec::new);
    for p in image.to_rgba16().pixels() {
        for k in 0..channels {
            color.extend(p[k].to_be_bytes());
        }
        if let Some(a) = &mut alpha {
            a.extend(p[3].to_be_bytes());
        }
    }
    // Nonuniform opaque markers survive SkPDF's solid-image optimization.
    // They are replaced before bytes leave the exporter.
    let key = laymesh_core::asset_cache::digest(
        format!(
            "{resource_id}:{}",
            laymesh_core::asset_cache::digest(&resource.data)
        )
        .as_bytes(),
    );
    let marker = (0..105)
        .map(|i| key.as_bytes()[i % key.len()].wrapping_add(i as u8))
        .collect();
    Ok(Some(OriginalImage {
        marker,
        width: image.width(),
        height: image.height(),
        channels: channels as i64,
        color,
        alpha,
    }))
}
pub(crate) fn render(list: &DisplayList, dpi: f64) -> Result<Vec<u8>> {
    let mut painter = Painter::new(list)?;
    painter.pdf = true;
    painter.dpi = dpi;
    let mut root = sk::pdf::StructureElementNode::new("Document");
    let mut semantic = BTreeMap::new();
    fn tags(
        nodes: &[DrawNode],
        root: &mut sk::pdf::StructureElementNode,
        ids: &mut BTreeMap<usize, i32>,
        texts: &mut BTreeMap<String, String>,
        inside: bool,
    ) {
        for n in nodes {
            let has = n.semantic.is_some();
            if let Some(text) = &n.semantic {
                if !inside {
                    let id = ids.len() as i32 + 1;
                    let marker = format!("laymesh-semantic-{id}");
                    let mut tag = sk::pdf::StructureElementNode::new("Span");
                    tag.set_node_id(id).set_alt(&marker);
                    root.append_child(tag);
                    ids.insert(n as *const _ as usize, id);
                    texts.insert(marker, text.clone());
                }
            }
            if let DrawKind::Group { children, .. } = &n.kind {
                tags(children, root, ids, texts, inside || has);
            }
        }
    }
    tags(
        &list.nodes,
        &mut root,
        &mut painter.semantic_ids,
        &mut semantic,
        false,
    );
    let mut originals = vec![];
    for (id, r) in &list.resources {
        if r.mime.starts_with("image/") && r.mime != "image/x-laymesh-rgba" {
            if let Some(original) = high_image(r, id)? {
                let rgba: Vec<_> = original
                    .marker
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
                painter
                    .images
                    .insert(id.clone(), skia::rgba_image(7, 5, &rgba)?);
                originals.push(original);
            }
        }
    }
    let metadata = sk::pdf::Metadata {
        raster_dpi: Some(dpi as f32),
        encoding_quality: Some(101),
        structure_element_tree_root: (!semantic.is_empty()).then_some(root),
        ..Default::default()
    };
    let mut bytes = vec![];
    {
        let doc = sk::pdf::new_document(&mut bytes, Some(&metadata));
        let mut page = doc.begin_page(
            (
                (list.width * 72. / 25.4) as f32,
                (list.height * 72. / 25.4) as f32,
            ),
            None,
        );
        page.canvas().scale((72. / 25.4, 72. / 25.4));
        painter.draw(page.canvas(), &BTreeMap::new())?;
        page.end_page().close();
    }
    let mut doc = Document::load_mem(&bytes).map_err(pdf_error)?;
    // SkPDF rounds its internal device to raster-DPI pixels. Restore the exact
    // physical page and top-edge origin instead of exposing that rounding.
    for id in doc.get_pages().values() {
        let page = doc
            .get_object(*id)
            .map_err(pdf_error)?
            .as_dict()
            .map_err(pdf_error)?;
        let bounds = page
            .get(b"MediaBox")
            .map_err(pdf_error)?
            .as_array()
            .map_err(pdf_error)?;
        let old_height = bounds[3].as_float().map_err(pdf_error)?;
        let width = (list.width * 72. / 25.4) as f32;
        let height = (list.height * 72. / 25.4) as f32;
        let mut content = format!("q\n1 0 0 1 0 {} cm\n", height - old_height).into_bytes();
        content.extend(doc.get_page_content(*id).map_err(pdf_error)?);
        content.extend(b"\nQ\n");
        doc.change_page_content(*id, content).map_err(pdf_error)?;
        doc.get_object_mut(*id)
            .map_err(pdf_error)?
            .as_dict_mut()
            .map_err(pdf_error)?
            .set(
                "MediaBox",
                vec![
                    0.into(),
                    0.into(),
                    Object::Real(width),
                    Object::Real(height),
                ],
            );
    }
    patch_images(&mut doc, originals)?;
    patch_semantics(&mut doc, &semantic)?;
    let mut out = vec![];
    doc.save_to(&mut out).map_err(pdf_error)?;
    Ok(out)
}
fn patch_images(doc: &mut Document, images: Vec<OriginalImage>) -> Result<()> {
    for image in images {
        let ids: Vec<_> = doc
            .objects
            .iter()
            .filter_map(|(id, o)| {
                let s = o.as_stream().ok()?;
                if s.dict.get(b"Subtype").ok()?.as_name().ok() != Some(b"Image") {
                    return None;
                }
                if s.dict.get(b"Width").ok()?.as_i64().ok() != Some(7)
                    || s.dict.get(b"Height").ok()?.as_i64().ok() != Some(5)
                    || s.dict.get(b"BitsPerComponent").ok()?.as_i64().ok() != Some(8)
                {
                    return None;
                }
                let bytes = s
                    .decompressed_content()
                    .unwrap_or_else(|_| s.content.clone());
                (bytes == image.marker).then_some(*id)
            })
            .collect();
        for id in ids {
            let mut dict = dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>image.width as i64,"Height"=>image.height as i64,"BitsPerComponent"=>16,"ColorSpace"=>if image.channels==1{"DeviceGray"}else{"DeviceRGB"}};
            if let Some(alpha) = &image.alpha {
                let mut s = Stream::new(
                    dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>image.width as i64,"Height"=>image.height as i64,"BitsPerComponent"=>16,"ColorSpace"=>"DeviceGray"},
                    alpha.clone(),
                );
                s.compress().map_err(pdf_error)?;
                dict.set("SMask", doc.add_object(s));
            }
            let mut stream = Stream::new(dict, image.color.clone());
            stream.compress().map_err(pdf_error)?;
            doc.objects.insert(id, Object::Stream(stream));
        }
    }
    Ok(())
}
fn patch_semantics(doc: &mut Document, texts: &BTreeMap<String, String>) -> Result<()> {
    // MCIDs are local to a page content stream or a Form XObject. SkPDF
    // creates Forms for opacity layers; MCID 0 in two Forms is not one tag.
    type Scope = Option<lopdf::ObjectId>;
    let mut mcids: BTreeMap<(Scope, i64), String> = BTreeMap::new();
    fn collect(
        doc: &Document,
        object: &Object,
        text: &str,
        out: &mut BTreeMap<(Scope, i64), String>,
    ) {
        let Ok((_, object)) = doc.dereference(object) else {
            return;
        };
        match object {
            Object::Integer(id) => {
                out.insert((None, *id), text.into());
            }
            Object::Array(a) => {
                for object in a {
                    collect(doc, object, text, out);
                }
            }
            Object::Dictionary(d) => {
                if let Ok(id) = d.get(b"MCID").and_then(Object::as_i64) {
                    let stream = d.get(b"Stm").ok().and_then(|o| o.as_reference().ok());
                    out.insert((stream, id), text.into());
                }
            }
            _ => {}
        }
    }
    let tags: Vec<_> = doc
        .objects
        .iter()
        .filter_map(|(id, object)| {
            let d = object.as_dict().ok()?;
            let marker = String::from_utf8_lossy(d.get(b"Alt").ok()?.as_str().ok()?);
            Some((
                *id,
                d.get(b"K").ok()?.clone(),
                texts.get(marker.as_ref())?.clone(),
            ))
        })
        .collect();
    for (id, kids, text) in tags {
        collect(doc, &kids, &text, &mut mcids);
        let d = doc.objects.get_mut(&id).unwrap().as_dict_mut().unwrap();
        d.set("ActualText", utf16(&text));
        d.remove(b"Alt");
    }
    // Only parse drawing streams, never embedded fonts or other binary data.
    let mut streams: BTreeMap<_, Scope> = doc
        .get_pages()
        .values()
        .flat_map(|id| doc.get_page_contents(*id))
        .map(|id| (id, None))
        .collect();
    for (id, object) in &doc.objects {
        if let Ok(s) = object.as_stream() {
            if s.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Form") {
                streams.insert(*id, Some(*id));
            }
        }
    }
    for (id, scope) in streams {
        let Some(Object::Stream(s)) = doc.objects.get_mut(&id) else {
            continue;
        };
        let bytes = s
            .decompressed_content()
            .unwrap_or_else(|_| s.content.clone());
        let Ok(content) = Content::decode(&bytes) else {
            continue;
        };
        let mut depth = Vec::new();
        let mut changed = false;
        let mut ops = vec![];
        for mut op in content.operations {
            if op.operator == "BDC" || op.operator == "BMC" {
                let text = op
                    .operands
                    .get(1)
                    .and_then(|o| o.as_dict().ok())
                    .and_then(|d| d.get(b"MCID").ok())
                    .and_then(|o| o.as_i64().ok())
                    .and_then(|id| mcids.get(&(scope, id)));
                depth.push(text.is_some());
                if let Some(text) = text {
                    op.operands[1]
                        .as_dict_mut()
                        .unwrap()
                        .set("ActualText", utf16(text));
                    changed = true;
                }
            }
            let invisible = depth.iter().any(|v| *v);
            if invisible && matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                ops.push(Operation::new("Tr", vec![3.into()]));
                ops.push(op);
                ops.push(Operation::new("Tr", vec![0.into()]));
            } else {
                if op.operator == "EMC" {
                    depth.pop();
                }
                ops.push(op);
            }
        }
        if changed {
            s.set_plain_content(Content { operations: ops }.encode().map_err(pdf_error)?);
            s.compress().map_err(pdf_error)?;
        }
    }
    Ok(())
}
pub(crate) fn merge(pages: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    if pages.len() == 1 {
        return Ok(pages.into_iter().next().unwrap());
    }
    let mut result = Document::with_version("1.7");
    let pages_id = result.new_object_id();
    let structure_id = result.new_object_id();
    let mut structure_kids = vec![];
    let mut parent_numbers = vec![];
    let mut parent_offset = 0i64;
    let mut kids = vec![];
    for bytes in pages {
        let mut doc = Document::load_mem(&bytes).map_err(pdf_error)?;
        doc.renumber_objects_with(result.max_id + 1);
        // Each SkPDF page starts its own structure-parent number tree at zero.
        // Rebase both page/Form keys and number-tree keys before joining pages.
        if let Some(root) = doc
            .catalog()
            .ok()
            .and_then(|c| c.get(b"StructTreeRoot").ok())
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
        {
            if let Ok(k) = root.get(b"K") {
                let children = match k {
                    Object::Array(a) => a.clone(),
                    other => vec![other.clone()],
                };
                for child in children {
                    if let Ok(id) = child.as_reference() {
                        if let Ok(d) = doc.get_object_mut(id).and_then(Object::as_dict_mut) {
                            d.set("P", structure_id);
                        }
                    }
                    structure_kids.push(child);
                }
            }
            fn numbers(doc: &Document, node: &Object, output: &mut Vec<Object>) {
                if let Ok((_, Object::Dictionary(d))) = doc.dereference(node) {
                    if let Ok(a) = d.get(b"Nums").and_then(Object::as_array) {
                        output.extend(a.iter().cloned());
                    }
                    if let Ok(a) = d.get(b"Kids").and_then(Object::as_array) {
                        for child in a {
                            numbers(doc, child, output);
                        }
                    }
                }
            }
            let mut entries = vec![];
            if let Ok(tree) = root.get(b"ParentTree") {
                numbers(&doc, tree, &mut entries);
            }
            let mut next = parent_offset;
            for pair in entries.chunks_exact_mut(2) {
                let key = pair[0].as_i64().map_err(pdf_error)? + parent_offset;
                pair[0] = key.into();
                next = next.max(key + 1);
            }
            for object in doc.objects.values_mut() {
                let dictionary = match object {
                    Object::Dictionary(d) => Some(d),
                    Object::Stream(s) => Some(&mut s.dict),
                    _ => None,
                };
                if let Some(d) = dictionary {
                    for key in [b"StructParents".as_slice(), b"StructParent".as_slice()] {
                        if let Ok(n) = d.get(key).and_then(Object::as_i64) {
                            d.set(key, n + parent_offset);
                        }
                    }
                }
            }
            parent_numbers.extend(entries);
            parent_offset = next;
        }
        let page_ids = doc.get_pages();
        for id in page_ids.values() {
            doc.get_object_mut(*id)
                .map_err(pdf_error)?
                .as_dict_mut()
                .map_err(pdf_error)?
                .set("Parent", pages_id);
            kids.push(Object::Reference(*id));
        }
        result.max_id = doc.max_id;
        result.objects.extend(doc.objects);
    }
    result.objects.insert(
        pages_id,
        dictionary! {"Type"=>"Pages","Count"=>kids.len() as i64,"Kids"=>kids}.into(),
    );
    let mut catalog = dictionary! {"Type"=>"Catalog","Pages"=>pages_id};
    if !structure_kids.is_empty() {
        let parent_tree = result.add_object(dictionary! {"Nums"=>parent_numbers});
        result.objects.insert(structure_id,dictionary!{"Type"=>"StructTreeRoot","K"=>structure_kids,"ParentTree"=>parent_tree,"ParentTreeNextKey"=>parent_offset}.into());
        catalog.set("StructTreeRoot", structure_id);
        catalog.set("MarkInfo", dictionary! {"Marked"=>true});
    }
    let catalog = result.add_object(catalog);
    result.trailer.set("Root", catalog);
    let mut bytes = vec![];
    result.save_to(&mut bytes).map_err(pdf_error)?;
    Ok(bytes)
}
