//! Shape complete text runs before either mathematical layout engine places them.
use super::{FontSystem, Outline};
use crate::{Diagnostic, Loc, model::jnum};
use kurbo::{Affine, BezPath, Shape};
use serde_json::{Value, json};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
mod katex;
pub(crate) use katex::KaTexText;

#[derive(Clone, Copy, Default)]
pub(crate) struct TextStyle {
    pub weight: u16,
    pub italic: bool,
}
impl TextStyle {
    pub fn from_spec(spec: &Value) -> Self {
        Self {
            weight: jnum(spec, "font_weight", 400.) as u16,
            italic: spec["font_style"] == "italic" || spec["italic"] == true,
        }
    }
    pub fn command(self, command: &str) -> Self {
        match command.trim_start_matches('\\') {
            "textbf" | "mathbf" | "bf" => Self {
                weight: 700,
                ..self
            },
            "textit" | "mathit" | "emph" | "it" => Self {
                italic: true,
                ..self
            },
            "textmd" => Self {
                weight: 400,
                ..self
            },
            "textup" => Self {
                italic: false,
                ..self
            },
            "textnormal" => Self {
                weight: 400,
                italic: false,
            },
            _ => self,
        }
    }
}

#[derive(Default)]
pub(crate) struct TextFonts(pub Vec<Value>);
impl TextFonts {
    pub fn record(&mut self, font: Value) {
        if let Some(old) = self.0.iter_mut().find(|old| {
            old["face"] == font["face"]
                && old["requested_weight"] == font["requested_weight"]
                && old["requested_italic"] == font["requested_italic"]
        }) {
            for field in ["glyph_count", "character_count"] {
                old[field] =
                    json!(old[field].as_u64().unwrap_or(0) + font[field].as_u64().unwrap_or(0));
            }
        } else {
            self.0.push(font);
        }
    }
}
pub(crate) struct ShapedText {
    /// Outline coordinates use em units with positive y above the baseline.
    pub path: BezPath,
    pub width: f64,
    pub ascent: f64,
    pub depth: f64,
}

/// A literal text leaf/group; mathematical and structural nodes delimit runs.
pub(crate) fn literal_text(node: &ratex_parser::ParseNode) -> Option<String> {
    use ratex_parser::{Mode, ParseNode as N};
    match node {
        N::TextOrd {
            mode: Mode::Text,
            text,
            ..
        }
        | N::MathOrd {
            mode: Mode::Text,
            text,
            ..
        }
        | N::Atom {
            mode: Mode::Text,
            text,
            ..
        }
        | N::OpToken {
            mode: Mode::Text,
            text,
            ..
        } => ratex_font::get_symbol(text, ratex_font::Mode::Text)
            .and_then(|s| s.codepoint)
            .map(|c| c.to_string())
            .or_else(|| (!text.starts_with('\\')).then(|| text.clone())),
        N::SpacingNode {
            mode: Mode::Text,
            text,
            ..
        } if matches!(
            text.as_str(),
            " " | "~" | "\\ " | "\\space" | "\\nobreakspace"
        ) =>
        {
            Some(" ".into())
        }
        N::Accent {
            mode: Mode::Text,
            label,
            base,
            ..
        } => {
            let mark = match label.as_str() {
                "\\'" | "\\acute" => '\u{0301}',
                "\\`" | "\\grave" => '\u{0300}',
                "\\^" | "\\hat" => '\u{0302}',
                "\\~" | "\\tilde" => '\u{0303}',
                "\\=" | "\\bar" => '\u{0304}',
                "\\u" | "\\breve" => '\u{0306}',
                "\\." | "\\dot" => '\u{0307}',
                "\\\"" | "\\ddot" => '\u{0308}',
                "\\r" | "\\mathring" => '\u{030A}',
                "\\H" => '\u{030B}',
                "\\v" | "\\check" => '\u{030C}',
                "\\c" => '\u{0327}',
                _ => return None,
            };
            let mut text = literal_text(base)?;
            text.push(mark);
            Some(text)
        }
        N::OrdGroup {
            mode: Mode::Text,
            body,
            ..
        } => body
            .iter()
            .map(literal_text)
            .collect::<Option<Vec<_>>>()
            .map(|s| s.concat()),
        _ => None,
    }
}

