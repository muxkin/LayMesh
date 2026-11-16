use crate::{
    geometry_query::geometry_scale,
    model::{jnum, jstr},
};
use kurbo::{Line, Point, Rect};
use serde_json::{Value as Json, json};

pub(crate) fn axis_spine(axis: &Json, area: &Json) -> (Rect, Vec<Line>) {
    let side = jstr(axis, "side", "bottom");
    let horizontal = matches!(side, "bottom" | "top");
    let offset = jnum(axis, "offset", 0.);
    let x = jnum(area, "x", 0.);
    let y = jnum(area, "y", 0.);
    let w = jnum(area, "width", 0.);
    let h = jnum(area, "height", 0.);
    let coord = |v| match side {
        "bottom" => Point::new(x + v, y + h + offset),
        "top" => Point::new(x + v, y - offset),
        "left" => Point::new(x - offset, y + v),
        _ => Point::new(x + w + offset, y + v),
    };
    let fallback = if horizontal { [0., w] } else { [h, 0.] };
    let bounds = Rect::from_points(coord(fallback[0]), coord(fallback[1]));
    let ranges = if let Some(segments) = axis["segments"].as_array() {
        segments
            .iter()
            .filter_map(|segment| {
                let range = &segment["range"];
                Some(Line::new(
                    coord(range[0].as_f64()?),
                    coord(range[1].as_f64()?),
                ))
            })
            .collect()
    } else if axis["reverse"] == true {
        vec![Line::new(coord(fallback[1]), coord(fallback[0]))]
    } else {
        vec![Line::new(coord(fallback[0]), coord(fallback[1]))]
    };
    (bounds, ranges)
}

pub(crate) fn decorations(node: &Json) -> impl Iterator<Item = &Json> {
    node["plotDecorations"].as_array().into_iter().flatten()
}

pub(crate) fn decoration_rect(decoration: &Json) -> Rect {
    Rect::new(
        jnum(decoration, "left", 0.),
        jnum(decoration, "top", 0.),
        jnum(decoration, "right", 0.),
        jnum(decoration, "bottom", 0.),
    )
}

pub(crate) fn axis_bounds(node: &Json, name: &str, spine_bounds: Rect) -> Rect {
    let prefix = format!("{name}.");
    decorations(node)
        .filter(|part| jstr(part, "name", "").starts_with(&prefix))
        .fold(spine_bounds, |bounds, part| {
            bounds.union(decoration_rect(part))
        })
}

/// Fixed, source-addressable chart anchors in the layout-local frame.
///
/// Preview expands box extents into nine anchors after object rotation. Tick
/// labels contribute their centers so labels remain legible when close together.
pub fn chart_anchors(node: &Json) -> Option<Json> {
    let area = node.get("plotBounds").filter(|area| area.is_object())?;
    let scale = geometry_scale(node);
    let x = jnum(area, "x", 0.);
    let y = jnum(area, "y", 0.);
    let mut boxes = Vec::new();
    let mut points = Vec::new();
    let mut push_box = |name, bounds: Rect| {
        let b = scale.transform_rect_bbox(bounds);
        boxes.push((name, [b.x0, b.y0, b.x1, b.y1]));
    };
    let mut push_point = |name, point: Point| {
        let point = scale * point;
        points.push((name, [point.x, point.y]));
    };
    push_box(
        "plot_area.bounds.".into(),
        Rect::new(
            x,
            y,
            x + jnum(area, "width", 0.),
            y + jnum(area, "height", 0.),
        ),
    );
    // Polar and radar charts do not have the Cartesian spine/side geometry.
    if !node["plotProjection"].is_object()
        && let Some(axes) = node["plotAxes"].as_object()
    {
        for (name, axis) in axes {
            let selector = format!("axes[{}]", json!(name));
            let (spine_bounds, spans) = axis_spine(axis, area);
            push_box(
                format!("{selector}.bounds."),
                axis_bounds(node, name, spine_bounds),
            );
            if let (Some(first), Some(last)) = (spans.first(), spans.last()) {
                push_point(format!("{selector}.min"), first.p0);
                push_point(format!("{selector}.max"), last.p1);
            }
            for (i, line) in spans.iter().enumerate() {
                let path = if spans.len() == 1 {
                    format!("{selector}.spine")
                } else {
                    format!("{selector}.spine.subpaths[{i}]")
                };
                push_point(format!("{path}.start"), line.p0);
                push_point(format!("{path}.end"), line.p1);
            }
            let tick_prefix = format!("{name}.tick:");
            let label = format!("{name}.label");
            let exponent = format!("{name}.exponent");
            let mut tick_index = 0;
            for decoration in decorations(node) {
                let part = jstr(decoration, "name", "");
                if part == label {
                    push_box(
                        format!("{selector}.label.bounds."),
                        decoration_rect(decoration),
                    );
                } else if part == exponent {
                    push_box(
                        format!("{selector}.exponent.bounds."),
                        decoration_rect(decoration),
                    );
                } else if part.starts_with(&tick_prefix) {
                    push_point(
                        format!("{selector}.ticks[{tick_index}].bounds.center"),
                        decoration_rect(decoration).center(),
                    );
                    tick_index += 1;
                }
            }
        }
    }
    Some(json!({"boxes": boxes, "points": points}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversed_broken_axis_uses_ordered_spans_and_measured_decorations() {
        let node = json!({
            "kind": "group",
            "width": 200, "height": 40, "contentWidth": 100, "contentHeight": 80,
            "plotBounds": {"x": 20, "y": 10, "width": 60, "height": 50},
            "plotAxes": {"x": {"side": "bottom", "offset": 2, "reverse": true,
                "segments": [{"range": [60, 40]}, {"range": [35, 0]}]}},
            "plotDecorations": [
                {"name": "x.tick:10", "left": 18, "right": 22, "top": 64, "bottom": 70},
                {"name": "unrelated", "left": 97, "right": 99, "top": 75, "bottom": 80},
                {"name": "x.tick:0", "left": 78, "right": 82, "top": 64, "bottom": 70},
                {"name": "x.label", "left": 45, "right": 55, "top": 71, "bottom": 76},
                {"name": "x.exponent", "left": 74, "right": 82, "top": 72, "bottom": 75}
            ]
        });
        let anchors = chart_anchors(&node).unwrap();
        let find = |kind: &str, name: &str| {
            &anchors[kind]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry[0] == name)
                .unwrap()[1]
        };
        assert_eq!(find("points", "axes[\"x\"].min"), &json!([160., 31.]));
        assert_eq!(find("points", "axes[\"x\"].max"), &json!([40., 31.]));
        assert_eq!(
            find("points", "axes[\"x\"].spine.subpaths[1].start"),
            &json!([110., 31.])
        );
        assert_eq!(
            find("boxes", "axes[\"x\"].bounds."),
            &json!([36., 31., 164., 38.])
        );
        assert_eq!(
            find("boxes", "axes[\"x\"].exponent.bounds."),
            &json!([148., 36., 164., 37.5])
        );
        assert_eq!(
            find("points", "axes[\"x\"].ticks[1].bounds.center"),
            &json!([160., 33.5])
        );
    }
}
