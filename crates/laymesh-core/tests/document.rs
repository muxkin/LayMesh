use laymesh_core::{document::*, model::Host};
use std::collections::BTreeMap;

fn host() -> Host {
    Host { files: BTreeMap::from([
        ("/project/pages/a.lay".into(), b"p=canvas(size=(10,20),unit=\"cm\")\np.add(rect(size=(1,2)))".to_vec()),
        ("/project/pages/b.lay".into(), b"p=canvas(size=(10,20),unit=\"mm\")\np.add(rect(size=(1,2)))".to_vec()),
        ("/project/helpers/order.lay".into(), b"export function entries() { return [page(src=\"../pages/b.lay\",id=\"b\"),page(src=\"../pages/a.lay\",id=\"a\")] }".to_vec()),
    ]), ..Host::default() }
}
#[test]
fn ordinary_language_orders_independent_pages_and_module_paths() {
    let doc = compile_document_source("import { entries } from \"./helpers/order.lay\"\ndoc=document(name=\"报告\")\nfor p in entries() { if true { doc.add(p) } }", "/project/report.laydoc", host()).unwrap();
    assert_eq!(doc.name, "报告");
    assert_eq!(
        doc.pages
            .iter()
            .map(|p| p.page.id.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(
        (doc.pages[0].scene.width, doc.pages[1].scene.width),
        (10., 100.)
    );
    assert_eq!(doc.pages[0].scene.canvas_unit, "mm");
    assert_eq!(doc.pages[1].scene.canvas_unit, "cm");
    assert_eq!(doc.pages[0].page.src, "/project/pages/b.lay");
}
#[test]
fn selection_is_lazy_and_repeated_sources_share_a_scene() {
    let plan = plan_source("d=document()\nd.add(page(src=\"pages/a.lay\",id=\"one\"))\nd.add(page(src=\"pages/missing.lay\",id=\"two\"))", "/project/report.laydoc", host()).unwrap();
    assert!(compile_page(plan.page(1).unwrap(), host()).is_ok());
    assert_eq!(
        compile_page(plan.page(2).unwrap(), host())
            .unwrap_err()
            .code,
        "E_ASSET"
    );
    assert!(plan.page(0).is_err());
    assert!(plan.page(3).is_err());
    let doc = compile_document_source("d=document()\nd.add(page(src=\"pages/a.lay\",id=\"one\"))\nd.add(page(src=\"pages/a.lay\",id=\"two\"))", "/project/report.laydoc", host()).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &doc.pages[0].scene,
        &doc.pages[1].scene
    ));
}
#[test]
fn document_failures_are_explicit() {
    for (source, code) in [
        ("d=document()", "E_DOCUMENT"),
        ("p=canvas(size=(1,1))", "E_DOCUMENT"),
        ("d=document()\ne=document()", "E_DOCUMENT"),
        (
            "d=document()\np=page(src=\"pages/a.lay\")\nd.add(p)\nd.add(p)",
            "E_PAGE",
        ),
        ("d=document()\nd.add(page(src=\"/a.lay\"))", "E_PAGE"),
        ("d=document()\nd.add(page(src=\"other.laydoc\"))", "E_PAGE"),
        (
            "d=document()\nd.add(page(src=\"pages/a.lay\"),transition=\"fade\")",
            "E_ARG",
        ),
        ("d=document()\nd.add(rect(size=(1,1)))", "E_TYPE"),
    ] {
        let error = plan_source(source, "/project/report.laydoc", host()).unwrap_err();
        assert_eq!(error.code, code, "{source}: {error}");
    }
    assert_eq!(
        laymesh_core::engine::compile_source("d=document()", "/project/one.lay", host())
            .unwrap_err()
            .code,
        "E_DOCUMENT"
    );
}
