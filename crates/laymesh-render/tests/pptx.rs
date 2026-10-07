#![cfg(feature = "native")]
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use laymesh_render::{ExportOptions, config, export_format, pptx::render_pptx, render_export};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

fn compile(source: &str) -> Scene {
    let source = source.replace("DejaVu Sans", "/font.ttf");
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        ratex_katex_fonts::ttf_bytes("KaTeX_Main-Regular.ttf")
            .unwrap()
            .to_vec(),
    );
    compile_source(&source, "/pptx.lay", host).unwrap()
}
fn files(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut entry = zip.by_index(i).unwrap();
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (name, data)
        })
        .collect()
}
fn xml<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> roxmltree::Document<'a> {
    roxmltree::Document::parse(std::str::from_utf8(&files[name]).unwrap()).unwrap()
}
fn normalize(path: &str) -> String {
    let mut result = vec![];
    for part in path.split('/') {
        match part {
            ".." => {
                result.pop();
            }
            "" | "." => {}
            p => result.push(p),
        }
    }
    result.join("/")
}
#[test]
fn package_links_ids_dimensions_and_editability() {
    let scene = compile(
        r##"
page=canvas(size=(160mm,100mm),background="#ffffff")
page.add(text(spans=[span("Editable & <text>",color="#203864"),span(" bold",font_weight=700)],font_family="DejaVu Sans",font_size=14pt),offset=(12mm,12mm))
page.add(formula(source=r"\frac{x^2}{3}",font_size=20pt),offset=(10mm,30mm))
inner=group()
inner.add(rect(size=(20mm,12mm),fill="#ff000080",border_width=0),offset=(0mm,0mm))
outer=group()
outer.add(inner,offset=(2mm,3mm))
page.add(outer,offset=(40mm,25mm),rotation=23deg)
page.add(line(dx=35mm,dy=0mm,line_width=0.4mm,line_dash=[2mm,1mm]),offset=(20mm,65mm))
"##,
    );
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(
        warnings.iter().all(|w| w.code == "W_PPTX_FONT"),
        "{warnings:?}"
    );
    let files = crate::files(&bytes);
    for (name, data) in &files {
        if name.ends_with(".xml") || name.ends_with(".rels") {
            let doc = roxmltree::Document::parse(std::str::from_utf8(data).unwrap()).unwrap();
            if name.ends_with(".rels") {
                let base = name
                    .rsplit_once("/_rels/")
                    .map(|(base, _)| base)
                    .unwrap_or("");
                let mut ids = BTreeSet::new();
                for rel in doc.root_element().children().filter(|n| n.is_element()) {
                    assert!(ids.insert(rel.attribute("Id").unwrap()));
                    let target = normalize(&format!("{base}/{}", rel.attribute("Target").unwrap()));
                    assert!(files.contains_key(&target), "{name}: {target}");
                }
            }
        }
    }
    assert!(
        std::str::from_utf8(&files["[Content_Types].xml"])
            .unwrap()
            .contains("presentationml.presentation.main+xml")
    );
    let p = xml(&files, "ppt/presentation.xml");
    let sz = p
        .descendants()
        .find(|n| {
            n.has_tag_name((
                "http://schemas.openxmlformats.org/presentationml/2006/main",
                "sldSz",
            ))
        })
        .unwrap();
    assert_eq!(sz.attribute("cx"), Some("5760000"));
    assert_eq!(sz.attribute("cy"), Some("3600000"));
    let slide = xml(&files, "ppt/slides/slide1.xml");
    for group in slide
        .descendants()
        .filter(|n| n.tag_name().name() == "grpSp")
    {
        let props = group
            .children()
            .find(|n| n.tag_name().name() == "grpSpPr")
            .unwrap();
        let ext = props
            .descendants()
            .find(|n| n.tag_name().name() == "ext")
            .unwrap();
        assert!(
            ext.attribute("cx").unwrap().parse::<i64>().unwrap() < 5_760_000,
            "group selection bounds must fit its contents"
        );
        assert!(ext.attribute("cy").unwrap().parse::<i64>().unwrap() < 3_600_000);
    }
    let mut ids = BTreeSet::new();
    for n in slide
        .descendants()
        .filter(|n| n.tag_name().name() == "cNvPr")
    {
        assert!(ids.insert(n.attribute("id").unwrap()));
    }
    let text = slide
        .descendants()
        .filter(|n| n.tag_name().name() == "t")
        .map(|n| n.text().unwrap())
        .collect::<String>();
    assert_eq!(text, "Editable & <text> bold");
    assert!(
        slide
            .descendants()
            .any(|n| matches!(n.tag_name().name(), "cubicBezTo" | "quadBezTo"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "noAutofit")
    );
    assert!(!files.keys().any(|n| n.starts_with("ppt/media/")));
    assert_eq!(export_format("PPTX").unwrap(), "pptx");
    assert_eq!(
        render_export(&scene, "pptx", &ExportOptions::default()).unwrap(),
        bytes
    );
}
#[test]
fn fallback_is_local_transparent_and_respects_dpi() {
    let scene = compile(
        r##"
page=canvas(size=(100mm,75mm),background="none")
page.add(text("Keep editable",font_family="DejaVu Sans",font_size=12pt),offset=(5mm,5mm))
page.add(rect(size=(10mm,8mm),fill=radial_gradient(stops=[(0,"#ff000080"),(1,"#0000ff80")]),border_width=0),offset=(40mm,30mm),rotation=17deg)
"##,
    );
    let mut dimensions = vec![];
    for dpi in [96., 192.] {
        let (bytes, warnings) = render_pptx(
            &scene,
            &ExportOptions {
                dpi: Some(dpi),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            warnings
                .iter()
                .filter(|w| w.code == "W_PPTX_RASTER")
                .count(),
            1
        );
        let files = crate::files(&bytes);
        let image = image::load_from_memory(&files["ppt/media/image1.png"])
            .unwrap()
            .to_rgba8();
        assert!(
            image.width() < dpi as u32,
            "must crop the object, not rasterize the page"
        );
        assert!(image.height() < dpi as u32);
        assert!(image.pixels().any(|p| p.0[3] == 0));
        assert!(image.pixels().any(|p| p.0[3] > 0 && p.0[3] < 255));
        dimensions.push(image.dimensions());
        assert!(
            xml(&files, "ppt/slides/slide1.xml")
                .descendants()
                .any(|n| n.text() == Some("Keep editable"))
        );
    }
    assert!((dimensions[1].0 as i32 - 2 * dimensions[0].0 as i32).abs() <= 2);
    assert!((dimensions[1].1 as i32 - 2 * dimensions[0].1 as i32).abs() <= 2);
}
#[test]
fn embedded_image_is_preserved_as_picture() {
    use base64::Engine;
    let mut scene = compile("page=canvas(size=(100mm,75mm),background=\"none\")");
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[255, 0, 0, 128, 0, 255, 0, 255, 0, 0, 255, 0, 0, 0, 0, 255])
        .unwrap();
    scene.nodes.push(serde_json::json!({"kind":"image","id":"image","x":12.,"y":10.,"width":20.,"height":20.,"rotation":15.,"opacity":0.5,"mime":"image/png","data":base64::engine::general_purpose::STANDARD.encode(&png),"intrinsicWidth":2.,"intrinsicHeight":2.,"fit":"stretch"}));
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let files = crate::files(&bytes);
    assert_eq!(files["ppt/media/image1.png"], png);
    let slide = xml(&files, "ppt/slides/slide1.xml");
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "alphaModFix" && n.attribute("amt") == Some("50000"))
    );
}
#[test]
fn options_and_config_follow_shared_contract() {
    for options in [
        ExportOptions {
            quality: Some(90.),
            ..Default::default()
        },
        ExportOptions {
            compression: Some("lzw".into()),
            ..Default::default()
        },
        ExportOptions {
            pdf_downsample: Some(true),
            ..Default::default()
        },
        ExportOptions {
            background: Some("#ffffff".into()),
            ..Default::default()
        },
        ExportOptions {
            dpi: Some(f64::NAN),
            ..Default::default()
        },
    ] {
        assert!(options.validate("pptx").is_err());
    }
    let options = config::export_options(
        &serde_json::json!({"export":{"dpi":1200,"pptx":{"dpi":300}}}),
        "pptx",
        &ExportOptions::default(),
    )
    .unwrap();
    assert_eq!(options.dpi, Some(300.));
    let scene = compile("page=canvas(size=(100mm,75mm))");
    assert!(render_export(&scene, "pptx", &options).is_ok());
    let mut invalid = scene.clone();
    invalid.export_dpi = f64::NAN;
    assert!(render_pptx(&invalid, &ExportOptions::default()).is_err());
    invalid.export_dpi = 1200.;
    invalid.width = 1500.;
    assert!(render_pptx(&invalid, &ExportOptions::default()).is_err());
    invalid.width = 20.;
    assert!(render_pptx(&invalid, &ExportOptions::default()).is_err());
}

