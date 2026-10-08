use super::layout::{Box as MathBox, Context, Item};
use ttf_parser::{Face, GlyphId, math};

pub(super) type MathResult<T> = std::result::Result<T, String>;

pub(super) struct MathFont<'a> {
    pub face: Face<'a>,
    pub constants: math::Constants<'a>,
    pub upem: f64,
}
impl<'a> MathFont<'a> {
    pub fn new(data: &'a [u8], index: u32) -> MathResult<Self> {
        let face = Face::parse(data, index).map_err(|_| "无效数学字体")?;
        let constants = face
            .tables()
            .math
            .and_then(|m| m.constants)
            .ok_or("数学字体缺少 OpenType MATH 参数")?;
        let upem = f64::from(face.units_per_em());
        if !(1..=100).contains(&constants.script_percent_scale_down())
            || !(1..=constants.script_percent_scale_down())
                .contains(&constants.script_script_percent_scale_down())
            || constants.fraction_rule_thickness().value <= 0
            || constants.radical_rule_thickness().value <= 0
        {
            return Err("数学字体包含无效的 OpenType MATH 参数".into());
        }
        Ok(Self {
            face,
            constants,
            upem,
        })
    }
    pub fn value(&self, value: math::MathValue<'_>, ctx: &Context) -> f64 {
        f64::from(value.value) / self.upem * ctx.scale
    }
    pub fn glyph(&self, ch: char, ctx: &Context) -> MathResult<MathBox> {
        let ch = alphabet(ch, ctx.alphabet);
        if ch.is_whitespace() {
            let width = self
                .face
                .glyph_index(' ')
                .and_then(|g| self.face.glyph_hor_advance(g))
                .map_or(0.25, |a| f64::from(a) / self.upem)
                * ctx.scale;
            return Ok(MathBox {
                width,
                ..Default::default()
            });
        }
        let id = self
            .face
            .glyph_index(ch)
            .ok_or_else(|| format!("所选数学字体缺少字形 U+{:04X}", ch as u32))?;
        Ok(self.glyph_id(id, Some(ch as u32), ctx))
    }
    pub fn glyph_id(&self, id: GlyphId, codepoint: Option<u32>, ctx: &Context) -> MathBox {
        let s = ctx.scale / self.upem;
        let bounds = self.face.glyph_bounding_box(id);
        let info = self.face.tables().math.and_then(|m| m.glyph_info);
        let italic = info
            .and_then(|i| i.italic_corrections)
            .and_then(|i| i.get(id))
            .map_or(0., |v| f64::from(v.value) * s);
        let width = f64::from(self.face.glyph_hor_advance(id).unwrap_or(0)) * s;
        let accent = info
            .and_then(|i| i.top_accent_attachments)
            .and_then(|i| i.get(id))
            .map_or(width / 2., |v| f64::from(v.value) * s);
        MathBox {
            width,
            ascent: bounds.map_or(0., |b| (f64::from(b.y_max) * s).max(0.)),
            depth: bounds.map_or(0., |b| (-f64::from(b.y_min) * s).max(0.)),
            italic,
            accent: Some(accent),
            glyph: Some((id, ctx.scale)),
            items: vec![Item::Glyph {
                id,
                codepoint,
                x: 0.,
                y: 0.,
                scale: ctx.scale,
                color: ctx.color.clone(),
            }],
            ..Default::default()
        }
    }
    /// Use the font's prepared variants first, then its connector/part recipe.
    /// Vertical recipes are ordered bottom-to-top; horizontal ones left-to-right.
    pub fn stretch(
        &self,
        ch: char,
        target: f64,
        vertical: bool,
        ctx: &Context,
    ) -> MathResult<MathBox> {
        let id = self
            .face
            .glyph_index(ch)
            .ok_or_else(|| format!("所选数学字体缺少伸缩符号 U+{:04X}", ch as u32))?;
        let base = self.glyph_id(id, Some(ch as u32), ctx);
        let dimension = |b: &MathBox| {
            if vertical {
                b.ascent + b.depth
            } else {
                b.width
            }
        };
        if dimension(&base) >= target {
            return Ok(base);
        }
        let variants = self
            .face
            .tables()
            .math
            .and_then(|m| m.variants)
            .ok_or("所选数学字体缺少伸缩字形表")?;
        let construction = if vertical {
            variants.vertical_constructions
        } else {
            variants.horizontal_constructions
        }
        .get(id)
        .ok_or_else(|| format!("所选数学字体没有符号 U+{:04X} 的伸缩构造", ch as u32))?;
        let unit = ctx.scale / self.upem;
        let mut largest = base;
        for variant in construction.variants {
            let mut b = self.glyph_id(variant.variant_glyph, Some(ch as u32), ctx);
            if !vertical {
                b.width = f64::from(variant.advance_measurement) * unit;
            }
            if f64::from(variant.advance_measurement) * unit + 1e-9 >= target {
                return Ok(b);
            }
            largest = b;
        }
        // Some operators/accents offer a finite set of sizes without an
        // assembly recipe. Use their largest native form; never invent a
        // KaTeX construction or anisotropically scale their outlines.
        let Some(assembly) = construction.assembly else {
            return Ok(largest);
        };
        let original: Vec<_> = assembly.parts.into_iter().collect();
        if original.len() > 32 {
            return Err("数学伸缩组件超过复杂度限制".into());
        }
        let overlap = f64::from(variants.min_connector_overlap) * unit;
        for repeats in 0..=256 {
            let mut parts = vec![];
            for p in &original {
                for _ in 0..if p.part_flags.extender() { repeats } else { 1 } {
                    parts.push(*p);
                }
            }
            if parts.is_empty() {
                continue;
            }
            let mut joins = vec![];
            let mut invalid_join = false;
            for pair in parts.windows(2) {
                let max = f64::from(
                    pair[0]
                        .end_connector_length
                        .min(pair[1].start_connector_length),
                ) * unit;
                if max + 1e-9 < overlap {
                    invalid_join = true;
                    break;
                }
                joins.push(max);
            }
            if invalid_join {
                // Omitting all extenders may join two terminal parts which
                // are not intended to touch. Retry with an extender present.
                if repeats == 0 {
                    continue;
                }
                return Err("数学字体的伸缩连接器无效".into());
            }
            let sum = parts
                .iter()
                .map(|p| f64::from(p.full_advance) * unit)
                .sum::<f64>();
            let longest = sum - overlap * joins.len() as f64;
            if longest + 1e-9 < target {
                continue;
            }
            // Maximize overlap to obtain the smallest assembly meeting target.
            let shortest = sum - joins.iter().sum::<f64>();
            let extra = (longest - target.max(shortest)).max(0.);
            let capacity = joins.iter().map(|max| max - overlap).sum::<f64>();
            let fraction = if capacity > 0. {
                (extra / capacity).clamp(0., 1.)
            } else {
                0.
            };
            let mut cursor = 0.;
            let mut result = MathBox::default();
            let width = parts
                .iter()
                .map(|p| self.glyph_id(p.glyph_id, None, ctx).width)
                .fold(0_f64, f64::max);
            for (i, p) in parts.iter().enumerate() {
                let b = self.glyph_id(p.glyph_id, None, ctx);
                let bounds = self
                    .face
                    .glyph_bounding_box(p.glyph_id)
                    .ok_or("伸缩组件缺少轮廓")?;
                let (x, y) = if vertical {
                    (
                        (width - b.width) / 2.,
                        cursor - f64::from(bounds.y_min) * unit,
                    )
                } else {
                    (cursor - f64::from(bounds.x_min) * unit, 0.)
                };
                result.add(b, x, y);
                cursor += f64::from(p.full_advance) * unit;
                if let Some(max) = joins.get(i) {
                    cursor -= overlap + fraction * (max - overlap);
                }
            }
            result.width = if vertical { width } else { cursor };
            result.italic = f64::from(assembly.italics_correction.value) * unit;
            result.accent = Some(result.width / 2.);
            result.glyph = None;
            return Ok(result);
        }
        Err("数学符号伸缩超过 256 次组件重复限制".into())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) enum Alphabet {
    #[default]
    Auto,
    Roman,
    Italic,
    Bold,
    BoldItalic,
    Sans,
    SansItalic,
    SansBold,
    SansBoldItalic,
    BoldScript,
    BoldFraktur,
    Mono,
    Script,
    Fraktur,
    Double,
}
pub(super) fn alphabet(ch: char, style: Alphabet) -> char {
    use Alphabet::*;
    let style = match style {
        Auto if ch.is_ascii_alphabetic()
            || ('α'..='ω').contains(&ch)
            || matches!(ch, 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ' | '∂') =>
        {
            Italic
        }
        Auto => return ch,
        x => x,
    };
    let latin = match style {
        Italic => Some((0x1D434, 0x1D44E, None)),
        Bold => Some((0x1D400, 0x1D41A, Some(0x1D7CE))),
        BoldItalic => Some((0x1D468, 0x1D482, Some(0x1D7CE))),
        Sans => Some((0x1D5A0, 0x1D5BA, Some(0x1D7E2))),
        SansItalic => Some((0x1D608, 0x1D622, None)),
        SansBold => Some((0x1D5D4, 0x1D5EE, Some(0x1D7EC))),
        SansBoldItalic => Some((0x1D63C, 0x1D656, Some(0x1D7EC))),
        BoldScript => Some((0x1D4D0, 0x1D4EA, Some(0x1D7CE))),
        BoldFraktur => Some((0x1D56C, 0x1D586, Some(0x1D7CE))),
        Mono => Some((0x1D670, 0x1D68A, Some(0x1D7F6))),
        Script => Some((0x1D49C, 0x1D4B6, None)),
        Fraktur => Some((0x1D504, 0x1D51E, None)),
        Double => Some((0x1D538, 0x1D552, Some(0x1D7D8))),
        _ => None,
    };
    let exception = match (style, ch) {
        (Italic, 'h') => Some('ℎ'),
        (Script, 'B') => Some('ℬ'),
        (Script, 'E') => Some('ℰ'),
        (Script, 'F') => Some('ℱ'),
        (Script, 'H') => Some('ℋ'),
        (Script, 'I') => Some('ℐ'),
        (Script, 'L') => Some('ℒ'),
        (Script, 'M') => Some('ℳ'),
        (Script, 'R') => Some('ℛ'),
        (Script, 'e') => Some('ℯ'),
        (Script, 'g') => Some('ℊ'),
        (Script, 'o') => Some('ℴ'),
        (Fraktur, 'C') => Some('ℭ'),
        (Fraktur, 'H') => Some('ℌ'),
        (Fraktur, 'I') => Some('ℑ'),
        (Fraktur, 'R') => Some('ℜ'),
        (Fraktur, 'Z') => Some('ℨ'),
        (Double, 'C') => Some('ℂ'),
        (Double, 'H') => Some('ℍ'),
        (Double, 'N') => Some('ℕ'),
        (Double, 'P') => Some('ℙ'),
        (Double, 'Q') => Some('ℚ'),
        (Double, 'R') => Some('ℝ'),
        (Double, 'Z') => Some('ℤ'),
        _ => None,
    };
    if let Some(c) = exception {
        return c;
    }
    if let Some((upper, lower, digits)) = latin {
        let cp = if ch.is_ascii_uppercase() {
            Some(upper + ch as u32 - 'A' as u32)
        } else if ch.is_ascii_lowercase() {
            Some(lower + ch as u32 - 'a' as u32)
        } else if ch.is_ascii_digit() {
            digits.map(|d| d + ch as u32 - '0' as u32)
        } else {
            None
        };
        if let Some(c) = cp.and_then(char::from_u32) {
            return c;
        }
    }
    let greek = match style {
        Bold => Some(0x1D6A8),
        Italic => Some(0x1D6E2),
        BoldItalic => Some(0x1D71C),
        SansBold => Some(0x1D756),
        SansBoldItalic => Some(0x1D790),
        _ => None,
    };
    if let Some(base) = greek {
        let offset = match ch {
            'Α'..='Ω' if ch != '\u{03A2}' => Some(ch as u32 - 0x391),
            'α'..='ω' => Some(ch as u32 - 0x3B1 + 26),
            'ϴ' => Some(17),
            '∇' => Some(25),
            '∂' => Some(51),
            'ϵ' => Some(52),
            'ϑ' => Some(53),
            'ϰ' => Some(54),
            'ϕ' => Some(55),
            'ϱ' => Some(56),
            'ϖ' => Some(57),
            _ => None,
        };
        if let Some(c) = offset.and_then(|o| char::from_u32(base + o)) {
            return c;
        }
    }
    ch
}
