#![cfg(feature = "native")]
use kurbo::{BezPath, Rect, Shape};
use laymesh_core::{engine::Engine, model::Host};
use laymesh_render::{
    ExportOptions, display_list::*, render_export, render_png, surface3d, to_display_list,
};
use serde_json::json;
fn compile(source: &str) -> laymesh_core::model::Scene {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    Engine::new(host).compile(source, "/test.lay").unwrap()
}
fn pixels(source: &str) -> image::RgbaImage {
    image::load_from_memory(&render_png(&compile(source), 254.).unwrap())
        .unwrap()
        .to_rgba8()
}
fn path_node(p: BezPath) -> DrawNode {
    DrawNode::new(DrawKind::Path {
        path: p,
        fill: Paint::Solid { rgba: [1.; 4] },
        stroke: None,
        even_odd: true,
    })
}
#[test]
fn reflection_fades_below_subject_and_soft_edges_preserve_color() {
    let im = pixels(
        "p=canvas(size=(30mm,40mm))\np.add(rect(size=(10mm,10mm),fill=\"#ff0000\",effects=[reflection(distance=2mm,length=1,opacity=0.8)]),offset=(5mm,5mm))",
    );
    assert_eq!(im.get_pixel(80, 80).0, [255, 0, 0, 255]);
    assert_eq!(im.get_pixel(80, 160)[3], 0);
    assert!(im.get_pixel(80, 175)[3] > im.get_pixel(80, 240)[3]);
    assert_eq!(im.get_pixel(80, 275)[3], 0);
    let im = pixels(
        "p=canvas(size=(30mm,30mm))\np.add(ellipse(size=(16mm,16mm),fill=\"#ff0000\",effects=[soft_edge(radius=2mm)]),offset=(5mm,5mm))",
    );
    assert!(im.get_pixel(130, 130)[3] > 250);
    assert!(im.get_pixel(130, 55)[3] < 150);
    assert_eq!(im.get_pixel(55, 55)[3], 0);
    assert!(im.get_pixel(130, 55)[0] > 245);
}
#[test]
fn zero_soft_edge_is_identity() {
    let plain = "p=canvas(size=(30mm,30mm))\np.add(rect(size=(10mm,10mm),fill=\"#0088ff\"{effects}),offset=(5mm,5mm))";
    assert_eq!(
        pixels(&plain.replace("{effects}", "")).as_raw(),
        pixels(&plain.replace("{effects}", ",effects=[soft_edge(radius=0mm)]")).as_raw()
    );
}
#[test]
fn extrusion_preserves_holes_and_raster_tiles_agree() {
    let mut path = Rect::new(0., 0., 20., 20.).to_path(0.01);
    path.extend(Rect::new(6., 6., 14., 14.).to_path(0.01).iter());
    let node = path_node(path);
    let mesh = surface3d::prepare(&node, &json!({"depth":4,"material":"flat"})).unwrap();
    let texture = vec![255; 200 * 200 * 4];
    let all = surface3d::raster(&mesh, &texture, 200, 200, [0., 0., 20., 20.], 200, 200).unwrap();
    assert_eq!(all[(100 * 200 + 100) * 4 + 3], 0);
    assert_eq!(all[(30 * 200 + 30) * 4 + 3], 255);
    let left = surface3d::raster(&mesh, &texture, 200, 200, [0., 0., 10., 20.], 100, 200).unwrap();
    let right =
        surface3d::raster(&mesh, &texture, 200, 200, [10., 0., 20., 20.], 100, 200).unwrap();
    for y in 0..200 {
        assert_eq!(&all[y * 800..y * 800 + 400], &left[y * 400..(y + 1) * 400]);
        assert_eq!(
            &all[y * 800 + 400..(y + 1) * 800],
            &right[y * 400..(y + 1) * 400]
        );
    }
}
#[test]
fn rotated_solid_has_side_faces_and_back_face() {
    let node = path_node(Rect::new(0., 0., 20., 10.).to_path(0.01));
    let front = surface3d::prepare(&node, &json!({"depth":5,"rotation":[0.,-0.6,0.]})).unwrap();
    assert!(front.triangles.len() >= 4, "side faces must not be culled");
    let back = surface3d::prepare(
        &node,
        &json!({"depth":5,"rotation":[0.,std::f64::consts::PI,0.]}),
    )
    .unwrap();
    assert_eq!(back.triangles.len(), 2);
    assert!(
        back.triangles
            .iter()
            .all(|t| t.v.iter().all(|v| v.p[2] > 0.))
    );
    assert!(surface3d::prepare(&node, &json!({"depth":5,"bevelTop":[12,1]})).is_err());
    assert!(
        surface3d::prepare(
            &node,
            &json!({"depth":5,"projection":"perspective","distance":1})
        )
        .is_err()
    );
}
#[test]
fn concave_shape_and_bevels_make_real_geometry() {
    let node = path_node(BezPath::from_svg("M0 0L20 0L20 8L8 8L8 20L0 20Z").unwrap());
    let mesh = surface3d::prepare(
        &node,
        &json!({"depth":4,"bevelTop":[1,1],"bevelBottom":[0.5,0.5],"rotation":[0.3,-0.5,0.]}),
    )
    .unwrap();
    assert!(mesh.triangles.len() > 12);
    assert!(
        mesh.triangles
            .iter()
            .flat_map(|t| &t.v)
            .all(|v| v.p.iter().all(|x| x.is_finite()))
    );
}
#[test]
fn constructors_reject_invalid_parameters() {
    for expression in [
        "reflection(length=2)",
        "reflection(blur=-1mm)",
        "soft_edge(radius=-1mm)",
        "three_d(rotation=(1deg,2deg))",
        "three_d(light_direction=(0,0,0))",
        "three_d(distance=0mm)",
        "three_d(depth=1mm,bevel_top=(1mm,2mm))",
        "three_d(material=\"unknown\")",
    ] {
        assert!(
            Engine::new(Host::default())
                .compile(
                    &format!("x={expression}\np=canvas(size=(20mm,20mm))"),
                    "bad.lay"
                )
                .is_err(),
            "{expression}"
        );
    }
}
#[test]
fn all_thirty_presets_compile_and_produce_valid_drawings() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/effects/presets");
    let mut count = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) != Some("lay") {
            continue;
        }
        let s = compile(&std::fs::read_to_string(&p).unwrap());
        to_display_list(&s).unwrap_or_else(|e| panic!("{}: {e:?}", p.display()));
        count += 1;
        assert!(
            !s.warnings.iter().any(|w| w.code == "W_LCSS"),
            "{}",
            p.display()
        );
    }
    assert_eq!(count, 30);
}
#[test]
fn effect_export_keeps_unrelated_text_editable() {
    let scene = compile(
        "p=canvas(size=(80mm,60mm))\np.add(text(\"Editable\",font_family=\"/font.ttf\",font_size=12pt),offset=(5mm,5mm))\np.add(rect(size=(20mm,10mm),fill=\"#437bc5\",three_d=three_d(depth=3mm,rotation=(20deg,30deg,0deg))),offset=(10mm,20mm))",
    );
    let opts = ExportOptions {
        dpi: Some(100.),
        ..Default::default()
    };
    for format in ["png", "pdf", "svg", "pptx"] {
        assert!(!render_export(&scene, format, &opts).unwrap().is_empty());
    }
    let bytes = render_export(&scene, "pptx", &opts).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    use std::io::Read;
    zip.by_name("ppt/slides/slide1.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("Editable"));
    assert!(xml.contains("<p:pic>"));
}

#[test]
fn chinese_formula_stroke_and_rounded_cropped_image_generate_surfaces() {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    host.files.insert(
        "/cjk.otf".into(),
        include_bytes!("../../../tests/fonts/text/CJK-Regular.otf").to_vec(),
    );
    host.files.insert(
        "/photo.png".into(),
        include_bytes!("../../../examples/gallery/assets/sample.png").to_vec(),
    );
    for expression in [
        r##"text("OB8",font_family="/font.ttf",font_size=10mm,text_stroke_width=0.3mm,text_stroke_color="#203050",three_d=three_d(depth=2mm,rotation=(20deg,-25deg,0deg)))"##,
        r##"text("中文",font_family="/cjk.otf",font_size=10mm,three_d=three_d(depth=2mm,rotation=(15deg,140deg,0deg)))"##,
        r##"formula(r"\frac{x^2+1}{\sqrt{y}}",font_size=24pt,three_d=three_d(depth=1mm,rotation=(20deg,-20deg,0deg)))"##,
        r##"image(src="/photo.png",size=(30mm,20mm),border_radius=3mm,three_d=three_d(depth=2mm,rotation=(20deg,145deg,0deg))),crop=box(offset=(0.2,0.1),size=(0.6,0.8))"##,
    ] {
        let scene = Engine::new(host.clone())
            .compile(
                &format!("p=canvas(size=(70mm,55mm))\np.add({expression},offset=(15mm,15mm))"),
                "/test.lay",
            )
            .unwrap();
        let list = to_display_list(&scene).unwrap();
        fn count(n: &DrawNode) -> usize {
            usize::from(n.hint["surface3d"].is_object())
                + match &n.kind {
                    DrawKind::Group { children, .. } => children.iter().map(count).sum(),
                    _ => 0,
                }
        }
        assert_eq!(
            list.nodes.iter().map(count).sum::<usize>(),
            1,
            "{expression}"
        );
        let im = image::load_from_memory(&render_png(&scene, 127.).unwrap())
            .unwrap()
            .to_rgba8();
        assert!(
            im.pixels().filter(|p| p[3] > 128).count() > 100,
            "{expression}"
        );
    }
}
#[test]
fn projected_bounds_and_mesh_updates_preserve_layout_and_old_lists() {
    let src = "p=canvas(size=(90mm,60mm))\np.add(rect(size=(20mm,15mm),fill=\"#579cb4\",three_d=three_d(depth=4mm,rotation=(20deg,ANGLE,0deg))),offset=(10mm,10mm))\np.add(rect(size=(20mm,15mm),fill=\"#b49557\",three_d=three_d(depth=4mm)),offset=(50mm,10mm))";
    let a = compile(&src.replace("ANGLE", "25deg"));
    let b = compile(&src.replace("ANGLE", "50deg"));
    assert_eq!(a.nodes[0]["width"], b.nodes[0]["width"]);
    assert_eq!(a.nodes[0]["height"], b.nodes[0]["height"]);
    let old = to_display_list(&a).unwrap();
    let next = laymesh_render::display_list::to_display_list_reusing(&b, &a, &old, false).unwrap();
    assert_eq!(old.nodes[1], next.nodes[1]);
    assert_ne!(old.nodes[0].bounds(), next.nodes[0].bounds());
    let update = laymesh_render::display_update::DisplayUpdate::between(&old, &next);
    assert_eq!(update.nodes.len(), 1);
    let mut actual = old;
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, next);
    for version in [1, 2] {
        let mut legacy = to_display_list(&compile(
            "p=canvas(size=(40mm,40mm))\np.add(rect(size=(20mm,20mm),effects=[shadow()]))",
        ))
        .unwrap();
        legacy.schema_version = version;
        let mut json = serde_json::to_value(&legacy).unwrap();
        for e in json["nodes"][0]["effects"].as_array_mut().unwrap() {
            e.as_object_mut().unwrap().remove("softEdge");
        }
        let decoded: DisplayList = serde_json::from_value(json).unwrap();
        assert!(
            laymesh_render::raster::render_region(&decoded, Rect::new(0., 0., 40., 40.), 200, 200)
                .is_ok()
        );
    }
}
#[test]
fn high_density_texture_tiles_match_whole_and_enforce_memory_limits() {
    let node = path_node(Rect::new(0., 0., 20., 15.).to_path(0.01));
    let mesh=surface3d::prepare(&node,&json!({"depth":4,"rotation":[0.3,-0.5,0.],"projection":"perspective","distance":70,"material":"flat"})).unwrap();
    let density = 20.;
    let b = mesh.bounds;
    let region = [
        (b[0] * density).floor() / density,
        (b[1] * density).floor() / density,
        (b[2] * density).ceil() / density,
        (b[3] * density).ceil() / density,
    ];
    let w = ((region[2] - region[0]) * density).round() as u32;
    let h = ((region[3] - region[1]) * density).round() as u32;
    let render = |r: [f64; 4], w, h| {
        let source = surface3d::texture_region(&mesh, r, density).unwrap();
        let tw = ((source[2] - source[0]) * density).round() as u32;
        let th = ((source[3] - source[1]) * density).round() as u32;
        let mut texture = vec![0; tw as usize * th as usize * 4];
        for y in 0..th {
            for x in 0..tw {
                let at = (y * tw + x) as usize * 4;
                texture[at] =
                    ((source[0] + (x as f64 + 0.5) / density) * 10.).clamp(0., 255.) as u8;
                texture[at + 1] =
                    ((source[1] + (y as f64 + 0.5) / density) * 12.).clamp(0., 255.) as u8;
                texture[at + 2] = 160;
                texture[at + 3] = 255;
            }
        }
        let mut m = mesh.clone();
        m.source = source;
        surface3d::raster(&m, &texture, tw, th, r, w, h).unwrap()
    };
    let whole = render(region, w, h);
    let split = w / 2;
    for (x, tw) in [(0, split), (split, w - split)] {
        let tile = render(
            [
                region[0] + x as f64 / density,
                region[1],
                region[0] + (x + tw) as f64 / density,
                region[3],
            ],
            tw,
            h,
        );
        for y in 0..h {
            let a = &whole[((y * w + x) * 4) as usize..((y * w + x + tw) * 4) as usize];
            let b = &tile[(y * tw * 4) as usize..((y + 1) * tw * 4) as usize];
            assert!(a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1));
        }
    }
    assert!(surface3d::raster(&mesh, &[255; 4], 1, 1, region, 4096, 4096).is_err());
}

