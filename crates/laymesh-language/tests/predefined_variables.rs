use laymesh_language::LanguageService;
fn service(source: &str) -> LanguageService {
    let mut s = LanguageService::new("zh-CN");
    s.update("test.lay", source);
    s
}
#[test]
fn ordinary_variable_hover_and_parameter_semantics_are_separate() {
    let text = "line(length=20mm,start_cap=round)";
    let s = service(text);
    let h = s.hover("test.lay", text.find("round").unwrap() + 2);
    assert_eq!(h["contents"], "```lay\nround: string = \"round\"\n```");
    let h = s.hover("test.lay", text.find("start_cap").unwrap() + 3);
    let doc = h["contents"].as_str().unwrap();
    assert!(
        doc.contains("\"butt\" | \"round\" | \"square\"")
            && doc.contains("默认继承")
            && doc.contains("半个线宽"),
        "{doc}"
    );
    let text = "round=\"square\"\nline(length=20mm,start_cap=round)";
    assert!(
        !service(text).hover("test.lay", text.len() - 3)["contents"]
            .as_str()
            .unwrap()
            .contains("string = \"round\"")
    );
}
#[test]
fn value_completion_handles_quotes_nested_calls_positional_and_shadowing() {
    for (source, label, apply) in [
        ("line(length=20mm,start_cap=ro", "round", "round"),
        (
            "page.add(line(length=20mm),\n anchor=top",
            "top_left",
            "top_left",
        ),
        ("page.add(line(length=20mm),anchor=", "self", "self."),
        ("head(shape=tri", "triangle", "triangle"),
        ("a.path.in_space(lo", "local", "local"),
        (
            "a.path.at(fraction=0.5).with_side(in",
            "incoming",
            "incoming",
        ),
        (
            "formula(\"x\",math_font=ratex",
            "ratex_katex",
            "ratex_katex",
        ),
    ] {
        let source =
            format!("page=canvas(size=(80mm,60mm))\na=page.add(line(length=20mm))\n{source}");
        let opts = service(&source).completions("test.lay", source.len());
        let o = opts
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["label"] == label)
            .unwrap_or_else(|| panic!("{source}: {opts}"));
        assert_eq!(o["apply"], apply);
    }
    let text = "# 中文\nline(length=20mm,start_cap=\"roxx\")";
    let opts = service(text).completions("test.lay", text.find("roxx").unwrap() + 2);
    let o = opts
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["label"] == "round")
        .unwrap();
    assert_eq!(o["apply"], "round");
    assert_eq!(o["from"], text.find("roxx").unwrap());
    assert_eq!(o["to"], text.find("roxx").unwrap() + 4);
    let text = "custom=round\nline(length=20mm,start_cap=";
    assert!(
        service(text)
            .completions("test.lay", text.len())
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["label"] == "custom")
    );
}
#[test]
fn predefined_names_have_no_unknown_name_diagnostics() {
    let text = "page=canvas(size=(80mm,60mm))\nopts=[round,triangle,top_left,ratex_katex]\npage.add(line(length=10mm,start_cap=opts[0]),anchor=top_left)";
    assert_eq!(service(text).diagnostics("test.lay"), serde_json::json!([]));
}

#[test]
fn option_diagnostics_respect_shadowed_variables() {
    let t = "line(length=20mm,start_cap=triangle)";
    assert!(
        service(t)
            .diagnostics("test.lay")
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_VALUE")
    );
    let t = "triangle=round\nline(length=20mm,start_cap=triangle)";
    assert!(
        !service(t)
            .diagnostics("test.lay")
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E_VALUE")
    );
}
