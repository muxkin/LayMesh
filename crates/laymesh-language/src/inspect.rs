//! Stable public scene inspection, with plot-local to page transforms in mm.
use laymesh_core::model::{Scene, jnum, jstr};
use serde_json::{Value as J, json};
fn multiply(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}
pub fn inspect_scene(scene: &Scene) -> J {
    fn visit(n: &J, parent: [f64; 6], path: String, inherited_clips: &[J], plots: &mut Vec<J>) {
        if jnum(n, "opacity", 1.) <= 0. {
            return;
        }
        let angle = jnum(n, "rotation", 0.).to_radians();
        let (c, s) = (angle.cos(), angle.sin());
        let (cx, cy) = (jnum(n, "width", 0.) / 2., jnum(n, "height", 0.) / 2.);
        let mut m = multiply(
            parent,
            [
                c,
                s,
                -s,
                c,
                jnum(n, "x", 0.) + cx - c * cx + s * cy,
                jnum(n, "y", 0.) + cy - s * cx - c * cy,
            ],
        );
        if jstr(n, "kind", "") != "group" {
            return;
        }
        m = multiply(
            m,
            [
                jnum(n, "width", 1.) / jnum(n, "contentWidth", jnum(n, "width", 1.)).max(1e-20),
                0.,
                0.,
                jnum(n, "height", 1.) / jnum(n, "contentHeight", jnum(n, "height", 1.)).max(1e-20),
                0.,
                0.,
            ],
        );
        let children = n["children"].as_array().cloned().unwrap_or_default();
        let mut clips = inherited_clips.to_vec();
        if n["clipPath"].is_object() {
            clips.push(json!({"transform":m,"path":n["clipPath"]}));
        } else if n["clip"].is_object() {
            clips.push(json!({"transform":m,"rect":{"x":jnum(&n["clip"],"x",0.),"y":jnum(&n["clip"],"y",0.),"width":jnum(&n["clip"],"width",jnum(n,"width",0.)),"height":jnum(&n["clip"],"height",jnum(n,"height",0.))}}));
        }
        let b = &n["plotBounds"];
        if b.is_object() {
            let mut axes = serde_json::Map::new();
            if let Some(am) = n["plotAxes"].as_object() {
                for (name, axis) in am {
                    let side = jstr(axis, "side", if name == "x" { "bottom" } else { "left" });
                    let horizontal = side == "top" || side == "bottom";
                    let range = if horizontal {
                        [0., jnum(b, "width", 0.)]
                    } else {
                        [jnum(b, "height", 0.), 0.]
                    };
                    let reverse = axis["reverse"].as_bool().unwrap_or(false);
                    let segments=axis["segments"].as_array().cloned().unwrap_or_else(||vec![json!({"domain":axis["domain"],"range":if reverse{[range[1],range[0]]}else{range}})]);
                    let offset = jnum(axis, "offset", 0.);
                    let position = if horizontal {
                        if side == "top" {
                            jnum(b, "y", 0.) - offset
                        } else {
                            jnum(b, "y", 0.) + jnum(b, "height", 0.) + offset
                        }
                    } else if side == "left" {
                        jnum(b, "x", 0.) - offset
                    } else {
                        jnum(b, "x", 0.) + jnum(b, "width", 0.) + offset
                    };
                    let coord = |v: f64| {
                        if horizontal {
                            [jnum(b, "x", 0.) + v, position]
                        } else {
                            [position, jnum(b, "y", 0.) + v]
                        }
                    };
                    let segs: Vec<J> = segments
                        .iter()
                        .map(|v| {
                            let mut v = v.clone();
                            let (a, z) = (
                                v["range"][0].as_f64().unwrap_or(0.),
                                v["range"][1].as_f64().unwrap_or(0.),
                            );
                            v["start"] = json!(coord(a));
                            v["end"] = json!(coord(z));
                            v["length"] = json!((z - a).abs());
                            v
                        })
                        .collect();
                    let gaps:Vec<J>=segments.windows(2).map(|vs|{let(a,z)=(vs[0]["range"][1].as_f64().unwrap_or(0.),vs[1]["range"][0].as_f64().unwrap_or(0.));json!({"domain":[vs[0]["domain"][1],vs[1]["domain"][0]],"start":coord(a),"end":coord(z),"width":(z-a).abs()})}).collect();
                    let mut a = axis.clone();
                    a["side"] = json!(side);
                    a["offset"] = json!(offset);
                    a["segments"] = json!(segs);
                    a["gaps"] = json!(gaps);
                    axes.insert(name.clone(), a);
                }
            }
            let mut plot = json!({"id":n["id"],"path":path,"plot_area":b,"axes":axes,"decorations":n.get("plotDecorations").cloned().unwrap_or(json!([])),"page_transform":m,"clips":clips});
            if n["plotProjection"].is_object() {
                plot["projection"] = n["plotProjection"].clone();
                if let Some(boundary) = children
                    .iter()
                    .find(|c| c["id"] == "plot-data")
                    .and_then(|c| c.get("clipPath"))
                {
                    plot["clip_boundary"] = boundary.clone();
                }
            }
            plots.push(plot);
        }
        for (i, child) in children.iter().enumerate() {
            visit(child, m, format!("{path}/{i}"), &clips, plots)
        }
    }
    let mut plots = vec![];
    for (i, n) in scene.nodes.iter().enumerate() {
        visit(n, [1., 0., 0., 1., 0., 0.], i.to_string(), &[], &mut plots)
    }
    json!({"schema_version":scene.schema_version,"units":"mm","page":{"width":scene.width,"height":scene.height,"unit":scene.canvas_unit,"layout_dpi":scene.layout_dpi},"plots":plots,"warnings":scene.warnings})
}