#[test]
fn native_3d_output_has_no_seam_at_1024_pixel_tile_boundary() {
    let source = "p=canvas(size=(70mm,40mm))\np.add(rect(size=(60mm,30mm),fill=linear_gradient(stops=[(0,\"#215ab3\"),(1,\"#e5aa57\")])PARAM),offset=(5mm,5mm))";
    let raster = |param: &str| {
        image::load_from_memory(
            &render_png(&compile(&source.replace("PARAM", param)), 508.).unwrap(),
        )
        .unwrap()
        .to_rgba8()
    };
    let plain = raster("");
    let projected = raster(",three_d=three_d(depth=2mm,material=\"flat\")");
    for y in 120..680 {
        for x in 1118..1130 {
            assert!(
                plain
                    .get_pixel(x, y)
                    .0
                    .iter()
                    .zip(projected.get_pixel(x, y).0)
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "tile seam at {x},{y}"
            );
        }
    }
}

#[test]
fn rounded_image_extrudes_the_clipped_contour() {
    let mut host = Host::default();
    host.files.insert(
        "/photo.png".into(),
        include_bytes!("../../../examples/gallery/assets/sample.png").to_vec(),
    );
    let scene=Engine::new(host).compile("p=canvas(size=(35mm,35mm))\np.add(image(src=\"/photo.png\",size=(20mm,20mm),border_radius=4mm,three_d=three_d(depth=2mm,material=\"flat\")),fit=\"stretch\",offset=(5mm,5mm))","/round.lay").unwrap();
    let im = image::load_from_memory(&render_png(&scene, 254.).unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(im.get_pixel(53, 53)[3], 0);
    assert!(im.get_pixel(150, 60)[3] > 250);
    assert!(im.get_pixel(150, 150)[3] > 250);
}

#[test]
fn material_alpha_is_independent_of_geometric_coverage() {
    let coverage = [
        255, 255, 255, 64, 255, 255, 255, 128, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0,
    ];
    let color = [
        30, 140, 180, 32, 30, 140, 180, 64, 30, 140, 180, 128, 30, 140, 180, 0, 0, 0, 0, 0,
    ];
    let material = surface3d::material_texture(&color, &coverage, 5, 1, None).unwrap();
    assert_eq!(material[3], 128);
    assert_eq!(material[7], 128);
    assert_eq!(material[11], 128);
    assert_eq!(
        material[15], 0,
        "intrinsic transparent image pixels remain transparent"
    );
    assert_eq!(
        material[19], 0,
        "gutter copies material alpha, never forces opacity"
    );
    let node = path_node(Rect::new(0., 0., 8., 8.).to_path(0.01));
    let mesh = surface3d::prepare(
        &node,
        &json!({"depth":3,"rotation":[0.3,-0.5,0.],"material":"flat"}),
    )
    .unwrap();
    let rgba = surface3d::raster(&mesh, &[30, 140, 180, 128], 1, 1, mesh.bounds, 160, 160).unwrap();
    assert_eq!(rgba.chunks_exact(4).map(|p| p[3]).max(), Some(128));
    assert!(rgba.chunks_exact(4).filter(|p| p[3] == 128).count() > 10000);
}

#[test]
fn refinement_rebuilds_curves_and_preserves_real_corners_and_legacy_meshes() {
    let curve = path_node(kurbo::RoundedRect::new(0., 0., 40., 25., 5.).to_path(0.001));
    let base=surface3d::prepare(&curve,&json!({"depth":4,"rotation":[0.3,-0.5,0.],"bevelTop":[1,1],"projection":"perspective","distance":100})).unwrap();
    let refined = surface3d::refine(&base, 90.).unwrap();
    assert!(refined.triangles.len() > base.triangles.len());
    assert!(refined.tolerance * 90. * refined.texture_scale <= 0.25);
    assert!(
        refined
            .triangles
            .iter()
            .any(|t| t.v[0].normal != t.v[1].normal),
        "smooth curves interpolate normals"
    );
    let square = surface3d::prepare(
        &path_node(Rect::new(0., 0., 20., 10.).to_path(0.001)),
        &json!({"depth":4,"rotation":[0.3,-0.5,0.]}),
    )
    .unwrap();
    assert!(
        square
            .triangles
            .iter()
            .all(|t| t.v[0].normal == t.v[1].normal && t.v[1].normal == t.v[2].normal),
        "true corners retain flat face normals"
    );
    let mut legacy = serde_json::to_value(&square).unwrap();
    for k in [
        "recipe",
        "coverage",
        "material_alpha",
        "lighting",
        "tolerance",
        "texture_scale",
    ] {
        legacy.as_object_mut().unwrap().remove(k);
    }
    for t in legacy["triangles"].as_array_mut().unwrap() {
        for v in t["v"].as_array_mut().unwrap() {
            v.as_object_mut().unwrap().remove("normal");
        }
    }
    let old: surface3d::Mesh = serde_json::from_value(legacy).unwrap();
    assert_eq!(
        surface3d::refine(&old, 90.).unwrap().triangles.len(),
        old.triangles.len()
    );
    assert!(
        surface3d::raster(&old, &[255; 4], 1, 1, old.bounds, 100, 100)
            .unwrap()
            .iter()
            .any(|v| *v > 0)
    );
}

#[test]
fn coverage_sampling_has_sixteen_levels_and_keeps_thin_edges() {
    let mesh = surface3d::prepare(
        &path_node(kurbo::Circle::new((5., 5.), 4.).to_path(0.001)),
        &json!({"material":"flat"}),
    )
    .unwrap();
    let rgba = surface3d::raster(&mesh, &[255; 4], 1, 1, [0., 0., 10., 10.], 37, 37).unwrap();
    let levels: std::collections::BTreeSet<_> = rgba
        .chunks_exact(4)
        .map(|p| p[3])
        .filter(|a| *a > 0 && *a < 255)
        .collect();
    assert!(
        levels.len() > 4,
        "4x4 coverage must provide more edge levels than 2x2: {levels:?}"
    );
}

#[test]
fn opaque_curved_sides_have_no_coverage_stripes_and_image_alpha_survives() {
    let src = "p=canvas(size=(60mm,45mm))\np.add(rect(size=(35mm,22mm),border_radius=5mm,fill=\"#48a9a680\",three_d=three_d(depth=5mm,rotation=(20deg,-30deg,0deg),material=\"flat\")),offset=(10mm,10mm))";
    let im = pixels(src);
    // Every interior pixel of a flat half-transparent solid has the same
    // material alpha. A partial 2D contour texel must not stripe the side wall.
    let mut interior = 0;
    for y in 1..im.height() - 1 {
        for x in 1..im.width() - 1 {
            if (-1..=1).all(|dy| {
                (-1..=1).all(|dx| {
                    im.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[3] >= 120
                })
            }) {
                interior += 1;
                assert!(
                    im.get_pixel(x, y)[3].abs_diff(128) <= 1,
                    "coverage stripe at {x},{y}: {}",
                    im.get_pixel(x, y)[3]
                );
            }
        }
    }
    assert!(interior > 40000);
    let image = image::RgbaImage::from_fn(20, 20, |x, _| {
        image::Rgba([50, 150, 200, if x < 10 { 0 } else { 96 }])
    });
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let mut host = Host::default();
    host.files.insert("/alpha.png".into(), png.into_inner());
    let scene=Engine::new(host).compile("p=canvas(size=(30mm,30mm))\np.add(image(src=\"/alpha.png\",size=(20mm,20mm),three_d=three_d(material=\"flat\")),offset=(5mm,5mm))","/test.lay").unwrap();
    let im = image::load_from_memory(&render_png(&scene, 254.).unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(im.get_pixel(80, 150)[3], 0);
    assert_eq!(im.get_pixel(200, 150)[3], 96);
}

#[test]
fn curved_side_color_does_not_amplify_premultiplied_edge_quantization() {
    // A genuine red/blue texture boundary is not a rounding error. Recovery
    // must preserve it even when geometry coverage is partial.
    let texture = [255, 0, 0, 64, 0, 0, 255, 255];
    let coverage = [255, 255, 255, 64, 255, 255, 255, 255];
    let recovered = surface3d::material_texture(&texture, &coverage, 2, 1, None).unwrap();
    assert_eq!(&recovered[..4], &[255, 0, 0, 255]);
    let src = "p=canvas(size=(60mm,45mm))\np.add(rect(size=(35mm,22mm),border_radius=5mm,fill=\"#48a9a6\",three_d=three_d(depth=5mm,rotation=(20deg,-30deg,0deg),material=\"flat\")),offset=(10mm,10mm))";
    let im = pixels(src);
    let mut interior = 0;
    let mut worst = 0;
    for y in 1..im.height() - 1 {
        for x in 1..im.width() - 1 {
            if (-1..=1).all(|dy| {
                (-1..=1).all(|dx| {
                    im.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[3] == 255
                })
            }) {
                interior += 1;
                for (actual, expected) in im.get_pixel(x, y).0[..3].iter().zip([72, 169, 166]) {
                    worst = worst.max(actual.abs_diff(expected));
                }
            }
        }
    }
    assert!(interior > 40000);
    assert!(
        worst <= 2,
        "constant material developed color stripes: {worst}"
    );
    for alpha in [26u8, 128, 200] {
        let im = pixels(&src.replace("#48a9a6", &format!("#48a9a6{alpha:02x}")));
        let mut worst = 0f64;
        let mut interior = 0;
        for y in 1..im.height() - 1 {
            for x in 1..im.width() - 1 {
                if (-1..=1).all(|dy| {
                    (-1..=1).all(|dx| {
                        im.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[3] == alpha
                    })
                }) {
                    interior += 1;
                    let pixel = im.get_pixel(x, y);
                    for (actual, expected) in pixel.0[..3].iter().zip([72u8, 169, 166]) {
                        worst = worst.max(actual.abs_diff(expected) as f64 * alpha as f64 / 255.);
                    }
                }
            }
        }
        assert!(
            interior > 40000,
            "alpha {alpha}: missing translucent interior"
        );
        assert!(
            worst <= 2.,
            "alpha {alpha}: premultiplied color stripe {worst}"
        );
    }
}
