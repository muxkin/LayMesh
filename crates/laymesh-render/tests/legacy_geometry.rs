#![cfg(feature = "native")]
//! Port of all geometry-regression.test.mjs scenarios at 78db22d.
//! SVG path decomposition may change; physical ink, topology and anchors may not.
use kurbo::Shape;
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene, jnum},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::Value;
use std::{io::Cursor, path::PathBuf};

fn capture(line: usize) -> Value {
    serde_json::from_str(
        include_str!("../../../migration/corpus/cases-1044528.jsonl")
            .lines()
            .nth(line - 1)
            .unwrap(),
    )
    .unwrap()
}
fn compile(line: usize) -> laymesh_core::Result<Scene> {
    let c = capture(line);
    let mut host = Host::default();
    let blobs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus/blobs");
    for (name, hash) in c["files"].as_object().unwrap() {
        host.files.insert(
            name.clone(),
            std::fs::read(blobs.join(hash.as_str().unwrap())).unwrap(),
        );
    }
    compile_source(
        &laymesh_core::migration::migrate_arrows(c["source"].as_str().unwrap()),
        c["file"].as_str().unwrap(),
        host,
    )
}
struct Raster {
    width: usize,
    height: usize,
    channels: usize,
    data: Vec<u8>,
    mm: [f64; 2],
}
impl Raster {
    fn new(scene: &Scene, dpi: f64) -> Self {
        let mut decoder = png::Decoder::new(Cursor::new(render_png(scene, dpi).unwrap()));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().unwrap();
        let mut data = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut data).unwrap();
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => panic!("{other:?}"),
        };
        Self {
            width: info.width as usize,
            height: info.height as usize,
            channels,
            data,
            mm: [scene.width, scene.height],
        }
    }
    fn at(&self, x: f64, y: f64) -> [u8; 3] {
        let x = (x * self.width as f64 / self.mm[0])
            .round()
            .clamp(0., self.width as f64 - 1.) as usize;
        let y = (y * self.height as f64 / self.mm[1])
            .round()
            .clamp(0., self.height as f64 - 1.) as usize;
        let i = (y * self.width + x) * self.channels;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }
    fn components(&self) -> usize {
        let mut seen = vec![false; self.width * self.height];
        let mut count = 0;
        for origin in 0..seen.len() {
            if seen[origin] || self.data[origin * self.channels] >= 100 {
                continue;
            }
            count += 1;
            seen[origin] = true;
            let mut queue = vec![origin];
            while let Some(p) = queue.pop() {
                let (x, y) = (p % self.width, p / self.width);
                let adjacent = [
                    (x > 0).then(|| p - 1),
                    (x + 1 < self.width).then(|| p + 1),
                    (y > 0).then(|| p - self.width),
                    (y + 1 < self.height).then(|| p + self.width),
                ];
                for n in adjacent.into_iter().flatten() {
                    if !seen[n] && self.data[n * self.channels] < 100 {
                        seen[n] = true;
                        queue.push(n)
                    }
                }
            }
        }
        count
    }
}
fn vector_exports(scene: &Scene) {
    assert!(render_svg(scene).unwrap().contains("<path "));
    let pdf = render_pdf(scene).unwrap();
    assert!(
        !String::from_utf8_lossy(&pdf)
            .split_whitespace()
            .collect::<String>()
            .contains("/Subtype/Image")
    );
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}

