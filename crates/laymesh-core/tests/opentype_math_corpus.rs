use kurbo::Shape;
use laymesh_core::{
    Loc,
    model::jnum,
    text::{FontSystem, formula},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn cases() -> Vec<(String, usize, String)> {
    let mut cases = vec![];
    let mut seen = BTreeSet::new();
    for (suite, text) in [
        (
            "golden",
            include_str!("../../../tests/math/ratex-0.1.14/golden.txt"),
        ),
        (
            "parser",
            include_str!("../../../tests/math/ratex-0.1.14/parser.txt"),
        ),
        (
            "layout",
            include_str!("../../../tests/math/ratex-0.1.14/layout.txt"),
        ),
        (
            "proofs",
            include_str!("../../../tests/math/ratex-0.1.14/proofs.txt"),
        ),
    ] {
        for (line, source) in text.lines().enumerate() {
            let source = source.trim();
            if source.is_empty() || source.starts_with('#') || !seen.insert(source.to_string()) {
                continue;
            }
            cases.push((suite.to_string(), line + 1, source.to_string()));
        }
    }
    for (suite, text) in [
        (
            "chemistry",
            include_str!("../../../tests/math/ratex-0.1.14/chemistry.json"),
        ),
        (
            "physics",
            include_str!("../../../tests/math/ratex-0.1.14/physics.json"),
        ),
    ] {
        let data: Value = serde_json::from_str(text).unwrap();
        for (line, source) in data["formulas"].as_array().unwrap().iter().enumerate() {
            let source = source.as_str().unwrap();
            let source = source
                .strip_prefix('$')
                .and_then(|s| s.strip_suffix('$'))
                .unwrap_or(source);
            // Keep category coverage even if a case also occurs in the math corpus.
            cases.push((suite.to_string(), line + 1, source.to_string()));
        }
    }
    let domain: Value = serde_json::from_str(include_str!(
        "../../../experiments/opentype-math/domain-cases.json"
    ))
    .unwrap();
    let mut number = 0;
    for rows in domain.as_object().unwrap().values() {
        for row in rows.as_array().unwrap() {
            number += 1;
            cases.push((
                "domain".into(),
                number,
                row[1].as_str().unwrap().to_string(),
            ));
        }
    }
    cases
}
#[test]
fn upstream_corpus() {
    let mut fonts = FontSystem::new(false);
    let faces = [
        (
            "latinmodern",
            include_bytes!("../../../tests/fonts/math/latinmodern-math.otf").as_slice(),
        ),
        (
            "stix",
            include_bytes!("../../../tests/fonts/math/STIX2Math.otf").as_slice(),
        ),
        (
            "xits",
            include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf").as_slice(),
        ),
    ];
    for (name, bytes) in faces {
        fonts.register_font(&format!("/{name}.otf"), bytes.to_vec());
    }
    let expected: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/math/ratex-0.1.14/expected-errors.json"
    ))
    .unwrap();
    let expected: BTreeMap<_, _> = expected
        .iter()
        .map(|v| {
            (
                (
                    v["suite"].as_str().unwrap().to_string(),
                    v["line"].as_u64().unwrap() as usize,
                    v["font"].as_str().unwrap().to_string(),
                ),
                v,
            )
        })
        .collect();
    let mut failures = vec![];
    let mut report = vec![];
    for (suite, line, source) in cases() {
        for font in ["ratex-katex", "latinmodern", "stix", "xits"] {
            for style in ["inline", "display"] {
                let request = if font == "ratex-katex" {
                    font.into()
                } else {
                    format!("/{font}.otf")
                };
                let result = formula(
                    &json!({"source":source,"math_font":request,"font_size":5.,"style":style,"math_text_fallback":false}),
                    &mut fonts,
                    &mut vec![],
                    "/corpus.lay",
                    Loc::default(),
                );
                let row = match result {
                    Err(e) => {
                        json!({"suite":suite,"line":line,"source":source,"font":font,"style":style,"error":e.message,"code":e.code})
                    }
                    Ok(node) => {
                        let (w, h) = (jnum(&node, "width", 0.), jnum(&node, "height", 0.));
                        let mut errors = vec![];
                        if !w.is_finite() || !h.is_finite() || w < 0. || h < 0. {
                            errors.push("non-finite or negative dimensions".to_string());
                        }
                        for item in node["items"].as_array().unwrap() {
                            if item["kind"] == "path" {
                                let p =
                                    kurbo::BezPath::from_svg(item["d"].as_str().unwrap()).unwrap();
                                let r = p.bounding_box();
                                if r.x0 < -1e-6
                                    || r.y0 < -1e-6
                                    || r.x1 > w + 1e-6
                                    || r.y1 > h + 1e-6
                                {
                                    errors.push(format!("outline {r:?} exceeds {w}x{h}"));
                                }
                            }
                        }
                        json!({"suite":suite,"line":line,"source":source,"font":font,"style":style,"width":w,"height":h,"bounds_errors":errors})
                    }
                };
                let key = (suite.clone(), line, font.to_string());
                match (expected.get(&key), row.get("error")) {
                    (Some(e), Some(message))
                        if e["error"] == *message && e["code"] == row["code"] => {}
                    (None, None) => {}
                    _ => failures.push(format!(
                        "{suite}:{line} {font} {style}: unexpected outcome {row}"
                    )),
                }
                if font != "ratex-katex"
                    && row.get("error").is_none()
                    && !row["bounds_errors"].as_array().unwrap().is_empty()
                {
                    failures.push(format!(
                        "{suite}:{line} {font} {style}: {}",
                        row["bounds_errors"]
                    ));
                }
                report.push(row);
            }
        }
    }
    if let Ok(path) = std::env::var("LAYMESH_MATH_AUDIT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} unexpected failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
