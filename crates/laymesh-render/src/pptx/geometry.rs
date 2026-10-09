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
pub(super) fn geometry(path: &BezPath, t: Affine, stroke: bool, hint: Option<&Json>) -> Geometry {
    let local = path.bounding_box();
    if let Some((b, xfrm, paint)) = frame(t, local) {
        let (sx, sy, _, _) = orthogonal(t).unwrap();
        let semantic = hint.and_then(|h| h["kind"].as_str()).unwrap_or("");
        let single = path
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
            == 1;
        let kind = match semantic {
            "ellipse" | "circle" => Some(("ellipse", 0.)),
            "rect" if single => {
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
pub(super) fn native_stroke(s: Option<&Stroke>, t: Affine, opacity: f64) -> Result<Option<String>> {
    let Some(s) = s else {
        return Ok(Some("<a:ln><a:noFill/></a:ln>".into()));
    };
    let Some((sx, sy, _, _)) = orthogonal(t) else {
        return Ok(None);
    };
    if (sx - sy).abs() > sx * 1e-8
        || s.offset != 0.
        || s.dash.iter().any(|v| !v.is_finite() || *v <= 0.)
    {
        return Ok(None);
    }
    let mut dash = s.dash.clone();
    if dash.len() % 2 == 1 {
        dash.extend(dash.clone());
    }
    let cap = match s.cap.as_str() {
        "round" => "rnd",
        "square" => "sq",
        _ => "flat",
    };
    let join = match s.join.as_str() {
        "round" => "<a:round/>".into(),
        "bevel" => "<a:bevel/>".into(),
        _ => format!("<a:miter lim=\"{}\"/>", (s.miter * 100000.).round() as i64),
    };
    let dashes = if dash.is_empty() {
        "<a:prstDash val=\"solid\"/>".into()
    } else {
        format!(
            "<a:custDash>{}</a:custDash>",
            dash.chunks_exact(2)
                .map(|v| format!(
                    "<a:ds d=\"{}\" sp=\"{}\"/>",
                    (v[0] / s.width * 100000.).round().max(1.) as i64,
                    (v[1] / s.width * 100000.).round().max(1.) as i64
                ))
                .collect::<String>()
        )
    };
    Ok(Some(format!(
        "<a:ln w=\"{}\" cap=\"{cap}\" cmpd=\"sng\" algn=\"ctr\">{}{dashes}{join}<a:headEnd type=\"none\"/><a:tailEnd type=\"none\"/></a:ln>",
        emu(s.width * sx).max(1),
        solid(s.rgba, opacity)?
    )))
}
pub(super) fn stroke_style(s: &Stroke) -> Json {
    json!({"width":s.width,"dash":s.dash,"dashOffset":s.offset,"cap":s.cap,"join":s.join,"miterLimit":s.miter})
}