#[test]
fn arrows_trim_shafts_in_all_five_directions() {
    for (i, (dx, dy)) in [
        (30_f64, 0_f64),
        (30., 10.),
        (-30., 0.),
        (-30., -10.),
        (0., 30.),
    ]
    .into_iter()
    .enumerate()
    {
        let s = compile(i + 1).unwrap();
        let n = &s.nodes[0];
        let end = &n["endpoints"][1];
        let (tx, ty) = (
            jnum(n, "x", 0.) + end[0].as_f64().unwrap(),
            jnum(n, "y", 0.) + end[1].as_f64().unwrap(),
        );
        let len = dx.hypot(dy);
        let (ux, uy) = (dx / len, dy / len);
        // Historical arrow sources are explicitly migrated; the new recipe
        // retains the full centerline and independent fixed-size decoration.
        assert_eq!(n["endpointRecipe"]["end_head"]["shape"], "triangle");
        assert!(n["endpointRecipe"]["end_head"]["size"][0].as_f64().unwrap() >= 7.99);
        assert!(laymesh_core::geometry::visible(n).elements().len() > 3);
        let raster = Raster::new(&s, 300.);
        assert_eq!(
            raster.at(tx - ux * 0.6 - uy * 0.8, ty - uy * 0.6 + ux * 0.8),
            [255; 3],
            "direction {i}: {n}"
        );
        assert!(raster.at(tx - ux * 2., ty - uy * 2.)[0] > 150);
        vector_exports(&s);
    }
}
#[test]
fn short_arrows_keep_the_tip_anchor_without_a_shaft() {
    let s = compile(6).unwrap();
    let n = &s.nodes[0];
    let layers = laymesh_core::endpoints::layers(n, kurbo::Affine::IDENTITY).unwrap();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].path.bounding_box().width() > 2.);
    assert_eq!(n["endpoints"][1], serde_json::json!([2., 0.]));
    close(jnum(&s.nodes[1], "x", 0.), 6.5);
    close(jnum(&s.nodes[1], "y", 0.), 9.5);
    assert!(Raster::new(&s, 300.).at(6.3, 10.)[0] > 150);
}
#[test]
fn capped_dashed_and_compound_arrow_ink_stops_behind_heads() {
    let s = compile(7).unwrap();
    assert_eq!(s.nodes.len(), 3);
    for n in &s.nodes {
        assert_eq!(n["endpointRecipe"]["end_head"]["shape"], "triangle")
    }
    assert_eq!(
        s.nodes[1]["strokeStyle"]["dash"],
        serde_json::json!([2., 1.])
    );
    assert_eq!(s.nodes[2]["strokeStyle"]["compound"], "double");
    let r = Raster::new(&s, 300.);
    for y in [10., 23., 36.] {
        assert_eq!(r.at(37.4, y + 0.7), [255; 3])
    }
    vector_exports(&s);
}
#[test]
fn fused_arrows_preserve_trimmed_shafts_and_intact_heads() {
    let s = compile(8).unwrap();
    assert_eq!(s.nodes.len(), 1);
    assert_eq!(s.nodes[0]["kind"], "path");
    let r = Raster::new(&s, 300.);
    assert_eq!(r.at(46.4, 15.8), [255; 3]);
    assert!(r.at(45., 15.)[0] > 150);
    vector_exports(&s);
}
#[test]
fn round_fuse_fillets_are_symmetric_continuous_and_monotone() {
    let s = compile(9).unwrap();
    let n = s.nodes.iter().find(|n| n["id"] == "joined").unwrap();
    assert_eq!(n["kind"], "path");
    for (key, value) in [("x", 16.), ("y", 35.), ("width", 71.), ("height", 21.)] {
        close(jnum(n, key, 0.), value)
    }
    let r = Raster::new(&s, 300.);
    let mut edges = vec![];
    for x in [39.25, 39.5, 39.75, 40., 40.5, 41., 42., 45.] {
        let painted = (0..=220)
            .map(|i| 40. + i as f64 * 0.05)
            .filter(|y| r.at(x, *y)[0] < 100)
            .collect::<Vec<_>>();
        assert!(!painted.is_empty(), "missing bridge at {x}");
        let pair = (painted[0], *painted.last().unwrap());
        assert!((pair.0 + pair.1 - 91.).abs() < 0.35);
        assert!(r.at(x, 45.5)[0] < 100);
        edges.push(pair);
    }
    for p in edges.windows(2) {
        assert!(p[1].0 >= p[0].0 - 0.2);
        assert!(p[1].1 <= p[0].1 + 0.2)
    }
    vector_exports(&s);
}
#[test]
fn angled_and_touching_round_joins_preserve_remote_holes() {
    let s = compile(10).unwrap();
    let r = Raster::new(&s, 300.);
    assert_eq!(r.at(18., 26.), [255; 3]);
    assert!(r.at(47., 27.8)[0] < 100);
    assert_eq!(s.nodes[0]["kind"], "path");
    let s = compile(11).unwrap();
    let r = Raster::new(&s, 300.);
    assert!(r.at(22.5, 19.)[0] < 100);
    assert_eq!(r.at(25., 15.), [255; 3]);
    assert_eq!(s.nodes.len(), 1);
}
#[test]
fn sloped_fusions_remain_connected_for_all_shapes_and_directions() {
    for shape in 0..4 {
        for (i, dy) in [-12., 0., 12.].into_iter().enumerate() {
            let s = compile(12 + shape * 3 + i).unwrap();
            assert_eq!(s.nodes.len(), 1);
            assert_eq!(s.nodes[0]["kind"], "path");
            assert!(jnum(&s.nodes[0], "x", 0.) + jnum(&s.nodes[0], "width", 0.) > 69.);
            let r = Raster::new(&s, 300.);
            for x in [46., 50., 60., 68.] {
                assert!(
                    r.at(x, 35. + dy * (x - 45.) / 25.)[0] < 100,
                    "shape={shape} dy={dy} x={x}"
                )
            }
            assert!(r.at(20., 35.)[0] < 100);
            if shape < 3 {
                assert!(r.at(40., 35.)[0] < 100)
            }
            assert_eq!(
                Raster::new(&s, 100.).components(),
                1,
                "shape={shape} dy={dy}"
            );
            vector_exports(&s);
        }
    }
}
#[test]
fn detached_bbox_anchors_fail_but_touching_curves_stay_connected() {
    for line in [24, 25] {
        let e = compile(line).unwrap_err();
        assert_eq!(e.code, "E_FUSE");
        assert_eq!(e.loc.line, 5);
        assert!(e.message.contains("没有接触"))
    }
    assert_eq!(Raster::new(&compile(26).unwrap(), 100.).components(), 1);
}
#[test]
fn hollow_frames_keep_holes_and_connect_both_anchors_for_all_seams() {
    for group in 0..2 {
        for junction in 0..3 {
            let s = compile(27 + group * 3 + junction).unwrap();
            assert_eq!(s.nodes.len(), 1);
            assert_eq!(s.nodes[0]["fillRule"], "evenodd");
            let r = Raster::new(&s, 300.);
            let join_y = if group == 0 { 35. } else { 24. };
            assert_eq!(r.at(22., 35.), [255; 3]);
            assert!(r.at(10., 35.)[0] < 100);
            assert!(r.at(35., join_y)[0] < 100);
            assert!(r.at(43., join_y - 8.)[0] < 100);
            assert_eq!(Raster::new(&s, 100.).components(), 1);
            assert!(render_svg(&s).unwrap().contains("fill-rule=\"evenodd\""));
            vector_exports(&s);
        }
    }
}
