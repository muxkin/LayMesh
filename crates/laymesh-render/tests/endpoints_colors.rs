#![cfg(feature = "native")]
mod plot_support;
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use laymesh_render::{render_pdf, render_png, render_svg};
fn scene(code: &str) -> Scene {
    compile_source(
        &format!("page=canvas(size=(80mm,60mm),background=\"#fff\")\n{code}"),
        "/render.lay",
        Host::default(),
    )
    .unwrap()
}
fn pixels(s: &Scene) -> (usize, usize, usize, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(render_png(s, 254.).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    (
        info.width as usize,
        info.height as usize,
        if info.color_type == png::ColorType::Rgba {
            4
        } else {
            3
        },
        data,
    )
}
#[test]
fn solid_and_translucent_head_seams_are_continuous_without_dark_overlap() {
    for color in ["#000000", "#00000080"] {
        let s = scene(&format!(
            "page.add(line(length=40mm,line_width=2mm,line_cap=\"round\",line_color=\"{color}\",end_head=head(size=(8mm,7mm))),offset=(10mm,20mm))"
        ));
        let (w, _, c, data) = pixels(&s);
        for x in [40., 41., 41.8, 42., 42.2, 43., 45.] {
            let i = (200 * w + (x * 10.) as usize) * c;
            let expected = if color.len() == 7 { 0 } else { 127 };
            assert!(
                (data[i] as i16 - expected).abs() < 3,
                "{color} x={x}: {:?}",
                &data[i..i + 3]
            );
        }
        assert!(render_pdf(&s).unwrap().starts_with(b"%PDF"));
        assert!(render_svg(&s).unwrap().contains("<path"));
    }
}
#[test]
fn alpha_multiplies_opacity_and_native_exports_accept_all_spaces() {
    let s = scene(
        "page.add(rect(size=(10mm,10mm),fill=rgb(255,0,0,alpha=0.5)),opacity=0.5,offset=(10mm,10mm))\npage.add(rect(size=(10mm,10mm),fill=hsv(120deg,1,1,alpha=0.5)),offset=(30mm,10mm))\npage.add(rect(size=(10mm,10mm),fill=oklch(0.7,0.15,200deg,alpha=0.6)),offset=(50mm,10mm))",
    );
    let (w, _, c, data) = pixels(&s);
    let i = (150 * w + 150) * c;
    assert_eq!(data[i], 255);
    assert!((data[i + 1] as i16 - 191).abs() < 2);
    let i = (150 * w + 350) * c;
    assert_eq!(data[i + 1], 255);
    assert!((data[i] as i16 - 128).abs() < 2);
    assert!(render_pdf(&s).unwrap().len() > 200);
}
#[test]
fn independent_colors_do_not_bleed_under_translucent_heads() {
    let s = scene(
        "page.add(line(length=40mm,line_width=2mm,line_color=\"#ff0000\",end_head=head(size=(8mm,7mm),fill=\"#0000ff80\")),offset=(10mm,20mm))",
    );
    let (w, _, c, data) = pixels(&s);
    let i = (200 * w + 440) * c;
    assert!(
        data[i] > 120 && data[i + 1] > 120 && data[i + 2] > 250,
        "{:?}",
        &data[i..i + 3]
    );
}

#[test]
fn rotated_and_curved_junctions_and_six_heads_match_pdf_png() {
    let mut code = String::new();
    for (angle, x, y) in [(0., 5., 8.), (90., 55., 5.), (45., 5., 20.)] {
        code.push_str(&format!("page.add(line(length=30mm,angle={angle}deg,line_width=2mm,line_color=\"#0008\",line_cap=\"round\",end_head=head(size=(6mm,5mm))),offset=({x}mm,{y}mm))\n"));
    }
    for (i, shape) in ["triangle", "open", "stealth", "dot", "diamond", "bar"]
        .iter()
        .enumerate()
    {
        code.push_str(&format!("page.add(line(length=0mm,angle=30deg,line_width=1mm,end_head=head(shape=\"{shape}\",size=(4mm,4mm))),offset=({}mm,55mm))\n",8+i*12));
    }
    code.push_str("page.add(path(commands=[move_to(0mm,0mm),cubic_to(c1x=8mm,c1y=0mm,c2x=18mm,c2y=12mm,x=30mm,y=12mm)],border_color=\"#0008\",border_width=2mm,border_cap=\"round\",end_head=head(size=(6mm,5mm))),offset=(36mm,33mm))");
    let s = scene(&code);
    let png = plot_support::raster(&s, 508.);
    let pdf = plot_support::pdf_raster(&s, 508.);
    for (x, y) in [
        (28.8, 8.),
        (29.2, 8.),
        (55., 28.8),
        (55., 29.2),
        (22., 37.),
        (60.1, 45.),
    ] {
        for p in [png.pixel(x, y), pdf.pixel(x, y)] {
            assert!(
                p.iter().all(|v| (*v as i32 - 119).abs() < 8),
                "junction ({x},{y}) {p:?}"
            );
        }
    }
    // Interior, exterior and direction samples independently compare both backends.
    for y in [53., 54., 55., 56.] {
        for x in 2..78 {
            let a = png.pixel(x as f64, y);
            let b = pdf.pixel(x as f64, y);
            assert!(
                a.iter()
                    .zip(b)
                    .all(|(a, b)| (*a as i32 - b as i32).abs() < 32),
                "head ({x},{y}) {a:?}/{b:?}"
            );
        }
    }
    assert!(render_svg(&s).unwrap().contains("rgba(0,0,0,"));
}

#[test]
fn transparent_gradient_shares_rgb_interpolation() {
    let s = scene(
        "g=linear_gradient(stops=[(0,rgb(255,0,0,alpha=0.5)),(1,hsv(120deg,1,1,alpha=0.5))])\npage.add(rect(size=(40mm,10mm),fill=g),offset=(10mm,10mm))",
    );
    for rendered in [
        plot_support::raster(&s, 254.),
        plot_support::pdf_raster(&s, 254.),
    ] {
        let p = rendered.pixel(30., 15.);
        assert!(
            (p[0] as i32 - 191).abs() < 3
                && (p[1] as i32 - 191).abs() < 3
                && (p[2] as i32 - 128).abs() < 3,
            "{p:?}"
        );
    }
}

#[test]
fn closed_hollow_heads_keep_the_shaft_outside_their_interior() {
    let s = scene(
        "page.add(line(length=40mm,line_width=2mm,end_head=head(size=(8mm,7mm),fill=\"none\",border_color=\"#000\",border_width=0.5mm)),offset=(10mm,20mm))\npage.add(line(length=40mm,line_width=2mm,end_head=head(shape=\"dot\",size=(8mm,7mm),fill=\"none\",border_color=\"#000\",border_width=0.5mm)),offset=(10mm,40mm))",
    );
    for rendered in [
        plot_support::raster(&s, 508.),
        plot_support::pdf_raster(&s, 508.),
    ] {
        assert_eq!(rendered.pixel(46., 20.), [255; 3]);
        assert_eq!(rendered.pixel(49., 40.), [255; 3]);
        assert_eq!(rendered.pixel(39., 20.), [0; 3]);
    }
}

#[test]
fn centered_heads_join_across_the_stroke_width_at_both_ends() {
    for shape in ["dot", "diamond"] {
        let s = scene(&format!(
            "page.add(line(length=40mm,line_width=2mm,line_color=\"#0008\",start_head=head(shape=\"{shape}\",size=(8mm,7mm)),end_head=head(shape=\"{shape}\",size=(8mm,7mm))),offset=(10mm,20mm))"
        ));
        for raster in [
            plot_support::raster(&s, 508.),
            plot_support::pdf_raster(&s, 508.),
        ] {
            for x in [13.9, 14.1, 45.9, 46.1] {
                for y in [19.2, 19.6, 20., 20.4, 20.8] {
                    assert!(
                        raster
                            .pixel(x, y)
                            .iter()
                            .all(|v| (*v as i32 - 119).abs() < 8),
                        "{shape} ({x},{y}): {:?}",
                        raster.pixel(x, y)
                    );
                }
            }
        }
    }
}

#[test]
fn pointed_head_clips_wide_strokes_and_preserves_original_endpoints() {
    use kurbo::{Point, Shape};
    for shape in ["triangle", "open", "stealth"] {
        for width in [2., 8.] {
            let s = scene(&format!(
                "a=page.add(line(length=40mm,line_width={width}mm,start_head=head(shape=\"{shape}\",size=(8mm,6mm),border_width=0.3mm),end_head=head(shape=\"{shape}\",size=(8mm,6mm),border_width=0.3mm)),offset=(10mm,20mm))\npage.add(rect(size=(1,1)),target=a.path.start)\npage.add(rect(size=(1,1)),target=a.path.end)"
            ));
            assert_eq!(s.nodes[1]["x"], 10.);
            assert_eq!(s.nodes[2]["x"], 50.);
            let ink = laymesh_core::geometry::visible(&s.nodes[0]);
            for x in [10.2, 49.8] {
                assert_eq!(
                    ink.winding(Point::new(x, 20.7)),
                    0,
                    "{shape} width={width} ({x},20.7)"
                );
                assert_ne!(
                    ink.winding(Point::new(x, 20.)),
                    0,
                    "{shape} missing tip interior"
                );
            }
            assert!(ink.bounding_box().x0 >= 9.5 && ink.bounding_box().x1 <= 50.5);
        }
    }
}

#[test]
fn clipping_keeps_earlier_self_crossings_and_original_node_joins() {
    use kurbo::{Point, Shape};
    let s = scene(
        "page.add(polyline(points=[(0mm,0mm),(40mm,0mm),(40mm,10mm),(0mm,10mm),(0mm,0.4mm),(40mm,0.4mm)],line_width=2mm,end_head=head(size=(8mm,6mm))),offset=(10mm,20mm))\npage.add(polyline(points=[(0mm,0mm),(32mm,0mm),(40mm,0mm)],line_width=2mm,start_head=head(size=(8mm,6mm)),end_head=head(size=(8mm,6mm))),offset=(10mm,40mm))",
    );
    let ink = laymesh_core::geometry::visible(&s.nodes[0]);
    // The earlier traversal next to the final tip retains its own stroke.
    assert_ne!(ink.winding(Point::new(49.8, 19.5)), 0);
    let ink = laymesh_core::geometry::visible(&s.nodes[1]);
    for x in [41.9, 42., 42.1] {
        assert_ne!(ink.winding(Point::new(x, 40.7)), 0);
    }
}

#[test]
fn curved_centered_heads_keep_a_full_width_connection() {
    use kurbo::{Affine, BezPath, Shape};
    let s = scene(
        "page.add(path(commands=[move_to(0mm,8mm),cubic_to(c1x=12mm,c1y=-4mm,c2x=32mm,c2y=20mm,x=47mm,y=8mm)],border_color=\"#000\",border_width=1.5mm,start_head=head(shape=\"dot\",size=(3mm,3mm)),end_head=head(shape=\"dot\",size=(3mm,3mm))),offset=(10mm,20mm))",
    );
    let n = &s.nodes[0];
    let mut raw = n.clone();
    raw.as_object_mut().unwrap().remove("endpointRecipe");
    let mut expected = laymesh_core::geometry::visible(&raw);
    let p = BezPath::from_svg(n["d"].as_str().unwrap()).unwrap();
    let tr = laymesh_core::geometry::node_transform(n);
    let first = p.elements().first().unwrap().end_point().unwrap();
    let last = p.elements().last().unwrap().end_point().unwrap();
    for point in [first, last] {
        let head = tr * kurbo::Circle::new(point, 1.5).to_path(0.00001);
        expected = laymesh_core::geometry::union(&expected, &head);
    }
    let actual = laymesh_core::geometry::visible_transformed(n, Affine::IDENTITY);
    assert!(
        (expected.area().abs() - actual.area().abs()).abs() < 0.004,
        "expected {} actual {}",
        expected.area(),
        actual.area()
    );
}
