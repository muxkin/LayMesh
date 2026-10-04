#![allow(dead_code)]
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene, jnum},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::Value as J;
use std::{
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
pub const COMPLETE: &str = "cases-1044564.jsonl";
pub const LABELS: &str = "cases-1044567.jsonl";
pub const POLAR: &str = "cases-1044575.jsonl";
pub const PLOT: &str = "cases-1044587.jsonl";
pub fn capture(file: &str, line: usize) -> J {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../migration/corpus")
        .join(file);
    serde_json::from_str(
        std::fs::read_to_string(p)
            .unwrap()
            .lines()
            .nth(line - 1)
            .unwrap(),
    )
    .unwrap()
}
pub fn compile_case(file: &str, line: usize) -> laymesh_core::Result<Scene> {
    let c = capture(file, line);
    compile_capture(&c)
}
pub fn compile_capture(c: &J) -> laymesh_core::Result<Scene> {
    let mut host = Host::default();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus/blobs");
    for (name, hash) in c["files"].as_object().unwrap() {
        host.files.insert(
            name.clone(),
            std::fs::read(root.join(hash.as_str().unwrap())).unwrap(),
        );
    }
    compile_source(
        &laymesh_core::migration::migrate_arrows(c["source"].as_str().unwrap()),
        c["file"].as_str().unwrap(),
        host,
    )
}
pub fn scene(file: &str, line: usize) -> Scene {
    compile_case(file, line).unwrap_or_else(|e| panic!("{file}:{line}: {e}"))
}
pub fn failure(file: &str, line: usize) {
    let c = capture(file, line);
    let e = compile_capture(&c).unwrap_err();
    assert_eq!(e.code, c["result"]["code"], "{file}:{line}");
    assert_eq!(e.file, c["result"]["file"]);
    assert_eq!(
        e.loc.line as u64,
        c["result"]["loc"]["line"].as_u64().unwrap()
    );
    assert!(e.loc.column > 0);
}
pub fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
pub fn walk<'a>(n: &'a J, out: &mut Vec<&'a J>) {
    out.push(n);
    for c in n["children"].as_array().into_iter().flatten() {
        walk(c, out)
    }
}
pub fn all(s: &Scene) -> Vec<&J> {
    let mut out = vec![];
    for n in &s.nodes {
        walk(n, &mut out)
    }
    out
}
pub fn descendants(n: &J) -> Vec<&J> {
    let mut out = vec![];
    walk(n, &mut out);
    out
}
pub fn data(p: &J) -> &J {
    p["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "plot-data")
        .unwrap()
}
pub fn texts(p: &J) -> Vec<&J> {
    descendants(p)
        .into_iter()
        .filter(|n| n["kind"] == "text")
        .collect()
}
pub fn text<'a>(p: &'a J, content: &str) -> &'a J {
    texts(p)
        .into_iter()
        .find(|n| n["content"] == content)
        .unwrap_or_else(|| panic!("missing text {content}"))
}
pub fn formulas(p: &J) -> Vec<&str> {
    descendants(p)
        .into_iter()
        .filter_map(|n| {
            if n["kind"] == "formula" {
                n["source"].as_str()
            } else {
                None
            }
        })
        .collect()
}
pub fn num(n: &J, k: &str) -> f64 {
    jnum(n, k, 0.)
}
pub fn paths(p: &J) -> Vec<&J> {
    descendants(p)
        .into_iter()
        .filter(|n| n["kind"] == "path")
        .collect()
}
pub fn points(p: &J) -> Vec<&J> {
    descendants(p)
        .into_iter()
        .filter(|n| n["kind"] == "points")
        .collect()
}
pub fn pos(p: &J) -> Vec<[f64; 2]> {
    points(p)
        .iter()
        .flat_map(|n| {
            n["positions"]
                .as_array()
                .unwrap()
                .chunks_exact(2)
                .map(|v| [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()])
        })
        .collect()
}
pub fn vertices(n: &J) -> Vec<[f64; 2]> {
    kurbo::BezPath::from_svg(n["d"].as_str().unwrap())
        .unwrap()
        .elements()
        .iter()
        .filter_map(|e| match e {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => Some([p.x, p.y]),
            _ => None,
        })
        .collect()
}
pub fn exports(s: &Scene) -> String {
    let svg = render_svg(s).unwrap();
    assert!(render_png(s, 96.).unwrap().len() > 100);
    assert!(render_pdf(s).unwrap().len() > 1000);
    assert!(!svg.contains("NaN") && !svg.contains("Infinity"));
    svg
}
pub struct Raster {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub dpi: f64,
}
impl Raster {
    pub fn pixel(&self, x: f64, y: f64) -> [u8; 3] {
        let x = (x * self.dpi / 25.4).round() as usize;
        let y = (y * self.dpi / 25.4).round() as usize;
        assert!(x < self.width && y < self.height);
        let i = (y * self.width + x) * self.channels;
        [self.bytes[i], self.bytes[i + 1], self.bytes[i + 2]]
    }
}
fn decode(bytes: Vec<u8>, dpi: f64) -> Raster {
    let mut r = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut bytes = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut bytes).unwrap();
    bytes.truncate(info.buffer_size());
    Raster {
        bytes,
        width: info.width as usize,
        height: info.height as usize,
        channels: info.color_type.samples(),
        dpi,
    }
}
pub fn raster(s: &Scene, dpi: f64) -> Raster {
    decode(render_png(s, dpi).unwrap(), dpi)
}
static TEMP: AtomicUsize = AtomicUsize::new(0);
pub fn pdf_raster(s: &Scene, dpi: f64) -> Raster {
    let dir = std::env::temp_dir().join(format!(
        "laymesh-plot-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let pdf = dir.join("plot.pdf");
    std::fs::write(&pdf, render_pdf(s).unwrap()).unwrap();
    let prefix = dir.join("plot");
    let result = std::process::Command::new("pdftoppm")
        .args(["-singlefile", "-r", &dpi.to_string(), "-png"])
        .arg(&pdf)
        .arg(&prefix)
        .output()
        .expect("pdftoppm required for plot backend regression checks");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(dir.join("plot.png")).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    decode(bytes, dpi)
}
pub fn pdf_text(s: &Scene) -> String {
    let p = std::env::temp_dir().join(format!(
        "laymesh-plot-text-{}-{}.pdf",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&p, render_pdf(s).unwrap()).unwrap();
    let out = std::process::Command::new("pdftotext")
        .arg("-raw")
        .arg(&p)
        .arg("-")
        .output()
        .unwrap();
    std::fs::remove_file(p).unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}
pub fn decoration<'a>(p: &'a J, name: &str) -> &'a J {
    p["plotDecorations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("missing decoration {name}"))
}
pub fn within(p: &J) {
    for b in p["plotDecorations"].as_array().unwrap() {
        assert!(
            num(b, "left") >= -0.001
                && num(b, "top") >= -0.001
                && num(b, "right") <= num(p, "width") + 0.001
                && num(b, "bottom") <= num(p, "height") + 0.001,
            "outside {b}"
        );
    }
}
pub fn legend(p: &J) -> &J {
    p["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| {
            n["kind"] == "group"
                && texts(n)
                    .iter()
                    .any(|t| t["content"].as_str().is_some_and(|s| !s.is_empty()))
                && n["id"].as_str().unwrap_or("") == ""
        })
        .unwrap()
}
pub fn source(body: &str) -> Scene {
    let mut h = Host::default();
    h.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    compile_source(&format!("page=canvas(size=(180 mm,150 mm),background=\"#ffffff\")\ns=plot_style(font_family=\"/font.ttf\")\n{body}"),"/test.lay",h).unwrap()
}
pub fn matrices_same(a: &Scene, b: &Scene) {
    assert_eq!(a.nodes.len(), b.nodes.len());
    for (a, b) in a.nodes.iter().zip(&b.nodes) {
        for k in [
            "x",
            "y",
            "width",
            "height",
            "rotation",
            "plotBounds",
            "plotAxes",
        ] {
            assert_eq!(a[k], b[k], "{k}");
        }
    }
}
