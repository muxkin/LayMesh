use laymesh_language::LanguageService;
#[test]
fn static_colors_do_not_execute_or_recolor_text() {
    let mut s = LanguageService::new("en");
    let text = "# 😀 #fff\naccent = hsv(120deg, 1, 1, alpha=0.5)\npage=canvas(size=(20,20),background=\"#fff8\")\npage.add(text(content=\"#abcdef\",color=oklch(0.7,0.15,200deg)))\nother=rgb(dynamic(),0,0)";
    s.update("/main.lay", text);
    let c = s.document_colors("/main.lay");
    let c = c.as_array().unwrap();
    assert_eq!(c.len(), 3, "{c:?}");
    assert!(c.iter().any(|c| c["space"] == "hsv"));
    assert!(
        c.iter()
            .all(|c| c["from"].as_u64().unwrap() > text.find("accent").unwrap() as u64)
    );
}
#[test]
fn presentations_preserve_context_and_quotes() {
    let mut s = LanguageService::new("en");
    for (uri, text) in [
        ("/main.lay", "page=canvas(background='#ffffff90')"),
        (
            "/main.lcss",
            "rect {fill: oklch(0.7 0.15 200 / 0.6); --accent:#abc8;}",
        ),
    ] {
        s.update(uri, text);
        let colors = s.document_colors(uri);
        assert!(!colors.as_array().unwrap().is_empty());
        let c = &colors[0];
        let from = c["from"].as_u64().unwrap() as usize;
        let to = c["to"].as_u64().unwrap() as usize;
        let p = s.color_presentations(uri, from, to, [1., 0., 0., 0.5]);
        assert_eq!(p.as_array().unwrap().len(), 4);
        for v in p.as_array().unwrap() {
            assert_eq!(v["from"], from);
            assert_eq!(v["to"], to);
            if uri.ends_with(".lay") {
                assert!(v["text"].as_str().unwrap().starts_with('\''));
            } else {
                assert!(!v["text"].as_str().unwrap().starts_with('"'));
            }
        }
    }
}
#[test]
fn retired_arrow_has_safe_source_fix() {
    let mut s = LanguageService::new("en");
    let text = "page=canvas(size=(20,20))\npage.add(arrow(dx=10,dy=0,line_color=\"#fff\"))";
    s.update("/main.lay", text);
    let d = s.diagnostics("/main.lay");
    let migration = d
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["code"] == "E_API_MIGRATION")
        .unwrap();
    assert!(
        migration["replacement"]
            .as_str()
            .unwrap()
            .starts_with("line(end_head=head(),")
    );
    let migrated = laymesh_core::migration::migrate_arrows(text);
    assert!(
        laymesh_core::engine::compile_source(&migrated, "/main.lay", Default::default()).is_ok()
    );
    assert_eq!(
        laymesh_core::migration::migrate_arrows(
            "function arrow(dx,dy){return line(dx=dx,dy=dy)}\narrow(10,0)"
        ),
        "function arrow(dx,dy){return line(dx=dx,dy=dy)}\narrow(10,0)"
    );
}

#[test]
fn palettes_and_gradient_stops_have_static_swatches() {
    let mut s = LanguageService::new("en");
    s.update("/main.lay","colors=[\"#fff8\",rgb(255,0,0),hsv(120deg,1,1)]\nfill=linear_gradient(stops=[(0,\"#0004\"),(1,oklch(0.7,0.15,200deg))])");
    assert_eq!(s.document_colors("/main.lay").as_array().unwrap().len(), 5);
    s.update(
        "/main.lay",
        "a=rgb(20deg,0,0)\nfunction hsv(h,s,v){return h}\nb=hsv(120,1,1)",
    );
    assert!(
        s.document_colors("/main.lay")
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_color_channels_and_zero_line_direction_have_static_diagnostics() {
    let mut s = LanguageService::new("en");
    for (source, expected) in [
        ("c=rgb(300,0,0)", "E_COLOR"),
        ("c=hsv(120,2,1)", "E_COLOR"),
        ("c=oklch(0.5,-0.1,0)", "E_COLOR"),
        ("r=rect(size=(2,2),fill=\"#xyz\")", "E_COLOR"),
        ("a=line(length=0)", "E_ANCHOR_DIRECTION"),
        ("a=line(length=10,dx=10,dy=0)", "E_ARG"),
    ] {
        s.update("/main.lay", source);
        assert!(
            s.diagnostics("/main.lay")
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == expected),
            "{source}: {}",
            s.diagnostics("/main.lay")
        );
    }
}

#[test]
fn quoted_head_colors_in_lcss_have_one_quote_preserving_range() {
    let mut s = LanguageService::new("en");
    for (uri, source) in [
        ("/head.lcss", ".pointer {end-head:head(fill=\"#fff8\");}"),
        (
            "/head.lay",
            "style {.pointer {end-head:head(fill=\"hsv(120 1 1 / 0.5)\");}}",
        ),
    ] {
        s.update(uri, source);
        let colors = s.document_colors(uri);
        assert_eq!(colors.as_array().unwrap().len(), 1);
        let c = &colors[0];
        assert_eq!(c["format"], "string");
        let options = s.color_presentations(
            uri,
            c["from"].as_u64().unwrap() as usize,
            c["to"].as_u64().unwrap() as usize,
            [1., 0., 0., 0.5],
        );
        assert!(options[2]["text"].as_str().unwrap().starts_with('"'));
    }
}
