//! Shared multi-slide packaging for native and WASM hosts.
use super::*;
use std::{collections::BTreeMap, io::Read};

#[derive(Default)]
pub struct PptxDocument {
    files: BTreeMap<String, Vec<u8>>,
    size: Option<(i64, i64)>,
    pages: usize,
}
impl PptxDocument {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(feature = "native")]
    pub fn append(&mut self, scene: &Scene, options: &ExportOptions) -> Result<Vec<Diagnostic>> {
        let mut page = scene.clone();
        if self.pages > 0 {
            page.media.cues.retain(|c| c.scope != "document");
        }
        let (bytes, warnings) = render_pptx(&page, options)?;
        self.add_page(&bytes, scene.width, scene.height)?;
        Ok(warnings)
    }
    /// Accepts a single-page package produced by render_pptx_list/PptxJob.
    pub fn add_page(&mut self, bytes: &[u8], width: f64, height: f64) -> Result<()> {
        let size = (emu(width), emu(height));
        if self.size.is_some_and(|old| old != size) {
            return Err(error(format!(
                "PPTX page {} has a different canvas size; all pages must match page 1",
                self.pages + 1
            )));
        }
        let mut archive =
            zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| error(e.to_string()))?;
        let mut parts = BTreeMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| error(e.to_string()))?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| error(e.to_string()))?;
            parts.insert(file.name().to_owned(), data);
        }
        let slide = parts
            .remove("ppt/slides/slide1.xml")
            .ok_or_else(|| error("Missing PPTX slide"))?;
        let rel = parts
            .remove("ppt/slides/_rels/slide1.xml.rels")
            .ok_or_else(|| error("Missing PPTX slide relationships"))?;
        let rel = String::from_utf8(rel).map_err(|e| error(e.to_string()))?;
        let xml = roxmltree::Document::parse(&rel).map_err(|e| error(e.to_string()))?;
        let mut relationships = Vec::new();
        let mut media = BTreeMap::new();
        for r in xml.root_element().children().filter(|n| n.is_element()) {
            if r.attribute("TargetMode") == Some("External") {
                return Err(error("PPTX media must be embedded"));
            }
            let id = r
                .attribute("Id")
                .ok_or_else(|| error("Missing relationship ID"))?;
            let kind = r
                .attribute("Type")
                .ok_or_else(|| error("Missing relationship type"))?;
            let target = r
                .attribute("Target")
                .ok_or_else(|| error("Missing relationship target"))?;
            let target = if let Some(name) = target.strip_prefix("../media/") {
                let data = parts
                    .get(&format!("ppt/media/{name}"))
                    .ok_or_else(|| error("Missing embedded media"))?;
                let ext = name.rsplit('.').next().unwrap_or("bin");
                let name = format!("{}.{}", laymesh_core::asset_cache::digest(data), ext);
                media
                    .entry(format!("ppt/media/{name}"))
                    .or_insert_with(|| data.clone());
                format!("../media/{name}")
            } else {
                target.to_owned()
            };
            relationships.push((id.to_owned(), kind.to_owned(), target));
        }
        let rel = rels(
            &relationships
                .iter()
                .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
                .collect::<Vec<_>>(),
        );
        // Commit the page only after all validation succeeds.
        if self.pages == 0 {
            for (name, data) in parts {
                if !name.starts_with("ppt/media/") {
                    self.files.insert(name, data);
                }
            }
        }
        self.files.extend(media);
        self.pages += 1;
        self.size = Some(size);
        self.files
            .insert(format!("ppt/slides/slide{}.xml", self.pages), slide);
        self.files.insert(
            format!("ppt/slides/_rels/slide{}.xml.rels", self.pages),
            rel.into_bytes(),
        );
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        if self.pages == 0 {
            return Err(error("PPTX requires at least one page"));
        }
        let mut files = self.files;
        let mut presentation = String::from_utf8(
            files
                .remove("ppt/presentation.xml")
                .ok_or_else(|| error("Missing presentation"))?,
        )
        .map_err(|e| error(e.to_string()))?;
        let start = presentation
            .find("<p:sldIdLst>")
            .ok_or_else(|| error("Missing slide list"))?;
        let end = presentation
            .find("</p:sldIdLst>")
            .ok_or_else(|| error("Missing slide list"))?
            + "</p:sldIdLst>".len();
        let ids = (0..self.pages)
            .map(|i| format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 256 + i, 2 + i))
            .collect::<String>();
        presentation.replace_range(start..end, &format!("<p:sldIdLst>{ids}</p:sldIdLst>"));
        files.insert("ppt/presentation.xml".into(), presentation.into_bytes());
        let mut relationships = vec![(
            "rId1".to_owned(),
            "slideMaster".to_owned(),
            "slideMasters/slideMaster1.xml".to_owned(),
        )];
        relationships.extend((0..self.pages).map(|i| {
            (
                format!("rId{}", i + 2),
                "slide".into(),
                format!("slides/slide{}.xml", i + 1),
            )
        }));
        files.insert(
            "ppt/_rels/presentation.xml.rels".into(),
            rels(
                &relationships
                    .iter()
                    .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
                    .collect::<Vec<_>>(),
            )
            .into_bytes(),
        );
        let mut types = String::from_utf8(
            files
                .remove("[Content_Types].xml")
                .ok_or_else(|| error("Missing content types"))?,
        )
        .map_err(|e| error(e.to_string()))?;
        let extra=(2..=self.pages).map(|i|format!("<Override PartName=\"/ppt/slides/slide{i}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>")).collect::<String>();
        types = types.replace("</Types>", &format!("{extra}</Types>"));
        files.insert("[Content_Types].xml".into(), types.into_bytes());
        // Set the span only on the first slide's document-scoped audio. A
        // standalone slide always stops its own audio on leaving the slide.
        let bytes = files.get_mut("ppt/slides/slide1.xml").unwrap();
        let mut text = String::from_utf8(bytes.clone()).map_err(|e| error(e.to_string()))?;
        let xml = roxmltree::Document::parse(&text).map_err(|e| error(e.to_string()))?;
        let ids = xml
            .descendants()
            .filter(|n| {
                n.has_tag_name((P, "cNvPr"))
                    && n.attribute("name")
                        .is_some_and(|s| s.starts_with("document audio "))
            })
            .filter_map(|n| n.attribute("id"))
            .collect::<BTreeSet<_>>();
        if !ids.is_empty() && self.pages > 999 {
            return Err(error("Cross-slide music supports at most 999 slides"));
        }
        let ranges = xml
            .descendants()
            .filter(|n| {
                n.has_tag_name((P, "cMediaNode"))
                    && n.descendants().any(|c| {
                        c.has_tag_name((P, "spTgt"))
                            && c.attribute("spid").is_some_and(|id| ids.contains(id))
                    })
            })
            .map(|n| n.range())
            .collect::<Vec<_>>();
        for range in ranges.into_iter().rev() {
            let replacement =
                text[range.clone()].replace("numSld=\"1\"", &format!("numSld=\"{}\"", self.pages));
            text.replace_range(range, &replacement);
        }
        *bytes = text.into_bytes();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in files {
            zip.start_file(
                name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(|e| error(e.to_string()))?;
            zip.write_all(&data).map_err(|e| error(e.to_string()))?;
        }
        Ok(zip.finish().map_err(|e| error(e.to_string()))?.into_inner())
    }
}