#[test]
fn clipping_group_alpha_and_effect_overflow_are_not_silently_lost() {
    let mut scene = compile(
        "page=canvas(size=(100mm,75mm),background=\"none\")\npage.add(text(\"Outside\",font_family=\"DejaVu Sans\",font_size=12pt),offset=(3mm,3mm))",
    );
    scene.nodes.push(serde_json::json!({
        "kind":"group", "id":"clipped-group", "x":20.,"y":20.,"width":15.,"height":15.,
        "contentWidth":15.,"contentHeight":15.,"rotation":23.,
        "clipPath":{"d":"M0 0L15 0L0 15Z"},
        "children":[{"kind":"rect","x":0.,"y":0.,"width":15.,"height":15.,"fill":"#ff0000","stroke":"none"}]
    }));
    scene.nodes.push(serde_json::json!({
        "kind":"group", "id":"alpha-group", "x":45.,"y":30.,"width":20.,"height":15.,
        "contentWidth":20.,"contentHeight":15.,"opacity":0.5,
        "children":[{"kind":"rect","x":0.,"y":0.,"width":15.,"height":15.,"fill":"#ff0000","stroke":"none"},
                    {"kind":"rect","x":5.,"y":0.,"width":15.,"height":15.,"fill":"#0000ff","stroke":"none"}]
    }));
    let (bytes, warnings) = render_pptx(
        &scene,
        &ExportOptions {
            dpi: Some(96.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.code == "W_PPTX_RASTER")
            .count(),
        2
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("clipped-group") && w.message.contains("裁剪"))
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("alpha-group") && w.message.contains("透明度"))
    );
    let files = crate::files(&bytes);
    let clipped = image::load_from_memory(&files["ppt/media/image1.png"])
        .unwrap()
        .to_rgba8();
    assert!(clipped.pixels().any(|p| p.0[3] == 0));
    let alpha = image::load_from_memory(&files["ppt/media/image2.png"])
        .unwrap()
        .to_rgba8();
    assert!((alpha.get_pixel(alpha.width() / 2, alpha.height() / 2).0[3] as i32 - 128).abs() <= 1);
    assert!(
        xml(&files, "ppt/slides/slide1.xml")
            .descendants()
            .any(|n| n.text() == Some("Outside"))
    );

    let effects = compile(
        r##"page=canvas(size=(100mm,75mm),background="none")
page.add(rect(size=(10mm,8mm),fill="#3782d6",border_width=0,effects=[shadow(blur=1mm,offset=(2mm,2mm))]),offset=(30mm,30mm))"##,
    );
    let (bytes, warnings) = render_pptx(
        &effects,
        &ExportOptions {
            dpi: Some(96.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.code == "W_PPTX_RASTER")
            .count(),
        1
    );
    let files = crate::files(&bytes);
    let image = image::load_from_memory(&files["ppt/media/image1.png"]).unwrap();
    assert!(image.width() as f64 > 10. * 96. / 25.4);
    assert!(image.height() as f64 > 8. * 96. / 25.4);
}

