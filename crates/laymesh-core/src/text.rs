//! Font discovery, Unicode shaping and RaTeX layout. Ordinary fonts are never embedded in this crate.
use crate::{Diagnostic, Loc, Result, model::*};
use fontdb::{Database, ID};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};
use ttf_parser::OutlineBuilder;
use unicode_segmentation::UnicodeSegmentation;

pub struct FontSystem {
    db: Database,
    pub assets: BTreeMap<String, FontAsset>,
    paths: BTreeMap<String, Vec<ID>>,
    choices: BTreeMap<String, Option<String>>,
    keys: BTreeMap<ID, String>,
    origins: BTreeMap<String, String>,
    face_names: BTreeMap<String, String>,
    host: Host,
}
fn warn(w: &mut Vec<Diagnostic>, s: impl Into<String>, file: &str, loc: Loc) {
    let s = s.into();
    if !w
        .iter()
        .any(|d| d.code == "W_FONT" && d.message == s && d.file == file && d.loc == loc)
    {
        w.push(Diagnostic::new("W_FONT", s, file, loc));
    }
}
fn canonical(s: &str) -> String {
    s.to_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn font_path(s: &str) -> bool {
    s.contains('/')
        || s.contains('\\')
        || [".ttf", ".otf", ".ttc", ".otc"].iter().any(|ext| {
            s.split('#')
                .next()
                .unwrap_or(s)
                .to_lowercase()
                .ends_with(ext)
        })
}
fn value_text(v: &Json) -> String {
    v.as_str()
        .or_else(|| v.get("value").and_then(Json::as_str))
        .unwrap_or("")
        .into()
}
// Keep the original font catalog's export contract: every selected face must
// permit embedding outlines and subsetting. Read the bits even on older OS/2
// versions, matching the previous catalog's conservative treatment of fsType.
fn embedding_restriction(face: &ttf_parser::Face<'_>) -> Option<&'static str> {
    let flags = face
        .raw_face()
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))
        .and_then(|table| table.get(8..10))
        .map(|v| u16::from_be_bytes([v[0], v[1]]))
        .unwrap_or(0);
    if flags & 2 != 0 {
        Some("禁止嵌入")
    } else if flags & 0x100 != 0 {
        Some("禁止子集嵌入")
    } else if flags & 0x200 != 0 {
        Some("仅允许位图嵌入")
    } else if face.is_variable() {
        Some("尚不支持变量字体嵌入")
    } else {
        None
    }
}
impl FontSystem {
    pub fn new(native: bool) -> Self {
        #[allow(unused_mut)]
        let mut db = Database::new();
        #[cfg(feature = "system-fonts")]
        if native
            && !std::env::var("LAYMESH_NO_SYSTEM_FONTS")
                .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        {
            static SYSTEM_FONTS: std::sync::OnceLock<Database> = std::sync::OnceLock::new();
            db = SYSTEM_FONTS
                .get_or_init(|| {
                    let mut db = Database::new();
                    db.load_system_fonts();
                    db
                })
                .clone();
        }
        Self {
            db,
            assets: BTreeMap::new(),
            paths: BTreeMap::new(),
            choices: BTreeMap::new(),
            keys: BTreeMap::new(),
            origins: BTreeMap::new(),
            face_names: BTreeMap::new(),
            host: Host {
                native,
                ..Host::default()
            },
        }
    }
    /// Add user-provided font bytes (including TTC/OTC collections), without accessing the filesystem.
    pub fn register_font(&mut self, name: &str, data: Vec<u8>) {
        self.choices.clear();
        let ids = self
            .db
            .load_font_source(fontdb::Source::Binary(std::sync::Arc::new(data)));
        self.paths.insert(name.into(), ids.to_vec());
    }
    pub fn load_requested(
        &mut self,
        v: &V,
        host: &Host,
        file: &str,
        loc: Loc,
        warnings: &mut Vec<Diagnostic>,
    ) {
        self.host.native = host.native;
        let requests = match v {
            V::Text(s, _) => vec![s.clone()],
            V::List(v) => v.iter().map(V::as_str).collect(),
            _ => vec![],
        };
        for s in requests {
            if font_path(&s) {
                let path = resolve(file, s.split('#').next().unwrap_or(&s));
                if !self.paths.contains_key(&path) {
                    if let Ok(data) = host.read(&path, file, loc) {
                        self.register_font(&path, data);
                    }
                }
            }
            self.load_path(&s, file, loc, warnings);
        }
    }
    fn load_path(&mut self, name: &str, file: &str, loc: Loc, w: &mut Vec<Diagnostic>) {
        if !font_path(name) {
            return;
        }
        let part = name.split('#').next().unwrap_or(name);
        let path = resolve(file, part);
        if self.paths.contains_key(&path) {
            return;
        }
        match self.host.read(&path, file, loc) {
            Ok(data) => self.register_font(&path, data),
            Err(_) => {
                self.paths.insert(path, vec![]);
                warn(w, format!("字体 {name} 不可用"), file, loc);
            }
        }
    }
    fn key(&mut self, id: ID) -> Option<String> {
        if let Some(key) = self.keys.get(&id) {
            return Some(key.clone());
        }
        let face = self.db.face(id)?.clone();
        let data = self
            .db
            .with_face_data(id, |data, index| extract_face(data, index))??;
        let key = format!(
            "LayMesh_{:016x}",
            data.iter()
                .fold(14695981039346656037u64, |h, b| (h ^ u64::from(*b))
                    .wrapping_mul(1099511628211))
        );
        #[allow(unused_mut)]
        let mut origin = self
            .paths
            .iter()
            .find(|(_, ids)| ids.contains(&id))
            .map(|(path, _)| path.clone())
            .unwrap_or_default();
        #[cfg(feature = "system-fonts")]
        if let fontdb::Source::File(path) = &face.source {
            origin = path.to_string_lossy().into_owned();
        }
        self.origins.insert(key.clone(), origin);
        self.face_names
            .insert(key.clone(), face.post_script_name.clone());
        self.assets.insert(
            key.clone(),
            FontAsset {
                data,
                index: 0,
                family: face
                    .families
                    .first()
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
            },
        );
        self.keys.insert(id, key.clone());
        Some(key)
    }
    fn choose(
        &mut self,
        request: &Json,
        weight: u16,
        italic: bool,
        content: &str,
        file: &str,
        loc: Loc,
        w: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        let choice_key = format!(
            "{request}:{weight}:{italic}:{content}:{file}:{}:{}",
            loc.line, loc.column
        );
        if let Some(choice) = self.choices.get(&choice_key) {
            return choice.clone();
        }
        let explicit_list = request.is_array();
        let requests: Vec<String> = if let Some(a) = request.as_array() {
            a.iter()
                .filter_map(Json::as_str)
                .map(str::to_owned)
                .collect()
        } else if let Some(s) = request.as_str() {
            vec![s.into()]
        } else {
            vec![]
        };
        let mut candidates = Vec::new();
        for name in requests {
            self.load_path(&name, file, loc, w);
            let mut ids: Vec<ID> = if font_path(&name) {
                let mut p = name.splitn(2, '#');
                let path = resolve(file, p.next().unwrap());
                let face = p.next();
                self.paths
                    .get(&path)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|id| {
                        face.is_none_or(|name| {
                            self.db.face(*id).is_some_and(|f| {
                                canonical(&f.post_script_name) == canonical(name)
                                    || f.families
                                        .iter()
                                        .any(|(n, _)| canonical(n) == canonical(name))
                            })
                        })
                    })
                    .collect()
            } else if matches!(name.as_str(), "sans-serif" | "serif" | "monospace") {
                let family = match name.as_str() {
                    "serif" => fontdb::Family::Serif,
                    "monospace" => fontdb::Family::Monospace,
                    _ => fontdb::Family::SansSerif,
                };
                self.db
                    .query(&fontdb::Query {
                        families: &[family],
                        weight: fontdb::Weight(weight),
                        style: if italic {
                            fontdb::Style::Italic
                        } else {
                            fontdb::Style::Normal
                        },
                        ..Default::default()
                    })
                    .into_iter()
                    .collect()
            } else {
                self.db
                    .faces()
                    .filter(|f| {
                        f.families
                            .iter()
                            .any(|(s, _)| canonical(s) == canonical(&name))
                            || canonical(&f.post_script_name) == canonical(&name)
                    })
                    .map(|f| f.id)
                    .collect()
            };
            if ids.is_empty() {
                warn(w, format!("字体 {name} 不可用"), file, loc);
            }
            ids.sort_by_key(|id| {
                let f = self.db.face(*id).unwrap();
                u32::from(f.weight.0.abs_diff(weight))
                    + 10000
                        * u32::from(
                            f.stretch
                                .to_number()
                                .abs_diff(fontdb::Stretch::Normal.to_number()),
                        )
                    + if (f.style != fontdb::Style::Normal) != italic {
                        2000
                    } else {
                        0
                    }
            });
            candidates.extend(ids);
        }
        if !explicit_list {
            let desired = self.db.query(&fontdb::Query {
                families: &[fontdb::Family::SansSerif],
                weight: fontdb::Weight(weight),
                style: if italic {
                    fontdb::Style::Italic
                } else {
                    fontdb::Style::Normal
                },
                ..Default::default()
            });
            candidates.extend(desired);
            let mut all: Vec<_> = self.db.faces().map(|f| f.id).collect();
            all.sort_by_key(|id| {
                let f = self.db.face(*id).unwrap();
                u32::from(f.weight.0.abs_diff(weight))
                    + 10000
                        * u32::from(
                            f.stretch
                                .to_number()
                                .abs_diff(fontdb::Stretch::Normal.to_number()),
                        )
                    + if (f.style != fontdb::Style::Normal) != italic {
                        2000
                    } else {
                        0
                    }
            });
            candidates.extend(all);
        }
        for id in candidates {
            let (restriction, supports) = self
                .db
                .with_face_data(id, |data, index| {
                    ttf_parser::Face::parse(data, index)
                        .ok()
                        .map_or((None, false), |f| {
                            (
                                embedding_restriction(&f),
                                content.chars().all(|c| {
                                    c.is_whitespace()
                                        || c == '\u{200d}'
                                        || ('\u{fe00}'..='\u{fe0f}').contains(&c)
                                        || f.glyph_index(c).is_some()
                                }),
                            )
                        })
                })
                .unwrap_or((None, false));
            if let Some(reason) = restriction {
                let face = self.db.face(id).unwrap();
                warn(
                    w,
                    format!("字体 {} 不可用：{reason}", face.post_script_name),
                    file,
                    loc,
                );
                continue;
            }
            if supports {
                let f = self.db.face(id).unwrap();
                if weight != f.weight.0 || italic != (f.style != fontdb::Style::Normal) {
                    warn(
                        w,
                        format!(
                            "字体 {} 没有匹配的字重或斜体，使用字重 {}",
                            f.families.first().map(|x| x.0.as_str()).unwrap_or(""),
                            f.weight.0
                        ),
                        file,
                        loc,
                    );
                }
                let key = self.key(id);
                self.choices.insert(choice_key, key.clone());
                return key;
            }
        }
        self.choices.insert(choice_key, None);
        None
    }
}
/// Extract exactly one collection face. Avoid shipping a whole CJK collection in a generated SVG.
fn extract_face(data: &[u8], index: u32) -> Option<Vec<u8>> {
    if data.get(..4) != Some(b"ttcf") {
        return ttf_parser::Face::parse(data, index)
            .ok()
            .map(|_| data.to_vec());
    }
    let u16_at = |p: usize| data.get(p..p + 2).map(|x| u16::from_be_bytes([x[0], x[1]]));
    let u32_at = |p: usize| {
        data.get(p..p + 4)
            .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    };
    let start = u32_at(12 + index as usize * 4)? as usize;
    let count = u16_at(start + 4)? as usize;
    let mut out = data.get(start..start + 12 + count * 16)?.to_vec();
    let mut head = None;
    for i in 0..count {
        let src = start + 12 + i * 16;
        let offset = u32_at(src + 8)? as usize;
        let len = u32_at(src + 12)? as usize;
        let dst = out.len();
        out.extend_from_slice(data.get(offset..offset + len)?);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out[12 + i * 16 + 8..12 + i * 16 + 12].copy_from_slice(&(dst as u32).to_be_bytes());
        if data.get(src..src + 4) == Some(b"head") {
            head = Some(dst);
        }
    }
    if let Some(head) = head {
        out.get_mut(head + 8..head + 12)?.fill(0);
        let checksum = out.chunks_exact(4).fold(0u32, |sum, s| {
            sum.wrapping_add(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        });
        out[head + 8..head + 12]
            .copy_from_slice(&0xB1B0AFBAu32.wrapping_sub(checksum).to_be_bytes());
    }
    Some(out)
}
#[derive(Default)]
struct Outline {
    d: String,
    scale: f64,
    x: f64,
    y: f64,
}
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.d += &format!(
            "M{} {}",
            self.x + x as f64 * self.scale,
            self.y - y as f64 * self.scale
        );
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.d += &format!(
            "L{} {}",
            self.x + x as f64 * self.scale,
            self.y - y as f64 * self.scale
        );
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.d += &format!(
            "Q{} {} {} {}",
            self.x + x1 as f64 * self.scale,
            self.y - y1 as f64 * self.scale,
            self.x + x as f64 * self.scale,
            self.y - y as f64 * self.scale
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.d += &format!(
            "C{} {} {} {} {} {}",
            self.x + x1 as f64 * self.scale,
            self.y - y1 as f64 * self.scale,
            self.x + x2 as f64 * self.scale,
            self.y - y2 as f64 * self.scale,
            self.x + x as f64 * self.scale,
            self.y - y as f64 * self.scale
        );
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}
fn css(c: ratex_types::Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c.r * 255.).round() as u8,
        (c.g * 255.).round() as u8,
        (c.b * 255.).round() as u8
    )
}
fn missing_box(x: f64, y: f64, size: f64, color: &str) -> Json {
    json!({"kind":"box","content":"□","x":x,"y":y,"width":size*0.65,"height":size*0.8,"strokeWidth":size*0.045,"color":color})
}
pub fn formula(
    spec: &Json,
    fonts: &mut FontSystem,
    w: &mut Vec<Diagnostic>,
    file: &str,
    loc: Loc,
) -> Result<Json> {
    use ratex_types::{DisplayItem as D, PathCommand as P};
    let metadata = spec
        .get("content")
        .or_else(|| spec.get("source"))
        .unwrap_or(&Json::Null);
    let file = jstr(metadata, "file", jstr(spec, "file", file));
    let loc = metadata
        .get("loc")
        .or_else(|| spec.get("loc"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(loc);
    let source = value_text(
        spec.get("content")
            .or_else(|| spec.get("source"))
            .unwrap_or(&Json::Null),
    );
    let size = jnum(spec, "font_size", 10. * PT);
    let style = jstr(spec, "style", "inline");
    if !["inline", "display"].contains(&style) {
        return Err(Diagnostic::new(
            "E_FORMULA",
            "公式 style 须为 inline 或 display",
            file,
            loc,
        ));
    }
    if !size.is_finite() || size <= 0. {
        return Err(Diagnostic::new("E_FORMULA", "公式字号须为正数", file, loc));
    }
    let font = jstr(spec, "math_font", "ratex-katex");
    if font != "ratex-katex" {
        warn(w, format!("公式字体 {font} 映射到 ratex-katex"), file, loc);
    }
    if source.trim().is_empty() || source.chars().count() > 4000 {
        return Err(Diagnostic::new(
            "E_FORMULA",
            "公式不能为空且最多 4000 个字符",
            file,
            loc,
        ));
    }
    let disallowed=regex::Regex::new(r"\\(?:require|href|class|style|html|url|include|input|def|newcommand|renewcommand|let|write|special)\b").unwrap();
    if disallowed.is_match(&source) {
        return Err(Diagnostic::new(
            "E_FORMULA",
            "公式包含不允许的命令",
            file,
            loc,
        ));
    }
    let parsed = ratex_parser::parse(&source)
        .map_err(|e| Diagnostic::new("E_FORMULA", format!("RaTeX 公式解析失败：{e}"), file, loc))?;
    let options = ratex_layout::LayoutOptions {
        style: if spec
            .get("display")
            .and_then(Json::as_bool)
            .unwrap_or(jstr(spec, "style", "inline") == "display")
        {
            ratex_types::MathStyle::Display
        } else {
            ratex_types::MathStyle::Text
        },
        color: ratex_types::Color::from_hex(jstr(spec, "color", "#000000"))
            .unwrap_or(ratex_types::Color::BLACK),
        ..Default::default()
    };
    let dl = ratex_layout::to_display_list(&ratex_layout::layout(&parsed, &options));
    let mut items = vec![];
    let mut missing = BTreeSet::new();
    for item in dl.items {
        match item{
        D::GlyphPath{x,y,scale,font,char_code,color}=>{let name=format!("KaTeX_{font}.ttf");let char_=char::from_u32(char_code).unwrap_or('\u{fffd}');let data=ratex_katex_fonts::ttf_bytes(&name);let mut rendered=false;if let Some(data)=data{if let Ok(face)=ttf_parser::Face::parse(&data,0){if let Some(gid)=face.glyph_index(char_){let mut outline=Outline{scale:size*scale/face.units_per_em()as f64,x:x*size,y:y*size,..Default::default()};face.outline_glyph(gid,&mut outline);items.push(json!({"kind":"path","d":outline.d,"fill":css(color),"opacity":color.a}));rendered=true;}}}
            if !rendered{let ch=char_.to_string();if let Some(key)=fonts.choose(&spec["font_family"],400,false,&ch,file,loc,w){let asset=&fonts.assets[&key];let (advance,_,_)=measure(asset,&ch,size*scale);items.push(json!({"kind":"glyph","x":x*size,"baseline":y*size,"fontFamily":key,"fontSystemFamily":asset.family,"fontSize":size*scale,"content":ch,"width":advance,"color":css(color)}));}else{missing.insert(format!("U+{char_code:04X}"));items.push(missing_box(x*size,(y-0.8*scale)*size,size*scale,&css(color)));}}
        },D::Line{x,y,width,thickness,color,dashed}=>items.push(json!({"kind":"rule","x":x*size,"y":(if dashed { y } else { y-thickness/2. })*size,"width":width*size,"height":thickness*size,"color":css(color),"opacity":color.a,"dashed":dashed})),D::Rect{x,y,width,height,color}=>items.push(json!({"kind":"rule","x":x*size,"y":y*size,"width":width*size,"height":height*size,"color":css(color),"opacity":color.a})),D::Path{x,y,commands,fill,color}=>{let mut d=String::new();for p in commands{d+=&match p{P::MoveTo{x:a,y:b}=>format!("M{} {}",(x+a)*size,(y+b)*size),P::LineTo{x:a,y:b}=>format!("L{} {}",(x+a)*size,(y+b)*size),P::CubicTo{x1,y1,x2,y2,x:a,y:b}=>format!("C{} {} {} {} {} {}",(x+x1)*size,(y+y1)*size,(x+x2)*size,(y+y2)*size,(x+a)*size,(y+b)*size),P::QuadTo{x1,y1,x:a,y:b}=>format!("Q{} {} {} {}",(x+x1)*size,(y+y1)*size,(x+a)*size,(y+b)*size),P::Close=>"Z".into()};}items.push(json!({"kind":"path","d":d,"fill":if fill{css(color)}else{"none".into()},"stroke":if fill{"none".into()}else{css(color)},"strokeWidth":size*0.04,"opacity":color.a}));}}
    }
    if !missing.is_empty() {
        warn(
            w,
            format!(
                "公式缺少字形 {}，使用矢量方框替代",
                missing.into_iter().collect::<Vec<_>>().join(" ")
            ),
            file,
            loc,
        );
    }
    let mut node = base("formula", dl.width * size, (dl.height + dl.depth) * size);
    node["source"] = json!(source);
    node["mathFont"] = json!("ratex-katex");
    node["ascent"] = json!(dl.height * size);
    node["items"] = json!(items);
    Ok(node)
}
fn measure(font: &FontAsset, text: &str, size: f64) -> (f64, f64, f64) {
    let Some(face) = rustybuzz::Face::from_slice(&font.data, font.index) else {
        return (size, 0.8 * size, 0.2 * size);
    };
    let scale = size / face.units_per_em() as f64;
    let bidi = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let mut width = 0.;
    for paragraph in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        for run in runs {
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&text[run.clone()]);
            buffer.guess_segment_properties();
            buffer.set_direction(if levels[run.start].is_rtl() {
                rustybuzz::Direction::RightToLeft
            } else {
                rustybuzz::Direction::LeftToRight
            });
            let shaped = rustybuzz::shape(&face, &[], buffer);
            width += shaped
                .glyph_positions()
                .iter()
                .map(|p| p.x_advance as f64 * scale)
                .sum::<f64>()
                .abs();
        }
    }
    (
        width,
        face.ascender() as f64 * scale,
        -face.descender() as f64 * scale,
    )
}
#[derive(Clone)]
struct Unit {
    content: String,
    end: usize,
    key: Option<String>,
    size: f64,
    color: String,
    weight: u16,
    italic: bool,
    width: f64,
    ascent: f64,
    descent: f64,
    formula: Option<Json>,
}
fn text_parts(spec: &Json) -> Vec<Json> {
    if let Some(spans) = spec["spans"].as_array() {
        spans
            .iter()
            .map(|s| {
                let mut v = spec.clone();
                v.as_object_mut().unwrap().remove("spans");
                if let Some(s) = s.as_object() {
                    for (k, val) in s {
                        v[k] = val.clone();
                    }
                } else {
                    v["content"] = s.clone();
                }
                v
            })
            .collect()
    } else {
        vec![spec.clone()]
    }
}

