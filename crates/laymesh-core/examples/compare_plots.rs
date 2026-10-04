//! Compare native plot coordinate behavior with the captured legacy scenes.
//! Font- and formula-dependent automatic margins are reported separately.
use laymesh_core::{engine::Engine, model::Host};
use serde_json::{Value as J, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
const TOL: f64 = 0.001;
fn plots<'a>(nodes: &'a J, out: &mut Vec<&'a J>) {
    if let Some(ns) = nodes.as_array() {
        for n in ns {
            if n.get("plotBounds").is_some() {
                out.push(n);
            }
            plots(&n["children"], out);
        }
    }
}
fn numbers(n: &J) -> Vec<f64> {
    if let Some(a) = n.as_array() {
        a.iter().filter_map(J::as_f64).collect()
    } else if let Some(a) = n.as_object() {
        let mut pairs = a
            .iter()
            .filter_map(|(k, v)| Some((k.parse::<usize>().ok()?, v.as_f64()?)))
            .collect::<Vec<_>>();
        pairs.sort_by_key(|v| v.0);
        pairs.into_iter().map(|v| v.1).collect()
    } else {
        vec![]
    }
}
fn numeric(a: &J, b: &J, path: &str, out: &mut Vec<J>) {
    match (a, b) {
        (J::Number(a), J::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if (a - b).abs() > TOL {
                out.push(json!({"field":path,"old":a,"new":b}));
            }
        }
        (J::Array(a), J::Array(b)) => {
            if a.len() != b.len() {
                out.push(json!({"field":path,"old_len":a.len(),"new_len":b.len(),"old":a,"new":b}));
            } else {
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    numeric(a, b, &format!("{path}[{i}]"), out);
                }
            }
        }
        (J::Object(a), J::Object(b)) => {
            for (k, v) in a {
                if let Some(b) = b.get(k) {
                    numeric(v, b, &format!("{path}.{k}"), out);
                } else {
                    out.push(json!({"field":format!("{path}.{k}"),"old":v,"missing_in_new":true}));
                }
            }
        }
        _ if a != b => out.push(json!({"field":path,"old":a,"new":b})),
        _ => {}
    }
}
// Include all ordinary text, including standalone legends outside plotBounds.
// Merge same-paint runs so backend line fragmentation does not hide or create
// differences. Formula glyph metrics have their own explicitly scoped audit.
fn text_paints(nodes: &J, out: &mut Vec<String>) {
    for n in nodes.as_array().into_iter().flatten() {
        if n["kind"] == "text" {
            let mut runs: Vec<(String, String)> = vec![];
            for run in n["runs"].as_array().into_iter().flatten() {
                if run["kind"] != "glyph" {
                    continue;
                }
                let content = run["content"].as_str().unwrap_or("");
                let color = run["color"].as_str().unwrap_or("<missing-color>");
                if let Some((text, paint)) = runs.last_mut().filter(|(_, paint)| paint == color) {
                    text.push_str(content);
                    let _ = paint;
                } else {
                    runs.push((content.into(), color.into()));
                }
            }
            if !runs.is_empty() {
                out.push(serde_json::to_string(&runs).unwrap());
            }
        }
        text_paints(&n["children"], out);
    }
}
fn data(n: &J) -> Option<&J> {
    n["children"]
        .as_array()?
        .iter()
        .find(|n| n["id"] == "plot-data")
}
fn points(n: &J, tx: f64, ty: f64, out: &mut Vec<[f64; 2]>) {
    let x = tx + n["x"].as_f64().unwrap_or(0.);
    let y = ty + n["y"].as_f64().unwrap_or(0.);
    if n["kind"] == "ellipse" {
        out.push([
            x + n["width"].as_f64().unwrap_or(0.) / 2.,
            y + n["height"].as_f64().unwrap_or(0.) / 2.,
        ]);
    }
    if n["kind"] == "points" {
        for p in numbers(&n["positions"]).chunks_exact(2) {
            out.push([p[0] + x, p[1] + y]);
        }
    }
    for n in n["children"].as_array().into_iter().flatten() {
        points(n, x, y, out);
    }
}
fn clips(n: &J, out: &mut BTreeSet<[i64; 4]>) {
    if n["clip"].is_object() {
        let c = &n["clip"];
        out.insert(
            ["x", "y", "width", "height"]
                .map(|k| (c[k].as_f64().unwrap_or(0.) / TOL).round() as i64),
        );
    }
    for n in n["children"].as_array().into_iter().flatten() {
        clips(n, out);
    }
}
fn formulas(nodes: &J, out: &mut BTreeMap<String, Vec<(f64, f64)>>) {
    for n in nodes.as_array().into_iter().flatten() {
        if n["kind"] == "formula" {
            out.entry(n["source"].as_str().unwrap_or("").into())
                .or_default()
                .push((
                    n["width"].as_f64().unwrap_or(0.),
                    n["height"].as_f64().unwrap_or(0.),
                ));
        }
        formulas(&n["children"], out);
    }
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
    let mut paths = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|v| v == "jsonl"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut cache = BTreeMap::new();
    let mut rows = vec![];
    let (mut cases, mut compared, mut failures, mut automatic_changes) = (0, 0, 0, 0);
    for path in paths {
        for (line, row) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let c: J = serde_json::from_str(row).unwrap();
            if c["result"]["ok"] != true {
                continue;
            }
            let mut old = vec![];
            plots(&c["result"]["scene"]["nodes"], &mut old);
            if old.is_empty() {
                continue;
            }
            cases += 1;
            let mut host = Host::default();
            for (name, hash) in c["files"].as_object().unwrap() {
                let hash = hash.as_str().unwrap();
                let bytes = cache
                    .entry(hash.to_string())
                    .or_insert_with(|| std::fs::read(root.join("blobs").join(hash)).unwrap());
                host.files.insert(name.clone(), bytes.clone());
            }
            let source = c["source"].as_str().unwrap();
            let fixed = source.contains("plot_area=")
                || source.contains("plot_area =")
                || source.contains("margins=");
            let mut engine = Engine::new(host.clone());
            for (name, bytes) in &host.files {
                if [".ttf", ".otf", ".ttc", ".otc"]
                    .iter()
                    .any(|ext| name.ends_with(ext))
                {
                    engine.fonts.register_font(name, bytes.clone());
                }
            }
            let result = engine.compile(source, c["file"].as_str().unwrap());
            let mut differences = vec![];
            let mut automatic = vec![];
            let mut formula_metrics = vec![];
            match result {
                Err(e) => differences.push(json!({"compile_error":e})),
                Ok(scene) => {
                    let nodes = serde_json::to_value(scene.nodes).unwrap();
                    let (mut old_paints, mut new_paints) = (vec![], vec![]);
                    text_paints(&c["result"]["scene"]["nodes"], &mut old_paints);
                    text_paints(&nodes, &mut new_paints);
                    old_paints.sort();
                    new_paints.sort();
                    numeric(
                        &json!(old_paints),
                        &json!(new_paints),
                        "scene.text_paints",
                        &mut differences,
                    );
                    let (mut old_formulas, mut new_formulas) = (BTreeMap::new(), BTreeMap::new());
                    formulas(&c["result"]["scene"]["nodes"], &mut old_formulas);
                    formulas(&nodes, &mut new_formulas);
                    for (source, old) in old_formulas {
                        if let Some(new) = new_formulas.get(&source) {
                            if old.len() != new.len() {
                                differences.push(json!({"formula_source":source,"old_count":old.len(),"new_count":new.len()}));
                            }
                            for (old, new) in old.into_iter().zip(new) {
                                if (old.0 - new.0).abs() > TOL || (old.1 - new.1).abs() > TOL {
                                    formula_metrics.push(json!({"source":source,"old_width":old.0,"new_width":new.0,"old_height":old.1,"new_height":new.1}));
                                }
                            }
                        } else {
                            differences.push(json!({"missing_formula_source":source}));
                        }
                    }
                    let mut new = vec![];
                    plots(&nodes, &mut new);
                    if old.len() != new.len() {
                        differences
                            .push(json!({"plot_count_old":old.len(),"plot_count_new":new.len()}));
                    }
                    for (i, (a, b)) in old.iter().zip(new).enumerate() {
                        compared += 1;
                        numeric(
                            &a["plotBounds"],
                            &b["plotBounds"],
                            &format!("plot[{i}].bounds"),
                            if fixed {
                                &mut differences
                            } else {
                                &mut automatic
                            },
                        );
                        if let Some(axes) = a["plotAxes"].as_object() {
                            for (name, ax) in axes {
                                let bx = &b["plotAxes"][name];
                                if ax["scale"] != bx["scale"] {
                                    differences.push(json!({"axis":name,"old_scale":ax["scale"],"new_scale":bx["scale"]}));
                                }
                                numeric(
                                    &ax["domain"],
                                    &bx["domain"],
                                    &format!("plot[{i}].axes.{name}.domain"),
                                    &mut differences,
                                );
                                if fixed && ax.get("segments").is_some() {
                                    numeric(
                                        &ax["segments"],
                                        &bx["segments"],
                                        &format!("plot[{i}].axes.{name}.segments"),
                                        &mut differences,
                                    );
                                }
                            }
                        }
                        if !a["plotProjection"].is_null() {
                            numeric(
                                &a["plotProjection"],
                                &b["plotProjection"],
                                &format!("plot[{i}].projection"),
                                if fixed {
                                    &mut differences
                                } else {
                                    &mut automatic
                                },
                            );
                        }
                        if fixed {
                            if data(a).is_some() && data(b).is_none() {
                                differences.push(json!({"plot":i,"missing_contract":"plot-data"}));
                            }
                            if let (Some(da), Some(db)) = (data(a), data(b)) {
                                let (mut pa, mut pb) = (vec![], vec![]);
                                points(da, 0., 0., &mut pa);
                                points(db, 0., 0., &mut pb);
                                pa.sort_by(|a, b| {
                                    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
                                });
                                pb.sort_by(|a, b| {
                                    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
                                });
                                numeric(
                                    &json!(pa),
                                    &json!(pb),
                                    &format!("plot[{i}].data.marker_positions"),
                                    &mut differences,
                                );
                                let (mut ca, mut cb) = (BTreeSet::new(), BTreeSet::new());
                                clips(da, &mut ca);
                                clips(db, &mut cb);
                                if ax_has_breaks(a) && ca != cb {
                                    differences
                                        .push(json!({"plot":i,"clips_old":ca,"clips_new":cb}));
                                }
                            }
                        }
                    }
                }
            }
            if !automatic.is_empty() {
                // All corpus text uses the same fixture font. Any residual
                // automatic box change must be accounted for by measured RaTeX
                // formula metrics, including stacking title/factor rows.
                let mut sums = vec![0f64];
                for metrics in formula_metrics.iter().take(8) {
                    let delta = metrics["new_height"].as_f64().unwrap()
                        - metrics["old_height"].as_f64().unwrap();
                    let prior = sums.clone();
                    sums.extend(prior.iter().map(|v| v + delta));
                    sums.extend(prior.iter().map(|v| v - delta));
                }
                for change in &automatic {
                    let delta =
                        change["new"].as_f64().unwrap_or(0.) - change["old"].as_f64().unwrap_or(0.);
                    if !sums.iter().any(|v| (v - delta).abs() <= TOL) {
                        differences.push(json!({"unexplained_automatic_layout_change":change}));
                    }
                }
            }
            if !differences.is_empty() {
                failures += 1;
            }
            if !automatic.is_empty() {
                automatic_changes += 1;
            }
            rows.push(json!({"capture":path.file_name().unwrap().to_string_lossy(),"line":line+1,"file":c["file"],"fixed_area":fixed,"differences":differences,"automatic_margin_changes":automatic,"formula_metric_changes":if automatic.is_empty(){vec![]}else{formula_metrics}}));
        }
    }
    let summary = json!({"cases":cases,"plots_compared":compared,"cases_with_mismatches":failures,"automatic_margin_change_cases":automatic_changes,"tolerance_mm":TOL,"coverage":"Coordinates, axes, projection metadata, clips and all ordinary text paints (including standalone legends). Full geometry and drawing semantics are checked by the mapped assertion suites; this report alone is not full visual parity."});
    println!("{summary}");
    std::fs::write(
        root.parent().unwrap().join("rust-plot-comparison.json"),
        serde_json::to_vec_pretty(&json!({"summary":summary,"cases":rows})).unwrap(),
    )
    .unwrap();
    if failures > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compared_contracts_reject_missing_fields_and_non_numeric_mutations() {
        let original = json!({"domain":["A","B"],"paint":{"color":"#005577","opacity":0.5}});
        for bad in [
            json!({"domain":["A","C"],"paint":{"color":"#005577","opacity":0.5}}),
            json!({"domain":["A","B"],"paint":{"opacity":0.5}}),
            json!({"domain":["A","B"],"paint":{"color":"#000000","opacity":0.5}}),
        ] {
            let mut differences = vec![];
            numeric(&original, &bad, "scene", &mut differences);
            assert!(!differences.is_empty());
        }
    }
}
fn ax_has_breaks(n: &J) -> bool {
    n["plotAxes"].as_object().is_some_and(|axes| {
        axes.values()
            .any(|ax| ax["segments"].as_array().is_some_and(|ss| ss.len() > 1))
    })
}
