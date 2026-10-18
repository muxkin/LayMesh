//! Document orchestration uses the ordinary Lay parser and evaluator. Page
//! handles are lazy: drawing a selected page never evaluates its neighbours.
use crate::{Diagnostic, Loc, Result, engine::Engine, model::*};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub const DOCUMENT_VERSION: u32 = 1;
pub const MAX_PAGES: usize = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub media_key: String,
    #[serde(skip)]
    pub background_media: std::rc::Rc<crate::media::Media>,
    pub id: String,
    pub name: String,
    pub src: String,
    pub file: String,
    pub loc: Loc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<TransitionSpec>,
}
/// A resolved, lazy document override. `None` inherits page CSS; name=none disables it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub args: BTreeMap<String, serde_json::Value>,
    pub file: String,
    pub loc: Loc,
}
impl TransitionSpec {
    pub(crate) fn compile(
        &self,
        host: Option<&Host>,
    ) -> Result<Option<crate::page_transition::PageTransition>> {
        let mut args: Args = self
            .args
            .iter()
            .map(|(k, v)| (k.clone(), V::from_json(v)))
            .collect();
        for (key, value) in &mut args {
            if key.starts_with("_laymesh_transition_texture_") {
                let path = value.as_str();
                let asset = if let Some(host) = host {
                    crate::assets::load(
                        &host.read(&path, &self.file, self.loc)?,
                        &path,
                        &self.file,
                        self.loc,
                    )?
                } else {
                    serde_json::json!({"data":"","mime":"image/png"})
                };
                *value = V::from_json(&asset);
            }
        }
        crate::page_transition::compile(&args, &self.file, self.loc)
    }
}
fn transition_spec(value: &V, file: &str, loc: Loc) -> Result<TransitionSpec> {
    let fail = |message: &str| Diagnostic::new("E_TRANSITION", message, file, loc);
    let (input, file, loc) = match value.value() {
        V::Text(name, _) => (
            Args::from([("name".into(), V::text(name))]),
            file.to_owned(),
            loc,
        ),
        V::Object(o) if o.borrow().kind == "transition" => {
            let o = o.borrow();
            ((*o.args).clone(), o.file.clone(), o.loc)
        }
        _ => return Err(fail("transition 需要效果名称或 transition(name=...)")),
    };
    let mut args = BTreeMap::new();
    for (key, value) in input {
        let target = match key.as_str() {
            "name" => "_laymesh_transition",
            "duration" => "_laymesh_transition_duration",
            "easing" => "_laymesh_transition_timing_function",
            "params" | "textures" => {
                let map = match value.value() {
                    V::Dict(m) => m.iter().collect::<Vec<_>>(),
                    V::Map(m) => m.iter().collect(),
                    _ => return Err(fail("params/textures 需要字典")),
                };
                for (name, v) in map {
                    let text = if key == "textures" {
                        let V::Text(path, _) = v.value() else {
                            return Err(fail("纹理路径需要字符串"));
                        };
                        if path.is_empty()
                            || path.contains(':')
                            || path.starts_with('/')
                            || path.contains('\\')
                        {
                            return Err(fail("纹理需要本地相对路径"));
                        }
                        resolve(&file, path)
                    } else {
                        match v.value() {
                            V::List(vs) => {
                                if vs
                                    .iter()
                                    .any(|v| !matches!(v.value(),V::Number(_,u) if u.is_empty()))
                                {
                                    return Err(fail("向量参数需要无单位数值"));
                                }
                                vs.iter().map(V::as_str).collect::<Vec<_>>().join(" ")
                            }
                            V::Number(_, unit) if !unit.is_empty() => {
                                return Err(fail("转场参数数值不接受单位"));
                            }
                            V::Number(..) | V::Bool(_) => v.as_str(),
                            _ => return Err(fail("转场参数需要数值、布尔或向量")),
                        }
                    };
                    args.insert(
                        format!(
                            "_laymesh_transition_{}_{}",
                            if key == "params" { "param" } else { "texture" },
                            name.replace('-', "_")
                        ),
                        serde_json::json!(text),
                    );
                }
                continue;
            }
            _ => return Err(fail("未知 transition 参数")),
        };
        let text = match value.value() {
            V::Number(n, u) if key == "duration" && matches!(u.as_str(), "ms" | "s") => {
                format!("{n}{u}")
            }
            V::Text(s, _) => s.clone(),
            _ => return Err(fail("name/easing 需要字符串，duration 需要 ms/s 时间")),
        };
        args.insert(target.into(), serde_json::json!(text));
    }
    if !args.contains_key("_laymesh_transition") {
        return Err(fail("transition 需要 name"));
    }
    let spec = TransitionSpec { args, file, loc };
    spec.compile(None)?;
    Ok(spec)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPlan {
    #[serde(default, skip_serializing_if = "crate::media::Media::is_empty")]
    pub media: crate::media::Media,
    pub schema_version: u32,
    pub name: String,
    pub pages: Vec<PageSpec>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentPage {
    #[serde(flatten)]
    pub page: PageSpec,
    pub scene: Arc<Scene>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub schema_version: u32,
    pub name: String,
    pub pages: Vec<DocumentPage>,
}
impl DocumentPlan {
    /// Public page numbers are one-based, including CLI/Python/preview.
    pub fn page(&self, number: usize) -> Result<&PageSpec> {
        number
            .checked_sub(1)
            .and_then(|i| self.pages.get(i))
            .ok_or_else(|| {
                Diagnostic::new(
                    "E_PAGE",
                    format!("页码须为 1–{}，收到 {number}", self.pages.len()),
                    "",
                    Loc::default(),
                )
            })
    }
    pub fn compile(&self, host: Host) -> Result<Document> {
        let mut cache = BTreeMap::<String, Arc<Scene>>::new();
        let mut pages = Vec::with_capacity(self.pages.len());
        for page in &self.pages {
            let key = format!(
                "{}|{}",
                page.src,
                serde_json::to_string(&page.transition).unwrap()
            );
            let scene = if let Some(scene) = cache.get(&key) {
                scene.clone()
            } else {
                let scene = Arc::new(compile_page(page, host.clone())?);
                cache.insert(key, scene.clone());
                scene
            };
            pages.push(DocumentPage {
                page: page.clone(),
                scene,
            });
        }
        Ok(Document {
            schema_version: DOCUMENT_VERSION,
            name: self.name.clone(),
            pages,
        })
    }
}
pub fn compile_page(page: &PageSpec, host: Host) -> Result<Scene> {
    let source = read_page(page, &host)?;
    let mut engine = Engine::new(host);
    engine.page_transition_override = page.transition.clone();
    engine.media = page.background_media.as_ref().clone();
    engine.compile(&source, &page.src)
}
pub fn compile_preview_page(page: &PageSpec, host: Host) -> Result<Scene> {
    crate::geometry_cache::interactive(|| compile_page(page, host))
}
fn read_page(page: &PageSpec, host: &Host) -> Result<String> {
    String::from_utf8(host.read(&page.src, &page.file, page.loc)?)
        .map_err(|_| Diagnostic::new("E_ASSET", "页面文件不是 UTF-8", &page.src, Loc::default()))
}
pub fn plan_source(source: &str, file: &str, host: Host) -> Result<DocumentPlan> {
    let mut engine = Engine::new(host);
    engine.document_mode = true;
    engine.run_entry(source, file)?;
    let doc = engine
        .document
        .as_ref()
        .ok_or_else(|| {
            engine.error(
                "E_DOCUMENT",
                ".laydoc 入口需要一个 document",
                Loc::default(),
            )
        })?
        .borrow();
    let pages = doc.args.get("__pages").map(V::list).unwrap_or_default();
    if pages.is_empty() {
        return Err(engine.error(
            "E_DOCUMENT",
            "文档至少需要一页；使用 doc.add(page(...))",
            doc.loc,
        ));
    }
    let pages = pages
        .iter()
        .map(|v| {
            let page = v.object().unwrap();
            let page = page.borrow();
            Ok(PageSpec {
                media_key: if engine.media.is_empty() {
                    String::new()
                } else {
                    crate::asset_cache::digest(
                        serde_json::to_string(&engine.media.cues)
                            .unwrap()
                            .as_bytes(),
                    )
                },
                background_media: std::rc::Rc::new(engine.media.clone()),
                id: string(&page.args, "id", ""),
                name: string(&page.args, "name", ""),
                src: string(&page.args, "__src", ""),
                file: page.file.clone(),
                loc: page.loc,
                transition: if let Some(v) = page.args.get("transition") {
                    Some(transition_spec(v, &page.file, page.loc)?)
                } else if let Some(v) = doc.args.get("transition") {
                    Some(transition_spec(v, &doc.file, doc.loc)?)
                } else {
                    None
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DocumentPlan {
        media: engine.media.clone(),
        schema_version: DOCUMENT_VERSION,
        name: string(&doc.args, "name", "Untitled"),
        pages,
    })
}
pub fn plan_file(file: &str) -> Result<DocumentPlan> {
    let path = std::fs::canonicalize(file)
        .map_err(|e| Diagnostic::new("E_ASSET", e.to_string(), file, Loc::default()))?;
    let source = std::fs::read_to_string(&path)
        .map_err(|e| Diagnostic::new("E_ASSET", e.to_string(), file, Loc::default()))?;
    plan_source(
        &source,
        &path.to_string_lossy(),
        Host {
            native: true,
            ..Host::default()
        },
    )
}
pub fn compile_document_file(file: &str) -> Result<Document> {
    plan_file(file)?.compile(Host {
        native: true,
        ..Host::default()
    })
}
pub fn compile_document_source(source: &str, file: &str, host: Host) -> Result<Document> {
    plan_source(source, file, host.clone())?.compile(host)
}

impl Engine {
    pub(crate) fn document_call(
        &mut self,
        parts: &[String],
        pos: &[V],
        args: &Args,
        scope: &Scope,
        loc: Loc,
    ) -> Option<Result<V>> {
        let builtin =
            parts.len() == 1 && matches!(parts[0].as_str(), "document" | "page" | "transition");
        let owner = (parts.len() == 2)
            .then(|| Environment::get(scope, &parts[0]).and_then(|v| v.object()))
            .flatten();
        let method = owner
            .as_ref()
            .is_some_and(|o| matches!(o.borrow().kind.as_str(), "document" | "page"));
        if !builtin && !method {
            return None;
        }
        Some((|| {
            if !self.document_mode {
                return Err(self.error("E_DOCUMENT", "document/page 仅用于 .laydoc 编排入口", loc));
            }
            let mut args: Args = args
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if builtin {
                let name = parts[0].as_str();
                let allowed: &[&str] = if name == "document" {
                    &["name", "transition"]
                } else if name == "transition" {
                    &["name", "duration", "easing", "params", "textures"]
                } else {
                    &["src", "id", "name", "transition"]
                };
                if !pos.is_empty() || args.keys().any(|k| !allowed.contains(&k.as_str())) {
                    return Err(self.error(
                        "E_ARG",
                        format!("{name} 只接受命名参数 {}", allowed.join(", ")),
                        loc,
                    ));
                }
                if name == "transition" {
                    let descriptor = self.object(name, args, loc);
                    transition_spec(&descriptor, &self.file, loc)?;
                    return Ok(descriptor);
                }
                if let Some(v) = args.get("transition") {
                    transition_spec(v, &self.file, loc)?;
                }
                if args
                    .iter()
                    .any(|(k, v)| k != "transition" && !matches!(v.value(), V::Text(..)))
                {
                    return Err(self.error("E_TYPE", "文档与页面参数需要字符串", loc));
                }
                if name == "document" {
                    if self.document.is_some() {
                        return Err(self.error("E_DOCUMENT", "编排入口只能创建一个 document", loc));
                    }
                    args.insert("__pages".into(), V::List(vec![]));
                    let doc = self.object(name, args, loc);
                    self.document = doc.object();
                    return Ok(doc);
                }
                let src = string(&args, "src", "");
                if src.is_empty()
                    || !src.ends_with(".lay")
                    // Page paths are portable slash-separated relative paths.
                    // On Windows `/a.lay` is rooted but is_absolute() is false.
                    || src.starts_with('/')
                    || src.contains(":")
                    || src.contains('\\')
                {
                    return Err(self.error("E_PAGE", "page.src 需要本地相对 .lay 文件路径", loc));
                }
                let default_id = Path::new(&src)
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let id = string(&args, "id", &default_id);
                if id.trim().is_empty() {
                    return Err(self.error("E_PAGE", "page.id 不能为空", loc));
                }
                let label = string(&args, "name", &id);
                args.insert("id".into(), V::text(id));
                args.insert("name".into(), V::text(label));
                args.insert("__src".into(), V::text(resolve(&self.file, &src)));
                return Ok(self.object(name, args, loc));
            }
            let owner = owner.unwrap();
            if owner.borrow().kind != "document" || parts[1] != "add" {
                return Err(self.error("E_CALL", "文档目前只支持 doc.add(page)", loc));
            }
            if pos.len() > 1
                || args.keys().any(|k| k != "page")
                || (pos.len() == 1 && args.contains_key("page"))
            {
                return Err(self.error(
                    "E_ARG",
                    "doc.add 需要一个 page；转场使用 document/page 的 transition 参数",
                    loc,
                ));
            }
            let item = pos
                .first()
                .cloned()
                .or_else(|| args.remove("page"))
                .ok_or_else(|| self.error("E_ARG", "doc.add 需要一个 page", loc))?;
            let page = item
                .object()
                .filter(|p| p.borrow().kind == "page")
                .ok_or_else(|| self.error("E_TYPE", "doc.add 需要 page(src=...) 引用", loc))?;
            let mut owner = owner.borrow_mut();
            let V::List(pages) = owner.args.get_mut("__pages").unwrap() else {
                unreachable!()
            };
            if pages.len() >= MAX_PAGES {
                return Err(self.error("E_LIMIT", "文档页面超过 10,000", loc));
            }
            let id = string(&page.borrow().args, "id", "");
            if pages
                .iter()
                .any(|p| string(&p.object().unwrap().borrow().args, "id", "") == id)
            {
                return Err(self.error("E_PAGE", format!("重复页面 id：{id}"), loc));
            }
            pages.push(item.clone());
            Ok(item)
        })())
    }
}