fn shifted(loc: Loc, text: &str, offset: usize) -> Loc {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let lines = before.bytes().filter(|b| *b == b'\n').count();
    Loc {
        line: loc.line + lines,
        column: if lines == 0 {
            loc.column + before.chars().count()
        } else {
            before.rsplit('\n').next().unwrap_or("").chars().count() + 1
        },
        offset: loc.offset + offset,
    }
}
fn expanded_parts(spec: &Json, file: &str, loc: Loc) -> Result<Vec<Json>> {
    let mut out = vec![];
    for mut part in text_parts(spec) {
        // Font selection belongs to the constructor; glyph/LaTeX diagnostics
        // belong to the literal. Keep both origins when expanding inline math.
        part["__font_file"] = json!(jstr(&part, "file", file));
        part["__font_loc"] = part.get("loc").cloned().unwrap_or_else(|| json!(loc));
        let content_meta = &part["content"];
        let part_file = jstr(content_meta, "file", jstr(&part, "file", file)).to_owned();
        let part_loc = content_meta
            .get("loc")
            .or_else(|| part.get("loc"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(loc);
        part["file"] = json!(part_file);
        part["loc"] = json!(part_loc);
        if part["kind"] == "formula" || part["content"]["raw"].as_bool().unwrap_or(false) {
            out.push(part);
            continue;
        }
        let content = value_text(&part["content"]);
        let mut text = String::new();
        let mut cursor = 0;
        while cursor < content.len() {
            let tail = &content[cursor..];
            if tail.starts_with("\\$") {
                text.push('$');
                cursor += 2;
                continue;
            }
            if !tail.starts_with('$') {
                let c = tail.chars().next().unwrap();
                text.push(c);
                cursor += c.len_utf8();
                continue;
            }
            if !text.is_empty() {
                let mut p = part.clone();
                p["content"] = json!(std::mem::take(&mut text));
                out.push(p);
            }
            let display = tail.starts_with("$$");
            let len = if display { 2 } else { 1 };
            let start = cursor + len;
            cursor = start;
            while cursor < content.len()
                && !content[cursor..].starts_with(if display { "$$" } else { "$" })
            {
                let c = content[cursor..].chars().next().unwrap();
                cursor += c.len_utf8();
                if c == '\\' && cursor < content.len() {
                    cursor += content[cursor..].chars().next().unwrap().len_utf8();
                }
            }
            if cursor == content.len() {
                return Err(Diagnostic::new(
                    "E_FORMULA",
                    "公式缺少结束定界符；字面文本使用原样字符串或 \\$",
                    &part_file,
                    shifted(part_loc, &content, start - len),
                ));
            }
            if display && !out.is_empty() {
                let mut p = part.clone();
                p["content"] = json!("\n");
                out.push(p);
            }
            let mut p = part.clone();
            p["kind"] = json!("formula");
            p["content"] = json!(&content[start..cursor]);
            p["display"] = json!(display);
            p["loc"] = json!(shifted(part_loc, &content, start));
            out.push(p);
            cursor += len;
            if display {
                let mut p = part.clone();
                p["content"] = json!("\n");
                out.push(p);
            }
        }
        if !text.is_empty() || out.is_empty() {
            let mut p = part;
            p["content"] = json!(text);
            out.push(p);
        }
    }
    Ok(out)
}
pub fn layout_text(
    spec: &Json,
    frame: Option<f64>,
    fonts: &mut FontSystem,
    w: &mut Vec<Diagnostic>,
    file: &str,
    loc: Loc,
) -> Result<Json> {
    if jstr(spec, "kind", "") == "formula" {
        return formula(spec, fonts, w, file, loc);
    }
    let size = jnum(spec, "font_size", 10. * PT);
    let line_height = jnum(spec, "line_height", size * 1.2);
    if !size.is_finite()
        || !line_height.is_finite()
        || size <= 0.
        || line_height <= 0.
        || frame.is_some_and(|v| !v.is_finite() || v <= 0.)
    {
        return Err(Diagnostic::new("E_LAYOUT", "文字尺寸必须为正", file, loc));
    }
    let mut units = vec![];
    let mut all = String::new();
    let mut missing: BTreeMap<(String, usize, usize, usize), (BTreeSet<String>, Vec<String>)> =
        BTreeMap::new();
    for part in expanded_parts(spec, file, loc)? {
        let mut part = part;
        let part_file = jstr(&part, "file", file).to_owned();
        let part_loc = part
            .get("loc")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(loc);
        let font_file = jstr(&part, "__font_file", &part_file);
        let font_loc = part
            .get("__font_loc")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(part_loc);
        let psize = jnum(&part, "font_size", size);
        let color = jstr(&part, "color", "#000000").to_owned();
        let weight = jnum(&part, "font_weight", 400.) as u16;
        let italic = jstr(&part, "font_style", "normal") == "italic"
            || part["italic"].as_bool().unwrap_or(false)
            || part["font_style"].as_bool().unwrap_or(false);
        let request = &part["font_family"];
        if !(100..=900).contains(&weight) || jnum(&part, "font_weight", 400.).fract() != 0. {
            return Err(Diagnostic::new(
                "E_FONT",
                "字重须为 100 到 900 的整数",
                &part_file,
                part_loc,
            ));
        }
        if !psize.is_finite() || psize <= 0. {
            return Err(Diagnostic::new(
                "E_LAYOUT",
                "文字尺寸必须为正",
                &part_file,
                part_loc,
            ));
        }
        if !request.is_null()
            && !(request.as_str().is_some_and(|s| !s.trim().is_empty())
                || request.as_array().is_some_and(|a| {
                    !a.is_empty()
                        && a.iter()
                            .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                }))
        {
            return Err(Diagnostic::new(
                "E_FONT",
                "font_family 需要非空字体名称、路径或字符串列表",
                &part_file,
                part_loc,
            ));
        }
        if jstr(&part, "kind", "") == "formula" {
            part["font_size"] = json!(psize);
            let f = formula(&part, fonts, w, &part_file, part_loc)?;
            all.push('\u{fffc}');
            units.push(Unit {
                content: "\u{fffc}".into(),
                end: all.len(),
                key: None,
                size: psize,
                color,
                weight,
                italic,
                width: jnum(&f, "width", 0.),
                ascent: jnum(&f, "ascent", 0.),
                descent: jnum(&f, "height", 0.) - jnum(&f, "ascent", 0.),
                formula: Some(f),
            });
            continue;
        }
        let content = value_text(&part["content"])
            .replace("\r\n", "\n")
            .replace('\t', "    ");
        for grapheme in content.graphemes(true) {
            all += grapheme;
            let key = if grapheme == "\n" {
                None
            } else {
                fonts.choose(request, weight, italic, grapheme, font_file, font_loc, w)
            };
            let (width, ascent, descent) = if let Some(key) = &key {
                measure(&fonts.assets[key], grapheme, psize)
            } else if grapheme == "\n" {
                (0., psize * 0.8, psize * 0.2)
            } else if grapheme.chars().all(char::is_whitespace) {
                (psize * 0.33, psize * 0.8, psize * 0.2)
            } else {
                let absent = missing
                    .entry((
                        part_file.clone(),
                        part_loc.line,
                        part_loc.column,
                        part_loc.offset,
                    ))
                    .or_default();
                if absent.0.insert(grapheme.to_owned()) {
                    absent.1.push(grapheme.to_owned());
                }
                (psize * 0.7, psize * 0.8, psize * 0.2)
            };
            units.push(Unit {
                content: grapheme.into(),
                end: all.len(),
                key,
                size: psize,
                color: color.clone(),
                weight,
                italic,
                width,
                ascent,
                descent,
                formula: None,
            });
        }
    }
    for ((file, line, column, offset), (_, values)) in missing {
        warn(
            w,
            format!(
                "字体列表缺少字形 {}，使用矢量方框替代",
                values
                    .iter()
                    .map(|s| format!(
                        "{s:?} ({})",
                        s.chars()
                            .map(|c| format!("U+{:04X}", c as u32))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ))
                    .collect::<Vec<_>>()
                    .join("、")
            ),
            &file,
            Loc {
                line,
                column,
                offset,
            },
        );
    }
    let breaks: BTreeSet<_> = unicode_linebreak::linebreaks(&all)
        .map(|(p, _)| p)
        .collect();
    let mut lines: Vec<(Vec<Unit>, bool)> = vec![];
    let mut cursor = 0;
    while cursor < units.len() {
        let start = cursor;
        let mut width = 0.;
        let mut last_break = None;
        while cursor < units.len() && units[cursor].content != "\n" {
            if frame.is_some_and(|w| width + units[cursor].width > w + 1e-6) {
                if cursor == start {
                    return Err(Diagnostic::new(
                        "E_LAYOUT",
                        "文字或行内公式宽于排版宽度",
                        file,
                        loc,
                    ));
                }
                if let Some(last) = last_break {
                    cursor = last;
                }
                break;
            }
            width += units[cursor].width;
            cursor += 1;
            if breaks.contains(&units[cursor - 1].end) {
                last_break = Some(cursor);
            }
        }
        let mut line = units[start..cursor].to_vec();
        while line
            .last()
            .is_some_and(|v| v.content.chars().all(char::is_whitespace))
        {
            line.pop();
        }
        let hard = cursor < units.len() && units[cursor].content == "\n";
        lines.push((line, hard));
        if hard {
            cursor += 1;
        } else {
            while cursor < units.len()
                && units[cursor].content != "\n"
                && units[cursor].content.chars().all(char::is_whitespace)
            {
                cursor += 1;
            }
        }
    }
    if lines.is_empty() || all.ends_with('\n') {
        lines.push((vec![], false));
    }
    let count = lines.len();
    let align = jstr(spec, "align", "left");
    if !["left", "center", "right", "justify"].contains(&align) {
        return Err(Diagnostic::new(
            "E_LAYOUT",
            "align 须为 left、center、right 或 justify",
            file,
            loc,
        ));
    }
    let mut runs = vec![];
    let mut y = 0.;
    let mut widest = 0f64;
    let mut first_ascent = line_height * 0.8;
    for (line_index, (line, hard)) in lines.into_iter().enumerate() {
        let ascent = line.iter().fold(line_height * 0.8, |a, u| a.max(u.ascent));
        let descent = line.iter().fold(line_height * 0.2, |a, u| a.max(u.descent));
        if line_index == 0 {
            first_ascent = ascent;
        }
        let baseline = y + ascent;
        let line_text = line.iter().map(|u| u.content.as_str()).collect::<String>();
        let bidi = unicode_bidi::BidiInfo::new(&line_text, Some(unicode_bidi::Level::ltr()));
        let mut group_levels: Vec<unicode_bidi::Level> = vec![];
        let mut byte = 0;
        let mut groups: Vec<Vec<Unit>> = vec![];
        for u in line {
            let level = bidi
                .levels
                .get(byte)
                .copied()
                .unwrap_or(unicode_bidi::Level::ltr());
            byte += u.content.len();
            if let Some(group) = groups.last_mut() {
                let p = &group[0];
                if u.formula.is_none()
                    && p.formula.is_none()
                    && u.key.is_some()
                    && u.key == p.key
                    && u.size == p.size
                    && u.color == p.color
                    && u.weight == p.weight
                    && u.italic == p.italic
                    && group_levels.last() == Some(&level)
                    && !(align == "justify" && (u.content == " " || p.content == " "))
                {
                    group.push(u);
                    continue;
                }
            }
            groups.push(vec![u]);
            group_levels.push(level);
        }
        let order = unicode_bidi::BidiInfo::reorder_visual(&group_levels);
        let groups: Vec<Vec<Unit>> = order
            .into_iter()
            .map(|i| std::mem::take(&mut groups[i]))
            .collect();
        let widths: Vec<f64> = groups
            .iter()
            .map(|g| {
                if let Some(key) = &g[0].key {
                    measure(
                        &fonts.assets[key],
                        &g.iter().map(|u| u.content.as_str()).collect::<String>(),
                        g[0].size,
                    )
                    .0
                } else {
                    g[0].width
                }
            })
            .collect();
        let natural = widths.iter().sum::<f64>();
        if frame.is_some_and(|width| natural > width + 0.05) {
            return Err(Diagnostic::new(
                "E_LAYOUT",
                "排版后文字超出指定宽度",
                file,
                loc,
            ));
        }
        widest = widest.max(natural);
        let mut x = match (frame, align) {
            (Some(w), "right") => w - natural,
            (Some(w), "center") => (w - natural) / 2.,
            _ => 0.,
        };
        let spaces = if align == "justify" && !hard && line_index + 1 < count {
            groups
                .iter()
                .filter(|g| g.len() == 1 && g[0].content == " ")
                .count()
        } else {
            0
        };
        let extra = if spaces > 0 {
            (frame.unwrap_or(natural) - natural) / spaces as f64
        } else {
            0.
        };
        for (group, width) in groups.into_iter().zip(widths) {
            let u = &group[0];
            if let Some(f) = &u.formula {
                let mut f = f.clone();
                f["x"] = json!(x);
                f["y"] = json!(baseline - u.ascent);
                runs.push(f);
            } else if let Some(key) = &u.key {
                let font = &fonts.assets[key];
                let face = ttf_parser::Face::parse(&font.data, font.index).ok();
                let weight = face
                    .as_ref()
                    .map(|f| f.weight().to_number())
                    .unwrap_or(u.weight);
                let italic = face.as_ref().map(|f| f.is_italic()).unwrap_or(u.italic);
                runs.push(json!({"kind":"glyph","content":group.iter().map(|u|u.content.as_str()).collect::<String>(),"x":x,"baseline":baseline,"fontFamily":key,"fontSystemFamily":font.family,"fontPath":fonts.origins.get(key),"fontFace":fonts.face_names.get(key),"fontSize":u.size,"fontWeight":weight,"fontItalic":italic,"color":u.color,"width":width}));
            } else if !u.content.chars().all(char::is_whitespace) {
                runs.push(missing_box(x, baseline - u.ascent, u.size, &u.color));
            }
            x += width
                + if group.len() == 1 && u.content == " " {
                    extra
                } else {
                    0.
                };
        }
        y += (ascent + descent).max(line_height);
    }
    let mut node = base("text", frame.unwrap_or(widest), y);
    node["runs"] = json!(runs);
    node["ascent"] = json!(first_ascent);
    node["lineHeight"] = json!(line_height);
    node["content"] = json!(all);
    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_fonts_produces_vector_boxes() {
        let mut fonts = FontSystem::new(false);
        let mut warnings = vec![];
        let n = layout_text(
            &json!({"content":"A中\u{10ffff}","font_size":4.}),
            None,
            &mut fonts,
            &mut warnings,
            "test.lay",
            Loc {
                line: 2,
                column: 3,
                offset: 0,
            },
        )
        .unwrap();
        assert!(fonts.assets.is_empty());
        assert_eq!(n["runs"].as_array().unwrap().len(), 3);
        assert!(
            n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["kind"] == "box")
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("U+10FFFF"));
        assert_eq!(warnings[0].loc.line, 2);
    }
    #[test]
    fn formula_corpus_has_outlines_without_body_fonts() {
        let corpus: Vec<String> =
            serde_json::from_str(include_str!("../../../migration/formulas.json")).unwrap();
        for source in corpus {
            let mut fonts = FontSystem::new(false);
            let mut w = vec![];
            let n = formula(
                &json!({"content":source,"font_size":4.}),
                &mut fonts,
                &mut w,
                "formula.lay",
                Loc::default(),
            )
            .unwrap();
            assert!(n["width"].as_f64().unwrap() > 0.);
            assert!(!n["items"].as_array().unwrap().is_empty());
            assert!(w.is_empty(), "{source}: {w:?}");
            assert!(fonts.assets.is_empty());
        }
    }
    #[test]
    fn font_collection_face_can_be_extracted() {
        let data = include_bytes!("../../../tests/assets/LayMeshTest.ttc");
        let a = extract_face(data, 0).unwrap();
        let b = extract_face(data, 1).unwrap();
        assert!(ttf_parser::Face::parse(&a, 0).is_ok());
        assert!(ttf_parser::Face::parse(&b, 0).is_ok());
        assert_ne!(a, b);
    }
    #[test]
    fn explicit_list_never_uses_unlisted_fonts() {
        let mut fonts = FontSystem::new(false);
        fonts.register_font(
            "test",
            include_bytes!("../../../tests/assets/GFSNeohellenic.otf").to_vec(),
        );
        let mut warnings = vec![];
        let n = layout_text(
            &json!({"content":"Hello","font_family":["Unavailable"]}),
            None,
            &mut fonts,
            &mut warnings,
            "x.lay",
            Loc::default(),
        )
        .unwrap();
        assert!(fonts.assets.is_empty());
        assert!(
            n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["kind"] == "box")
        );
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn inline_math_raw_text_and_delimiters() {
        let mut f = FontSystem::new(false);
        let mut w = vec![];
        let node = layout_text(
            &json!({"content":"before $E=mc^2$ after","font_size":4.}),
            None,
            &mut f,
            &mut w,
            "test.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(
            node["runs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["kind"] == "formula")
                .count(),
            1
        );
        let raw = layout_text(
            &json!({"content":{"value":"$x$","raw":true},"font_size":4.}),
            None,
            &mut f,
            &mut w,
            "test.lay",
            Loc::default(),
        )
        .unwrap();
        assert!(
            raw["runs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["kind"] != "formula")
        );
        assert_eq!(
            layout_text(
                &json!({"content":"$unfinished"}),
                None,
                &mut f,
                &mut w,
                "test.lay",
                Loc::default()
            )
            .unwrap_err()
            .code,
            "E_FORMULA"
        );
    }
    #[test]
    fn legacy_math_font_warns_and_maps() {
        let mut f = FontSystem::new(false);
        let mut w = vec![];
        let n = formula(
            &json!({"source":"x","math_font":"mathjax-newcm"}),
            &mut f,
            &mut w,
            "f.lay",
            Loc {
                line: 2,
                column: 3,
                offset: 0,
            },
        )
        .unwrap();
        assert_eq!(n["mathFont"], "ratex-katex");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].code, "W_FONT");
        assert_eq!(w[0].loc.line, 2);
    }
    #[test]
    fn missing_formula_unicode_uses_vector_boxes() {
        let mut f = FontSystem::new(false);
        let mut w = vec![];
        let n = formula(
            &json!({"source":r"\text{中文}"}),
            &mut f,
            &mut w,
            "f.lay",
            Loc::default(),
        )
        .unwrap();
        assert!(
            n["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["kind"] == "box")
        );
        assert!(w.iter().any(|d| d.code == "W_FONT"));
        assert!(f.assets.is_empty());
    }
    #[test]
    fn collection_postscript_face_is_preserved() {
        for (ext, data) in [
            (
                "ttc",
                include_bytes!("../../../tests/assets/LayMeshTest.ttc").as_slice(),
            ),
            (
                "otc",
                include_bytes!("../../../tests/assets/LayMeshTest.otc").as_slice(),
            ),
        ] {
            let face = ttf_parser::Face::parse(data, 1).unwrap();
            let name = face
                .names()
                .into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
                .unwrap()
                .to_string()
                .unwrap();
            let mut host = Host::default();
            host.files
                .insert(format!("/test/font.{ext}"), data.to_vec());
            let mut f = FontSystem::new(false);
            let request = V::text(format!("./font.{ext}#{name}"));
            let mut w = vec![];
            f.load_requested(&request, &host, "/test/x.lay", Loc::default(), &mut w);
            let n = layout_text(
                &json!({"content":"A","font_family":[request.as_str()],"font_size":4.}),
                None,
                &mut f,
                &mut w,
                "/test/x.lay",
                Loc::default(),
            )
            .unwrap();
            assert_eq!(n["runs"][0]["kind"], "glyph");
            let key = n["runs"][0]["fontFamily"].as_str().unwrap();
            let chosen = ttf_parser::Face::parse(&f.assets[key].data, 0).unwrap();
            let chosen_name = chosen
                .names()
                .into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
                .unwrap()
                .to_string()
                .unwrap();
            assert_eq!(chosen_name, name);
            assert!(w.is_empty());
        }
    }
    #[test]
    fn wrapping_and_alignment_are_measured() {
        let mut f = FontSystem::new(false);
        f.register_font(
            "test",
            include_bytes!("../../../tests/assets/GFSNeohellenic.otf").to_vec(),
        );
        let mut w = vec![];
        let n = layout_text(
            &json!({"content":"alpha beta gamma delta","align":"right","font_size":4.}),
            Some(22.),
            &mut f,
            &mut w,
            "x",
            Loc::default(),
        )
        .unwrap();
        let runs = n["runs"].as_array().unwrap();
        assert!(runs.len() > 1);
        assert!(
            runs.iter()
                .all(|r| (jnum(r, "x", 0.) + jnum(r, "width", 0.) - 22.).abs() < 1e-6)
        );
        assert!(n["height"].as_f64().unwrap() > 4.8);
    }
}

#[cfg(test)]
mod matching_tests {
    use super::*;
    #[test]
    fn normal_stretch_wins_over_condensed_with_the_same_family_and_weight() {
        let bytes = include_bytes!("../../../tests/assets/GFSNeohellenic.otf").to_vec();
        let mut fs = FontSystem::new(false);
        fs.register_font("condensed.otf", bytes.clone());
        let id = fs.paths["condensed.otf"][0];
        let mut face = fs.db.face(id).unwrap().clone();
        let family = face.families[0].0.clone();
        face.stretch = fontdb::Stretch::Condensed;
        fs.db.remove_face(id);
        let id = fs.db.push_face_info(face);
        fs.paths.insert("condensed.otf".into(), vec![id]);
        fs.register_font("normal.otf", bytes);
        let mut warnings = vec![];
        let key = fs
            .choose(
                &json!([family]),
                400,
                false,
                "Abc",
                "x.lay",
                Loc::default(),
                &mut warnings,
            )
            .unwrap();
        assert_eq!(fs.origins[&key], "normal.otf");
    }
}
