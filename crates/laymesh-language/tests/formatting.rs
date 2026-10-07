use laymesh_language::LanguageService;
use serde_json::{Value, json};

fn at(source: &str, p: &Value) -> usize {
    let mut start = 0;
    for _ in 0..p["line"].as_u64().unwrap() {
        start += source[start..].find('\n').unwrap() + 1;
    }
    let mut units = 0;
    for (n, c) in source[start..].char_indices() {
        if units == p["character"].as_u64().unwrap() as usize {
            return start + n;
        }
        units += c.len_utf16();
    }
    source.len()
}
fn apply(source: &str, edits: &Value) -> String {
    let mut result = source.to_string();
    for e in edits.as_array().unwrap().iter().rev() {
        result.replace_range(
            at(source, &e["range"]["start"])..at(source, &e["range"]["end"]),
            e["newText"].as_str().unwrap(),
        );
    }
    result
}
fn format(uri: &str, source: &str, options: Value, range: Option<Value>) -> String {
    let mut service = LanguageService::default();
    service.update(uri, source);
    let edits = service.formatting(uri, &options, range.as_ref());
    let result = apply(source, &edits);
    service.update(uri, &result);
    assert_eq!(
        service.formatting(uri, &options, range.as_ref()),
        json!([]),
        "Formatting was not idempotent:\n{result}"
    );
    result
}
fn options() -> Value {
    json!({"tabSize":2,"insertSpaces":true,"lineWidth":50})
}

