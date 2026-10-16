use laymesh_language::LanguageService;
#[test]
fn css_completion_hover_colors_and_warning_recovery_share_supported_metadata() {
    let mut s = LanguageService::new("en");
    let source = ".card { ";
    s.update("/theme.lcss", source);
    let completions = s.completions("/theme.lcss", source.len());
    let options = completions.as_array().unwrap();
    for key in [
        "box-shadow",
        "box-shadow-blur",
        "box-shadow-offset",
        "background-color",
        "text-shadow",
    ] {
        assert!(options.iter().any(|v| v["label"] == key), "{key}");
    }
    for key in ["size", "offset", "src", "width"] {
        assert!(!options.iter().any(|v| v["label"] == key), "{key}");
    }
    s.update("/theme.lcss", ".a{box-shadow-blur:2mm}");
    assert!(s.hover("/theme.lcss", 8).to_string().contains("draft"));
    let source = ".a{effects:[shadow(";
    s.update("/theme.lcss", source);
    assert!(
        s.completions("/theme.lcss", source.len())
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == "blur")
    );
    s.update("/theme.lcss",".a{color:hsl(120deg 100% 25% / 50%);box-shadow:0 1mm 2mm rebeccapurple;background:linear-gradient(red,blue)}");
    let colors = s.document_colors("/theme.lcss");
    assert_eq!(colors.as_array().unwrap().len(), 4, "{colors}");
    s.update("/theme.lcss", ".a{width:2mm;bad declaration;color:red}");
    let diagnostics = s.diagnostics("/theme.lcss");
    assert_eq!(diagnostics.as_array().unwrap().len(), 1, "{diagnostics}");
    assert_eq!(diagnostics[0]["severity"], "warning");
}
#[test]
fn builtin_library_imports_complete_full_paths_and_hover_pinned_versions() {
    let mut s = LanguageService::new("en");
    for (uri, source) in [
        ("/main.lay", "p=canvas(stylesheet=\"@laymesh/bo"),
        (
            "/main.lay",
            "p=canvas(stylesheet=[\"@laymesh/open-props.css\",'@laymesh/an",
        ),
        ("/main.lay", "style { @import \"@laymesh/an"),
        ("/theme.lcss", "@import url('@laymesh/op"),
        ("/theme.css", "@import \"@laymesh/bo"),
    ] {
        s.update(uri, source);
        let completions = s.completions(uri, source.len());
        let options = completions.as_array().unwrap();
        assert_eq!(
            options.len(),
            laymesh_core::style::bundled_stylesheets().len(),
            "{source}: {completions}"
        );
        for library in laymesh_core::style::bundled_stylesheets() {
            let option = options
                .iter()
                .find(|v| v["label"] == library.path())
                .unwrap();
            let from = option["from"].as_u64().unwrap() as usize;
            let to = option["to"].as_u64().unwrap() as usize;
            let completed = format!(
                "{}{}{}",
                &source[..from],
                option["apply"].as_str().unwrap(),
                &source[to..]
            );
            assert!(
                completed.contains(&format!("\"{}", library.path()))
                    || completed.contains(&format!("'{}", library.path())),
                "{completed}"
            );
            assert!(option["info"].as_str().unwrap().contains(&library.version));
            assert!(laymesh_core::style::bundled_stylesheet(&library.path()).is_some());
        }
    }
    for (uri, source) in [
        ("/main.lay", "p=canvas(stylesheet=\"@laymesh/animate.css\")"),
        ("/theme.lcss", "@import url('@laymesh/bootstrap.css');"),
    ] {
        s.update(uri, source);
        let offset = source.find("@laymesh").unwrap() + 12;
        let hover = s.hover(uri, offset);
        let info = hover["contents"].as_str().unwrap();
        assert!(
            info.contains(if uri.ends_with(".lay") {
                "4.1.1"
            } else {
                "5.3.8"
            }),
            "{hover}"
        );
        assert!(info.contains("MIT") && info.contains("Explicitly import"));
    }
    for source in ["p=canvas(stylesheet=", "p=canvas(stylesheet=["] {
        s.update("/main.lay", source);
        let options = s.completions("/main.lay", source.len());
        assert!(
            options
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v["apply"].as_str().unwrap().starts_with('"'))
        );
    }
    let source = "text(\"@laymesh/bo";
    s.update("/main.lay", source);
    assert!(
        !s.completions("/main.lay", source.len())
            .to_string()
            .contains("bootstrap.css")
    );
    s.update("/main.lay", "# stylesheet=\"@laymesh/animate.css\"");
    assert!(!s.hover("/main.lay", 28).to_string().contains("4.1.1"));
}