#[test]
fn formula_text_glyphs_become_outlines_and_anisotropic_text_falls_back() {
    let mut scene = compile(
        "page=canvas(size=(100mm,75mm),background=\"none\")\npage.add(text(\"AB\",font_family=\"DejaVu Sans\",font_size=12pt),offset=(3mm,3mm))",
    );
    let run = scene.nodes[0]["runs"][0].clone();
    scene.nodes.push(serde_json::json!({"kind":"formula","id":"glyph-formula","source":"AB","x":30.,"y":20.,"width":20.,"height":10.,"rotation":17.,"items":[run]}));
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.iter().all(|w| w.code == "W_PPTX_FONT"));
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "t")
            .count(),
        1
    );
    assert!(
        slide
            .descendants()
            .any(|n| matches!(n.tag_name().name(), "cubicBezTo" | "quadBezTo"))
    );

    let mut scaled = scene.clone();
    scaled.nodes = vec![
        serde_json::json!({"kind":"group","id":"stretched-text","x":10.,"y":10.,"width":40.,"height":10.,"contentWidth":20.,"contentHeight":10.,"children":[scene.nodes[0].clone()]}),
    ];
    let (bytes, warnings) = render_pptx(
        &scaled,
        &ExportOptions {
            dpi: Some(96.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W_PPTX_RASTER" && w.message.contains("文字变换"))
    );
    assert_eq!(
        files(&bytes)
            .keys()
            .filter(|k| k.starts_with("ppt/media/"))
            .count(),
        1
    );
}

#[test]
fn native_presets_strokes_and_source_groups_replace_wrappers() {
    let scene = compile(
        r##"
page=canvas(size=(160mm,100mm),background="none")
outer=group()
inner=group()
inner.add(rect(size=(30mm,20mm),border_radius=4mm,fill="#ff000080",border_color="#203864",border_width=0.5mm),offset=(0mm,0mm))
inner.add(ellipse(size=(10mm,8mm),fill="#3782d6",border_width=0),offset=(40mm,0mm))
outer.add(inner,offset=(2mm,3mm))
page.add(outer,offset=(10mm,10mm),rotation=23deg)
page.add(line(dx=-30mm,dy=10mm,line_width=0.4mm,line_dash=[2mm,1mm],line_cap=round,start_cap=round,end_cap=round),offset=(20mm,60mm))
"##,
    );
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    let count = |tag| {
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == tag)
            .count()
    };
    assert_eq!(count("grpSp"), 2, "Only the two explicit groups survive");
    assert_eq!(
        count("sp"),
        3,
        "Each fill and border share one editable shape"
    );
    for kind in ["roundRect", "ellipse", "line"] {
        assert!(
            slide
                .descendants()
                .any(|n| n.attribute("prst") == Some(kind))
        );
    }
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("fmla") == Some("val 20000")),
        "4mm radius on 20mm short edge"
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("w") == Some("18000") && n.tag_name().name() == "ln")
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("cap") == Some("rnd"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("d") == Some("500000") && n.attribute("sp") == Some("250000"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("rot") == Some("1380000"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("flipH") == Some("1"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("val") == Some("50196") && n.tag_name().name() == "alpha")
    );
    assert_eq!(count("custGeom"), 0);
}

fn png_fixture() -> Vec<u8> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, 4, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[
            255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 128, 0, 0, 255, 128, 255, 0, 0, 255, 255, 0,
            0, 255, 0, 0, 255, 128, 0, 0, 255, 128,
        ])
        .unwrap();
    png
}
#[test]
fn rounded_image_fill_fit_and_rectangular_crop_keep_original_media() {
    use base64::Engine;
    let png = png_fixture();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
    let mut scene = compile("page=canvas(size=(160mm,100mm),background=\"none\")");
    for (i, fit) in ["cover", "contain", "stretch"].into_iter().enumerate() {
        scene.nodes.push(serde_json::json!({"kind":"rect","id":fit,"x":10.+i as f64*30.,"y":10.,"width":20.,"height":20.,"radius":3.,"rotation":17.,"opacity":0.5,"fill":{"kind":"image_fill","mime":"image/png","data":encoded,"width":4.,"height":2.,"fit":fit}}));
    }
    scene.nodes.push(serde_json::json!({"kind":"image","id":"cropped","x":110.,"y":10.,"width":20.,"height":20.,"fit":"cover","crop":{"x":1.,"y":0.,"width":2.,"height":2.},"intrinsicWidth":4.,"intrinsicHeight":2.,"mime":"image/png","data":encoded}));
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    assert_eq!(
        data.keys().filter(|n| n.starts_with("ppt/media/")).count(),
        1,
        "{:?}",
        data.iter()
            .filter(|(n, _)| n.starts_with("ppt/media/"))
            .map(|(n, b)| (
                n,
                b.len(),
                image::load_from_memory(b).map(|im| (im.width(), im.height()))
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(data["ppt/media/image1.png"], png);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "pic")
            .count(),
        4
    );
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.attribute("prst") == Some("roundRect"))
            .count(),
        3
    );
    assert!(slide.descendants().any(|n| n.tag_name().name() == "srcRect"
        && n.attribute("l") == Some("25000")
        && n.attribute("r") == Some("25000")));
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "fillRect"
                && n.attribute("t") == Some("25000")
                && n.attribute("b") == Some("25000"))
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "alphaModFix" && n.attribute("amt") == Some("50000"))
    );
    // The shared SVG must fit the image to the rectangle, not a unit square.
    let svg = String::from_utf8(render_export(&scene, "svg", &ExportOptions::default()).unwrap())
        .unwrap();
    assert!(svg.contains("viewBox='0 0 20 20'"));
    assert!(svg.contains("preserveAspectRatio='xMidYMid slice'"));
}
#[test]
fn gradients_are_native_when_exact_and_other_paints_warn_locally() {
    let scene = compile(
        r##"
page=canvas(size=(160mm,100mm),background="none")
page.add(rect(size=(30mm,20mm),fill=linear_gradient(stops=[(0,"#ff000080"),(1,"#0000ff80")]),border_width=0),offset=(10mm,10mm),rotation=17deg)
page.add(rect(size=(20mm,15mm),fill=linear_gradient(stops=[(0,"#ff000000"),(1,"#0000ff")]),border_width=0),offset=(60mm,10mm))
page.add(rect(size=(20mm,15mm),fill=hatch(color="#3782d6"),border_width=0),offset=(90mm,10mm))
"##,
    );
    let (bytes, warnings) = render_pptx(
        &scene,
        &ExportOptions {
            dpi: Some(96.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.code == "W_PPTX_RASTER")
            .count(),
        2
    );
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "gradFill")
            .count(),
        1
    );
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "pic")
            .count(),
        2
    );
}
#[test]
fn anisotropic_stroke_keeps_vector_outline_and_mirrored_image_is_native() {
    use base64::Engine;
    let mut scene = compile("page=canvas(size=(160mm,100mm),background=\"none\")");
    scene.nodes.push(serde_json::json!({"kind":"group","id":"scaled","x":10.,"y":10.,"width":40.,"height":20.,"contentWidth":20.,"contentHeight":20.,"children":[{"kind":"rect","id":"rect","width":20.,"height":20.,"fill":"#ff0000","stroke":"#000000","strokeWidth":1.}]}));
    scene.nodes.push(serde_json::json!({"kind":"group","id":"mirrored","x":100.,"y":10.,"width":-20.,"height":20.,"contentWidth":20.,"contentHeight":20.,"children":[{"kind":"image","id":"image","width":20.,"height":20.,"fit":"stretch","intrinsicWidth":4.,"intrinsicHeight":2.,"mime":"image/png","data":base64::engine::general_purpose::STANDARD.encode(png_fixture())}]}));
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "custGeom")
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.attribute("flipV") == Some("1"))
    );
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "pic")
            .count(),
        1
    );
}

