//! SVG effect layers deliberately exclude SourceGraphic from their filter result.
//! PDF can rasterize these layers independently of the vector subject.
use super::*;
use kurbo::{Affine, Rect};
pub(crate) fn local_bounds(node: &Json) -> Rect {
    let mut b = if let Some(s) = node.get("subjectBounds") {
        Rect::new(
            jnum(s, "x", 0.),
            jnum(s, "y", 0.),
            jnum(s, "x", 0.) + jnum(s, "width", 0.),
            jnum(s, "y", 0.) + jnum(s, "height", 0.),
        )
    } else {
        Rect::new(0., 0., jnum(node, "width", 0.), jnum(node, "height", 0.))
    };
    if node["endpointRecipe"].is_object() {
        let p = laymesh_core::geometry::node_transform(node).inverse()
            * laymesh_core::geometry::visible(node);
        if !p.elements().is_empty() {
            b = b.union(kurbo::Shape::bounding_box(&p));
        }
    }
    if node["kind"] == "group" {
        let sx = jnum(node, "width", 0.)
            / jnum(node, "contentWidth", jnum(node, "width", 1.)).max(1e-12);
        let sy = jnum(node, "height", 0.)
            / jnum(node, "contentHeight", jnum(node, "height", 1.)).max(1e-12);
        for c in node["children"].as_array().into_iter().flatten() {
            let cb = effect_bounds(c);
            let tr = Affine::scale_non_uniform(sx, sy) * laymesh_core::geometry::node_transform(c);
            b = b.union(tr.transform_rect_bbox(cb));
        }
    }
    let sw = jnum(&node["strokeStyle"], "width", jnum(node, "strokeWidth", 0.)) * 2.;
    b.inflate(sw, sw)
}
pub(crate) fn effect_bounds(node: &Json) -> Rect {
    let b = local_bounds(node);
    let mut out = b;
    let sx = if node["kind"] == "group" {
        jnum(node, "width", 1.) / jnum(node, "contentWidth", 1.).max(1e-12)
    } else {
        1.
    };
    let sy = if node["kind"] == "group" {
        jnum(node, "height", 1.) / jnum(node, "contentHeight", 1.).max(1e-12)
    } else {
        1.
    };
    for e in node["effects"].as_array().into_iter().flatten() {
        if e["mode"] == "inner" {
            continue;
        }
        let pad = 4. * jnum(e, "blur", 1.) + jnum(e, "spread", 0.);
        let xy = pair(&e["offset"], [0., 0.]);
        out = out.union(b.inflate(pad * sx, pad * sy) + kurbo::Vec2::new(xy[0] * sx, xy[1] * sy));
    }
    out
}
pub(crate) fn apply(node: &Json, body: String, defs: &mut Vec<String>) -> String {
    if !node["effects"].as_array().is_some_and(|e| !e.is_empty()) {
        return body;
    }
    let mut back = String::new();
    let mut front = String::new();
    let bbox = local_bounds(node);
    for e in node["effects"].as_array().into_iter().flatten() {
        let id = format!("effect-{}", defs.len());
        let inner = e["mode"] == "inner";
        let blur = jnum(e, "blur", 1.);
        let spread = jnum(e, "spread", 0.);
        let xy = pair(&e["offset"], [0., 0.]);
        let pad = 4. * blur + spread + xy[0].abs().max(xy[1].abs()) + 0.01;
        let b = bbox.inflate(pad, pad);
        let col = laymesh_core::color::Color::parse(jstr(e, "color", "#000000")).unwrap();
        let opacity = jnum(e, "opacity", 0.5) * col.alpha;
        let mut stages = String::new();
        if inner {
            stages += "<feFlood flood-color='white' result='full'/><feComposite in='full' in2='SourceAlpha' operator='out' result='alpha'/>";
        } else {
            stages += "<feColorMatrix in='SourceAlpha' type='matrix' values='0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0' result='alpha'/>";
        }
        if spread > 0. {
            stages += &format!(
                "<feMorphology in='alpha' operator='dilate' radius='{spread}' result='alpha'/>"
            );
        }
        if blur > 0. {
            stages += &format!("<feGaussianBlur in='alpha' stdDeviation='{blur}' result='alpha'/>");
        }
        stages += &format!(
            "<feOffset in='alpha' dx='{}' dy='{}' result='alpha'/>",
            xy[0], xy[1]
        );
        if inner {
            stages += "<feComposite in='alpha' in2='SourceAlpha' operator='in' result='alpha'/>";
        }
        stages += &format!(
            "<feFlood flood-color='{}' flood-opacity='{opacity}' result='color'/><feComposite in='color' in2='alpha' operator='in'/>",
            escape(&col.hex()[..7])
        );
        defs.push(format!("<filter id='{id}' filterUnits='userSpaceOnUse' primitiveUnits='userSpaceOnUse' color-interpolation-filters='sRGB' x='{}' y='{}' width='{}' height='{}'>{stages}</filter>",b.x0,b.y0,b.width(),b.height()));
        let layer = format!(
            "<g id=\"{id}-layer\" data-laymesh-effect=\"true\" filter=\"url(#{id})\">{body}</g>"
        );
        if inner {
            front += &layer
        } else {
            back += &layer
        }
    }
    back + &body + &front
}
