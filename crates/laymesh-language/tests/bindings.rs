use laymesh_language::LanguageService;
use serde_json::Value as J;
const FILE: &str = "file:///work/main.lay";
fn service(source: &str) -> LanguageService {
    let mut s = LanguageService::new("en");
    s.update(FILE, source);
    s
}
fn apply(s: &mut LanguageService, edits: J, name: &str) {
    let mut changes = std::collections::BTreeMap::<String, Vec<(usize, usize)>>::new();
    for edit in edits.as_array().unwrap() {
        changes
            .entry(edit["uri"].as_str().unwrap().into())
            .or_default()
            .push((
                edit["from"].as_u64().unwrap() as usize,
                edit["to"].as_u64().unwrap() as usize,
            ));
    }
    for (uri, mut edits) in changes {
        edits.sort_by_key(|e| std::cmp::Reverse(e.0));
        let mut source = s.documents[&uri].clone();
        for (from, to) in edits {
            source.replace_range(from..to, name);
        }
        s.update(&uri, &source);
    }
}
#[test]
fn variables_have_types_and_aliases_keep_their_own_definition() {
    let source = "page=canvas(size=(15cm,10cm),unit=\"cm\")\nrec=rect(size=(2cm,2cm))\nimg1=image(src=\"missing.bmp\")\ncopy=rec\npage.add(copy)\nrec=text(content=\"later\")";
    let s = service(source);
    let at = source.find("page.add(").unwrap() + 9;
    let options = s.completions(FILE, at);
    for (name, typ) in [
        ("page", "canvas"),
        ("rec", "rect"),
        ("img1", "image"),
        ("copy", "rect"),
    ] {
        assert_eq!(
            options
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["label"] == name)
                .unwrap()["detail"],
            typ
        );
    }
    let at = source.find("add(copy").unwrap() + 5;
    assert!(
        s.hover(FILE, at)["contents"]
            .as_str()
            .unwrap()
            .contains("copy: rect")
    );
    assert_eq!(
        s.definition(FILE, at)["from"],
        source.find("copy=").unwrap()
    );
    assert!(
        s.references(FILE, source.find("rec=").unwrap(), true)
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == 3)
    );
    let s = service("page=canvas(size=(10,10))\npage.");
    assert!(
        s.completions(FILE, s.documents[FILE].len())
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["label"] == "add")
    );
}
#[test]
fn reassignments_are_writes_and_type_information_follows_the_cursor() {
    let source = "value=1\nfirst=value\nvalue=\"later\"\nsecond=value";
    let s = service(source);
    assert!(
        s.hover(FILE, source.find("first=value").unwrap() + 7)["contents"]
            .as_str()
            .unwrap()
            .contains("value: number")
    );
    assert!(
        s.hover(FILE, source.rfind("value").unwrap() + 1)["contents"]
            .as_str()
            .unwrap()
            .contains("value: string")
    );
    let refs = s.references(FILE, 0, true);
    assert_eq!(refs.as_array().unwrap().len(), 4);
    assert_eq!(
        refs.as_array()
            .unwrap()
            .iter()
            .filter(|o| o["kind"] == 3)
            .count(),
        2
    );
    assert_eq!(
        s.definition(FILE, source.rfind("value").unwrap())["from"],
        0
    );
    let future = service("first=1\nfirst\nfuture=2");
    assert!(
        !future
            .completions(FILE, 8)
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["label"] == "future")
    );
}
#[test]
fn block_scope_and_outer_assignment_follow_runtime_binding_rules() {
    let source = "value=1\nif true { value=2 secret=3 }\nfunction f(value) {\n private_value=value\n for value in [1,2] { loop=value }\n while false { hidden=1 }\n return value\n}\nlast=value";
    let s = service(source);
    let refs = s.references(FILE, 0, true);
    assert_eq!(refs.as_array().unwrap().len(), 3, "{refs}");
    let at = source.find("private_value=value").unwrap() + 15;
    assert_eq!(
        s.definition(FILE, at)["from"],
        source.find("f(value").unwrap() + 2
    );
    let at = source.find("loop=value").unwrap() + 6;
    assert_eq!(
        s.definition(FILE, at)["from"],
        source.find("for value").unwrap() + 4
    );
    let options = s.completions(FILE, source.len());
    for name in ["secret", "private_value", "loop", "hidden"] {
        assert!(
            !options
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["label"] == name && o["type"] == "variable"),
            "{name}"
        );
    }
}
#[test]
fn references_ignore_strings_comments_and_members_but_include_interpolations() {
    let source = "value=3\n# value ignored\nplain=\"value\"\nobj={\"value\":1}\nfriendly=f\"中文 😀 \\n{{value}} {value:.2f} {obj['value']}\"\nother=obj.value\n";
    let mut s = service(source);
    let refs = s.references(FILE, 0, true);
    assert_eq!(refs.as_array().unwrap().len(), 2, "{refs}");
    let edits = s.rename(FILE, 0, "measurement").unwrap();
    apply(&mut s, edits, "measurement");
    assert!(s.documents[FILE].contains("{{value}} {measurement:.2f}"));
    assert!(s.documents[FILE].contains("obj.value"));
    assert!(s.documents[FILE].contains("plain=\"value\""));
    assert!(s.documents[FILE].contains("# value ignored"));
}
#[test]
fn explicit_import_aliases_are_local_but_export_references_include_their_uses() {
    let source = "import {width as w} from \"./lib.lay\"\nresult=w";
    let mut s = service(source);
    let lib = "file:///work/lib.lay";
    s.update(lib, "export width=10");
    let other = "file:///work/other.lay";
    s.update(other, "import {width} from \"./lib.lay\"\nresult=width");
    assert_eq!(s.references(lib, 8, true).as_array().unwrap().len(), 6);
    let edits = s
        .rename(FILE, source.rfind('w').unwrap(), "extent")
        .unwrap();
    assert_eq!(edits.as_array().unwrap().len(), 2);
    assert!(edits.as_array().unwrap().iter().all(|o| o["uri"] == FILE));
    apply(&mut s, edits, "extent");
    assert_eq!(s.documents[lib], "export width=10");
    let edits = s.rename(lib, 8, "size").unwrap();
    apply(&mut s, edits, "size");
    assert!(s.documents[FILE].contains("{size as extent}"));
    assert!(s.documents[FILE].ends_with("result=extent"));
    assert!(s.documents[other].ends_with("result=size"));
    assert_eq!(
        s.definition(FILE, s.documents[FILE].rfind("extent").unwrap())["from"],
        s.documents[FILE].find("extent").unwrap()
    );
}
#[test]
fn parameter_rename_updates_named_arguments_through_function_aliases() {
    let source = "function card(size=1) { return size }\nwidth=2\nresult=card(size=width)\nalias=card\nsecond=alias(size=3)";
    let mut s = service(source);
    let at = source.find("size=").unwrap();
    assert_eq!(s.references(FILE, at, true).as_array().unwrap().len(), 4);
    let edits = s.rename(FILE, at, "width").unwrap();
    apply(&mut s, edits, "width");
    assert!(s.documents[FILE].contains("card(width=width)"));
    assert!(s.documents[FILE].contains("alias(width=3)"));
    assert!(s.documents[FILE].contains("return width"));
}
#[test]
fn rename_rejects_keywords_capture_and_incomplete_affected_documents() {
    let source = "value=1\nother=2\nresult=value\nfunction f(other) { return value }";
    let mut s = service(source);
    for name in [
        "if", "self", "auto", "1bad", "bad-name", "other", "rect", "round",
    ] {
        assert!(s.rename(FILE, 0, name).is_err(), "{name}");
    }
    assert!(
        s.prepare_rename(FILE, source.find("=1").unwrap() + 1)
            .is_null()
    );
    s.update(FILE, "value=1\nresult=value\npage.add(");
    assert!(s.prepare_rename(FILE, 0).is_null());
    assert!(s.rename(FILE, 0, "safe").is_err());
    assert!(s.completions(FILE, s.documents[FILE].len()).is_array());
    s.update(FILE, "value=1\nresult=value");
    assert!(!s.prepare_rename(FILE, 0).is_null());
    s.update(FILE, "value=1\nfunction f() { captured=2 }");
    assert!(s.rename(FILE, 0, "captured").is_err());
}
#[test]
fn edits_and_removal_refresh_cached_bindings_and_multiline_types() {
    let mut s = service("shape=rect(\n size=(10,5)\n)\ncopy=shape");
    assert!(
        s.hover(FILE, s.documents[FILE].rfind("shape").unwrap())["contents"]
            .as_str()
            .unwrap()
            .contains("shape: rect")
    );
    s.update(FILE, "shape=\"text\"\ncopy=shape");
    assert!(
        s.hover(FILE, s.documents[FILE].rfind("shape").unwrap())["contents"]
            .as_str()
            .unwrap()
            .contains("shape: string")
    );
    s.remove(FILE);
    assert_eq!(s.references(FILE, 0, true), serde_json::json!([]));
    s.documents
        .insert(FILE.into(), "shape=1\ncopy=shape".into());
    assert_eq!(s.references(FILE, 0, true).as_array().unwrap().len(), 2);
}