#[test]
fn cli_compiled_crops_and_jpeg_fills_share_the_original_resource() {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(
            &[255, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 255],
            2,
            2,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    let mut host = Host::default();
    host.files.insert("/photo.jpg".into(), jpeg.clone());
    let scene = compile_source(r##"
page=canvas(size=(160mm,100mm),background="none")
page.add(rect(size=(20mm,20mm),border_radius=3mm,fill=image_fill(src="photo.jpg",fit=cover),border_width=0),offset=(10mm,10mm))
page.add(image(src="photo.jpg"),size=(20mm,20mm),crop=box(offset=(0.5,0),size=(0.5,1)),fit=cover,offset=(40mm,10mm))
"##, "/pptx.lay", host).unwrap();
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    assert_eq!(
        data.keys().filter(|n| n.starts_with("ppt/media/")).count(),
        1
    );
    assert_eq!(data["ppt/media/image1.jpeg"], jpeg);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert_eq!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "pic")
            .count(),
        2
    );
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "srcRect" && n.attribute("l") == Some("50000"))
    );
}

#[test]
fn group_selection_contains_acute_miter_stroke_ink() {
    let mut scene = compile("page=canvas(size=(160mm,100mm),background=\"none\")");
    scene.nodes.push(serde_json::json!({"kind":"group","id":"sharp","width":20.,"height":20.,"contentWidth":20.,"contentHeight":20.,"x":30.,"y":30.,"children":[{"kind":"path","id":"acute","d":"M0 20L10 0L11 20","width":11.,"height":20.,"intrinsicWidth":11.,"intrinsicHeight":20.,"fill":"none","strokeStyle":{"color":"#000000","width":2.,"join":"miter","miterLimit":40.}}]}));
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    let group = slide
        .descendants()
        .find(|n| n.tag_name().name() == "grpSp")
        .unwrap()
        .children()
        .find(|n| n.tag_name().name() == "grpSpPr")
        .unwrap();
    let height = group
        .descendants()
        .find(|n| n.tag_name().name() == "ext")
        .unwrap()
        .attribute("cy")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    assert!(
        height > 24 * 36_000,
        "acute miter ink exceeds a half-width inflated path box"
    );
}

