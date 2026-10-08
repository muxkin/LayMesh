//! Opt-in OpenType MATH layout. The existing KaTeX backend remains the default.
//! Unsupported AST nodes and missing glyphs fail explicitly: a selected face
//! never silently borrows KaTeX metrics, outlines, or stretch constructions.
mod font;
mod layout;

use crate::{
    Diagnostic, Loc, Result,
    model::{FontAsset, base, jnum, jstr},
    text::Outline,
};
use ratex_parser::ParseNode;
use serde_json::{Value as Json, json};

pub(crate) fn formula(
    parsed: &[ParseNode],
    asset: &FontAsset,
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
    let ctx = layout::Context::new(
        display,
        jstr(spec, "color", "#000000"),
        size / crate::model::PT,
        x_height,
    );
    let b = layout::Engine::new(&font).row(parsed, &ctx).map_err(fail)?;
    let mut left = 0_f64;
    let mut right = b.width;
    for item in &b.items {
        if let layout::Item::Glyph { id, x, scale, .. } = item {
            if let Some(bounds) = font.face.glyph_bounding_box(*id) {
                left = left.min(x + f64::from(bounds.x_min) * scale / font.upem);
                right = right.max(x + f64::from(bounds.x_max) * scale / font.upem);
            }
        }
    }
    let mut items = vec![];
    for item in b.items {
        match item {
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
                    y: (b.ascent - y) * size,
                    ..Default::default()
                };
                if font.face.outline_glyph(id, &mut outline).is_none() {
                    return Err(fail(format!("数学字体的字形 {} 缺少可导出的轮廓", id.0)));
                }
                items.push(json!({"kind":"path","d":outline.d,"fill":color,
                    "glyphId":id.0,"mathCodepoint":codepoint,
                    "x":(x-left)*size,"baseline":(b.ascent-y)*size,"fontSize":scale*size}));
            }
            layout::Item::Rule {
                x,
                top,
                width,
                height,
                color,
            } => {
                items.push(json!({"kind":"rule","x":(x-left)*size,
                    "y":(b.ascent-top)*size,"width":width*size,"height":height*size,"color":color}));
            }
        }
    }
    let mut node = base(
        "formula",
        (right - left) * size,
        (b.ascent + b.depth) * size,
    );
    node["source"] = json!(source);
    node["ascent"] = json!(b.ascent * size);
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
    node["items"] = json!(items);
    Ok(node)
}
