//! Geometry eligibility shared by diagnostics and retained drawing.
use crate::model::{jnum, jstr};
use kurbo::{BezPath, ParamCurveArclen};
use serde_json::Value;

pub fn eligible(node: &Value) -> bool {
    if node["kind"] == "group" {
        return node["children"].as_array().is_some_and(|cs| cs.iter().any(eligible));
    }
    let st = &node["strokeStyle"];
    node["kind"] == "path"
        && !crate::color::visible(jstr(node, "fill", "none"))
        && crate::color::visible(jstr(st, "color", "none"))
        && jnum(st, "width", 0.) > 0.
        && jnum(st, "opacity", 1.) > 0.
        && BezPath::from_svg(jstr(node, "d", "")).is_ok_and(|p| {
            p.segments().map(|s| s.arclen(1e-6)).sum::<f64>() > 1e-10
        })
}