#[test]
fn imported_symbols_follow_dependencies_and_refresh_unsaved_css() {
    let mut s = LanguageService::new("en");
    s.update("/other.css", ".unrelated{--unrelated:red}");
    s.update(
        "/vendor.css",
        "@import 'nested.css'; .card{color:red} .card{border-radius:2mm}",
    );
    s.update(
        "/nested.css",
        ".nested{--accent:blue} @keyframes wave{to{opacity:1}}",
    );
    let prefix = "p=canvas(stylesheet=\"vendor.css\")\np.add(rect(size=(10,10),class=\"";
    s.update("/main.lay", prefix);
    let names = s.completions("/main.lay", prefix.len());
    assert!(names.to_string().contains("card") && names.to_string().contains("nested"));
    assert!(!names.to_string().contains("unrelated"));
    let source = "p=canvas(stylesheet=\"vendor.css\")\nstyle{.a{color:var(--accent)}}\np.add(rect(class=\"card\"))";
    s.update("/main.lay", source);
    let defs = s.definition("/main.lay", source.rfind("card").unwrap() + 2);
    assert_eq!(defs.as_array().unwrap().len(), 2);
    assert!(
        s.hover("/main.lay", source.find("--accent").unwrap() + 4)
            .to_string()
            .contains("blue")
    );
    s.update("/nested.css", ".changed{--accent:green}");
    assert!(
        s.hover("/main.lay", source.find("--accent").unwrap() + 4)
            .to_string()
            .contains("green")
    );
    s.remove("/nested.css");
    assert!(
        s.diagnostics("/main.lay")
            .to_string()
            .contains("nested.css")
    );
    s.update("/nested.css", "@import 'vendor.css';");
    assert!(s.diagnostics("/main.lay").to_string().contains("E_LCSS"));
}
#[test]
fn bundled_symbols_and_readonly_definitions_do_not_require_files() {
    let mut s = LanguageService::new("en");
    let source = "p=canvas(stylesheet=[\"@laymesh/bootstrap.css\",\"@laymesh/open-props.css\",\"@laymesh/animate.css\"])\nstyle{.card{color:var(--blue-7)}}\np.add(rect(class=\"bg-primary\"))";
    s.update("/main.lay", source);
    let at = source.find("--blue-7").unwrap() + 5;
    assert!(s.hover("/main.lay", at).to_string().contains("1.7.23"));
    let d = s.definition("/main.lay", at);
    assert!(d["uri"].as_str().unwrap().starts_with("laymesh-library:/"));
    assert!(
        s.query("stylesheetContent", d["uri"].as_str().unwrap(), 0)["text"]
            .as_str()
            .unwrap()
            .contains("--blue-7")
    );
    let prefix = source[..source.find("\nstyle").unwrap()].to_owned()
        + "\ns=p.step()\ns.animate(card,class=\"animate__fa";
    s.update("/main.lay", &prefix);
    assert!(
        s.completions("/main.lay", prefix.len())
            .to_string()
            .contains("animate__fadeInUp")
    );
    s.update(
        "/main.lay",
        "p=canvas()\ns=p.step()\ns.animate(card,class=\"animate__fa",
    );
    assert!(
        !s.completions("/main.lay", s.documents["/main.lay"].len())
            .to_string()
            .contains("animate__fadeInUp")
    );
}
#[test]
fn local_paths_complete_ranges_and_constant_imports() {
    let mut s = LanguageService::new("en");
    s.update("/project/主题.css", ".a{}");
    let source = "p=canvas(stylesheet=\"./xx.css\")";
    s.update("/project/main.lay", source);
    let at = source.find("xx").unwrap() + 1;
    let candidates = s.completions("/project/main.lay", at);
    let candidate = candidates
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "./主题.css")
        .unwrap();
    assert_eq!(
        &source[candidate["from"].as_u64().unwrap() as usize
            ..candidate["to"].as_u64().unwrap() as usize],
        "./xx.css"
    );
    s.update(
        "/project/main.lay",
        "styles=[\"主题.css\"]\np=canvas(stylesheet=styles)",
    );
    assert!(
        s.stylesheet_symbols("/project/main.lay")
            .iter()
            .any(|s| s.name == "a")
    );
    s.update("/project/main.lay", "p=canvas(stylesheet=make_theme())");
    assert!(
        !s.diagnostics("/project/main.lay")
            .to_string()
            .contains("E_ASSET")
    );
}