#[test]
fn calls_lists_dictionaries_and_blocks() {
    let source = "export function panel(value=1){x={\"a\":[1,2,3],\"b\":(4,)} if x[\"a\"][0]==1{return text(content=\"a rather long title\",font_size=12pt,color=\"#123456\")}else{return value}}";
    let result = format("test.lay", source, options(), None);
    assert!(
        result
            .contains("export function panel(value = 1) {\n  x = {\"a\": [1, 2, 3], \"b\": (4,)}"),
        "{result}"
    );
    assert!(result.contains("text(\n"), "{result}");
    assert!(result.contains("12 pt"));
    assert!(result.contains("\n  } else {\n"));
}
#[test]
fn comments_strings_interpolation_and_spelling_are_opaque() {
    let source = "## Summary 中文 😀\n## @param n: quantity\nfunction f(n){\n# Keep  this\nx=1.20e-3mm # inline  comment  \ny=r'c:\\tmp'\nz=f\"中文 😀 {n+1} # text\"\nw='''literal\n  whitespace  \n'''\nreturn -x\n}\n";
    let result = format("test.lay", source, options(), None);
    for spelling in [
        "## Summary 中文 😀",
        "## @param n: quantity",
        "# Keep  this",
        "# inline  comment  ",
        "1.20e-3 mm",
        "r'c:\\tmp'",
        "f\"中文 😀 {n+1} # text\"",
        "'''literal\n  whitespace  \n'''",
        "return -x",
    ] {
        assert!(result.contains(spelling), "Missing {spelling:?}:\n{result}");
    }
}
#[test]
fn comments_between_arguments_force_safe_breaks() {
    let source = "x=text(content=\"中文\", # title\n# font\nfont_size=12pt,color=\"#123456\")";
    let result = format("test.lay", source, options(), None);
    assert!(result.contains("text(\n"), "{result}");
    assert!(result.contains("# title\n"));
    assert!(result.contains("# font\n"));
}
#[test]
fn stylesheet_and_embedded_style() {
    let css = "/* keep */\ncanvas{--ink:#123456;color:var(--ink);font-family:\"中文\",\"DejaVu Sans\";}\ntext.title,plot::axis(x){background:linear-gradient(to bottom,#ffffff,#123456);}";
    let result = format("test.lcss", css, options(), None);
    assert!(
        result.contains("canvas {\n  --ink: #123456;\n  color: var(--ink);"),
        "{result}"
    );
    assert!(result.contains("plot::axis(x)"));
    assert!(
        result.contains("font-family: \"中文\", \"DejaVu Sans\";"),
        "{result}"
    );
    assert!(result.contains("text.title, plot::axis(x) {"), "{result}");
    assert!(result.contains("linear-gradient(\n"), "{result}");
    let result = format(
        "test.lay",
        &format!("function f(){{style {{{css}}} return 1}}"),
        options(),
        None,
    );
    assert!(
        result.contains("style {\n    /* keep */\n    canvas {\n      --ink: #123456;"),
        "{result}"
    );
}
#[test]
fn imports_selectors_and_css_whitespace_retain_meaning() {
    let css = "@import \"./base.lcss\";\ncanvas text.title, canvas>.label{color:rgb(20 30 40 / .5);border:1mm solid #abcdef;}";
    let result = format("test.lcss", css, options(), None);
    assert!(result.contains("canvas text.title"));
    assert!(result.contains("canvas>.label"));
    assert!(result.contains("rgb(20 30 40 / .5)"));
    assert!(result.contains("border: 1mm solid #abcdef;"));
    let result = format(
        "test.lay",
        "import {a as b,c} from './lib.lay'\nx=1",
        options(),
        None,
    );
    assert_eq!(result, "import {a as b, c} from './lib.lay'\nx = 1");
}
#[test]
fn units_and_in_operator_keep_ast_meaning() {
    let source = "x=1\ny=2mm\nz=2in\nw=2 in\ninches=2 in\nmember=2 in [1,2]\nlist=[1\n,2]\nfunction f(){return 1}\nf()";
    let result = format("test.lay", source, options(), None);
    assert!(result.contains("x = 1\ny = 2 mm"), "{result}");
    assert!(result.contains("member = 2 in [1, 2]"));
}
#[test]
fn control_flow_and_dictionary_conditions() {
    let source = "for (a,b) in [[1,2]]{while false{continue} if {\"x\":1}{break}else if true{a=a+1}else{a=0}}";
    let result = format("test.lay", source, options(), None);
    assert!(result.contains("for (a, b) in [[1, 2]] {\n"), "{result}");
    assert!(result.contains("} else if true {"));
    assert!(result.contains("if {\"x\": 1} {"));
    assert!(
        format("test.lay", "for a,b in [[1,2]]{break}", options(), None)
            .starts_with("for a, b in [[1, 2]] {")
    );
}
#[test]
fn crlf_tabs_final_newlines_and_blank_lines() {
    let source = "function f(){\r\n\r\n\r\nx=1\r\nreturn x\r\n}\r\n\r\n";
    let result = format(
        "test.lay",
        source,
        json!({"tabSize":4,"insertSpaces":false,"trimFinalNewlines":true}),
        None,
    );
    assert_eq!(result, "function f() {\r\n\tx = 1\r\n\treturn x\r\n}\r\n");
    assert_eq!(
        format(
            "test.lay",
            "x=1\n",
            json!({"insertFinalNewline":false}),
            None
        ),
        "x = 1\n"
    );
    assert_eq!(
        format("test.lay", "x=1", json!({"insertFinalNewline":true}), None),
        "x = 1\n"
    );
    assert_eq!(
        format("test.lay", "x=1\n\n\ny=2", options(), None),
        "x = 1\n\ny = 2"
    );
}
#[test]
fn document_range_changes_only_selected_statement() {
    let source =
        "first=1\nfunction f(){\n  chosen=text(content=\"😀\",font_size=10pt)\n  last=3\n}\n";
    let range = json!({"start":{"line":2,"character":12},"end":{"line":2,"character":22}});
    let result = format("test.lay", source, options(), Some(range));
    assert!(result.starts_with("first=1\nfunction f(){\n"));
    assert!(result.ends_with("  last=3\n}\n"));
    assert!(result.contains("  chosen = text("), "{result}");
}
#[test]
fn range_spanning_siblings_does_not_format_parent() {
    let source = "function f(){\n x=1\n y=2\n z=3\n}";
    let mut service = LanguageService::default();
    service.update("test.lay", source);
    let range = json!({"start":{"line":1,"character":1},"end":{"line":2,"character":4}});
    let result = apply(
        source,
        &service.formatting("test.lay", &options(), Some(&range)),
    );
    assert_eq!(result, "function f(){\n  x = 1\n  y = 2\n z=3\n}");
}
#[test]
fn css_range_and_embedded_declaration_range() {
    let source = "canvas {\n color:#123456;\n font-size:12pt;\n}";
    let range = json!({"start":{"line":1,"character":3},"end":{"line":1,"character":6}});
    let result = format("test.lcss", source, options(), Some(range));
    assert_eq!(result, "canvas {\n  color: #123456;\n font-size:12pt;\n}");
    let source = "function f(){\n style {\n canvas {\n color:#123456;\n font-size:12pt;\n }\n }\n}";
    let range = json!({"start":{"line":3,"character":3},"end":{"line":3,"character":6}});
    let result = format("test.lay", source, options(), Some(range));
    assert!(
        result.contains("\n      color: #123456;\n font-size:12pt;"),
        "{result}"
    );
}
#[test]
fn incomplete_code_and_literal_selections_are_unchanged() {
    for (uri, source) in [
        ("test.lay", "x=text("),
        ("test.lay", "x='open"),
        ("test.lcss", "text { color:"),
        ("test.lcss", "/* open"),
        ("test.lay", "style { text { color: } }"),
    ] {
        assert_eq!(format(uri, source, options(), None), source);
    }
    let embedded = "style {\n text {\n font-family: \"DejaVu Sans\";\n }\n}";
    let literal_range = json!({"start":{"line":2,"character":17},"end":{"line":2,"character":20}});
    assert_eq!(
        format("test.lay", embedded, options(), Some(literal_range)),
        embedded
    );
    let source = "x='literal'\n# comment";
    let range = json!({"start":{"line":0,"character":4},"end":{"line":0,"character":6}});
    assert_eq!(format("test.lay", source, options(), Some(range)), source);
}
#[test]
fn shipped_examples_format_without_changing_semantics() {
    fn visit(path: &std::path::Path, count: &mut usize) {
        for entry in std::fs::read_dir(path).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                if p.file_name().unwrap() != "output" {
                    visit(&p, count);
                }
            } else if matches!(p.extension().and_then(|s| s.to_str()), Some("lay" | "lcss")) {
                let source = std::fs::read_to_string(&p).unwrap();
                let uri = p.to_str().unwrap();
                let mut service = LanguageService::default();
                service.update(uri, &source);
                let edits = service.formatting(uri, &options(), None);
                let result = apply(&source, &edits);
                assert_ne!(edits, json!([]), "Example could not be formatted: {uri}");
                service.update(uri, &result);
                assert_eq!(
                    service.formatting(uri, &options(), None),
                    json!([]),
                    "Example not idempotent: {uri}"
                );
                *count += 1;
            }
        }
    }
    let mut count = 0;
    visit(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"),
        &mut count,
    );
    assert!(count > 100);
}
