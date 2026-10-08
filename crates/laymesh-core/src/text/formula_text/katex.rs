//! Bridge measured host text into the pinned RaTeX layout before placement.
use super::*;
use ratex_layout::{LayoutBox, LayoutOptions, layout_box::BoxContent, layout_options::TextLayout};
use ratex_types::PathCommand;
use std::cell::RefCell;

pub(crate) struct KaTexText<'a> {
    pub fonts: RefCell<&'a mut FontSystem>,
    pub warnings: RefCell<&'a mut Vec<Diagnostic>>,
    pub used: RefCell<TextFonts>,
    pub error: RefCell<Option<String>>,
    pub spec: &'a Value,
    pub file: &'a str,
    pub loc: Loc,
}
impl TextLayout for KaTexText<'_> {
    fn layout(
        &self,
        body: &[ratex_parser::ParseNode],
        options: &LayoutOptions,
    ) -> Option<LayoutBox> {
        let plain = body
            .iter()
            .map(literal_text)
            .collect::<Option<Vec<_>>>()?
            .concat();
        if plain.is_empty() {
            return None;
        }
        let default = TextStyle::from_spec(self.spec);
        let style = TextStyle {
            weight: options.text_weight.unwrap_or(default.weight),
            italic: options.text_italic.unwrap_or(default.italic),
        };
        if self.spec["font_family"].is_null()
            && self.spec["font_weight"].is_null()
            && self.spec["font_style"].is_null()
            && self.spec["italic"].is_null()
        {
            let suffix = match (style.weight >= 600, style.italic) {
                (true, true) => "BoldItalic",
                (true, false) => "Bold",
                (false, true) => "Italic",
                _ => "Regular",
            };
            let name = format!("KaTeX_Main-{suffix}.ttf");
            if let Some(bytes) = ratex_katex_fonts::ttf_bytes(&name)
                && let Ok(face) = ttf_parser::Face::parse(&bytes, 0)
                && plain
                    .chars()
                    .all(|c| c.is_whitespace() || face.glyph_index(c).is_some())
            {
                return None;
            }
        }
        let result = shape_text(
            &mut self.fonts.borrow_mut(),
            self.spec,
            &plain,
            style,
            self.file,
            self.loc,
            &mut self.warnings.borrow_mut(),
            &mut self.used.borrow_mut(),
        );
        let shaped = match result {
            Ok(shaped) => shaped,
            Err(error) => {
                self.error.borrow_mut().get_or_insert(error);
                return None;
            }
        };
        let commands = shaped
            .path
            .elements()
            .iter()
            .map(|element| match *element {
                kurbo::PathEl::MoveTo(p) => PathCommand::MoveTo { x: p.x, y: -p.y },
                kurbo::PathEl::LineTo(p) => PathCommand::LineTo { x: p.x, y: -p.y },
                kurbo::PathEl::QuadTo(a, p) => PathCommand::QuadTo {
                    x1: a.x,
                    y1: -a.y,
                    x: p.x,
                    y: -p.y,
                },
                kurbo::PathEl::CurveTo(a, b, p) => PathCommand::CubicTo {
                    x1: a.x,
                    y1: -a.y,
                    x2: b.x,
                    y2: -b.y,
                    x: p.x,
                    y: -p.y,
                },
                kurbo::PathEl::ClosePath => PathCommand::Close,
            })
            .collect();
        Some(LayoutBox {
            width: shaped.width,
            height: shaped.ascent,
            depth: shaped.depth,
            content: BoxContent::SvgPath {
                commands,
                fill: true,
            },
            color: options.color,
        })
    }
}
