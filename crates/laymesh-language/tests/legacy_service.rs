//! Assertion-for-assertion ports of baseline language-service and locale tests.
use laymesh_language::{LanguageService, byte_offset};
use serde_json::Value as J;
const FILE: &str = "main.lay";
const LIB: &str = r###"## Create a titled panel.
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
"###;
const LOCALIZED: &str = r###"## Shared **overview**.
## @param {size} size - Default dimensions.
## @returns {group} Default result.
## @example
## card("Shared")
## @see [Guide](https://example.com/guide)
## @lang zh-CN
## 创建带标题的面板。
## @param {string} title - 面板标题。
## @returns {group} 可复用的组合。
## @example
## card("结果")
## @lang en
## Create a titled panel.
## @param {string} title - Panel title.
## @param {size} size - Panel dimensions.
## @returns {group} Reusable group.
export function card(title, size=(40,25)) {return title}
## @lang zh-CN
## 面板强调色。
## @type {color}
## @lang en
## Panel accent.
## @type {color}
export accent="#245447"
"###;
fn service(source: &str, locale: &str) -> LanguageService {
    let mut s = LanguageService::new(locale);
    s.update(FILE, source);
    s
}
fn has(value: &J, expected: &str) {
    assert!(
        value.as_str().unwrap_or("").contains(expected),
        "{value} must contain {expected}"
    );
}
fn completion(s: &LanguageService, uri: &str, at: usize, name: &str) -> J {
    s.completions(uri, at)
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["label"] == name)
        .cloned()
        .unwrap_or(J::Null)
}
fn sig(s: &LanguageService) -> J {
    s.signature(FILE, s.documents[FILE].len())
}
fn active(s: &J) -> &J {
    &s["parameters"][s["activeParameter"].as_u64().unwrap() as usize]
}
#[test]
fn incomplete_dsl_and_embedded_lcss() {
    let source = "page=canvas(size=(100,80))\np=plot(size=(90,70))\np.line(x=[0,1],y=[1,2], line_";
    let mut s = service(source, "zh");
    assert!(!completion(&s, FILE, source.len(), "line_width").is_null());
    assert!(sig(&s)["label"].as_str().unwrap().starts_with("plot.line("));
    assert_eq!(s.diagnostics(FILE)[0]["severity"], "error");
    let css = "style { .paper { background:#ffffff; } plot::axis {line-width:0.6pt;} }";
    let at = css.find("line-width").unwrap();
    let from = laymesh_language::css_region(css, at).unwrap();
    assert!(css[from..].contains("#ffffff"));
    s.update(FILE, css);
    has(&s.hover(FILE, at + 3)["contents"], "pt");
}
#[test]
fn parameter_migration_fix_and_definition() {
    let source = "page=canvas(size=(100,80))\nr=rect(size=(40,25),stroke_width=1pt)";
    let mut s = service(source, "zh");
    let ds = s.diagnostics(FILE);
    let issue = ds
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "E_API_MIGRATION")
        .unwrap();
    assert_eq!(issue["replacement"], "border_width");
    assert_eq!(
        &source[issue["from"].as_u64().unwrap() as usize..issue["to"].as_u64().unwrap() as usize],
        "stroke_width"
    );
    s.update(FILE, &source.replace("stroke_width", "border_width"));
    has(
        &s.hover(FILE, source.find("stroke_width").unwrap() + 3)["contents"],
        "pt",
    );
    assert_eq!(s.diagnostics(FILE), serde_json::json!([]));
    assert_eq!(
        s.definition(FILE, source.find("r=").unwrap())["from"],
        source.find("r=").unwrap()
    );
}
#[test]
fn placement_signatures_and_ranges() {
    let mut s = service("", "zh");
    for (tail, name) in [
        ("page.add(", "material"),
        ("page.add(rect(size=(30,20)), ", "size"),
        ("page.add(rect(size=(30,20)), offset=", "offset"),
        ("page.add(rect(size=(30,20)), offset=(1,2), ", "size"),
    ] {
        s.update(FILE, &format!("page=canvas(size=(100,80))\n{tail}"));
        let v = sig(&s);
        let p = active(&v);
        assert_eq!(p["name"], name);
        let label = v["label"].as_str().unwrap();
        let a = byte_offset(label, p["label"][0].as_u64().unwrap() as usize);
        let b = byte_offset(label, p["label"][1].as_u64().unwrap() as usize);
        assert!(label[a..b].starts_with(name));
        has(&v["documentation"], "重复放置");
        has(&v["parameters"][0]["documentation"], "素材");
    }
    s.update(FILE, "page=canvas(size=(100,80))\npage.add(rect(size=");
    let v = sig(&s);
    assert_eq!(v["name"], "rect");
    assert_eq!(active(&v)["name"], "size");
    s.update(
        FILE,
        "page=canvas(size=(100,80))\npage.add(rect(size=(4,2)), bad=",
    );
    assert!(sig(&s)["activeParameter"].is_null());
}
#[test]
fn authored_markdown_defaults_and_parameters() {
    let source = format!("{LIB}\nvalue=card(\"Energy\", size=");
    let s = service(&source, "en");
    let v = sig(&s);
    for text in ["Create a titled panel", "Returns", "```lay"] {
        has(&v["documentation"], text)
    }
    assert_eq!(active(&v)["name"], "size");
    assert_eq!(v["parameters"][1]["default"], "(40, 25)");
    assert_eq!(v["parameters"][1]["required"], false);
    has(&v["parameters"][1]["documentation"], "Physical size");
    has(
        &s.hover(FILE, source.rfind("card(").unwrap() + 2)["contents"],
        "A reusable panel",
    );
    let h = s.hover(FILE, source.find("accent=").unwrap() + 2);
    has(&h["contents"], "Accent for panels");
    has(&h["contents"], "color");
    has(
        &s.hover(FILE, source.find("text(title)").unwrap() + 6)["contents"],
        "Title supporting",
    );
}
#[test]
fn imports_transitive_edits_and_removal() {
    let main = "examples/figure.lay";
    let module = "examples/lib/card.lay";
    let source = "import {card as tile, accent} from \"./lib/card.lay\"\nvalue=tile(\"x\", ";
    let mut s = LanguageService::new("zh");
    s.update(main, source);
    s.update(module, LIB);
    s.update("other.lay", "export function tile(other) {return other}");
    assert_eq!(
        s.signature(main, source.len())["parameters"][1]["name"],
        "size"
    );
    has(
        &completion(&s, main, source.len(), "size")["info"],
        "Physical size",
    );
    assert_eq!(
        s.definition(main, source.rfind("tile").unwrap() + 1)["uri"],
        module
    );
    has(
        &completion(&s, main, source.len(), "accent")["info"],
        "Accent",
    );
    s.update(
        module,
        &LIB.replace("Physical size", "Panel dimensions")
            .replace("(40, 25)", "(50, 30)"),
    );
    let v = s.signature(main, source.len());
    has(&v["parameters"][1]["documentation"], "Panel dimensions");
    assert_eq!(v["parameters"][1]["default"], "(50, 30)");
    let wrapper =
        "import {card} from \"./lib/card.lay\"\nexport function wrap(label) {return card(label)}";
    s.update("examples/wrapper.lay", wrapper);
    has(
        &s.hover("examples/wrapper.lay", wrapper.rfind("card(").unwrap() + 1)["contents"],
        "Create a titled panel",
    );
    s.remove(module);
    assert!(s.signature(main, source.len()).is_null());
}
#[test]
fn incomplete_bodies_do_not_invent_symbols() {
    let source =
        format!("{LIB}\nfunction unfinished(first, second=rect(size=(3, 4))) {{\n unfinished(1, ");
    let mut s = service(&source, "zh");
    let v = sig(&s);
    assert_eq!(v["name"], "unfinished");
    assert_eq!(v["parameters"][1]["default"], "rect(size=(3, 4))");
    assert_eq!(active(&v)["name"], "second");
    assert!(
        !s.completions(FILE, source.len())
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["type"] == "variable" && c["label"] == "size")
    );
    let source = format!("{LIB}\nmessage=\"function fake(x) {{\"\n# function ghost(x) {{\n");
    s.update(FILE, &source);
    assert!(
        !s.completions(FILE, source.len())
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["label"] == "fake" || c["label"] == "ghost")
    );
}
#[test]
fn lexical_shadowing_and_scope() {
    let mut s = service(
        "function helper(outside) {return outside}\nfunction outer() {\n function helper(inside=2) {return inside}\n helper(",
        "zh",
    );
    assert_eq!(sig(&s)["parameters"][0]["name"], "inside");
    s.update(FILE, "function rect(custom) {return custom}\nrect(");
    assert_eq!(sig(&s)["parameters"][0]["name"], "custom");
    s.update(
        FILE,
        "function rect(custom) {return custom}\nfunction other() { secret=1 }\nvalue=",
    );
    assert!(completion(&s, FILE, s.documents[FILE].len(), "secret").is_null());
}
#[test]
fn doc_warnings_and_detached_comments() {
    let source = "## Summary.\n## @param missing - Typo.\n## @param missing - Again.\n## @returns first\n## @returns second\n## @unknown tag\nfunction value(real=1) {return real}\n";
    let mut s = service(source, "zh");
    let ds = s.diagnostics(FILE);
    let ds = ds.as_array().unwrap();
    assert_eq!(ds.len(), 5);
    assert!(
        ds.iter()
            .all(|d| d["severity"] == "warning" && d["code"] == "W_DOC")
    );
    assert!(ds.iter().all(|d| {
        source[d["from"].as_u64().unwrap() as usize..d["to"].as_u64().unwrap() as usize]
            .starts_with("## @")
    }));
    s.update(
        FILE,
        "## Detached.\n\nfunction no_doc(value) {return value}\nno_doc(",
    );
    assert_eq!(sig(&s)["documentation"], "");
}
#[test]
fn doc_templates_follow_declaration() {
    let source = "## \nexport function card(title, size=(40,25)) {return title}";
    let mut s = service(source, "zh");
    let v = completion(&s, FILE, 3, "文档模板");
    has(&v["apply"], "@param title");
    has(&v["apply"], "@param size");
    let source = source.replace("## ", "## @param {size} s");
    s.update(FILE, &source);
    assert_eq!(
        completion(&s, FILE, source.find('\n').unwrap(), "size")["apply"],
        "size - "
    );
    s.update(
        FILE,
        "## @re\nexport function card(title, size=(40,25)) {return title}",
    );
    assert!(!completion(&s, FILE, 6, "@returns").is_null());
}
#[test]
fn builtin_and_lcss_markdown() {
    let source = "page=canvas(size=(80,50))\npage.";
    let mut s = service(source, "zh");
    has(
        &completion(&s, FILE, source.len(), "add")["info"],
        "重复放置",
    );
    let source = "page=canvas(size=(80,50))\npage.add(rect(size=(3,2)), cl";
    s.update(FILE, source);
    let c = completion(&s, FILE, source.len(), "class");
    for expected in ["默认", "空格分隔", "#add"] {
        has(&c["info"], expected)
    }
    s.update("theme.lcss", ".paper {border-");
    has(
        &completion(&s, "theme.lcss", 15, "border-width")["info"],
        "pt",
    );
}
#[test]
fn partial_parameters_and_aliases() {
    let mut s = service(
        "page=canvas(size=(80,50))\npage.add(rect(size=(3,2)), cl",
        "zh",
    );
    assert_eq!(active(&sig(&s))["name"], "class");
    s.update(
        FILE,
        "page=canvas(size=(80,50))\npage.add(rect(size=(3,2)), c",
    );
    assert!(sig(&s)["activeParameter"].is_null());
    let source = "import {tile as panel} from \"./wrapper.lay\"\nx=panel(";
    s.update(FILE, source);
    s.update(
        "wrapper.lay",
        "import {card} from \"./card.lay\"\nexport tile=card",
    );
    s.update("card.lay", LIB);
    has(&sig(&s)["documentation"], "Create a titled panel");
    assert_eq!(
        s.definition(FILE, source.rfind("panel").unwrap() + 1)["uri"],
        "card.lay"
    );
}
#[test]
fn end_of_source_diagnostics_are_finite() {
    let source = "page=canvas(size=(80,50))\npage.add(";
    let s = service(source, "zh");
    let ds = s.diagnostics(FILE);
    let d = ds
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "E_SYNTAX")
        .unwrap();
    assert_eq!(d["from"], source.len());
    assert_eq!(d["to"], source.len());
}
#[test]
fn authored_doc_errors_do_not_change_compilation() {
    let mut host = laymesh_core::model::Host::default();
    host.files.insert(
        "/card.lay".into(),
        LIB.replace("## @param {string} title", "## @param {string} misspelled")
            .into_bytes(),
    );
    let scene = laymesh_core::engine::compile_source(
        "page=canvas(size=(100,80))\nimport {card} from \"./card.lay\"\npage.add(card(\"Result\"))",
        "/main.lay",
        host,
    )
    .unwrap();
    assert_eq!(scene.width, 100.);
    assert_eq!(scene.height, 80.);
    assert!(!scene.warnings.iter().any(|w| w.code == "W_DOC"));
}
fn localized(locale: &str) -> LanguageService {
    let mut s = service(
        "import {card as tile, accent} from \"./library.lay\"\nitem=tile(\"x\", ",
        locale,
    );
    s.update("library.lay", LOCALIZED);
    s
}
#[test]
fn locale_selects_fields_preserves_types_and_updates() {
    let mut s = localized("zh-CN");
    let source = s.documents[FILE].clone();
    let zh = sig(&s);
    assert_eq!(zh["summary"], "创建带标题的面板。");
    assert_eq!(zh["parameters"][0]["description"], "面板标题。");
    assert_eq!(zh["parameters"][1]["description"], "Default dimensions.");
    has(&zh["documentation"], "card(\"结果\")");
    has(&zh["documentation"], "https://example.com");
    has(
        &completion(&s, FILE, source.len(), "accent")["info"],
        "面板强调色",
    );
    has(
        &s.hover("library.lay", LOCALIZED.find("return title").unwrap() + 8)["contents"],
        "面板标题",
    );
    assert_eq!(s.diagnostics("library.lay"), serde_json::json!([]));
    s.locale = "en-GB".into();
    let en = sig(&s);
    assert_eq!(en["summary"], "Create a titled panel.");
    assert_eq!(en["parameters"][1]["description"], "Panel dimensions.");
    has(&en["documentation"], "card(\"Shared\")");
    assert_eq!(en["label"], zh["label"]);
    assert_eq!(
        en["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| &p["type"])
            .collect::<Vec<_>>(),
        zh["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| &p["type"])
            .collect::<Vec<_>>()
    );
    assert_eq!(s.documents[FILE], source);
    has(
        &s.hover("library.lay", LOCALIZED.find("return title").unwrap() + 8)["contents"],
        "Panel title",
    );
    has(
        &completion(&s, FILE, source.len(), "accent")["info"],
        "Panel accent",
    );
    s.locale = "zh".into();
    assert_eq!(sig(&s)["summary"], zh["summary"]);
    s.update(
        "library.lay",
        &LOCALIZED.replace("面板标题。", "更新后的标题。"),
    );
    assert_eq!(sig(&s)["parameters"][0]["description"], "更新后的标题。");
}
#[test]
fn locale_fallback_and_transitive_aliases() {
    let mut s = localized("zh");
    s.update("library.lay","## @lang fr\n## Un panneau.\n## @param title - Un titre.\nexport function card(title) {return title}");
    assert_eq!(sig(&s)["summary"], "Un panneau.");
    s.update("library.lay","## @lang fr\n## Un panneau.\n## @param title - Un titre.\n## @lang en\n## A panel.\nexport function card(title) {return title}");
    assert_eq!(sig(&s)["summary"], "A panel.");
    assert_eq!(sig(&s)["parameters"][0]["description"], "Un titre.");
    s.update("library.lay", LOCALIZED);
    s.update(
        "wrapper.lay",
        "import {card} from \"./library.lay\"\nexport alias=card",
    );
    s.update(
        FILE,
        "import {alias as panel} from \"./wrapper.lay\"\npanel(",
    );
    assert_eq!(sig(&s)["summary"], "创建带标题的面板。");
    s.locale = "en".into();
    assert_eq!(sig(&s)["summary"], "Create a titled panel.");
}
#[test]
fn localized_warning_ranges_and_conflicting_types() {
    let source = "## @lang zh-CN\n## @param {string} title - 标题\n## @param {string} title - 重复\n## @lang en\n## @param {number} title - Conflict\n## @param typo - Wrong\n## @lang en\n## Duplicate\n## @lang !!!\nfunction card(title) {\ncard(";
    let mut s = service(source, "zh");
    let warnings = |s: &LanguageService| {
        s.diagnostics(FILE)
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == "W_DOC")
            .cloned()
            .collect::<Vec<_>>()
    };
    let zh = warnings(&s);
    assert_eq!(zh.len(), 5);
    assert!(zh.iter().all(|d| {
        d["severity"] == "warning"
            && source[d["from"].as_u64().unwrap() as usize..d["to"].as_u64().unwrap() as usize]
                .starts_with("## @")
    }));
    assert!(
        zh.iter()
            .any(|d| d["message"].as_str().unwrap().contains("类型冲突"))
    );
    assert_eq!(sig(&s)["parameters"][0]["type"], "string");
    s.locale = "en".into();
    let en = warnings(&s);
    assert!(
        en.iter()
            .any(|d| d["message"].as_str().unwrap().contains("Conflicting"))
    );
    let ranges = |ds: &[J]| {
        ds.iter()
            .map(|d| (d["from"].clone(), d["to"].clone(), d["code"].clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ranges(&en), ranges(&zh));
    assert_eq!(sig(&s)["parameters"][0]["type"], "string");
}
#[test]
fn locale_documentation_completion_sections() {
    let source = "## @lang zh-CN\n## @param title - 标题\n## @lang en\n## @param \nfunction card(title,size=(40,25)) {return title}";
    let mut s = service(source, "zh");
    for name in ["title", "size"] {
        assert!(!completion(&s, FILE, source.find("\nfunction").unwrap(), name).is_null())
    }
    let tagged = source.replace("## @param \n", "## @lang e\n");
    s.update(FILE, &tagged);
    let v = s.completions(FILE, tagged.find("\nfunction").unwrap());
    assert_eq!(
        v.as_array()
            .unwrap()
            .iter()
            .map(|c| c["label"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["zh-CN", "en"]
    );
    s.update(FILE, "## \nfunction card(title) {return title}");
    has(
        &completion(&s, FILE, 3, "双语文档模板")["apply"],
        "@lang en",
    );
}
#[test]
fn static_lcss_and_literal_types_match_runtime_boundaries() {
    for (source, needle) in [
        ("text {font-size:12}", "font_size"),
        ("text {nonsense:1pt}", "nonsense"),
        ("text {color:red !important}", "!important"),
        ("plot::bad {color:red}", "bad"),
    ] {
        let mut s = service("", "en");
        s.update("theme.lcss", source);
        let d = s.diagnostics("theme.lcss");
        assert_eq!(d[0]["code"], "E_LCSS", "{source}: {d}");
        has(&d[0]["message"], needle);
    }
    let source = "# 😀 中文\nstyle { text { nonsense:1pt; } }";
    let s = service(source, "en");
    let d = s.diagnostics(FILE);
    assert_eq!(d[0]["code"], "E_LCSS");
    assert_eq!(d[0]["from"], source.find("nonsense").unwrap());
    for (source, code) in [
        ("page=canvas(size=(10deg,20))", "E_UNIT"),
        ("t=text(font_family=[\"DejaVu Sans\",3])", "E_TYPE"),
        ("r=rect(border_width=true)", "E_TYPE"),
    ] {
        let d = service(source, "en").diagnostics(FILE);
        assert_eq!(d[0]["code"], code, "{source}: {d}");
    }
    // Valid text, comments, variables and imports require no disk or font access.
    let mut s = service("", "en");
    s.update("theme.lcss","@import './not-on-disk.lcss';\ntext { --size: 12; font-size:var(--size); font-family:'a}b'; color: red; } /* } */");
    assert_eq!(s.diagnostics("theme.lcss"), serde_json::json!([]));
}
#[test]
fn units_classes_and_factory_return_type_completion() {
    let source = "page=canvas(size=(40,30))\nt=text(font_size=";
    let mut s = service(source, "en");
    for name in ["mm", "cm", "in", "inch", "pt", "px", "auto", "page"] {
        assert!(
            !completion(&s, FILE, source.len(), name).is_null(),
            "{name}"
        );
    }
    let source = "style { .title {color:red;} }\nt=text(class=\"ti";
    s.update(FILE, source);
    assert_eq!(
        completion(&s, FILE, source.len(), "title")["apply"],
        "title"
    );
    s.update("external.lcss", ".external {color:red}");
    assert!(!completion(&s, FILE, source.len(), "external").is_null());
    let source = "## @returns {plot} A plot.\nfunction makeplot() {return plot(size=(20,20))}\np=makeplot()\np.";
    s.update(FILE, source);
    assert!(!completion(&s, FILE, source.len(), "line").is_null());
    let source = source.to_string() + "line(";
    s.update(FILE, &source);
    assert!(sig(&s)["label"].as_str().unwrap().starts_with("plot.line("));
    s.update("factory.lay","## @returns {polar_plot} A polar plot.\nexport function create() {return polar_plot()}\nexport factory=create");
    let source = "import {factory as renamed} from './factory.lay'\np=renamed()\np.";
    s.update(FILE, source);
    assert!(!completion(&s, FILE, source.len(), "line").is_null());
}
