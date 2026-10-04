#![cfg(feature = "native")]
mod plot_support;
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use plot_support::*;
use serde_json::{Value as J, json};
fn fixtures() -> J {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/contour-contracts.json"
    ))
    .unwrap()
}
fn build(c: &J) -> Scene {
    let mut h = Host::default();
    for (k, v) in c["files"].as_object().into_iter().flatten() {
        h.files
            .insert(k.clone(), v.as_str().unwrap().as_bytes().to_vec());
    }
    compile_source(
        &laymesh_core::migration::migrate_arrows(c["source"].as_str().unwrap()),
        "/fixtures/contour.lay",
        h,
    )
    .unwrap()
}
fn semantics(s: &Scene, c: &J) -> bool {
    let pp = paths(data(&s.nodes[0]));
    if let Some(count) = c["data_path_count"].as_u64() {
        if pp.len() != count as usize {
            return false;
        }
    }
    if let Some(count) = c["minimum_closed_subpaths"].as_u64() {
        if pp
            .iter()
            .map(|p| p["d"].as_str().unwrap().matches('Z').count())
            .sum::<usize>()
            < count as usize
        {
            return false;
        }
    }
    for q in c["boundary_vertices"].as_array().into_iter().flatten() {
        let x = q[0].as_f64().unwrap() - 10.;
        let y = q[1].as_f64().unwrap() - 10.;
        if !pp
            .iter()
            .flat_map(|p| vertices(p))
            .any(|p| (p[0] - x).hypot(p[1] - y) < 0.001)
        {
            return false;
        }
    }
    true
}
#[test]
fn shared_native_wasm_contour_fixtures_match_independent_geometry_and_pixels() {
    let f = fixtures();
    for c in f["cases"].as_array().unwrap() {
        let s = build(c);
        assert!(semantics(&s, c), "{} geometry", c["name"]);
        for r in [raster(&s, 254.), pdf_raster(&s, 254.)] {
            for sample in c["pixel_samples"].as_array().unwrap() {
                let actual = r.pixel(num(sample, "x"), num(sample, "y"));
                let expected = sample["rgb"].as_array().unwrap();
                for i in 0..3 {
                    assert!(
                        (actual[i] as i64 - expected[i].as_i64().unwrap()).abs()
                            <= sample["tolerance"].as_i64().unwrap(),
                        "{} {sample} got {actual:?}",
                        c["name"]
                    );
                }
            }
        }
    }
}
#[test]
fn contour_contracts_reject_missing_geometry_and_changed_opacity() {
    let f = fixtures();
    let c = &f["cases"][0];
    let mut s = build(c);
    let data = s.nodes[0]["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "plot-data")
        .unwrap();
    data["children"] = json!([]);
    assert!(!semantics(&s, c));
    let c = &f["cases"][2];
    let mut s = build(c);
    fn mutate(n: &mut J) {
        if n["kind"] == "path" && n["fill"] == "#000000" {
            n["opacity"] = json!(1.);
        }
        for ch in n["children"].as_array_mut().into_iter().flatten() {
            mutate(ch)
        }
    }
    mutate(&mut s.nodes[0]);
    assert_eq!(raster(&s, 254.).pixel(30., 30.), [0; 3]);
    assert_ne!(raster(&s, 254.).pixel(30., 30.), [127; 3]);
}