#[test]
fn windows_uri_paths_and_module_styles_stay_in_their_own_scope() {
    let mut service = LanguageService::new("en");
    let entry = "file:///C:/Figures/main.lay";
    let source = "import {card} from \"components.lay\"\npage=canvas(stylesheet=\"themes/ink.css\")\npage.add(rect(size=(20,10),class=\"ink\"))";
    service.update(entry, source);
    service.update(
        "file:///C:/Figures/themes/ink.css",
        "@import '../tokens.css'; .ink {color:var(--ink)}",
    );
    service.update("file:///C:/Figures/tokens.css", ":root {--ink:#123456}");
    service.update("file:///C:/Figures/components.lay","style {.private-card {color:red}}\nexport function card(){return rect(size=(20,10),class=\"private-card\")}");
    let choices = service.completions(entry, source.find("class=\"ink").unwrap() + 9);
    assert!(
        choices
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == "ink")
    );
    assert!(
        !choices
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == "private-card")
    );
    let definition = service.definition(entry, source.rfind("ink").unwrap() + 1);
    assert_eq!(definition["uri"], "file:///C:/Figures/themes/ink.css");
    assert!(service.stylesheet_issues(entry).is_empty());
    let module = "file:///C:/Figures/components.lay";
    let own = service.stylesheet_symbols(module);
    assert!(own.iter().any(|s| s.name == "private-card"));
}

#[test]
fn selectors_index_nested_classes_and_literal_class_attributes_without_inventing_patterns() {
    let source = ":is(.nested,.alternate){color:red}\n[class~=\"tag\"]{border-radius:2mm}\n[class=\"first second\"]{opacity:.5}\n[class*=\"animate__\"]{color:blue}";
    let index = laymesh_language::stylesheets::parse(
        source,
        "/theme.css",
        laymesh_core::Loc {
            offset: 0,
            line: 1,
            column: 1,
        },
    );
    for name in ["nested", "alternate", "tag", "first", "second"] {
        let symbol = index
            .symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("Missing {name}"));
        assert_eq!(&source[symbol.from..symbol.to], name);
    }
    assert!(!index.symbols.iter().any(|s| s.name == "animate__"));
    let mut service = LanguageService::new("en");
    let lay = "page=canvas(stylesheet=\"theme.css\")\npage.add(rect(class=\"tag\"))";
    service.update("/main.lay", lay);
    service.update("/theme.css", source);
    let definition = service.definition("/main.lay", lay.rfind("tag").unwrap() + 1);
    assert_eq!(definition["uri"], "/theme.css");
    assert_eq!(definition["from"], source.find("tag").unwrap());
}
