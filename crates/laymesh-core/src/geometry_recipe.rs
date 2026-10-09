//! Retained analytic recipes let later group transforms choose curve accuracy
//! in final millimetres instead of magnifying an earlier polygon approximation.
use crate::geometry::{difference, intersection, outline_transformed, union, visible_transformed};
use crate::model::{jnum, jstr};
use kurbo::{Affine, BezPath, Point, Shape};
use serde_json::{Value as Json, json};
pub fn resolved_path(n: &Json, transform: Affine) -> BezPath {
    let recipe = &n["geometryRecipe"];
    let [a, b, c, d, _, _] = transform.as_coeffs();
    let magnification = ((a + d).hypot(b - c) + (a - d).hypot(b + c)) / 2.;
    if !recipe.is_object() || magnification <= 1. + 1e-12 {
        return transform * BezPath::from_svg(jstr(n, "d", "")).unwrap_or_default();
    }
    let tol = 0.0001 / magnification.max(1.);
    let kind = jstr(recipe, "kind", "");
    if kind == "arrow" {
        let origin = &recipe["origin"];
        return transform
            * Affine::translate((-origin[0].as_f64().unwrap(), -origin[1].as_f64().unwrap()))
            * crate::arrows::resolved(recipe, tol);
    }
    if kind == "polar" {
        return transform * crate::plot::resolved_polar_path(recipe, tol);
    }
    if kind == "polar_hatch" {
        use i_overlay::{
            core::fill_rule::FillRule, float::clip::FloatClip, string::clip::ClipRule,
        };
        let region = transform * crate::plot::resolved_polar_path(&recipe["region"], tol);
        let rings = crate::geometry::points(&region, 0.0001);
        let width = jnum(recipe, "width", 0.);
        let height = jnum(recipe, "height", 0.);
        let spacing = jnum(recipe, "spacing", 1.);
        let mut lines = Vec::<Vec<[f64; 2]>>::new();
        for slope in if recipe["cross"] == true {
            vec![-1., 1.]
        } else {
            vec![-1.]
        } {
            let (min, max) = if slope < 0. {
                (0., width + height)
            } else {
                (-width, height)
            };
            for i in (min / spacing).floor() as i64..=(max / spacing).floor() as i64 {
                let k = i as f64 * spacing;
                let a = transform * Point::new(0., k);
                let b = transform * Point::new(width, slope * width + k);
                lines.push(vec![[a.x, a.y], [b.x, b.y]]);
            }
        }
        let clipped = lines.clip_by(
            &rings,
            FillRule::EvenOdd,
            ClipRule {
                invert: false,
                boundary_included: false,
            },
        );
        let mut path = BezPath::new();
        for line in clipped {
            for (i, point) in line.into_iter().enumerate() {
                if i == 0 {
                    path.move_to((point[0], point[1]));
                } else {
                    path.line_to((point[0], point[1]));
                }
            }
        }
        return path;
    }
    if kind == "fuse" {
        let origin = &recipe["origin"];
        let tr = transform
            * Affine::translate((-origin[0].as_f64().unwrap(), -origin[1].as_f64().unwrap()));
        let mut combined = union(
            &visible_transformed(&recipe["nodes"][0], tr),
            &visible_transformed(&recipe["nodes"][1], tr),
        );
        let p = &recipe["points"];
        let start = Point::new(p[0][0].as_f64().unwrap(), p[0][1].as_f64().unwrap());
        let end = Point::new(p[1][0].as_f64().unwrap(), p[1][1].as_f64().unwrap());
        let width = jnum(recipe, "width", 0.);
        let radius = jnum(recipe, "radius", 0.);
        let junction = jstr(recipe, "junction", "miter");
        let mut center = BezPath::new();
        center.move_to(start);
        center.line_to(end);
        let bridge = if junction == "bevel" {
            tr * BezPath::from_svg(jstr(recipe, "bevel", "")).unwrap_or_default()
        } else if start.distance(end) > 1e-8 {
            outline_transformed(
                &center,
                &json!({"width":width,"cap":if junction=="round"{"round"}else{"butt"}}),
                tr,
            )
        } else {
            BezPath::new()
        };
        combined = union(&combined, &bridge);
        if junction == "round" && tr.determinant().abs() > 1e-30 {
            let stroke = json!({"width":radius*2.,"cap":"round","join":"round"});
            let expanded = union(
                &combined,
                &outline_transformed(&(tr.inverse() * combined.clone()), &stroke, tr),
            );
            let closed = difference(
                &expanded,
                &outline_transformed(&(tr.inverse() * expanded.clone()), &stroke, tr),
            );
            let additions = difference(&closed, &combined);
            let reach = width / 2. + radius * 2.;
            let region = union(
                &(tr * kurbo::Circle::new(start, reach).to_path(tol)),
                &(tr * kurbo::Circle::new(end, reach).to_path(tol)),
            );
            combined = union(&combined, &intersection(&additions, &region));
        }
        return combined;
    }
    let origin = &recipe["origin"];
    let tr = transform
        * Affine::translate((
            -origin[0].as_f64().unwrap_or(0.),
            -origin[1].as_f64().unwrap_or(0.),
        ));
    let mut path = match kind {
        "rect" => kurbo::RoundedRect::new(
            0.,
            0.,
            jnum(recipe, "width", 0.),
            jnum(recipe, "height", 0.),
            jnum(recipe, "radius", 0.),
        )
        .to_path(tol),
        "ellipse" => {
            let w = jnum(recipe, "width", 0.);
            let h = jnum(recipe, "height", 0.);
            kurbo::Ellipse::new((w / 2., h / 2.), (w / 2., h / 2.), 0.).to_path(tol)
        }
        "ring" => {
            let r = jnum(recipe, "radius", 0.);
            let mut p = kurbo::Circle::new((r, r), r).to_path(tol);
            p.extend(
                kurbo::Circle::new((r, r), jnum(recipe, "inner", 0.))
                    .to_path(tol)
                    .iter(),
            );
            p
        }
        "arc" | "sector" => {
            let r = jnum(recipe, "radius", 0.);
            let mut p = kurbo::Arc::new(
                (r, r),
                (r, r),
                jnum(recipe, "start", 0.),
                jnum(recipe, "sweep", 0.),
                0.,
            )
            .to_path(tol);
            if kind == "sector" {
                p.line_to((r, r));
                p.close_path();
            }
            p
        }
        "path" => {
            let mut p = BezPath::new();
            let mut current = Point::ORIGIN;
            let mut beginning = current;
            for command in recipe["commands"].as_array().into_iter().flatten() {
                let get = |s| jnum(command, s, 0.);
                let end = Point::new(get("x"), get("y"));
                match jstr(command, "kind", "") {
                    "move_to" => {
                        p.move_to(end);
                        beginning = end;
                    }
                    "line_to" => p.line_to(end),
                    "quad_to" => p.quad_to((get("cx"), get("cy")), (end.x, end.y)),
                    "cubic_to" => p.curve_to(
                        (get("c1x"), get("c1y")),
                        (get("c2x"), get("c2y")),
                        (end.x, end.y),
                    ),
                    "close" => {
                        p.close_path();
                        current = beginning;
                        continue;
                    }
                    "arc_to" => {
                        let arc = kurbo::SvgArc {
                            from: current,
                            to: end,
                            radii: kurbo::Vec2::new(get("rx"), get("ry")),
                            x_rotation: get("rotation"),
                            large_arc: command["large_arc"] == true,
                            sweep: command["sweep"] == true,
                        };
                        if let Some(a) = kurbo::Arc::from_svg_arc(&arc) {
                            a.to_cubic_beziers(tol, |a, b, c| p.curve_to(a, b, c));
                        } else {
                            p.line_to(end);
                        }
                    }
                    _ => {}
                }
                current = end;
            }
            p
        }
        _ => return transform * BezPath::from_svg(jstr(n, "d", "")).unwrap_or_default(),
    };
    path = tr * path;
    path
}

