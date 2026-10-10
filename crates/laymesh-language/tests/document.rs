use laymesh_language::LanguageService;
use serde_json::json;

#[test]
fn document_language_services_share_the_lay_syntax_and_page_types() {
    let file = "/project/report.laydoc";
    let mut service = LanguageService::new("en");
    let source = "doc=document(name=\"Report\")\ncover=page(src=\"./cover.lay\",id=\"cover\")\ndoc.add(cover)";
    service.update(file, source);
    assert_eq!(service.diagnostics(file), json!([]));
    let source = format!("{source}\ndoc.");
    service.update(file, &source);
    let names: Vec<_> = service
        .completions(file, source.len())
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["label"].as_str().unwrap().to_owned())
        .collect();
    assert!(names.contains(&"add".into()));
    assert!(names.contains(&"pages".into()));
    assert!(!names.contains(&"fuse".into()));
    let source = "doc=document()\ndoc.add(";
    service.update(file, source);
    let signature = service.signature(file, source.len());
    assert!(
        signature["label"].as_str().unwrap().contains("page"),
        "{signature}"
    );
    assert!(!signature["label"].as_str().unwrap().contains("anchor"));
    let source = "doc=document()\np=page(src=\"./cover.lay\")\ndoc.add(p,transition=\"fade\")";
    service.update(file, source);
    assert!(!service.diagnostics(file).as_array().unwrap().is_empty());
}
