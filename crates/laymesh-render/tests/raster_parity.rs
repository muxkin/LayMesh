#![cfg(feature = "native")]
//! Samples physical interiors and boundaries, rather than matching antialiasing implementation details.
use laymesh_core::{
    Loc,
    model::{Scene, base},
    text::{FontSystem, formula, layout_text},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::{Value as Json, json};
use std::{
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Raster {
    width: u32,
    height: u32,
    channels: usize,
    data: Vec<u8>,
}
impl Raster {
    fn at(&self, x_mm: f64, y_mm: f64) -> [u8; 3] {
        let x = (x_mm * 144. / 25.4)
            .round()
            .clamp(0., self.width as f64 - 1.) as usize;
        let y = (y_mm * 144. / 25.4)
            .round()
            .clamp(0., self.height as f64 - 1.) as usize;
        let i = (y * self.width as usize + x) * self.channels;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }
}
fn decode(bytes: Vec<u8>) -> Raster {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    data.truncate(info.buffer_size());
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => panic!("expected RGB PNG"),
    };
    Raster {
        width: info.width,
        height: info.height,
        channels,
        data,
    }
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn pdf_raster(scene: &Scene) -> Raster {
    static SERIAL: AtomicUsize = AtomicUsize::new(0);
    let dir = Temp(std::env::temp_dir().join(format!(
        "laymesh-raster-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&dir.0).unwrap();
    let file = dir.0.join("output.pdf");
    std::fs::write(&file, render_pdf(scene).unwrap()).unwrap();
    let result = std::process::Command::new("pdftoppm")
        .args(["-png", "-r", "144", "-singlefile"])
        .arg(file)
        .arg(dir.0.join("output"))
        .output()
        .expect("pdftoppm is required for native PDF raster regression checks");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    decode(std::fs::read(dir.0.join("output.png")).unwrap())
}
fn scene(nodes: Vec<Json>) -> Scene {
    Scene {
        schema_version: 8,
        width: 50.8,
        height: 25.4,
        background: json!("#ffffff"),
        layout_dpi: 96.,
        export_dpi: 144.,
        nodes,
        warnings: vec![],
        fonts: Default::default(),
    }
}
fn close(actual: [u8; 3], expected: [u8; 3], tol: u8) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!(
            a.abs_diff(b) <= tol,
            "actual {actual:?}, expected {expected:?}, tolerance {tol}"
        );
    }
}
fn all_rasters(scene: &Scene) -> Vec<Raster> {
    let images = vec![decode(render_png(scene, 144.).unwrap()), pdf_raster(scene)];
    for image in &images {
        assert_eq!((image.width, image.height), (288, 144));
    }
    images
}
#[test]
fn scatter_alpha_accumulates_and_hollow_markers_keep_their_interiors() {
    let points = json!({"kind":"points","positions":[10.,10.,10.,10.],"marker":"circle","markerSize":6.,"markerFill":"#0000ff","markerStroke":"none","pointOpacity":0.5});
    let hollow = json!({"kind":"points","positions":[25.,10.],"marker":"square","markerSize":8.,"markerFill":"none","markerStroke":"#000000","markerStrokeWidth":1.,"pointOpacity":1.});
    let s = scene(vec![points, hollow]);
    for r in all_rasters(&s) {
        close(r.at(10., 10.), [64, 64, 255], 3);
        close(r.at(25., 10.), [255, 255, 255], 0);
        close(r.at(25., 6.5), [0, 0, 0], 8);
        close(r.at(25., 5.5), [255, 255, 255], 0);
    }
}
#[test]
fn absolute_point_alpha_checker_rejects_repeated_multiplication() {
    let points = json!({"kind":"points","positions":[10.,10.,30.,10.],"marker":"circle","markerSize":8.,"markerFill":"#000000","markerStroke":"none","pointOpacity":0.5,"pointOpacities":[0.5,0.25]});
    let original = scene(vec![points]);
    let correct_alpha = |r: &Raster| {
        [(10., 128u8), (30., 191u8)].iter().all(|(x, gray)| {
            r.at(*x, 10.)
                .into_iter()
                .all(|channel| channel.abs_diff(*gray) <= 2)
        })
    };
    for r in all_rasters(&original) {
        assert!(correct_alpha(&r));
    }
    // Fault injection at the shared scene boundary: reproduce the former
    // scalar-by-array multiplication, then exercise both real exporters.
    let mut old = original.clone();
    old.nodes[0]["pointOpacities"] = json!([0.25, 0.125]);
    for r in all_rasters(&old) {
        assert!(
            !correct_alpha(&r),
            "PNG and PDF must both reject multiplied point alpha"
        );
    }
}
#[test]
fn evenodd_holes_and_physical_group_clips_match_pdf() {
    let mut group = base("group", 30., 20.);
    group["contentWidth"] = json!(30.);
    group["contentHeight"] = json!(20.);
    group["clipPath"] = json!({"d":"M2 2H28V18H2ZM10 7H20V13H10Z","fillRule":"evenodd"});
    group["children"] = json!([{"kind":"rect","width":35.,"height":20.,"fill":"#ff0000"}]);
    let s = scene(vec![group]);
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("clip-rule=\"evenodd\""));
    for r in all_rasters(&s) {
        close(r.at(5., 5.), [255, 0, 0], 0);
        close(r.at(15., 10.), [255, 255, 255], 0);
        close(r.at(30., 10.), [255, 255, 255], 0);
    }
}
#[test]
fn gradients_and_compound_stroke_gaps_match_native_pdf() {
    let gradient = json!({"kind":"rect","width":20.,"height":8.,"x":2.,"y":2.,"fill":{"kind":"linearGradient","start":[0.,0.],"end":[1.,0.],"stops":[{"at":0.,"color":"#ff0000","opacity":1.},{"at":1.,"color":"#0000ff","opacity":1.}]}});
    let double = json!({"kind":"path","d":"M2 15H25","width":25.,"height":20.,"fill":"none","strokeStyle":{"color":"#000000","width":3.,"compound":"double","cap":"butt","join":"miter","dash":[]}});
    let s = scene(vec![gradient, double]);
    let images = all_rasters(&s);
    for r in &images {
        let center = r.at(12., 5.);
        assert!((115..=140).contains(&center[0]) && (115..=140).contains(&center[2]));
        close(r.at(12., 15.), [255, 255, 255], 0);
        close(r.at(12., 14.), [0, 0, 0], 8);
    }
    if images.len() == 2 {
        for x in [5., 12., 19.] {
            close(images[0].at(x, 5.), images[1].at(x, 5.), 4);
        }
    }
}
#[test]
fn ordinary_text_and_formula_outlines_have_the_same_ink_bounds_in_pdf() {
    let mut fonts = FontSystem::new(false);
    fonts.register_font("test", include_bytes!("assets/ShapingTest.ttf").to_vec());
    let mut warnings = vec![];
    let mut label = layout_text(
        &json!({"content":"Body ffi","font_size":4.}),
        None,
        &mut fonts,
        &mut warnings,
        "x",
        Loc::default(),
    )
    .unwrap();
    label["x"] = json!(3.);
    label["y"] = json!(2.);
    let mut eq = formula(
        &json!({"source":r"\int_0^1 x^2\,dx=\frac{1}{3}","font_size":4.}),
        &mut fonts,
        &mut warnings,
        "x",
        Loc::default(),
    )
    .unwrap();
    eq["x"] = json!(3.);
    eq["y"] = json!(11.);
    let mut s = scene(vec![label, eq]);
    s.fonts = fonts.assets;
    let images = all_rasters(&s);
    let bounds = |r: &Raster, y0: usize, y1: usize| {
        let mut b = [usize::MAX, usize::MAX, 0, 0];
        let mut count = 0;
        for y in y0..y1 {
            for x in 0..r.width as usize {
                let i = (y * r.width as usize + x) * r.channels;
                if r.data[i] < 100 {
                    b[0] = b[0].min(x);
                    b[1] = b[1].min(y);
                    b[2] = b[2].max(x);
                    b[3] = b[3].max(y);
                    count += 1;
                }
            }
        }
        assert!(count > 50);
        b
    };
    for (y0, y1) in [(0, 55), (55, 144)] {
        let expected = bounds(&images[0], y0, y1);
        for image in &images[1..] {
            let actual = bounds(image, y0, y1);
            for (a, b) in actual.into_iter().zip(expected) {
                assert!(a.abs_diff(b) <= 2, "{actual:?} vs {expected:?}");
            }
        }
    }
}

#[test]
fn cropped_raster_does_not_sample_excluded_neighbor_pixels() {
    let asset = laymesh_core::assets::load(
        include_bytes!("../../../examples/assets/photo.png"),
        "photo.png",
        "test.lay",
        Loc::default(),
    )
    .unwrap();
    let image = json!({"kind":"image","x":2.,"y":2.,"width":20.,"height":20.,"fit":"stretch","mime":asset["mime"],"data":asset["data"],"intrinsicWidth":32.,"intrinsicHeight":16.,"crop":{"x":16.,"y":0.,"width":16.,"height":16.}});
    let s = scene(vec![image]);
    for r in all_rasters(&s) {
        let expected = r.at(10., 10.);
        assert!(expected[2] > expected[0]);
        for x in [2.3, 2.6, 3., 5.] {
            close(r.at(x, 10.), expected, 1);
        }
    }
}

#[test]
fn image_fill_loads_definition_relative_assets_and_hatch_keeps_physical_units() {
    use laymesh_core::{engine::compile_source, model::Host};
    let mut host = Host::default();
    host.files.insert(
        "/parts/photo.png".into(),
        include_bytes!("../../../examples/assets/photo.png").to_vec(),
    );
    host.files.insert(
        "/parts/paint.lay".into(),
        b"export texture=image_fill(src=\"photo.png\",fit=stretch)".to_vec(),
    );
    let s=compile_source("import {texture} from \"parts/paint.lay\"\npage=canvas(size=(50.8,25.4),background=\"#ffffff\")\npage.add(rect(size=(20,20),fill=texture),offset=(2,2))\npage.add(rect(size=(20,20),fill=hatch(pattern=\"dots\",spacing=0.5cm,line_width=4pt)),offset=(26,2))","/main.lay",host).unwrap();
    assert_eq!(s.nodes[0]["fill"]["kind"], "image_fill");
    assert_eq!(s.nodes[0]["fill"]["width"], 32);
    assert_eq!(s.nodes[1]["fill"]["spacing"], 5);
    assert!((s.nodes[1]["fill"]["line_width"].as_f64().unwrap() - 4. * 25.4 / 72.).abs() < 1e-10);
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("<image"));
    for image in all_rasters(&s) {
        assert!(image.at(5., 10.)[0] > image.at(5., 10.)[2]);
        assert!(image.at(18., 10.)[2] > image.at(18., 10.)[0]);
        close(image.at(33.5, 9.5), [0, 0, 0], 20);
        close(image.at(31., 9.5), [255, 255, 255], 1);
    }
}

#[test]
fn normalized_icc_pixels_have_the_same_colors_in_svg_png_and_pdf() {
    let mut profile = moxcms::ColorProfile::new_srgb();
    std::mem::swap(&mut profile.red_colorant, &mut profile.green_colorant);
    profile.cicp = None;
    let mut info = png::Info::with_size(1, 1);
    info.color_type = png::ColorType::Rgb;
    info.bit_depth = png::BitDepth::Eight;
    info.icc_profile = Some(std::borrow::Cow::Owned(profile.encode().unwrap()));
    let mut source = vec![];
    png::Encoder::with_info(&mut source, info)
        .unwrap()
        .write_header()
        .unwrap()
        .write_image_data(&[255, 0, 0])
        .unwrap();
    let mut host = laymesh_core::model::Host::default();
    host.files.insert("/profile.png".into(), source);
    let scene = laymesh_core::engine::compile_source("page=canvas(size=(50.8mm,25.4mm),background=\"#ffffff\")\npage.add(image(src=\"/profile.png\"),size=(20mm,20mm))", "/main.lay", host).unwrap();
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("data:image/png;base64,"));
    let correct_color = |image: &Raster| {
        image
            .at(10., 10.)
            .into_iter()
            .zip([0u8, 255, 0])
            .all(|(a, b)| a.abs_diff(b) <= 3)
    };
    for image in all_rasters(&scene) {
        assert!(correct_color(&image));
    }
    // Fault injection: strip the profile while retaining the original red
    // samples, the pre-fix normalization behavior. Both exports must expose it.
    let mut raw = vec![];
    let mut encoder = png::Encoder::new(&mut raw, 1, 1);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[255, 0, 0])
        .unwrap();
    let mut old = scene.clone();
    use base64::Engine as _;
    old.nodes[0]["data"] = json!(base64::engine::general_purpose::STANDARD.encode(raw));
    for image in all_rasters(&old) {
        assert!(
            !correct_color(&image),
            "PNG and PDF must both reject discarded ICC conversion"
        );
    }
}