/// Selection is per grapheme, then adjacent runs of the same font, script and
/// direction are shaped together. Splitting into codepoints would lose marks,
/// ligatures and Arabic joining. Font assets used only as outlines stay private.
pub(crate) fn shape_text(
    fonts: &mut FontSystem,
    spec: &Value,
    text: &str,
    style: TextStyle,
    file: &str,
    loc: Loc,
    warnings: &mut Vec<Diagnostic>,
    used: &mut TextFonts,
) -> Result<ShapedText, String> {
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    let mut path = BezPath::new();
    let mut cursor = 0.;
    for paragraph in &bidi.paragraphs {
        let (levels, visual) = bidi.visual_runs(paragraph, paragraph.range.clone());
        for run in visual {
            let rtl = levels[run.start].is_rtl();
            let mut segments: Vec<(String, Script, String)> = vec![];
            for grapheme in text[run].graphemes(true) {
                if grapheme
                    .chars()
                    .all(|c| matches!(c as u32, 0x202A..=0x202E | 0x2066..=0x2069))
                {
                    continue;
                }
                let key = fonts
                    .choose_mode(
                        &spec["font_family"],
                        style.weight,
                        style.italic,
                        grapheme,
                        file,
                        loc,
                        warnings,
                        true,
                    )
                    .ok_or_else(|| {
                        format!(
                            "文本字体列表缺少字形 {}",
                            grapheme
                                .chars()
                                .map(|c| format!("U+{:04X}", c as u32))
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                    })?;
                let script = grapheme
                    .chars()
                    .map(|c| c.script())
                    .find(|s| !matches!(s, Script::Common | Script::Inherited))
                    .unwrap_or(Script::Common);
                if let Some((last_key, last_script, pending)) = segments.last_mut()
                    && *last_key == key
                    && (script == *last_script
                        || script == Script::Common
                        || *last_script == Script::Common)
                {
                    pending.push_str(grapheme);
                    if *last_script == Script::Common {
                        *last_script = script;
                    }
                } else {
                    segments.push((key, script, grapheme.into()));
                }
            }
            if rtl {
                segments.reverse();
            }
            for (key, _, content) in segments {
                let asset = fonts
                    .assets
                    .get(&key)
                    .or_else(|| fonts.outline_assets.get(&key))
                    .ok_or("文本字体缓存缺失")?;
                let face =
                    rustybuzz::Face::from_slice(&asset.data, asset.index).ok_or("无效文本字体")?;
                let unit = 1. / f64::from(face.units_per_em());
                let mut buffer = rustybuzz::UnicodeBuffer::new();
                buffer.push_str(&content);
                buffer.guess_segment_properties();
                buffer.set_direction(if rtl {
                    rustybuzz::Direction::RightToLeft
                } else {
                    rustybuzz::Direction::LeftToRight
                });
                let output = rustybuzz::shape(&face, &[], buffer);
                used.record(
                    json!({"family":asset.family,"face":fonts.face_names.get(&key),
                    "path":fonts.origins.get(&key),"weight":face.weight().to_number(),"italic":face.is_italic(),
                    "requested_weight":style.weight,"requested_italic":style.italic,
                    "glyph_count":output.glyph_infos().len(),"character_count":content.chars().count()}),
                );
                let mut pen_x = 0.;
                let mut pen_y = 0.;
                for (glyph, position) in output.glyph_infos().iter().zip(output.glyph_positions()) {
                    let id = ttf_parser::GlyphId(glyph.glyph_id as u16);
                    if id.0 == 0 {
                        return Err(format!("文本字体 {} 排版后出现缺字", asset.family));
                    }
                    let mut outline = Outline {
                        scale: unit,
                        ..Default::default()
                    };
                    if face.outline_glyph(id, &mut outline).is_some() {
                        let mut p =
                            BezPath::from_svg(&outline.d).map_err(|_| "无效文本字形轮廓")?;
                        p.apply_affine(Affine::new([
                            1.,
                            0.,
                            0.,
                            -1.,
                            cursor + (pen_x + f64::from(position.x_offset)) * unit,
                            (pen_y + f64::from(position.y_offset)) * unit,
                        ]));
                        path.extend(p.elements().iter().copied());
                    } else if face.glyph_bounding_box(id).is_some() {
                        return Err(format!(
                            "文本字体 {} 的字形 {} 缺少可导出的轮廓",
                            asset.family, id.0
                        ));
                    }
                    pen_x += f64::from(position.x_advance);
                    pen_y += f64::from(position.y_advance);
                }
                cursor += pen_x * unit;
            }
        }
    }
    let ink = path.bounding_box();
    if ![cursor, ink.x0, ink.x1, ink.y0, ink.y1]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("文本字体产生无效尺寸".into());
    }
    Ok(ShapedText {
        path,
        width: cursor,
        ascent: ink.y1.max(0.),
        depth: (-ink.y0).max(0.),
    })
}
