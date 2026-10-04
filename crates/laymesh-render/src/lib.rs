//! SVG is shared by native and WASM. Native exports use resvg and krilla.
use base64::Engine;
use laymesh_core::{
    Diagnostic, Loc, Result,
    model::{Scene, escape, jnum, jstr},
};
use serde_json::{Value as Json, json};
fn number(n: f64) -> String {
    if n == 0. { "0".into() } else { n.to_string() }
}
fn n(v: &Json, k: &str, d: f64) -> String {
    number(jnum(v, k, d))
}
fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("E_RENDER", message, "", Loc::default())
}
fn pair(v: &Json, default: [f64; 2]) -> [f64; 2] {
    [
        v[0].as_f64().unwrap_or(default[0]),
        v[1].as_f64().unwrap_or(default[1]),
    ]
}
fn paint(p: &Json, defs: &mut Vec<String>) -> String {
    if let Some(s) = p.as_str() {
        return escape(&laymesh_core::color::css(s));
    }
    if p.is_null() {
        return "none".into();
    }
    let id = format!("paint-{}", defs.len());
    let kind = jstr(p, "kind", "");
    match kind {
        "pattern" | "hatch" => {
            let size = jnum(p, "spacing", 2.).max(0.001);
            let sw = jnum(p, "lineWidth", jnum(p, "line_width", 0.2));
            let color = escape(&laymesh_core::color::css(jstr(p, "color", "#000000")));
            let bg = escape(&laymesh_core::color::css(jstr(p, "background", "none")));
            let mark = if jstr(p, "pattern", jstr(p, "style", "lines")) == "dots" {
                format!(
                    "<circle cx='{}' cy='{}' r='{}' fill='{color}'/>",
                    size / 2.,
                    size / 2.,
                    sw / 2.
                )
            } else {
                let mut d = format!(
                    "M{} {}L{} {}M0 {}L{} 0M0 {}L{} 0",
                    -size,
                    size,
                    size,
                    -size,
                    size,
                    size,
                    size * 2.,
                    size * 2.
                );
                if jstr(p, "pattern", jstr(p, "style", "lines")) == "cross" {
                    d += &format!(
                        "M{} 0L{} {}M0 0L{} {}M0 {}L{} {}",
                        -size,
                        size,
                        2. * size,
                        size,
                        size,
                        -size,
                        2. * size,
                        size
                    );
                }
                format!("<path d='{d}' fill='none' stroke='{color}' stroke-width='{sw}'/>")
            };
            defs.push(format!("<pattern id='{id}' patternUnits='userSpaceOnUse' width='{size}' height='{size}' patternTransform='rotate({})'><rect width='{size}' height='{size}' fill='{bg}'/>{mark}</pattern>",n(p,"angle",0.)));
        }
        "imagePaint" | "image_paint" | "image_fill" => {
            let aspect = if jstr(p, "fit", "contain") == "stretch" {
                "none"
            } else if p["fit"] == "cover" {
                "xMidYMid slice"
            } else {
                "xMidYMid meet"
            };
            defs.push(format!("<pattern id='{id}' width='1' height='1' patternContentUnits='objectBoundingBox'><svg width='1' height='1' viewBox='0 0 {} {}' preserveAspectRatio='{aspect}'><image width='{}' height='{}' href='data:{};base64,{}'/></svg></pattern>",n(p,"width",1.),n(p,"height",1.),n(p,"width",1.),n(p,"height",1.),escape(jstr(p,"mime","image/png")),jstr(p,"data","")));
        }
        _ => {
            let stops = p["stops"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|s| {
                    let at = s
                        .get("at")
                        .and_then(Json::as_f64)
                        .or_else(|| s[0].as_f64())
                        .unwrap_or(0.);
                    let color = s
                        .get("color")
                        .and_then(Json::as_str)
                        .or_else(|| s[1].as_str())
                        .unwrap_or("#000000");
                    let opacity = s
                        .get("opacity")
                        .and_then(Json::as_f64)
                        .or_else(|| s[2].as_f64())
                        .unwrap_or(1.);
                    format!(
                        "<stop offset='{}%' stop-color='{}' stop-opacity='{}'/>",
                        number(at * 100.),
                        escape(&laymesh_core::color::css(color)),
                        number(opacity)
                    )
                })
                .collect::<String>();
            if matches!(kind, "linearGradient" | "linear_gradient") {
                let a = pair(&p["start"], [0., 0.]);
                let b = pair(&p["end"], [1., 1.]);
                defs.push(format!("<linearGradient id='{id}' x1='{}%' y1='{}%' x2='{}%' y2='{}%'>{stops}</linearGradient>",a[0]*100.,a[1]*100.,b[0]*100.,b[1]*100.));
            } else {
                let c = pair(&p["center"], [0.5, 0.5]);
                defs.push(format!(
                    "<radialGradient id='{id}' cx='{}%' cy='{}%' r='{}%'>{stops}</radialGradient>",
                    c[0] * 100.,
                    c[1] * 100.,
                    jnum(p, "radius", 0.5) * 100.
                ));
            }
        }
    }
    format!("url(#{id})")
}
fn wrap(node: &Json, body: String) -> String {
    let (w, h) = (jnum(node, "width", 0.), jnum(node, "height", 0.));
    let (x, y) = (jnum(node, "x", 0.) + w / 2., jnum(node, "y", 0.) + h / 2.);
    format!(
        "<g data-id=\"{}\" opacity=\"{}\" transform=\"translate({} {}) rotate({}) translate({} {})\">{body}</g>",
        escape(jstr(node, "id", "")),
        n(node, "opacity", 1.),
        number(x),
        number(y),
        n(node, "rotation", 0.),
        number(-w / 2.),
        number(-h / 2.)
    )
}
fn item_svg(item: &Json, defs: &mut Vec<String>) -> Result<String> {
    let kind = jstr(item, "kind", "");
    Ok(match kind {
        "glyph" => format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"{}\" xml:space=\"preserve\">{}</text>",
            n(item, "x", 0.),
            n(item, "baseline", 0.),
            escape(jstr(item, "fontFamily", "")),
            n(item, "fontSize", 3.),
            n(item, "fontWeight", 400.),
            if item["fontItalic"].as_bool().unwrap_or(false) {
                "italic"
            } else {
                "normal"
            },
            escape(&laymesh_core::color::css(jstr(item, "color", "#000000"))),
            escape(jstr(item, "content", ""))
        ),
        "box" => format!(
            "<rect data-missing-glyph=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
            n(item, "x", 0.),
            n(item, "y", 0.),
            n(item, "width", 2.),
            n(item, "height", 3.),
            escape(&laymesh_core::color::css(jstr(item, "color", "#000000"))),
            n(item, "strokeWidth", 0.15)
        ),
        "rule" => {
            if item["dashed"].as_bool().unwrap_or(false) {
                format!(
                    "<path d='M{} {}h{}' stroke='{}' stroke-width='{}' stroke-dasharray='{} {}' opacity='{}'/>",
                    n(item, "x", 0.),
                    n(item, "y", 0.),
                    n(item, "width", 0.),
                    escape(&laymesh_core::color::css(jstr(item, "color", "#000000"))),
                    n(item, "height", 0.1),
                    jnum(item, "height", 0.1) * 3.,
                    jnum(item, "height", 0.1) * 2.,
                    n(item, "opacity", 1.)
                )
            } else {
                format!(
                    "<rect x='{}' y='{}' width='{}' height='{}' fill='{}' opacity='{}'/>",
                    n(item, "x", 0.),
                    n(item, "y", 0.),
                    n(item, "width", 0.),
                    n(item, "height", 0.),
                    escape(&laymesh_core::color::css(jstr(item, "color", "#000000"))),
                    n(item, "opacity", 1.)
                )
            }
        }
        "path" if item.get("strokeStyle").is_none() => format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"/>",
            escape(jstr(item, "d", "")),
            escape(&laymesh_core::color::css(jstr(item, "fill", "#000000"))),
            escape(&laymesh_core::color::css(jstr(item, "stroke", "none"))),
            n(item, "strokeWidth", 0.),
            n(item, "opacity", 1.)
        ),
        "formula" => formula_svg(item, defs)?,
        _ => node_svg(item, defs)?,
    })
}
fn formula_svg(node: &Json, defs: &mut Vec<String>) -> Result<String> {
    let mut body = String::new();
    for item in node["items"].as_array().into_iter().flatten() {
        body += &item_svg(item, defs)?;
    }
    let source = escape(jstr(node, "source", ""));
    Ok(format!(
        "<g transform=\"translate({} {})\" data-latex-source=\"{source}\"><title>{source}</title>{body}</g>",
        n(node, "x", 0.),
        n(node, "y", 0.)
    ))
}
fn stroke_attrs(s: &Json) -> String {
    let dash = s["dash"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Json::as_f64)
        .map(number)
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{}\" stroke-linecap=\"{}\" stroke-linejoin=\"{}\" stroke-miterlimit=\"{}\"{}",
        escape(&laymesh_core::color::css(jstr(s, "color", "none"))),
        n(s, "width", 0.),
        n(s, "opacity", 1.),
        escape(jstr(s, "cap", "butt")),
        escape(jstr(s, "join", "miter")),
        n(s, "miterLimit", 4.),
        if dash.is_empty() {
            String::new()
        } else {
            format!(
                " stroke-dasharray=\"{dash}\" stroke-dashoffset=\"{}\"",
                n(s, "dashOffset", 0.)
            )
        }
    )
}
fn node_svg(node: &Json, defs: &mut Vec<String>) -> Result<String> {
    node_svg_transformed(node, defs, kurbo::Affine::IDENTITY)
}
fn node_svg_transformed(
    node: &Json,
    defs: &mut Vec<String>,
    parent: kurbo::Affine,
) -> Result<String> {
    let transform = parent * laymesh_core::geometry::node_transform(node);
    if node["kind"] == "image" && node["mime"] == "image/png" && node["crop"].is_object() {
        let c = &node["crop"];
        let asset = laymesh_core::assets::crop_raster(
            jstr(node, "data", ""),
            [
                jnum(c, "x", 0.),
                jnum(c, "y", 0.),
                jnum(c, "width", 0.),
                jnum(c, "height", 0.),
            ],
            "",
            Loc::default(),
        )?;
        let mut node = node.clone();
        node["crop"] = Json::Null;
        node["data"] = asset["data"].clone();
        node["intrinsicWidth"] = asset["width"].clone();
        node["intrinsicHeight"] = asset["height"].clone();
        return node_svg_transformed(&node, defs, parent);
    }
    // Clipping contours need the same final physical precision as their ink.
    // A parent group may magnify an already laid out polar plot.
    if node["clipPath"]["geometryRecipe"].is_object() {
        let mut node = node.clone();
        if transform.determinant().abs() > 1e-30 {
            let physical = laymesh_core::geometry::resolved_path(&node["clipPath"], transform);
            node["clipPath"]["d"] = json!((transform.inverse() * physical).to_svg());
        }
        node["clipPath"]
            .as_object_mut()
            .unwrap()
            .remove("geometryRecipe");
        return node_svg_transformed(&node, defs, parent);
    }
    if node["kind"] == "path" && node["geometryRecipe"].is_object() {
        let sx = if jnum(node, "intrinsicWidth", 0.) > 0. {
            jnum(node, "width", 1.) / jnum(node, "intrinsicWidth", 1.)
        } else {
            1.
        };
        let sy = if jnum(node, "intrinsicHeight", 0.) > 0. {
            jnum(node, "height", 1.) / jnum(node, "intrinsicHeight", 1.)
        } else {
            1.
        };
        let final_transform = transform * kurbo::Affine::scale_non_uniform(sx, sy);
        let mut node = node.clone();
        if final_transform.determinant().abs() > 1e-30 {
            let physical = laymesh_core::geometry::resolved_path(&node, final_transform);
            node["d"] = json!((final_transform.inverse() * physical).to_svg());
        }
        node.as_object_mut().unwrap().remove("geometryRecipe");
        return node_svg_transformed(&node, defs, parent);
    }
    if node["kind"] == "path"
        && node["outlineD"].is_null()
        && matches!(
            jstr(&node["strokeStyle"], "compound", "single"),
            "double" | "triple"
        )
    {
        let path = kurbo::BezPath::from_svg(jstr(node, "d", "")).map_err(|_| error("无效路径"))?;
        let mut node = node.clone();
        let sx = if jnum(&node, "intrinsicWidth", 0.) > 0. {
            jnum(&node, "width", 1.) / jnum(&node, "intrinsicWidth", 1.)
        } else {
            1.
        };
        let sy = if jnum(&node, "intrinsicHeight", 0.) > 0. {
            jnum(&node, "height", 1.) / jnum(&node, "intrinsicHeight", 1.)
        } else {
            1.
        };
        let final_transform = transform * kurbo::Affine::scale_non_uniform(sx, sy);
        if final_transform.determinant().abs() > 1e-30 {
            let physical = laymesh_core::geometry::compound_outline_transformed(
                &path,
                &node["strokeStyle"],
                final_transform,
            );
            node["outlineD"] = json!((final_transform.inverse() * physical).to_svg());
        } else {
            node["outlineD"] = json!("");
        }
        return node_svg_transformed(&node, defs, parent);
    }
    let kind = jstr(node, "kind", "");
    let (w, h) = (jnum(node, "width", 0.), jnum(node, "height", 0.));
    let body = match kind {
        "group" => {
            let mut clip = String::new();
            if node["clipPath"].is_object() || node["clip"].is_object() {
                let id = format!("clip-{}", defs.len());
                let c = if node["clipPath"].is_object() {
                    format!(
                        "<path d=\"{}\" clip-rule=\"{}\"/>",
                        escape(jstr(&node["clipPath"], "d", "")),
                        escape(jstr(&node["clipPath"], "fillRule", "nonzero"))
                    )
                } else {
                    let c = &node["clip"];
                    format!(
                        "<rect x='{}' y='{}' width='{}' height='{}'/>",
                        n(c, "x", 0.),
                        n(c, "y", 0.),
                        n(c, "width", w),
                        n(c, "height", h)
                    )
                };
                defs.push(format!(
                    "<clipPath id='{id}' clipPathUnits='userSpaceOnUse'>{c}</clipPath>"
                ));
                clip = format!(" clip-path='url(#{id})'");
            }
            let sx = if jnum(node, "contentWidth", 0.) > 0. {
                w / jnum(node, "contentWidth", w)
            } else {
                1.
            };
            let sy = if jnum(node, "contentHeight", 0.) > 0. {
                h / jnum(node, "contentHeight", h)
            } else {
                1.
            };
            let mut children = String::new();
            for c in node["children"].as_array().into_iter().flatten() {
                children += &node_svg_transformed(
                    c,
                    defs,
                    transform * kurbo::Affine::scale_non_uniform(sx, sy),
                )?;
            }
            format!(
                "<g transform='scale({} {})'><g{clip}>{children}</g></g>",
                number(sx),
                number(sy)
            )
        }
        "text" => {
            let mut runs = String::new();
            for r in node["runs"].as_array().into_iter().flatten() {
                runs += &item_svg(r, defs)?;
            }
            runs
        }
        "formula" => {
            let mut local = node.clone();
            local["x"] = json!(0);
            local["y"] = json!(0);
            formula_svg(&local, defs)?
        }
        "path" => {
            if node["endpointRecipe"].is_object() {
                let layers = laymesh_core::endpoints::layers(node, transform).map_err(error)?;
                let inverse = transform.inverse();
                let body = layers
                    .into_iter()
                    .map(|layer| {
                        format!(
                            "<path d=\"{}\" fill=\"{}\" fill-rule=\"nonzero\" opacity=\"{}\"/>",
                            escape(&(inverse * layer.path).to_svg()),
                            escape(&layer.color),
                            number(layer.opacity)
                        )
                    })
                    .collect::<String>();
                return Ok(wrap(node, body));
            }
            let fill = paint(&node["fill"], defs);
            let attrs = stroke_attrs(&node["strokeStyle"]);
            let d = escape(jstr(node, "d", ""));
            let rule = escape(jstr(node, "fillRule", "nonzero"));
            let sx = if jnum(node, "intrinsicWidth", 0.) > 0. {
                w / jnum(node, "intrinsicWidth", w)
            } else {
                1.
            };
            let sy = if jnum(node, "intrinsicHeight", 0.) > 0. {
                h / jnum(node, "intrinsicHeight", h)
            } else {
                1.
            };
            let path = if let Some(outline) = node["outlineD"].as_str() {
                format!(
                    "<path d=\"{d}\" fill=\"{fill}\" fill-rule=\"{rule}\"/><path d=\"{}\" fill=\"{}\" fill-rule=\"evenodd\" fill-opacity=\"{}\"/>",
                    escape(outline),
                    escape(&laymesh_core::color::css(jstr(
                        &node["strokeStyle"],
                        "color",
                        "#000000"
                    ))),
                    n(&node["strokeStyle"], "opacity", 1.)
                )
            } else {
                format!("<path d=\"{d}\" fill=\"{fill}\" fill-rule=\"{rule}\" {attrs}/>")
            };
            format!(
                "<g transform='scale({} {})'>{path}</g>",
                number(sx),
                number(sy)
            )
        }
        "rect" => format!(
            "<rect width='{w}' height='{h}' rx='{}' fill='{}' stroke='{}' stroke-width='{}'/>",
            n(node, "radius", 0.),
            paint(&node["fill"], defs),
            escape(&laymesh_core::color::css(jstr(node, "stroke", "none"))),
            n(node, "strokeWidth", 0.)
        ),
        "ellipse" => format!(
            "<ellipse cx='{}' cy='{}' rx='{}' ry='{}' fill='{}' stroke='{}' stroke-width='{}'/>",
            w / 2.,
            h / 2.,
            w / 2.,
            h / 2.,
            paint(&node["fill"], defs),
            escape(&laymesh_core::color::css(jstr(node, "stroke", "none"))),
            n(node, "strokeWidth", 0.)
        ),
        "image" => {
            let iw = jnum(node, "intrinsicWidth", w).max(1e-12);
            let ih = jnum(node, "intrinsicHeight", h).max(1e-12);
            let fit = jstr(node, "fit", "contain");
            let crop = &node["crop"];
            let (cx, cy, cw, ch) = if crop.is_object() {
                (
                    jnum(crop, "x", 0.),
                    jnum(crop, "y", 0.),
                    jnum(crop, "width", iw),
                    jnum(crop, "height", ih),
                )
            } else {
                (0., 0., iw, ih)
            };
            let scale = if fit == "cover" {
                (w / cw).max(h / ch)
            } else {
                (w / cw).min(h / ch)
            };
            let (sw, sh) = if fit == "stretch" {
                (w / cw, h / ch)
            } else {
                (scale, scale)
            };
            let (x, y) = ((w - cw * sw) / 2. - cx * sw, (h - ch * sh) / 2. - cy * sh);
            let image = format!(
                "<image x='{}' y='{}' width='{}' height='{}' href='data:{};base64,{}' preserveAspectRatio='none'{} />",
                number(x),
                number(y),
                number(iw * sw),
                number(ih * sh),
                escape(jstr(node, "mime", "image/png")),
                jstr(node, "data", ""),
                if jstr(node, "interpolation", "") == "nearest" {
                    " image-rendering='optimizeSpeed'"
                } else {
                    ""
                }
            );
            if fit == "cover" || crop.is_object() {
                let id = format!("clip-{}", defs.len());
                defs.push(format!(
                    "<clipPath id='{id}'><rect width='{w}' height='{h}'/></clipPath>"
                ));
                format!("<g clip-path='url(#{id})'>{image}</g>")
            } else {
                image
            }
        }
        "points" => {
            let mut out = String::new();
            let positions = node["positions"].as_array().cloned().unwrap_or_default();
            for (i, xy) in positions.chunks_exact(2).enumerate() {
                let x = xy[0].as_f64().unwrap_or(0.);
                let y = xy[1].as_f64().unwrap_or(0.);
                let size = node["markerSizes"][i]
                    .as_f64()
                    .unwrap_or(jnum(node, "markerSize", 2.));
                // Batched values replace the scalar default: the scalar is the
                // first marker's value, not a second group opacity.
                let alpha =
                    node["pointOpacities"][i]
                        .as_f64()
                        .unwrap_or(jnum(node, "pointOpacity", 1.));
                let fill = node["markerFills"][i].as_str().unwrap_or(jstr(
                    node,
                    "markerFill",
                    jstr(node, "fill", jstr(node, "color", "#000000")),
                ));
                let stroke = jstr(node, "stroke", jstr(node, "markerStroke", "none"));
                let sw = if stroke == "none" {
                    0.
                } else {
                    jnum(node, "strokeWidth", jnum(node, "markerStrokeWidth", 0.))
                };
                let r = (size - sw).max(0.) / 2.;
                let attrs = format!(
                    "fill=\"{}\" stroke=\"{}\" stroke-width=\"{sw}\" opacity=\"{alpha}\" stroke-linejoin=\"round\"",
                    escape(fill),
                    escape(stroke)
                );
                let marker = jstr(node, "marker", "circle");
                if marker == "circle" {
                    out += &format!("<circle cx='{x}' cy='{y}' r='{r}' {attrs}/>");
                } else {
                    let pts: Vec<[f64; 2]> = match marker {
                        "square" => vec![[-r, -r], [r, -r], [r, r], [-r, r]],
                        "diamond" => vec![[0., -r], [r, 0.], [0., r], [-r, 0.]],
                        "triangle_down" => vec![[-r, -r], [r, -r], [0., r]],
                        "triangle_left" => vec![[r, -r], [r, r], [-r, 0.]],
                        "triangle_right" => vec![[-r, -r], [r, 0.], [-r, r]],
                        "cross" | "plus" => {
                            let t = r / 3.;
                            vec![
                                [-t, -r],
                                [t, -r],
                                [t, -t],
                                [r, -t],
                                [r, t],
                                [t, t],
                                [t, r],
                                [-t, r],
                                [-t, t],
                                [-r, t],
                                [-r, -t],
                                [-t, -t],
                            ]
                        }
                        "star" => (0..10)
                            .map(|i| {
                                let theta = (i as f64 * 36. - 90.).to_radians();
                                let rr = if i % 2 == 0 { r } else { r * 0.4 };
                                [rr * theta.cos(), rr * theta.sin()]
                            })
                            .collect(),
                        _ => vec![[0., -r], [r, r], [-r, r]],
                    };
                    let d = pts
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            format!(
                                "{}{} {}",
                                if i == 0 { 'M' } else { 'L' },
                                x + p[0],
                                y + p[1]
                            )
                        })
                        .collect::<String>();
                    out += &format!("<path d='{d}Z' {attrs}/>");
                }
            }
            out
        }
        _ => return Err(error(format!("未知 Scene 对象：{kind}"))),
    };
    Ok(wrap(node, body))
}
fn used_fonts(scene: &Scene) -> std::collections::BTreeMap<String, (u16, bool)> {
    fn visit(node: &Json, fonts: &mut std::collections::BTreeMap<String, (u16, bool)>) {
        if node["kind"] == "glyph" {
            if let Some(key) = node["fontFamily"].as_str() {
                fonts.insert(
                    key.into(),
                    (
                        jnum(node, "fontWeight", 400.) as u16,
                        node["fontItalic"].as_bool().unwrap_or(false),
                    ),
                );
            }
        }
        for key in ["children", "runs", "items"] {
            for child in node[key].as_array().into_iter().flatten() {
                visit(child, fonts);
            }
        }
    }
    let mut fonts = std::collections::BTreeMap::new();
    for node in &scene.nodes {
        visit(node, &mut fonts);
    }
    fonts
}
/// Render a portable SVG containing only the user fonts that the scene actually uses.
pub fn render_svg(scene: &Scene) -> Result<String> {
    svg(scene, true)
}
fn svg(scene: &Scene, embed_fonts: bool) -> Result<String> {
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.
        || scene.height <= 0.
    {
        return Err(error("画布尺寸必须为正"));
    }
    let mut defs = vec![];
    let mut body = String::new();
    if !scene.background.is_null() && scene.background != "none" {
        body += &format!(
            "<rect width='{}' height='{}' fill='{}'/>",
            number(scene.width),
            number(scene.height),
            paint(&scene.background, &mut defs)
        );
    }
    for node in &scene.nodes {
        body += &node_svg(node, &mut defs)?;
    }
    let mut css = String::new();
    if embed_fonts {
        for (key, (weight, italic)) in used_fonts(scene) {
            let Some(font) = scene.fonts.get(&key) else {
                continue;
            };
            let mime = if font.data.starts_with(b"OTTO") {
                "font/otf"
            } else {
                "font/ttf"
            };
            css += &format!(
                "@font-face{{font-family:'{}';font-weight:{weight};font-style:{};src:url(data:{mime};base64,{})}}",
                escape(&key),
                if italic { "italic" } else { "normal" },
                base64::engine::general_purpose::STANDARD.encode(&font.data)
            );
        }
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\"><defs><style>{css}</style>{}</defs>{body}</svg>\n",
        number(scene.width),
        number(scene.height),
        number(scene.width),
        number(scene.height),
        defs.join("")
    ))
}

