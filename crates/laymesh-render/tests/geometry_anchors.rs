#![cfg(feature = "native")]
mod plot_support;
use laymesh_core::{
    engine::compile_source,
    model::{Host, jnum},
};
use laymesh_render::render_svg;
use plot_support::{pdf_raster, raster};

#[test]
fn geometry_selected_markers_remain_at_the_same_points_in_svg_png_and_pdf() {
    let scene=compile_source(r##"
page=canvas(size=(80mm,55mm),background="#ffffff")
a=page.add(ellipse(size=(30mm,20mm),fill="#d0e8f0",border_color="#287dc3",border_width=2mm),offset=(12mm,10mm),rotation=17deg)
page.add(rect(size=(1mm,1mm),fill="#ff0000"),anchor=center,target=a.bounds.top_left)
page.add(rect(size=(1mm,1mm),fill="#00cc00"),anchor=center,target=a.path.nearest(to=a.bounds.top_left)[0])
page.add(rect(size=(1mm,1mm),fill="#0000ff"),anchor=center,target=a.ink.boundary.nearest(to=a.bounds.bottom_right)[0])
b=page.add(line(dx=32mm,dy=0mm,line_width=4mm,line_cap="round"),offset=(22mm,42mm))
page.add(rect(size=(1mm,1mm),fill="#ff0000"),anchor=center,target=b.path.start)
page.add(rect(size=(1mm,1mm),fill="#00cc00"),anchor=center,target=b.ink.boundary.nearest(to=(20mm,42mm))[0])
page.add(line(dx=5mm,dy=0mm,line_color="#0000ff",line_width=0.6mm),anchor=self.path.start,target=a.path.at(fraction=0.3),rotation=a.path.at(fraction=0.3).tangent_angle,offset=(0mm,3mm),offset_space="target")
"##,"/exports.lay",Host::default()).unwrap();
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("<svg") && svg.contains("viewBox"));
    let samples = [
        (1, [255, 0, 0]),
        (2, [0, 204, 0]),
        (3, [0, 0, 255]),
        (5, [255, 0, 0]),
        (6, [0, 204, 0]),
    ];
    for rendered in [raster(&scene, 508.), pdf_raster(&scene, 508.)] {
        for (index, expected) in samples {
            let node = &scene.nodes[index];
            let pixel = rendered.pixel(jnum(node, "x", 0.) + 0.5, jnum(node, "y", 0.) + 0.5);
            assert!(
                pixel
                    .iter()
                    .zip(expected)
                    .all(|(&value, target)| (value as i32 - target).abs() < 8),
                "marker {index}: {pixel:?}"
            );
        }
    }
}
