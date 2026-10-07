use super::*;

pub(super) struct Geometry {
    pub rect: Rect,
    pub bounds: Rect,
    pub xml: String,
    pub transform: String,
    // Map source coordinates into the unrotated shape's paint coordinate space.
    pub paint_transform: Affine,
}
pub(super) fn orthogonal(t: Affine) -> Option<(f64, f64, f64, bool)> {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let sx = a.hypot(b);
    let sy = c.hypot(d);
    if sx <= 1e-12 || sy <= 1e-12 || (a * c + b * d).abs() > sx * sy * 1e-8 {
        return None;
    }
    Some((sx, sy, b.atan2(a), t.determinant() < 0.))
}
pub(super) fn frame(t: Affine, r: Rect) -> Option<(Rect, String, Affine)> {
    let (sx, sy, angle, flip) = orthogonal(t)?;
    let b = Rect::from_center_size(t * r.center(), (r.width() * sx, r.height() * sy));
    let transform = xfrm(b, angle).replacen(
        "<a:xfrm",
        if flip {
            "<a:xfrm flipV=\"1\""
        } else {
            "<a:xfrm"
        },
        1,
    );
    let paint =
        Affine::translate((b.x0 - r.x0 * sx, b.y0 - r.y0 * sy)) * Affine::scale_non_uniform(sx, sy);
    Some((b, transform, paint))
}
pub(super) fn preset(kind: &str, radius: f64, r: Rect) -> String {
    let adjustments = if kind == "roundRect" {
        format!(
            "<a:gd name=\"adj\" fmla=\"val {}\"/>",
            (radius / r.width().min(r.height()).max(1e-12) * 100_000.)
                .clamp(0., 50_000.)
                .round() as i64
        )
    } else {
        String::new()
    };
    format!("<a:prstGeom prst=\"{kind}\"><a:avLst>{adjustments}</a:avLst></a:prstGeom>")
}
pub(super) fn custom(path: &BezPath, b: Rect, stroke: bool) -> String {
    let pt = |p: Point| {
        format!(
            "<a:pt x=\"{}\" y=\"{}\"/>",
            emu(p.x - b.x0),
            emu(p.y - b.y0)
        )
    };
    let mut commands = String::new();
    for el in path.iter() {
        commands += &match el {
            PathEl::MoveTo(p) => format!("<a:moveTo>{}</a:moveTo>", pt(p)),
            PathEl::LineTo(p) => format!("<a:lnTo>{}</a:lnTo>", pt(p)),
            PathEl::QuadTo(a, b) => format!("<a:quadBezTo>{}{}</a:quadBezTo>", pt(a), pt(b)),
            PathEl::CurveTo(a, b, c) => {
                format!("<a:cubicBezTo>{}{}{}</a:cubicBezTo>", pt(a), pt(b), pt(c))
            }
            PathEl::ClosePath => "<a:close/>".into(),
        };
    }
    format!(
        "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/><a:pathLst><a:path w=\"{}\" h=\"{}\" stroke=\"{}\" extrusionOk=\"0\">{commands}</a:path></a:pathLst></a:custGeom>",
        emu(b.width()).max(1),
        emu(b.height()).max(1),
        u8::from(stroke)
    )
}
pub(super) fn svg_path(n: Node<'_, '_>) -> Result<BezPath> {
    Ok(match n.tag_name().name() {
        "path" => {
            BezPath::from_svg(n.attribute("d").unwrap_or("")).map_err(|_| error("PPTX 无效路径"))?
        }
        "rect" => {
            let r = Rect::new(
                attr(n, "x", 0.),
                attr(n, "y", 0.),
                attr(n, "x", 0.) + attr(n, "width", 0.),
                attr(n, "y", 0.) + attr(n, "height", 0.),
            );
            let rx = attr(n, "rx", attr(n, "ry", 0.)).min(r.width() / 2.);
            let ry = attr(n, "ry", rx).min(r.height() / 2.);
            if rx > 0. && ry > 0. {
                // Elliptical corners, including those produced by unequal scaling.
                let k = 0.5522847498307936;
                let mut p = BezPath::new();
                p.move_to((r.x0 + rx, r.y0));
                p.line_to((r.x1 - rx, r.y0));
                p.curve_to(
                    (r.x1 - rx + k * rx, r.y0),
                    (r.x1, r.y0 + ry - k * ry),
                    (r.x1, r.y0 + ry),
                );
                p.line_to((r.x1, r.y1 - ry));
                p.curve_to(
                    (r.x1, r.y1 - ry + k * ry),
                    (r.x1 - rx + k * rx, r.y1),
                    (r.x1 - rx, r.y1),
                );
                p.line_to((r.x0 + rx, r.y1));
                p.curve_to(
                    (r.x0 + rx - k * rx, r.y1),
                    (r.x0, r.y1 - ry + k * ry),
                    (r.x0, r.y1 - ry),
                );
                p.line_to((r.x0, r.y0 + ry));
                p.curve_to(
                    (r.x0, r.y0 + ry - k * ry),
                    (r.x0 + rx - k * rx, r.y0),
                    (r.x0 + rx, r.y0),
                );
                p.close_path();
                p
            } else {
                r.to_path(0.001)
            }
        }
        _ => kurbo::Ellipse::new(
            (attr(n, "cx", 0.), attr(n, "cy", 0.)),
            (
                attr(n, "rx", attr(n, "r", 0.)),
                attr(n, "ry", attr(n, "r", 0.)),
            ),
            0.,
        )
        .to_path(0.001),
    })
}
pub(super) fn geometry(
    n: Node<'_, '_>,
    path: &BezPath,
    t: Affine,
    stroke: bool,
    hint: Option<&Json>,
) -> Geometry {
    let local = path.bounding_box();
    if let Some((b, xfrm, paint)) = frame(t, local) {
        let (sx, sy, _, _) = orthogonal(t).unwrap();
        let semantic = hint.and_then(|h| h["kind"].as_str()).unwrap_or("");
        let single = path
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
            == 1;
        let kind = match n.tag_name().name() {
            "ellipse" | "circle" => Some(("ellipse", 0.)),
            "rect" => {
                let rx = attr(n, "rx", attr(n, "ry", 0.)).min(local.width() / 2.) * sx;
                let ry = attr(n, "ry", attr(n, "rx", 0.)).min(local.height() / 2.) * sy;
                if rx == 0. && ry == 0. {
                    Some(("rect", 0.))
                } else if (rx - ry).abs() < 1e-8 {
                    Some(("roundRect", rx))
                } else {
                    None
                }
            }
            "path" if single && semantic == "ellipse" => Some(("ellipse", 0.)),
            "path" if single && semantic == "rect" => {
                let mut rx = f64::INFINITY;
                let mut ry = f64::INFINITY;
                for e in path.iter() {
                    let p = match e {
                        PathEl::MoveTo(p)
                        | PathEl::LineTo(p)
                        | PathEl::QuadTo(_, p)
                        | PathEl::CurveTo(_, _, p) => p,
                        _ => continue,
                    };
                    if (p.y - local.y0).abs() < 1e-8 || (p.y - local.y1).abs() < 1e-8 {
                        let distance = (p.x - local.x0).min(local.x1 - p.x);
                        if distance > 1e-8 {
                            rx = rx.min(distance);
                        }
                    }
                    if (p.x - local.x0).abs() < 1e-8 || (p.x - local.x1).abs() < 1e-8 {
                        let distance = (p.y - local.y0).min(local.y1 - p.y);
                        if distance > 1e-8 {
                            ry = ry.min(distance);
                        }
                    }
                }
                if !rx.is_finite() && !ry.is_finite() {
                    Some(("rect", 0.))
                } else if ((rx * sx) - (ry * sy)).abs() < 1e-7 {
                    Some(("roundRect", rx * sx))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((kind, radius)) = kind {
            return Geometry {
                rect: b,
                bounds: t.transform_rect_bbox(local),
                xml: preset(kind, radius, b),
                transform: xfrm,
                paint_transform: paint,
            };
        }
        // Keep custom geometry in the unrotated picture/paint frame too.
        // This preserves image fill orientation on rotated non-preset paths.
        if !matches!(path.elements(), [PathEl::MoveTo(_), PathEl::LineTo(_)]) {
            return Geometry {
                rect: b,
                bounds: (t * path.clone()).bounding_box(),
                xml: custom(&(paint * path.clone()), b, stroke),
                transform: xfrm,
                paint_transform: paint,
            };
        }
    }
    let path = t * path.clone();
    let b = path.bounding_box();
    let els = path.elements();
    let (xml, xfrm) = if let [PathEl::MoveTo(a), PathEl::LineTo(z)] = els {
        let mut transform = xfrm(b, 0.);
        if a.x > z.x {
            transform = transform.replacen("<a:xfrm", "<a:xfrm flipH=\"1\"", 1);
        }
        if a.y > z.y {
            transform = transform.replacen("<a:xfrm", "<a:xfrm flipV=\"1\"", 1);
        }
        (preset("line", 0., b), transform)
    } else {
        (custom(&path, b, stroke), xfrm(b, 0.))
    };
    Geometry {
        rect: b,
        bounds: b,
        xml,
        transform: xfrm,
        paint_transform: t,
    }
}
// Native stroke widths have one scalar; anisotropic/sheared stroke footprints
// and shifted dash patterns retain the existing exact vector-outline lowering.
pub(super) fn native_stroke(n: Node<'_, '_>, t: Affine, opacity: f64) -> Result<Option<String>> {
    let color = n.attribute("stroke").unwrap_or("none");
    let width = attr(n, "stroke-width", 1.);
    if color == "none" || width <= 0. {
        return Ok(Some("<a:ln><a:noFill/></a:ln>".into()));
    }
    let Some((sx, sy, _, _)) = orthogonal(t) else {
        return Ok(None);
    };
    if (sx - sy).abs() > sx * 1e-8
        || attr(n, "stroke-dashoffset", 0.) != 0.
        || color.starts_with("url(")
    {
        return Ok(None);
    }
    let mut dash = n
        .attribute("stroke-dasharray")
        .unwrap_or("")
        .split([' ', ','])
        .filter(|s| !s.is_empty() && *s != "none")
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| error("PPTX 无效虚线"))?;
    if dash.iter().any(|v| !v.is_finite() || *v <= 0.) {
        return Ok(None);
    }
    if dash.len() % 2 == 1 {
        dash.extend(dash.clone());
    }
    let cap = match n.attribute("stroke-linecap").unwrap_or("butt") {
        "round" => "rnd",
        "square" => "sq",
        _ => "flat",
    };
    let join = match n.attribute("stroke-linejoin").unwrap_or("miter") {
        "round" => "<a:round/>".into(),
        "bevel" => "<a:bevel/>".into(),
        _ => format!(
            "<a:miter lim=\"{}\"/>",
            (attr(n, "stroke-miterlimit", 4.) * 100_000.).round() as i64
        ),
    };
    let dashes = if dash.is_empty() {
        "<a:prstDash val=\"solid\"/>".into()
    } else {
        format!(
            "<a:custDash>{}</a:custDash>",
            dash.chunks_exact(2)
                .map(|v| format!(
                    "<a:ds d=\"{}\" sp=\"{}\"/>",
                    (v[0] / width * 100_000.).round().max(1.) as i64,
                    (v[1] / width * 100_000.).round().max(1.) as i64
                ))
                .collect::<String>()
        )
    };
    Ok(Some(format!(
        "<a:ln w=\"{}\" cap=\"{cap}\" cmpd=\"sng\" algn=\"ctr\">{}{dashes}{join}<a:headEnd type=\"none\"/><a:tailEnd type=\"none\"/></a:ln>",
        emu(width * sx).max(1),
        solid(color, opacity * attr(n, "stroke-opacity", 1.))?
    )))
}

pub(super) fn stroke_style(n: Node<'_, '_>) -> Result<Json> {
    let dash = n
        .attribute("stroke-dasharray")
        .unwrap_or("")
        .split([' ', ','])
        .filter(|s| !s.is_empty() && *s != "none")
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| error("PPTX 无效虚线"))?;
    Ok(
        json!({"width":attr(n,"stroke-width",1.),"dash":dash,"dashOffset":attr(n,"stroke-dashoffset",0.),"cap":n.attribute("stroke-linecap").unwrap_or("butt"),"join":n.attribute("stroke-linejoin").unwrap_or("miter"),"miterLimit":attr(n,"stroke-miterlimit",4.)}),
    )
}
