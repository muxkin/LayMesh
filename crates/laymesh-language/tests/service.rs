use laymesh_language::LanguageService;
const LIB: &str = r##"## Create a titled panel.
##
## Keeps **title** above the content.
## @param {string} title - Title supporting inline math.
## @param {size} size - Physical size in the figure unit.
## @returns {group} A reusable panel.
## @example
## card("Result", size=(40, 25))
## @see [Units](units.en.md)
export function card(title, size=(40, 25)) {
 panel=group()
 panel.add(rect(size=size))
 panel.add(text(title))
 return panel
}
## Accent for panels.
## @type {color}
export accent="#245447"
"##;
#[test]
fn incomplete_completion_and_signature() {
    let mut s = LanguageService::new("zh");
    let input = "page=canvas(size=(100,80))\np=plot(size=(90,70))\np.line(x=[0,1],y=[1,2], line_";
    s.update("test.lay", input);
    assert!(
        s.completions("test.lay", input.len())
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == "line_width")
    );
    assert!(
        s.signature("test.lay", input.len())["label"]
            .as_str()
            .unwrap()
            .starts_with("plot.line(")
    );
    assert_eq!(s.diagnostics("test.lay")[0]["severity"], "error");
}
#[test]
fn migration_fix_and_hover() {
    let mut s = LanguageService::new("zh");
    let input = "page=canvas(size=(100,80))\nr=rect(size=(40,25),stroke_width=1pt)";
    s.update("test.lay", input);
    let issues = s.diagnostics("test.lay");
    let issue = issues
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["code"] == "E_API_MIGRATION")
        .unwrap();
    assert_eq!(issue["replacement"], "border_width");
    assert_eq!(
        &input[issue["from"].as_u64().unwrap() as usize..issue["to"].as_u64().unwrap() as usize],
        "stroke_width"
    );
    let updated = input.replace("stroke_width", "border_width");
    s.update("test.lay", &updated);
    assert!(
        s.hover("test.lay", updated.find("border_width").unwrap() + 3)["contents"]
            .as_str()
            .unwrap()
            .contains("pt")
    );
    assert_eq!(s.diagnostics("test.lay").as_array().unwrap().len(), 0);
}
#[test]
fn placement_signature_active_parameters() {
    let mut s = LanguageService::new("zh");
    for (tail, name) in [
        ("page.add(", "material"),
        ("page.add(rect(size=(30,20)), ", "size"),
        ("page.add(rect(size=(30,20)), offset=", "offset"),
        ("page.add(rect(size=(30,20)), offset=(1,2), ", "size"),
    ] {
        let input = format!("page=canvas(size=(100,80))\n{tail}");
        s.update("main.lay", &input);
        let sig = s.signature("main.lay", input.len());
        let active = sig["activeParameter"].as_u64().unwrap() as usize;
        assert_eq!(sig["parameters"][active]["name"], name, "{sig}");
        assert!(sig["documentation"].as_str().unwrap().contains("重复放置"));
    }
}
#[test]
fn authored_defaults_and_returns() {
    let mut s = LanguageService::new("en");
    let input = format!("{LIB}\nvalue=card(\"Energy\", size=");
    s.update("card.lay", &input);
    let sig = s.signature("card.lay", input.len());
    assert!(
        sig["documentation"]
            .as_str()
            .unwrap()
            .contains("A reusable panel")
    );
    assert_eq!(sig["parameters"][1]["default"], "(40, 25)");
    assert_eq!(sig["parameters"][1]["required"], false);
    assert!(
        sig["parameters"][1]["documentation"]
            .as_str()
            .unwrap()
            .contains("Physical size")
    );
    assert!(
        s.hover("card.lay", input.find("accent=").unwrap() + 2)["contents"]
            .as_str()
            .unwrap()
            .contains("color")
    );
    assert!(
        s.hover("card.lay", input.find("text(title)").unwrap() + 6)["contents"]
            .as_str()
            .unwrap()
            .contains("Title supporting")
    );
}
#[test]
fn import_alias_edits_removal() {
    let mut s = LanguageService::new("en");
    let input = "import {card as tile, accent} from \"./lib/card.lay\"\nvalue=tile(\"x\", ";
    s.update("examples/figure.lay", input);
    s.update("examples/lib/card.lay", LIB);
    assert_eq!(
        s.signature("examples/figure.lay", input.len())["parameters"][1]["name"],
        "size"
    );
    assert_eq!(
        s.definition("examples/figure.lay", input.rfind("tile").unwrap() + 1)["uri"],
        "examples/lib/card.lay"
    );
    s.update(
        "examples/lib/card.lay",
        &LIB.replace("Physical size", "Panel dimensions")
            .replace("(40, 25)", "(50, 30)"),
    );
    assert_eq!(
        s.signature("examples/figure.lay", input.len())["parameters"][1]["default"],
        "(50, 30)"
    );
    s.remove("examples/lib/card.lay");
    assert!(s.signature("examples/figure.lay", input.len()).is_null());
}
#[test]
fn incomplete_nested_defaults_and_shadowing() {
    let mut s = LanguageService::new("en");
    let input =
        format!("{LIB}\nfunction unfinished(first, second=rect(size=(3, 4))) {{\n unfinished(1, ");
    s.update("main.lay", &input);
    assert_eq!(
        s.signature("main.lay", input.len())["parameters"][1]["default"],
        "rect(size=(3, 4))"
    );
    let input = "function helper(outside) {return outside}\nfunction outer() {\n function helper(inside=2) {return inside}\n helper(";
    s.update("main.lay", input);
    assert_eq!(
        s.signature("main.lay", input.len())["parameters"][0]["name"],
        "inside"
    );
}
#[test]
fn doc_warnings_and_templates() {
    let mut s = LanguageService::new("zh");
    let input = "## Summary.\n## @param missing - Typo.\n## @param missing - Again.\n## @returns first\n## @returns second\n## @unknown tag\nfunction value(real=1) {return real}\n";
    s.update("main.lay", input);
    let ds = s.diagnostics("main.lay");
    assert_eq!(ds.as_array().unwrap().len(), 5, "{ds}");
    assert!(
        ds.as_array()
            .unwrap()
            .iter()
            .all(|v| v["severity"] == "warning")
    );
    s.update(
        "main.lay",
        "## \nexport function card(title, size=(40,25)) {return title}",
    );
    let options = s.completions("main.lay", 3);
    assert!(
        options
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["label"] == "文档模板")
            .unwrap()["apply"]
            .as_str()
            .unwrap()
            .contains("@param title")
    );
}
#[test]
fn partial_parameter_and_transitive_alias() {
    let mut s = LanguageService::new("en");
    for (tail, name) in [("cl", Some("class")), ("c", None)] {
        let input = format!("page=canvas(size=(80,50))\npage.add(rect(size=(3,2)), {tail}");
        s.update("main.lay", &input);
        let sig = s.signature("main.lay", input.len());
        if let Some(name) = name {
            assert_eq!(
                sig["parameters"][sig["activeParameter"].as_u64().unwrap() as usize]["name"],
                name
            )
        } else {
            assert!(sig["activeParameter"].is_null(), "{sig}")
        }
    }
    let input = "import {tile as panel} from \"./wrapper.lay\"\nx=panel(";
    s.update("main.lay", input);
    s.update(
        "wrapper.lay",
        "import {card} from \"./card.lay\"\nexport tile=card",
    );
    s.update("card.lay", LIB);
    assert!(
        s.signature("main.lay", input.len())["documentation"]
            .as_str()
            .unwrap()
            .contains("Create a titled panel")
    );
    assert_eq!(
        s.definition("main.lay", input.rfind("panel").unwrap() + 1)["uri"],
        "card.lay"
    );
}
#[test]
fn localized_docs_field_fallback() {
    let mut s = LanguageService::new("zh-CN");
    s.update("lib.lay","## Shared overview.\n## @param {size} size - Default dimensions.\n## @lang zh-CN\n## 中文说明。\n## @param {string} title - 中文标题。\n## @lang en\n## English summary.\n## @param {string} title - English title.\n## @param {size} size - Panel dimensions.\nexport function card(title,size=(40,25)) {return title}");
    let input = "import {card} from \"./lib.lay\"\ncard(";
    s.update("main.lay", input);
    let zh = s.signature("main.lay", input.len());
    assert_eq!(zh["summary"], "中文说明。");
    assert_eq!(zh["parameters"][1]["description"], "Default dimensions.");
    s.locale = "en".into();
    let en = s.signature("main.lay", input.len());
    assert_eq!(en["summary"], "English summary.");
    assert_eq!(en["parameters"][1]["description"], "Panel dimensions.");
    assert_eq!(en["label"], zh["label"]);
}
#[test]
fn shipped_examples_static_diagnostics() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fn collect(p: &std::path::Path, s: &mut LanguageService) {
        for e in std::fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                collect(&p, s)
            } else if p.extension().is_some_and(|s| s == "lay" || s == "lcss") {
                s.update(&p.to_string_lossy(), &std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let mut s = LanguageService::new("en");
    collect(&root.join("examples"), &mut s);
    let errors: Vec<_> = s
        .documents
        .keys()
        .flat_map(|file| {
            s.diagnostics(file)
                .as_array()
                .unwrap()
                .iter()
                .map(|e| format!("{file}: {e}"))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn lcss_parts_classes_variables_and_values() {
    let mut service = LanguageService::new("en");
    service.update(
        "/theme.lcss",
        ":root { --accent: red; } .card { fill: blue; }",
    );
    for (source, expected) in [
        ("plot::axis-", "axis-label"),
        (".ca", "card"),
        ("rect { fill: var(--a", "--accent"),
        ("rect { fill: lin", "linear-gradient()"),
        ("rect { border-w", "border-width"),
    ] {
        service.update("/main.lcss", source);
        let result = service.completions("/main.lcss", source.len());
        assert!(
            result
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["label"] == expected),
            "{source}: {result}"
        );
    }
}
