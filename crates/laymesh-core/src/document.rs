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
    pub id: String,
    pub name: String,
    pub src: String,
    pub file: String,
    pub loc: Loc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPlan {
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
            let scene = if let Some(scene) = cache.get(&page.src) {
                scene.clone()
            } else {
                let scene = Arc::new(compile_page(page, host.clone())?);
                cache.insert(page.src.clone(), scene.clone());
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
    crate::engine::compile_source(&source, &page.src, host)
}
pub fn compile_preview_page(page: &PageSpec, host: Host) -> Result<Scene> {
    let source = read_page(page, &host)?;
    crate::engine::compile_preview_source(&source, &page.src, host)
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
            PageSpec {
                id: string(&page.args, "id", ""),
                name: string(&page.args, "name", ""),
                src: string(&page.args, "__src", ""),
                file: page.file.clone(),
                loc: page.loc,
            }
        })
        .collect();
    Ok(DocumentPlan {
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
        let builtin = parts.len() == 1 && matches!(parts[0].as_str(), "document" | "page");
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
                    &["name"]
                } else {
                    &["src", "id", "name"]
                };
                if !pos.is_empty() || args.keys().any(|k| !allowed.contains(&k.as_str())) {
                    return Err(self.error(
                        "E_ARG",
                        format!("{name} 只接受命名参数 {}", allowed.join(", ")),
                        loc,
                    ));
                }
                if args.values().any(|v| !matches!(v.value(), V::Text(..))) {
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
                return Err(self.error("E_ARG", "doc.add 需要一个 page；切换动画尚未实现", loc));
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
