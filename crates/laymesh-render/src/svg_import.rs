//! SVG is parsed only at the file-format boundary, never as a rendering IR.
use super::*;
use display_list::*;
use kurbo::{Affine, BezPath, Rect, Shape};
use std::{collections::BTreeMap, sync::Arc};

fn affine(t: usvg::Transform) -> Affine {
    Affine::new([
        t.sx as f64,
        t.ky as f64,
        t.kx as f64,
        t.sy as f64,
        t.tx as f64,
        t.ty as f64,
    ])
}
fn path(p: &usvg::tiny_skia_path::Path) -> BezPath {
    let mut out = BezPath::new();
    for s in p.segments() {
        use usvg::tiny_skia_path::PathSegment::*;
        match s {
            MoveTo(p) => out.move_to((p.x as f64, p.y as f64)),
            LineTo(p) => out.line_to((p.x as f64, p.y as f64)),
            QuadTo(a, b) => out.quad_to((a.x as f64, a.y as f64), (b.x as f64, b.y as f64)),
            CubicTo(a, b, c) => out.curve_to(
                (a.x as f64, a.y as f64),
                (b.x as f64, b.y as f64),
                (c.x as f64, c.y as f64),
            ),
            Close => out.close_path(),
        }
    }
    out
}
fn color(c: usvg::Color, a: f64) -> [f64; 4] {
    [
        c.red as f64 / 255.,
        c.green as f64 / 255.,
        c.blue as f64 / 255.,
        a,
    ]
}
fn paint(p: &usvg::Paint, opacity: f64) -> Result<Paint> {
    let stops = |g: &usvg::BaseGradient| {
        g.stops()
            .iter()
            .map(|s| Stop {
                offset: s.offset().get() as f64,
                rgba: color(s.color(), s.opacity().get() as f64 * opacity),
            })
            .collect()
    };
    let spread = |s: usvg::SpreadMethod| {
        match s {
            usvg::SpreadMethod::Pad => "pad",
            usvg::SpreadMethod::Reflect => "reflect",
            usvg::SpreadMethod::Repeat => "repeat",
        }
        .into()
    };
    Ok(match p {
        usvg::Paint::Color(c) => Paint::Solid {
            rgba: color(*c, opacity),
        },
        usvg::Paint::LinearGradient(g) => Paint::Linear {
            start: [g.x1() as f64, g.y1() as f64],
            end: [g.x2() as f64, g.y2() as f64],
            stops: stops(g),
            transform: affine(g.transform()),
            spread: spread(g.spread_method()),
        },
        usvg::Paint::RadialGradient(g) => Paint::Radial {
            center: [g.cx() as f64, g.cy() as f64],
            focal: [g.fx() as f64, g.fy() as f64],
            radius: g.r().get() as f64,
            stops: stops(g),
            transform: affine(g.transform()),
            spread: spread(g.spread_method()),
        },
        usvg::Paint::Pattern(_) => return Err(error("SVG 导入不支持 pattern")),
    })
}
fn clip(c: &usvg::ClipPath) -> Clip {
    let mut combined = BezPath::new();
    let mut even_odd = false;
    fn append(g: &usvg::Group, t: Affine, out: &mut BezPath, rule: &mut bool) {
        let t = t * affine(g.transform());
        for n in g.children() {
            match n {
                usvg::Node::Group(g) => append(g, t, out, rule),
                usvg::Node::Path(p) => {
                    out.extend((t * path(p.data())).elements().iter().copied());
                    *rule |= p
                        .fill()
                        .is_some_and(|f| f.rule() == usvg::FillRule::EvenOdd);
                }
                usvg::Node::Text(text) => append(text.flattened(), t, out, rule),
                _ => {}
            }
        }
    }
    append(
        c.root(),
        affine(c.transform()),
        &mut combined,
        &mut even_odd,
    );
    Clip {
        path: combined,
        even_odd,
    }
}
fn group(g: &usvg::Group) -> Result<DrawNode> {
    if !g.filters().is_empty() {
        return Err(error("SVG 导入不支持 filter"));
    }
    let mut children = Vec::new();
    for n in g.children() {
        children.push(match n {
            usvg::Node::Group(g) => group(g)?,
            usvg::Node::Path(p) => {
                let fill = p
                    .fill()
                    .map(|f| paint(f.paint(), f.opacity().get() as f64))
                    .transpose()?
                    .unwrap_or(Paint::None);
                let mut layers = vec![path_node(
                    path(p.data()),
                    fill,
                    None,
                    p.fill()
                        .is_some_and(|f| f.rule() == usvg::FillRule::EvenOdd),
                )];
                if let Some(s) = p.stroke() {
                    let style = s.to_tiny_skia();
                    if let Some(outline) = p.data().stroke(&style, 1.) {
                        layers.push(path_node(
                            path(&outline),
                            paint(s.paint(), s.opacity().get() as f64)?,
                            None,
                            false,
                        ));
                    }
                }
                if p.paint_order() == usvg::PaintOrder::StrokeAndFill {
                    layers.reverse();
                }
                DrawNode::group(layers)
            }
            usvg::Node::Text(t) => {
                let mut out = group(t.flattened())?;
                out.semantic = Some(
                    t.chunks()
                        .iter()
                        .map(usvg::TextChunk::text)
                        .collect::<String>(),
                );
                out
            }
            usvg::Node::Image(_) => return Err(error("SVG 导入不支持嵌入图片")),
        });
    }
    let mask = g
        .mask()
        .map(|m| -> Result<Mask> {
            let mut node = group(m.root())?;
            let r = m.rect();
            node = DrawNode::new(DrawKind::Group {
                children: vec![node],
                clip: Some(Clip {
                    path: Rect::new(
                        r.x() as f64,
                        r.y() as f64,
                        r.right() as f64,
                        r.bottom() as f64,
                    )
                    .to_path(0.001),
                    even_odd: false,
                }),
                mask: m
                    .mask()
                    .map(|parent| -> Result<Mask> {
                        Ok(Mask {
                            node: Box::new(group(parent.root())?),
                            luminance: parent.kind() == usvg::MaskType::Luminance,
                        })
                    })
                    .transpose()?,
            });
            Ok(Mask {
                node: Box::new(node),
                luminance: m.kind() == usvg::MaskType::Luminance,
            })
        })
        .transpose()?;
    let mut result = DrawNode::new(DrawKind::Group {
        children,
        clip: g.clip_path().map(clip),
        mask,
    });
    result.transform = affine(g.transform());
    result.opacity = g.opacity().get() as f64;
    result.name = g.id().into();
    // Multiple clipPath references represent intersections, not a union.
    let mut c = g.clip_path().and_then(usvg::ClipPath::clip_path);
    while let Some(parent) = c {
        let transform = result.transform;
        result.transform = Affine::IDENTITY;
        result = DrawNode::new(DrawKind::Group {
            children: vec![result],
            clip: Some(clip(parent)),
            mask: None,
        });
        result.transform = transform;
        c = parent.clip_path();
    }
    Ok(result)
}
pub(crate) fn import(
    bytes: &[u8],
    scene: &Scene,
    _resources: &mut BTreeMap<String, Resource>,
) -> Result<DrawNode> {
    let xml = std::str::from_utf8(bytes).map_err(|_| error("SVG 不是 UTF-8"))?;
    let xml = laymesh_core::assets::sanitize_svg(xml, "", Loc::default())?;
    let mut db = usvg::fontdb::Database::new();
    for font in scene.fonts.values() {
        db.load_font_source(usvg::fontdb::Source::Binary(Arc::new(font.data.clone())));
    }
    let options = usvg::Options {
        fontdb: Arc::new(db),
        ..Default::default()
    };
    let tree =
        usvg::Tree::from_str(&xml, &options).map_err(|e| error(format!("SVG 导入失败：{e}")))?;
    group(tree.root())
}
