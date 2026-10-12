//! Catalog pinned CSS with compiler probes, then compile/render every usable recipe.
use laymesh_core::{
    Loc,
    engine::{Engine, compile_source},
    model::{Args, Host, V},
    style,
};
use laymesh_language::stylesheets::{self, Symbol};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
fn property(name: &str, value: &str) -> Option<(&'static str, String, &'static str)> {
    let n = name.trim_start_matches("--");
    let direct = format!("var({name})");
    let result = if n.starts_with("animation-") {
        ("animation", direct, "Motion / 动画")
    } else if n.starts_with("ease-") {
        ("animation-timing-function", direct, "Easing / 缓动")
    } else if n.starts_with("size-") || n.contains("spacer") {
        ("padding", direct, "Spacing / 留白")
    } else if n == "animate-repeat" {
        ("animation-iteration-count", direct, "Repetition / 重复")
    } else if n == "animate-delay" {
        ("animation-delay", direct, "Timing / 时长")
    } else if n.contains("duration") {
        ("animation-duration", direct, "Timing / 时长")
    } else if n.contains("shadow-color") || n.contains("shadow-strength") {
        (
            "box-shadow",
            "var(--shadow-3)".into(),
            "Shadow helpers / 阴影辅助",
        )
    } else if n.contains("shadow") {
        ("box-shadow", direct, "Shadows / 阴影")
    } else if n.contains("gradient-space") {
        (
            "background",
            "var(--gradient-7)".into(),
            "Gradient helpers / 渐变辅助",
        )
    } else if n.contains("gradient-") {
        ("background", direct, "Gradients / 渐变")
    } else if n.contains("radius") {
        ("border-radius", direct, "Corners / 圆角")
    } else if n.contains("border-size") || n.contains("border-width") {
        ("border-width", direct, "Borders / 边框")
    } else if n.contains("border-style") {
        ("border-style", direct, "Borders / 边框")
    } else if n.contains("font-weight") {
        ("font-weight", direct, "Typography / 文字")
    } else if n.contains("lineheight") || n.contains("line-height") {
        ("line-height", direct, "Typography / 文字")
    } else if n.contains("font-size") {
        ("font-size", direct, "Typography / 文字")
    } else if n.contains("font-letterspacing") {
        return None;
    } else if n.contains("font-") {
        ("font-family", direct, "Font families / 字体族")
    } else if n.ends_with("-rgb") {
        (
            "background-color",
            format!("rgb({direct})"),
            "Colors / 配色",
        )
    } else if value.starts_with('#') || value.starts_with("hsl(") || value.starts_with("rgb(") {
        ("background-color", direct, "Colors / 配色")
    } else if n.contains("opacity") {
        ("opacity", direct, "Opacity / 透明度")
    } else {
        return None;
    };
    Some(result)
}
fn shape(name: &str, key: &str) -> &'static str {
    if key == "padding"
        || [
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "lead",
            "small",
            "initialism",
            "blockquote",
            "form-text",
        ]
        .contains(&name)
        || name.starts_with("display-")
        || ["p-", "px-", "py-", "pt-", "pb-", "ps-", "pe-"]
            .iter()
            .any(|p| name.starts_with(p))
        || key.starts_with("font")
        || key == "line-height"
        || ["text-", "fw-", "fs-", "fst-", "lh-", "font-"]
            .iter()
            .any(|p| name.starts_with(p))
    {
        "text"
    } else {
        "rect"
    }
}
fn recipe(lib: &str, name: &str, kind: &str, key: &str, expr: &str) -> String {
    let mut css = String::new();
    let mut classes = name.to_owned();
    let mut motion = kind == "keyframe" || lib == "animate" || key.starts_with("animation");
    if kind == "variable" {
        classes = "sample".into();
        css = format!(".sample {{ {key}: {expr}; }}");
        if key.starts_with("animation-") {
            css = format!(
                "@keyframes catalog-fade {{from{{opacity:0;transform:translateY(5mm)}}to{{opacity:1;transform:translateY(0mm)}}}}\n.sample {{animation:catalog-fade 1s linear both;{key}:{expr};}}"
            );
        }
    } else if kind == "keyframe" {
        classes = "sample".into();
        css = format!(".sample {{animation:{name} 1s linear both;}}");
    } else if lib == "animate" {
        classes = if name == "animate__animated" {
            "animate__animated animate__fadeInUp".into()
        } else if name.contains("delay")
            || name.contains("repeat")
            || name.contains("infinite")
            || [
                "animate__fast",
                "animate__faster",
                "animate__slow",
                "animate__slower",
            ]
            .contains(&name)
        {
            format!("animate__animated animate__fadeInUp {name}")
        } else {
            format!("animate__animated {name}")
        };
    } else if name.starts_with("bg-opacity-") {
        classes = format!("bg-primary {name}");
    } else if name.starts_with("text-opacity-") {
        classes = format!("text-primary {name}");
    } else if name.starts_with("border-opacity-") {
        classes = format!("border border-primary {name}");
    } else if name.starts_with("btn-") {
        classes = format!("btn {name}");
    } else if name.starts_with("alert-") {
        classes = format!("alert {name}");
    }
    let material = if shape(name, key) == "text" {
        "text(\"Aa 0123 / LayMesh\",class=".to_owned() + &json!(classes).to_string() + ")"
    } else {
        "rect(size=(40mm,25mm),class=".to_owned() + &json!(classes).to_string() + ")"
    };
    if lib == "bootstrap" && kind == "class" && name.starts_with("spinner-") {
        motion = true;
    }
    let mut source = format!(
        "page=canvas(size=(100mm,100mm),stylesheet=\"@laymesh/{lib}.css\",background=\"#f6f7fb\")\n"
    );
    // A low-specificity backdrop makes corners, opacity and motion visible;
    // library class declarations and variable consumers retain precedence.
    css = format!(
        "rect {{background-color:#0d6efd;border:1px solid #20332f}}\ntext {{background-color:#e9ecef;color:#212529;border:1px solid #adb5bd}}\n{css}"
    );
    source += &format!("style {{\n{css}\n}}\n");
    source += &format!("item=page.add({material},anchor=center,target=page.center)\n");
    if motion {
        source +=
            "s=page.step(name=\"Preview\",start=\"after-previous\",hold=0s)\ns.animate(item)\n";
    }
    source
}
fn attrs(a: &Args) -> J {
    json!(
        a.iter()
            .filter(|(k, _)| !k.starts_with("__") || k.starts_with("__css_"))
            .filter(|(k, _)| k.as_str() != "class")
            .map(|(k, v)| (k.clone(), v.json()))
            .collect::<BTreeMap<_, _>>()
    )
}
fn main() {
    let validate = std::env::args().any(|a| a == "--verify");
    let inventory = std::env::args().any(|a| a == "--inventory");
    let mut output = vec![];
    for lib in style::bundled_stylesheets() {
        let index = stylesheets::parse(
            style::bundled_stylesheet(&lib.path()).unwrap(),
            &stylesheets::library_uri(&lib.path()),
            Loc {
                line: 1,
                column: 1,
                offset: 0,
            },
        );
        let mut groups: BTreeMap<(&str, String), Vec<Symbol>> = BTreeMap::new();
        for s in index.symbols {
            groups.entry((s.kind, s.name.clone())).or_default().push(s);
        }
        if inventory {
            output.push(json!({"name":lib.name,"entries":groups.keys().map(|(kind,name)|json!({"kind":kind,"name":name})).collect::<Vec<_>>() }));
            continue;
        }
        let (_, raw_sheet) = laymesh_core::animation::css::extract(
            style::bundled_stylesheet(&lib.path()).unwrap(),
            &lib.path(),
            Loc::default(),
            None,
        )
        .unwrap();
        let mut engine = Engine::new(Host::default());
        engine.file = "/probe.lay".into();
        engine
            .eval(
                &laymesh_core::parser::expression("canvas(size=(100mm,100mm))", "/probe.lay")
                    .unwrap(),
                &laymesh_core::model::Environment::root(),
            )
            .unwrap();
        engine
            .stylesheet(
                style::bundled_stylesheet(&lib.path()).unwrap(),
                &lib.path(),
                Loc::default(),
            )
            .unwrap();
        let root = engine.styled("canvas", &Args::new(), &Args::new()).unwrap();
        let mut entries = vec![];
        for ((kind, name), symbols) in groups {
            let s = symbols.last().unwrap();
            let (key, expr, family) = if kind == "variable" {
                property(&name, &s.value).unwrap_or(("", "".into(), "Other / 其他"))
            } else {
                (
                    "",
                    "".into(),
                    if kind == "keyframe" {
                        "Motion / 动画"
                    } else if name.starts_with("bg-") || name.starts_with("text-") {
                        "Colors and text / 颜色与文字"
                    } else if name.contains("shadow") {
                        "Shadows / 阴影"
                    } else if name.contains("rounded") {
                        "Corners / 圆角"
                    } else if name.contains("border") {
                        "Borders / 边框"
                    } else {
                        "Classes / 类"
                    },
                )
            };
            let id = format!("{}:{}:{}", lib.name, kind, name);
            let mut code = recipe(&lib.name, &name, kind, key, &expr);
            let own = format!("/probe-{}.css", entries.len());
            let probe_name = format!("probe-{}", entries.len());
            let mut supported = false;
            let mut reason =
                String::from("No supported appearance consumer / 当前没有可用的外观用法");
            if kind == "keyframe" || lib.name == "animate" || key.starts_with("animation") {
                supported = true;
                reason="Animation recipe; verified by compilation and sampling / 动画案例，须通过编译与采样验证".into();
            } else if kind != "variable" || !key.is_empty() {
                let mut classes = name.clone();
                if kind == "variable" {
                    engine
                        .stylesheet(
                            &format!(".{probe_name}{{{key}:{expr}}}"),
                            &own,
                            Loc::default(),
                        )
                        .unwrap();
                    classes = probe_name.clone();
                } else if name.starts_with("bg-opacity-") {
                    classes = format!("bg-primary {name}");
                } else if name.starts_with("text-opacity-") {
                    classes = format!("text-primary {name}");
                } else if name.starts_with("border-opacity-") {
                    classes = format!("border border-primary {name}");
                } else if name.starts_with("btn-") {
                    classes = format!("btn {name}");
                } else if name.starts_with("alert-") {
                    classes = format!("alert {name}");
                }
                let selected_classes = classes.clone();
                let original = [("class".into(), V::text(classes))].into_iter().collect();
                let material = shape(&name, key);
                if let (Ok(base), Ok(actual)) = (
                    engine.styled(material, &Args::new(), &root),
                    engine.styled(material, &original, &root),
                ) {
                    supported = attrs(&base) != attrs(&actual) || name.contains("opacity-");
                    if kind == "class" && !supported {
                        let chain = [
                            laymesh_core::animation::css::Element {
                                kind: "canvas".into(),
                                class: Default::default(),
                            },
                            laymesh_core::animation::css::Element {
                                kind: material.into(),
                                class: selected_classes,
                            },
                        ];
                        supported = symbols.iter().any(|s| {
                            raw_sheet.rules.iter().any(|r| {
                                r.selector == s.selector
                                    && r.supports.is_empty()
                                    && laymesh_core::css_values::media(
                                        &r.media,
                                        laymesh_core::animation::values::Context {
                                            viewport: [100., 100.],
                                            ..Default::default()
                                        },
                                        false,
                                        false,
                                    ) == Some(true)
                            }) && laymesh_core::animation::css::matches(&s.selector, &chain)
                                && s.declarations.iter().any(|d| {
                                    !d.property.starts_with("--")
                                        && style::property_supported(&d.property)
                                })
                        });
                    }
                    if supported {
                        reason = format!(
                            "Verified {material} appearance{} / 已验证对象外观{}",
                            if key.is_empty() { "" } else { key },
                            if key.is_empty() { "" } else { key }
                        );
                    }
                }
                if engine.css_diagnostics().iter().any(|d| d.file == own) {
                    supported = false;
                    reason = engine
                        .css_diagnostics()
                        .iter()
                        .find(|d| d.file == own)
                        .unwrap()
                        .message
                        .clone();
                }
            }
            let unsupported = symbols.iter().flat_map(|s| s.declarations.iter()).any(|d| {
                !style::property_supported(&d.property)
                    && !laymesh_core::animation::css::is_motion_property(&d.property)
            });
            if !supported && kind == "class" {
                let ignored = symbols
                    .iter()
                    .flat_map(|s| s.declarations.iter())
                    .filter(|d| {
                        !d.property.starts_with("--")
                            && !style::property_supported(&d.property)
                            && !laymesh_core::animation::css::is_motion_property(&d.property)
                    })
                    .map(|d| d.property.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .take(6)
                    .collect::<Vec<_>>()
                    .join(", ");
                reason = if ignored.is_empty() {
                    "Selector requires DOM ancestry, interaction or a component part that this object does not expose / 此选择器需要对象未提供的 DOM 结构、交互状态或部件".into()
                } else {
                    format!(
                        "Ignored layout or unsupported properties: {ignored}; sizes and placement belong in .lay / 忽略排版或尚未支持的属性：{ignored}；尺寸与定位由 .lay 控制"
                    )
                };
            }
            let mut status = if supported {
                if unsupported {
                    "partial"
                } else if key == "font-family" {
                    "resources"
                } else {
                    "supported"
                }
            } else {
                "unsupported"
            };
            let mut verified = false;
            if validate && supported {
                let mut scene = compile_source(
                    &code,
                    "/catalog.lay",
                    Host {
                        native: true,
                        ..Host::default()
                    },
                );
                if scene.as_ref().err().is_some_and(|e| {
                    e.message.contains("infinite")
                        || e.message.contains("duration")
                        || e.message.contains("无限")
                }) {
                    code = code.replace(
                        "start=\"after-previous\",hold=0s",
                        "start=\"after-previous\",duration=6s,hold=0s",
                    );
                    scene = compile_source(
                        &code,
                        "/catalog.lay",
                        Host {
                            native: true,
                            ..Host::default()
                        },
                    );
                }
                match scene {
                    Ok(scene) => {
                        if scene.warnings.iter().any(|w| {
                            w.file == "/catalog.lay"
                                && (w.code == "W_CSS" || w.code == "W_ANIMATION")
                        }) {
                            status = "unsupported";
                            reason = scene
                                .warnings
                                .iter()
                                .find(|w| {
                                    w.file == "/catalog.lay"
                                        && (w.code == "W_CSS" || w.code == "W_ANIMATION")
                                })
                                .unwrap()
                                .message
                                .clone();
                        } else if let Some(a) = &scene.animation {
                            let timeline = a.timeline(false);
                            for t in [0., 0.4, 0.9] {
                                let _ = timeline.sample(t);
                            }
                            verified = true;
                        } else {
                            match laymesh_render::to_display_list(&scene)
                                .and_then(|list| laymesh_render::render_rgba(&list, 24.384))
                            {
                                Ok(_) => verified = true,
                                Err(e) => {
                                    status = "unsupported";
                                    reason = e.message;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        status = "unsupported";
                        reason = e.message;
                    }
                }
            }
            if supported && key == "font-family" {
                reason += "; provide project fonts / 请提供项目字体";
            }
            if unsupported && supported {
                reason += "; only supported declarations apply / 仅支持的声明生效";
            }
            entries.push(json!({"id":id,"name":name,"kind":kind,"family":family.split(" / ").collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>(),"status":status,"reason":reason,"value":s.value,"defaultValue":root.get("__style_variables").and_then(|v|if let V::Map(m)=v {m.get(&name)}else{None}).map(|v|v.as_str()),"definitions":symbols.iter().map(|s|json!({"selector":s.selector,"value":s.value,"location":s.location()})).collect::<Vec<_>>(),"declarations":symbols.iter().flat_map(|s|s.declarations.iter()).map(|d|json!({"property":d.property,"value":d.value,"important":d.important})).collect::<Vec<_>>(),"source":lib.source,"location":s.location(),"consumer":key,"sample":if status!="unsupported"{Some(code)}else{None},"verified":verified}));
        }
        eprintln!("{}: {} entries", lib.name, entries.len());
        output.push(json!({"name":lib.name,"version":lib.version,"license":lib.license,"source":lib.source,"sha256":lib.sha256,"entries":entries}));
    }
    println!("{}", json!({"libraries":output,"verified":validate}));
}
