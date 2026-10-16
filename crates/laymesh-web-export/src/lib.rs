//! Offline export codecs. No filesystem, source execution, or system font access.
use laymesh_render::{DisplayList, ExportOptions};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
};
use wasm_bindgen::prelude::*;
fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn options(raw: &str, format: &str) -> Result<ExportOptions, JsValue> {
    let options: ExportOptions = serde_json::from_str(raw).map_err(err)?;
    options.validate(format).map_err(err)?;
    Ok(options)
}
thread_local! { static RESOURCES: RefCell<BTreeMap<String, Vec<u8>>> = RefCell::new(BTreeMap::new()); }
#[wasm_bindgen]
pub fn clear_resources() {
    RESOURCES.with(|r| r.borrow_mut().clear());
}
#[wasm_bindgen]
pub fn register_resource(id: &str, bytes: &[u8]) {
    RESOURCES.with(|r| r.borrow_mut().insert(id.into(), bytes.into()));
}
#[wasm_bindgen]
pub fn dimensions(width: f64, height: f64, format: &str, raw: &str) -> Result<Vec<u32>, JsValue> {
    let options = options(raw, format)?;
    let (w, h) =
        laymesh_render::raster_dimensions(width, height, options.dpi.unwrap_or(1200.), format)
            .map_err(err)?;
    Ok(vec![w, h])
}
#[wasm_bindgen]
pub fn encode_pixels(
    w: u32,
    h: u32,
    pixels: &[u8],
    format: &str,
    raw: &str,
) -> Result<Vec<u8>, JsValue> {
    let options = options(raw, format)?;
    laymesh_render::encode_rgba(w, h, pixels, options.dpi.unwrap_or(1200.), format, &options)
        .map_err(err)
}
fn hydrated_list(raw: &str) -> Result<DisplayList, JsValue> {
    let mut list: DisplayList = serde_json::from_str(raw).map_err(err)?;
    RESOURCES.with(|resources| {
        let resources = resources.borrow();
        for (id, r) in &mut list.resources {
            if let Some(bytes) = resources.get(id) {
                r.data = bytes.clone().into();
            }
        }
    });
    Ok(list)
}
/// Assemble vector output on demand from the same retained commands used for
/// playback. Only PDF effect layers need pixels; SVG retains SVG filters.
#[wasm_bindgen]
pub struct VectorJob {
    list: DisplayList,
    dpi: f64,
    pdf: bool,
    requests: Vec<Value>,
    raster_nodes: Vec<Value>,
    images: Vec<Vec<u8>>,
}
#[wasm_bindgen]
impl VectorJob {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_list: &str, raw_options: &str, pdf: bool) -> Result<VectorJob, JsValue> {
        let options = options(raw_options, if pdf { "pdf" } else { "svg" })?;
        let dpi = options.dpi.unwrap_or(1200.);
        let list = hydrated_list(raw_list)?;
        let mut requests = Vec::new();
        let mut raster_nodes = Vec::new();
        let mut node_ids = BTreeMap::new();
        if pdf {
            laymesh_render::portable_svg::render(&list,dpi,Some(&mut |node,rect,width,height,effects_only| {
                let key = serde_json::to_string(node).unwrap();
                let index = *node_ids.entry(key).or_insert_with(|| {
                    let index=raster_nodes.len();raster_nodes.push(json!(node));index
                });
                requests.push(json!({"node":index,"rect":[rect.x0,rect.y0,rect.x1,rect.y1],"width":width,"height":height,"effectsOnly":effects_only}));
                Ok(Vec::new())
            })).map_err(err)?;
        }
        Ok(Self {
            list,
            dpi,
            pdf,
            images: vec![Vec::new(); requests.len()],
            requests,
            raster_nodes,
        })
    }
    pub fn request_count(&self) -> usize {
        self.requests.len()
    }
    pub fn request(&self, index: usize) -> Result<String, JsValue> {
        let mut r = self
            .requests
            .get(index)
            .cloned()
            .ok_or_else(|| err("Invalid vector effect index"))?;
        let node_index = r["node"].as_u64().unwrap() as usize;
        r.as_object_mut().unwrap().remove("node");
        let resources: BTreeMap<_, _> = self
            .list
            .resources
            .iter()
            .map(|(id, r)| {
                (
                    id,
                    json!({"mime":r.mime,"data":[],"fontIndex":r.font_index,"family":r.family}),
                )
            })
            .collect();
        let list = json!({"schemaVersion":self.list.schema_version,"width":self.list.width,
            "height":self.list.height,"nodes":[self.raster_nodes[node_index]],"resources":resources});
        r["list"] = list;
        Ok(r.to_string())
    }
    pub fn requests(&self) -> String {
        let requests: Vec<Value> = (0..self.requests.len())
            .map(|i| serde_json::from_str(&self.request(i).unwrap()).unwrap())
            .collect();
        serde_json::to_string(&requests).unwrap()
    }
    pub fn set_image(&mut self, index: usize, bytes: &[u8]) -> Result<(), JsValue> {
        *self
            .images
            .get_mut(index)
            .ok_or_else(|| err("Invalid vector effect index"))? = bytes.into();
        Ok(())
    }
    pub fn finish(&self) -> Result<String, JsValue> {
        if self.images.iter().any(Vec::is_empty) {
            return Err(err("Vector effect image missing"));
        }
        let mut index = 0;
        let mut raster = |_: &laymesh_render::display_list::DrawNode, _, _, _, _| {
            let bytes = self.images[index].clone();
            index += 1;
            Ok(bytes)
        };
        laymesh_render::portable_svg::render(
            &self.list,
            self.dpi,
            if self.pdf { Some(&mut raster) } else { None },
        )
        .map_err(err)
    }
}
/// Incremental full PDF converter. Each SVG tree is dropped after adding its
/// compressed PDF objects, rather than retaining and parsing a whole deck.
#[wasm_bindgen]
pub struct PdfJob {
    pdf: Option<Pdf>,
    alloc: Ref,
    root: Ref,
    ids: Vec<Ref>,
    dpi: f64,
}
#[wasm_bindgen]
impl PdfJob {
    #[wasm_bindgen(constructor)]
    pub fn new(raw: &str) -> Result<PdfJob, JsValue> {
        let options = options(raw, "pdf")?;
        let mut pdf = Pdf::new();
        let mut alloc = Ref::new(1);
        let catalog = alloc.bump();
        let root = alloc.bump();
        pdf.catalog(catalog).pages(root);
        Ok(Self {
            pdf: Some(pdf),
            alloc,
            root,
            ids: Vec::new(),
            dpi: options.dpi.unwrap_or(1200.),
        })
    }
    pub fn add_page(&mut self, svg: &str, width: f64, height: f64) -> Result<(), JsValue> {
        let pdf = self
            .pdf
            .as_mut()
            .ok_or_else(|| err("PDF already finished"))?;
        if svg.contains("<filter") {
            return Err(err("PDF effects must be prepared by the shared renderer"));
        }
        if ![width, height].iter().all(|v| v.is_finite() && *v > 0.) {
            return Err(err("Invalid PDF page dimensions"));
        }
        let tree =
            svg2pdf::usvg::Tree::from_str(svg, &svg2pdf::usvg::Options::default()).map_err(err)?;
        let (chunk, form) = svg2pdf::to_chunk(
            &tree,
            svg2pdf::ConversionOptions {
                embed_text: false,
                raster_scale: (self.dpi / 96.) as f32,
                ..Default::default()
            },
        )
        .map_err(err)?;
        let mut map = HashMap::new();
        let alloc = &mut self.alloc;
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let form = map[&form];
        let id = alloc.bump();
        let content_id = alloc.bump();
        self.ids.push(id);
        let (w, h) = ((width * 72. / 25.4) as f32, (height * 72. / 25.4) as f32);
        let mut page = pdf.page(id);
        page.parent(self.root)
            .media_box(Rect::new(0., 0., w, h))
            .contents(content_id);
        page.resources().x_objects().pair(Name(b"Page"), form);
        page.finish();
        let mut content = Content::new();
        content
            .transform([w, 0., 0., h, 0., 0.])
            .x_object(Name(b"Page"));
        pdf.stream(content_id, &content.finish());
        pdf.extend(&chunk);
        Ok(())
    }
    pub fn finish(&mut self) -> Result<Vec<u8>, JsValue> {
        if self.ids.is_empty() {
            return Err(err("PDF has no pages"));
        }
        let mut pdf = self.pdf.take().ok_or_else(|| err("PDF already finished"))?;
        pdf.pages(self.root)
            .kids(self.ids.iter().copied())
            .count(self.ids.len() as i32);
        Ok(pdf.finish())
    }
}
#[wasm_bindgen]
pub fn export_pdf(pages: &str, raw: &str) -> Result<Vec<u8>, JsValue> {
    let pages: Vec<Value> = serde_json::from_str(pages).map_err(err)?;
    let mut job = PdfJob::new(raw)?;
    for page in pages {
        job.add_page(
            page["svg"]
                .as_str()
                .ok_or_else(|| err("Missing assembled vector page"))?,
            page["width"]
                .as_f64()
                .ok_or_else(|| err("Missing page width"))?,
            page["height"]
                .as_f64()
                .ok_or_else(|| err("Missing page height"))?,
        )?;
    }
    PdfJob::finish(&mut job)
}
#[wasm_bindgen]
pub struct PptxJob {
    list: DisplayList,
    dpi: f64,
    requests: Vec<Value>,
    images: Vec<Vec<u8>>,
}
#[wasm_bindgen]
impl PptxJob {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_list: &str, raw_options: &str) -> Result<PptxJob, JsValue> {
        let options = options(raw_options, "pptx")?;
        let dpi = options.dpi.unwrap_or(1200.);
        let list = hydrated_list(raw_list)?;
        let mut requests = Vec::new();
        laymesh_render::pptx::render_pptx_list(&list, dpi, &mut |list, rect, width, height| {
            let mut list = list.clone(); for r in list.resources.values_mut() { r.data=Default::default(); }
            requests.push(json!({"list":list,"rect":[rect.x0,rect.y0,rect.x1,rect.y1],"width":width,"height":height}));
            Ok(Vec::new())
        }).map_err(err)?;
        Ok(Self {
            list,
            dpi,
            images: vec![Vec::new(); requests.len()],
            requests,
        })
    }
    pub fn requests(&self) -> String {
        serde_json::to_string(&self.requests).unwrap()
    }
    pub fn set_image(&mut self, index: usize, bytes: &[u8]) -> Result<(), JsValue> {
        let image = self
            .images
            .get_mut(index)
            .ok_or_else(|| err("Invalid PPTX fallback index"))?;
        *image = bytes.into();
        Ok(())
    }
    pub fn finish(&self) -> Result<Vec<u8>, JsValue> {
        if self.images.iter().any(Vec::is_empty) {
            return Err(err("PPTX fallback image missing"));
        }
        let mut i = 0;
        let (bytes, _) =
            laymesh_render::pptx::render_pptx_list(&self.list, self.dpi, &mut |_, _, _, _| {
                let data = self.images[i].clone();
                i += 1;
                Ok(data)
            })
            .map_err(err)?;
        Ok(bytes)
    }
    pub fn warnings(&self) -> Result<String, JsValue> {
        let (_, warnings) =
            laymesh_render::pptx::render_pptx_list(&self.list, self.dpi, &mut |_, _, _, _| {
                Ok(Vec::new())
            })
            .map_err(err)?;
        Ok(serde_json::to_string(&warnings).unwrap())
    }
}