#[test]
fn compound_stroke_outlines_stay_vector_with_transparent_gaps() {
    let scene = compile(
        r##"
page=canvas(size=(160mm,100mm),background="none")
page.add(rect(size=(30mm,20mm),border_radius=4mm,fill="#e6f0ff",border_color="#1268ff80",border_width=1.2mm,border_style=double),offset=(20mm,20mm),rotation=17deg)
"##,
    );
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    assert!(!data.keys().any(|n| n.starts_with("ppt/media/")));
    let slide = xml(&data, "ppt/slides/slide1.xml");
    assert!(
        slide
            .descendants()
            .any(|n| n.tag_name().name() == "custGeom")
    );
    assert!(
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "moveTo")
            .count()
            >= 4,
        "The double stroke retains its separate contours and gaps"
    );
}

#[test]
fn gradient_hard_stops_at_endpoints_preserve_stop_order() {
    let scene = compile(
        r##"
page=canvas(size=(160mm,100mm),background="none")
page.add(rect(size=(40mm,20mm),fill=linear_gradient(start=(0,0),end=(1,0),stops=[(0,"#ff0000"),(0,"#0000ff"),(1,"#0000ff"),(1,"#00ff00")]),border_width=0),offset=(10mm,10mm))
"##,
    );
    let (bytes, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = files(&bytes);
    let slide = xml(&data, "ppt/slides/slide1.xml");
    let colors = |position| {
        slide
            .descendants()
            .filter(|n| n.tag_name().name() == "gs" && n.attribute("pos") == Some(position))
            .map(|n| {
                n.descendants()
                    .find(|n| n.tag_name().name() == "srgbClr")
                    .unwrap()
                    .attribute("val")
                    .unwrap()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(colors("0").last(), Some(&"0000FF"));
    assert_eq!(colors("100000").first(), Some(&"0000FF"));
    assert_eq!(colors("100000").last(), Some(&"00FF00"));
}