#[cfg(feature = "native")]
mod export;
#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use export::{ExportOptions, export_format, render_export};
#[cfg(feature = "native")]
pub use native::{render_pdf, render_png};

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Scene {
        Scene {
            schema_version: 8,
            width: 25.4,
            height: 12.7,
            background: json!("none"),
            layout_dpi: 96.,
            canvas_unit: "mm".into(),
            export_dpi: 300.,
            nodes: vec![],
            warnings: vec![],
            fonts: Default::default(),
        }
    }
    #[test]
    fn physical_svg_dimensions() {
        let svg = render_svg(&empty()).unwrap();
        assert!(svg.contains("width=\"25.4mm\""));
        assert!(!svg.contains("data:font"));
    }
    #[test]
    fn missing_glyph_is_a_font_independent_path() {
        let mut scene = empty();
        let mut fs = laymesh_core::text::FontSystem::new(false);
        scene.nodes.push(
            laymesh_core::text::layout_text(
                &json!({"content":"中","font_size":4.}),
                None,
                &mut fs,
                &mut scene.warnings,
                "test.lay",
                Loc::default(),
            )
            .unwrap(),
        );
        let svg = render_svg(&scene).unwrap();
        assert!(svg.contains("data-missing-glyph"));
        assert!(!svg.contains("<text "));
    }
}
