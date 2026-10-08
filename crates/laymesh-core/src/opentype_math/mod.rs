//! Opt-in OpenType MATH layout. The existing KaTeX backend remains the default.
//! Missing glyphs use measured vector boxes. A selected face never silently
//! borrows KaTeX metrics, outlines, or stretch constructions.
mod font;
mod layout;
mod parse;
pub(crate) use parse::parse;

use crate::{
    Diagnostic, Loc, Result,
    model::{FontAsset, base, jnum, jstr},
    text::{FontSystem, Outline, TextFonts, TextStyle, shape_text},
};
use kurbo::Shape;
use ratex_parser::ParseNode;
use serde_json::{Value as Json, json};

pub(crate) fn formula(
    parsed: &[ParseNode],
    asset: &FontAsset,
    fonts: &mut FontSystem,
    warnings: &mut Vec<Diagnostic>,
    spec: &Json,
    source: &str,
    file: &str,
    loc: Loc,
) -> Result<Json> {
    let fail = |message: String| Diagnostic::new("E_FORMULA", message, file, loc);
    let font = font::MathFont::new(&asset.data, asset.index).map_err(fail)?;
    let size = jnum(spec, "font_size", 10. * crate::model::PT);
    let display = spec
        .get("display")
        .and_then(Json::as_bool)
        .unwrap_or(jstr(spec, "style", "inline") == "display");
    let x_height = font
        .face
        .x_height()
        .map_or(0.43, |h| f64::from(h) / font.upem);
    let mut ctx = layout::Context::new(
        display,
        jstr(spec, "color", "#000000"),
        size / crate::model::PT,
        x_height,
    );
    ctx.text_style = TextStyle::from_spec(spec);
    let mut used = TextFonts::default();
    let enabled = spec["math_text_fallback"].as_bool().unwrap_or(true);
    let mut render_text = |text: &str, c: &layout::Context| {
        let mut shaped = shape_text(
            fonts,
            spec,
            text,
            c.text_style,
            file,
            loc,
            warnings,
            &mut used,
        )?;
        shaped.path.apply_affine(kurbo::Affine::scale(c.scale));
        Ok(layout::Box {
            width: shaped.width * c.scale,
            ascent: shaped.ascent * c.scale,
            depth: shaped.depth * c.scale,
            items: vec![layout::Item::Path {
                path: shaped.path,
                fill: Some(c.color.clone()),
                stroke: None,
                thickness: 0.,
                missing: vec![],
            }],
            ..Default::default()
        })
    };
    let b = layout::Engine::new(
        &font,
        enabled.then_some(&mut render_text),
        !spec["font_family"].is_null()
            || !spec["font_weight"].is_null()
            || !spec["font_style"].is_null()
            || !spec["italic"].is_null(),
    )
    .row(parsed, &ctx)
    .map_err(fail)?;
    let mut ascent = b.ascent;
    let mut depth = b.depth;
    let mut left = 0_f64;
    let mut right = b.width.max(0.);
    for item in &b.items {
        let bounds = match item {
            layout::Item::Glyph {
                id, x, y, scale, ..
            } => font.face.glyph_bounding_box(*id).map(|r| {
                kurbo::Rect::new(
                    x + f64::from(r.x_min) * scale / font.upem,
                    y + f64::from(r.y_min) * scale / font.upem,
                    x + f64::from(r.x_max) * scale / font.upem,
                    y + f64::from(r.y_max) * scale / font.upem,
                )
            }),
            layout::Item::Rule {
                x,
                top,
                width,
                height,
                ..
            } => Some(kurbo::Rect::new(*x, top - height, x + width, *top)),
            layout::Item::Path {
                path,
                stroke,
                thickness,
                ..
            } => Some(path.bounding_box().inflate(
                if stroke.is_some() { thickness / 2. } else { 0. },
                if stroke.is_some() { thickness / 2. } else { 0. },
            )),
        };
        if let Some(r) = bounds {
            left = left.min(r.x0);
            right = right.max(r.x1);
            ascent = ascent.max(r.y1);
            depth = depth.max(-r.y0);
        }
    }
    let mut items = vec![];
    let mut missing = used.1;
    for item in b.items {
        match item {
            layout::Item::Path {
                mut path,
                fill,
                stroke,
                thickness,
                missing: glyphs,
            } => {
                missing.extend(glyphs);
                path.apply_affine(kurbo::Affine::new([
                    size,
                    0.,
                    0.,
                    -size,
                    -left * size,
                    ascent * size,
                ]));
                items.push(json!({"kind":"path","d":path.to_svg(),"fill":fill.unwrap_or_else(||"none".into()),
                "stroke":stroke.unwrap_or_else(||"none".into()),"strokeWidth":thickness*size}));
            }
            layout::Item::Glyph {
                id,
                x,
                y,
                scale,
                color,
                codepoint,
            } => {
                let mut outline = Outline {
                    scale: size * scale / font.upem,
                    x: (x - left) * size,
                    y: (ascent - y) * size,
                    ..Default::default()
                };
                if font.face.outline_glyph(id, &mut outline).is_none() {
                    return Err(fail(format!("数学字体的字形 {} 缺少可导出的轮廓", id.0)));
                }
                items.push(json!({"kind":"path","d":outline.d,"fill":color,
                    "glyphId":id.0,"mathCodepoint":codepoint,
                    "x":(x-left)*size,"baseline":(ascent-y)*size,"fontSize":scale*size}));
            }
            layout::Item::Rule {
                x,
                top,
                width,
                height,
                color,
            } => {
                items.push(json!({"kind":"rule","x":(x-left)*size,
                    "y":(ascent-top)*size,"width":width*size,"height":height*size,"color":color}));
            }
        }
    }
    let mut node = base("formula", (right - left) * size, (ascent + depth) * size);
    node["source"] = json!(source);
    node["ascent"] = json!(ascent * size);
    node["mathFont"] = json!(asset.family);
    node["mathFontFace"] = json!(
        font.face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
            .and_then(|n| n.to_string())
    );
    node["mathFontBackend"] = json!("opentype-math");
    node["mathFontRequest"] = spec["math_font"].clone();
    node["mathTextFallback"] = json!(enabled);
    node["mathTextFonts"] = json!(used.0);
    node["mathMissingGlyphs"] = json!(missing);
    crate::text::warn_missing(warnings, &missing, file, loc);
    node["items"] = json!(items);
    Ok(node)
}