/// Exact analytic bounds avoid magnifying the tiny Bezier arc approximation
/// in a group's later placement dimensions and its anchor positions.
pub(crate) fn analytic_bounds(recipe: &Json, fallback: &BezPath) -> kurbo::Rect {
    let kind = jstr(recipe, "kind", "");
    if kind == "ring" {
        let r = jnum(recipe, "radius", 0.);
        return kurbo::Rect::new(0., 0., 2. * r, 2. * r);
    }
    if kind == "arc" || kind == "sector" {
        let r = jnum(recipe, "radius", 0.);
        let arc = kurbo::Arc::new(
            (r, r),
            (r, r),
            jnum(recipe, "start", 0.),
            jnum(recipe, "sweep", 0.),
            0.,
        );
        let b = arc_bounds(arc);
        return if kind == "sector" {
            b.union_pt(Point::new(r, r))
        } else {
            b
        };
    }
    if kind != "path" {
        return fallback.bounding_box();
    }
    let mut bounds: Option<kurbo::Rect> = None;
    let mut current = Point::ORIGIN;
    let mut beginning = current;
    for c in recipe["commands"].as_array().into_iter().flatten() {
        let get = |s| jnum(c, s, 0.);
        let end = Point::new(get("x"), get("y"));
        let b = match jstr(c, "kind", "") {
            "move_to" => {
                beginning = end;
                kurbo::Rect::from_points(end, end)
            }
            "line_to" => kurbo::Rect::from_points(current, end),
            "quad_to" => {
                kurbo::QuadBez::new(current, Point::new(get("cx"), get("cy")), end).bounding_box()
            }
            "cubic_to" => kurbo::CubicBez::new(
                current,
                Point::new(get("c1x"), get("c1y")),
                Point::new(get("c2x"), get("c2y")),
                end,
            )
            .bounding_box(),
            "arc_to" => {
                let svg = kurbo::SvgArc {
                    from: current,
                    to: end,
                    radii: kurbo::Vec2::new(get("rx"), get("ry")),
                    x_rotation: get("rotation"),
                    large_arc: c["large_arc"] == true,
                    sweep: c["sweep"] == true,
                };
                kurbo::Arc::from_svg_arc(&svg)
                    .map(arc_bounds)
                    .unwrap_or(kurbo::Rect::from_points(current, end))
            }
            "close" => {
                current = beginning;
                continue;
            }
            _ => continue,
        };
        bounds = Some(bounds.map_or(b, |old| old.union(b)));
        current = end;
    }
    bounds.unwrap_or_default()
}
fn arc_bounds(arc: kurbo::Arc) -> kurbo::Rect {
    let (rx, ry) = (arc.radii.x, arc.radii.y);
    let (sin, cos) = arc.x_rotation.sin_cos();
    let x = (-ry * sin).atan2(rx * cos);
    let y = (ry * cos).atan2(rx * sin);
    let point = |angle: f64| {
        Point::new(
            arc.center.x + rx * angle.cos() * cos - ry * angle.sin() * sin,
            arc.center.y + rx * angle.cos() * sin + ry * angle.sin() * cos,
        )
    };
    let start = arc.start_angle;
    let sweep = arc.sweep_angle;
    let mut bounds = kurbo::Rect::from_points(point(start), point(start + sweep));
    for a in [x, x + std::f64::consts::PI, y, y + std::f64::consts::PI] {
        let distance =
            if sweep >= 0. { a - start } else { start - a }.rem_euclid(std::f64::consts::TAU);
        if sweep.abs() >= std::f64::consts::TAU || distance <= sweep.abs() + 1e-12 {
            bounds = bounds.union_pt(point(a));
        }
    }
    bounds
}
