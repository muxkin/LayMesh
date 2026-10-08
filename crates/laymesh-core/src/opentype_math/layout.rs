use super::font::{Alphabet, MathFont, MathResult};
use ratex_parser::{
    Mode, ParseNode as N,
    parse_node::{AlignSpec, AlignType, AtomFamily, Measurement, StyleStr},
};
use ttf_parser::{GlyphId, math};

#[derive(Clone)]
pub(super) struct Context {
    pub scale: f64,
    pub level: u8,
    pub display: bool,
    pub cramped: bool,
    pub alphabet: Alphabet,
    pub color: String,
    em_pt: f64,
    x_height: f64,
    sizing: f64,
    depth: usize,
}
impl Context {
    pub fn new(display: bool, color: &str, em_pt: f64, x_height: f64) -> Self {
        Self {
            scale: 1.,
            level: 0,
            display,
            cramped: false,
            alphabet: Alphabet::Auto,
            color: color.into(),
            em_pt,
            x_height,
            sizing: 1.,
            depth: 0,
        }
    }
    fn script(&self, font: &MathFont<'_>) -> Self {
        let mut c = self.clone();
        let first = f64::from(font.constants.script_percent_scale_down()) / 100.;
        let second = f64::from(font.constants.script_script_percent_scale_down()) / 100.;
        c.scale *= match c.level {
            0 => first,
            1 => second / first,
            _ => 1.,
        };
        c.level = (c.level + 1).min(2);
        c.display = false;
        c
    }
    fn fraction(&self, font: &MathFont<'_>) -> Self {
        if self.display {
            let mut c = self.clone();
            c.display = false;
            c
        } else {
            self.script(font)
        }
    }
    fn font(&self, alphabet: Alphabet) -> Self {
        let mut c = self.clone();
        use Alphabet::*;
        c.alphabet = match (self.alphabet, alphabet) {
            (Sans, Bold) | (Bold, Sans) | (SansBold, Bold) => SansBold,
            (Italic, Bold) | (Bold, Italic) => BoldItalic,
            _ => alphabet,
        };
        c
    }
}
#[derive(Clone)]
pub(super) enum Item {
    Glyph {
        id: GlyphId,
        codepoint: Option<u32>,
        x: f64,
        y: f64,
        scale: f64,
        color: String,
    },
    Rule {
        x: f64,
        top: f64,
        width: f64,
        height: f64,
        color: String,
    },
}
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum Class {
    #[default]
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}
#[derive(Clone, Default)]
pub(super) struct Box {
    pub width: f64,
    pub ascent: f64,
    pub depth: f64,
    pub italic: f64,
    pub accent: Option<f64>,
    pub glyph: Option<(GlyphId, f64)>,
    pub class: Class,
    pub limits: bool,
    pub items: Vec<Item>,
}
impl Box {
    pub fn add(&mut self, mut b: Self, x: f64, y: f64) {
        self.width = self.width.max(x + b.width);
        self.ascent = self.ascent.max(y + b.ascent);
        self.depth = self.depth.max(b.depth - y);
        for item in &mut b.items {
            match item {
                Item::Glyph { x: a, y: b, .. } => {
                    *a += x;
                    *b += y;
                }
                Item::Rule { x: a, top: b, .. } => {
                    *a += x;
                    *b += y;
                }
            }
        }
        self.items.extend(b.items);
    }
    fn rule(&mut self, x: f64, top: f64, width: f64, height: f64, c: &Context) {
        self.add(
            Self {
                width,
                ascent: top.max(0.),
                depth: (height - top).max(0.),
                items: vec![Item::Rule {
                    x: 0.,
                    top,
                    width,
                    height,
                    color: c.color.clone(),
                }],
                ..Default::default()
            },
            x,
            0.,
        );
    }
}
pub(super) struct Engine<'a, 'f> {
    font: &'f MathFont<'a>,
    steps: usize,
}
impl<'a, 'f> Engine<'a, 'f> {
    pub fn new(font: &'f MathFont<'a>) -> Self {
        Self { font, steps: 0 }
    }
    pub fn row(&mut self, nodes: &[N], c: &Context) -> MathResult<Box> {
        let mut boxes = nodes
            .iter()
            .map(|n| self.node(n, c))
            .collect::<MathResult<Vec<_>>>()?;
        // TeX binary-operator cancellation, including unary signs.
        for i in 0..boxes.len() {
            if boxes[i].class == Class::Bin
                && (i == 0
                    || i + 1 == boxes.len()
                    || matches!(
                        boxes[i - 1].class,
                        Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct
                    )
                    || matches!(boxes[i + 1].class, Class::Rel | Class::Close | Class::Punct))
            {
                boxes[i].class = Class::Ord;
            }
        }
        if boxes.len() == 1 {
            return Ok(boxes.remove(0));
        }
        let mut result = Box::default();
        let mut previous = None;
        let mut x = 0.;
        for b in boxes {
            if let Some(a) = previous {
                x += spacing(a, b.class, c.level > 0) * c.scale;
            }
            previous = Some(b.class);
            let width = b.width;
            result.italic = b.italic;
            result.add(b, x, 0.);
            x += width;
        }
        result.width = x;
        Ok(result)
    }
    fn node(&mut self, n: &N, c: &Context) -> MathResult<Box> {
        self.steps += 1;
        if self.steps > 20_000 || c.depth > 128 {
            return Err("OpenType 数学排版超过复杂度限制".into());
        }
        let mut c = c.clone();
        c.depth += 1;
        let c = &c;
        match n {
            N::MathOrd { mode, text, .. }
            | N::TextOrd { mode, text, .. }
            | N::Atom { mode, text, .. }
            | N::OpToken { mode, text, .. } => {
                let mut b = self.symbol(text, *mode, c)?;
                b.class = class(n);
                Ok(b)
            }
            N::OrdGroup { body, .. } => self.row(body, c),
            N::Font { font, body, .. } => self.node(body, &c.font(font_style(font)?)),
            N::Text { font, body, .. } => self.row(
                body,
                &c.font(
                    font.as_deref()
                        .map(font_style)
                        .transpose()?
                        .unwrap_or(Alphabet::Roman),
                ),
            ),
            N::Color { color, body, .. } => {
                let mut c = c.clone();
                c.color = color.clone();
                self.row(body, &c)
            }
            N::SupSub { base, sup, sub, .. } => {
                self.scripts(base.as_deref(), sup.as_deref(), sub.as_deref(), c)
            }
            N::GenFrac {
                numer,
                denom,
                has_bar_line,
                left_delim,
                right_delim,
                bar_size,
                ..
            } => {
                let result = self.fraction(numer, denom, *has_bar_line, bar_size.as_ref(), c)?;
                self.delimit(
                    result,
                    left_delim.as_deref().unwrap_or("."),
                    right_delim.as_deref().unwrap_or("."),
                    c,
                )
            }
            N::Sqrt { body, index, .. } => self.radical(body, index.as_deref(), c),
            N::LeftRight {
                body, left, right, ..
            } => {
                let inner = self.row(body, c)?;
                self.delimit(inner, left, right, c)
            }
            N::DelimSizing {
                delim,
                size,
                mclass,
                ..
            } => {
                let mut b =
                    self.delimiter(delim, f64::from(*size) * 0.6 * c.scale + 0.6 * c.scale, c)?;
                b.class = mclass_type(mclass);
                Ok(b)
            }
            N::Op {
                symbol: is_symbol,
                name,
                body,
                limits,
                always_handle_sup_sub,
                ..
            } => {
                let mut b = if *is_symbol {
                    let ch = symbol(name.as_deref().ok_or("运算符缺少名称")?, Mode::Math)?;
                    let min = if c.display {
                        f64::from(self.font.constants.display_operator_min_height())
                            / self.font.upem
                            * c.scale
                    } else {
                        0.
                    };
                    let mut b = self.font.stretch(ch, min, true, &c.font(Alphabet::Roman))?;
                    let shift = self.font.value(self.font.constants.axis_height(), c)
                        - (b.ascent - b.depth) / 2.;
                    let mut shifted = Box::default();
                    shifted.italic = b.italic;
                    // Baseline centering is part of operator layout, not stretch.
                    b.glyph = None;
                    shifted.add(b, 0., shift);
                    shifted
                } else if let Some(body) = body {
                    self.row(body, &c.font(Alphabet::Roman))?
                } else {
                    self.symbol(
                        name.as_deref().unwrap_or("").trim_start_matches('\\'),
                        Mode::Text,
                        &c.font(Alphabet::Roman),
                    )?
                };
                b.class = Class::Op;
                b.limits = *limits && (c.display || *always_handle_sup_sub == Some(true));
                Ok(b)
            }
            N::OperatorName {
                body,
                limits,
                always_handle_sup_sub,
                ..
            } => {
                let mut b = self.row(body, &c.font(Alphabet::Roman))?;
                b.class = Class::Op;
                b.limits = *limits && (c.display || *always_handle_sup_sub);
                Ok(b)
            }
            N::Array {
                body,
                cols,
                row_gaps,
                arraystretch,
                hlines_before_row,
                tags,
                col_separation_type,
                ..
            } => {
                if col_separation_type.as_deref().is_some_and(|s| s != "small") {
                    return Err("OpenType 数学排版尚不支持 aligned/align/gather 等环境".into());
                }
                if hlines_before_row.iter().any(|r| !r.is_empty())
                    || tags.as_ref().is_some_and(|t| !t.is_empty())
                {
                    return Err("OpenType 数学排版尚不支持数组横线或编号".into());
                }
                self.array(body, cols.as_deref(), row_gaps, *arraystretch, c)
            }
            N::Accent {
                label,
                base,
                is_stretchy,
                ..
            } => self.accent(label, base, *is_stretchy == Some(true), false, c),
            N::AccentUnder {
                label,
                base,
                is_stretchy,
                ..
            } => self.accent(label, base, *is_stretchy == Some(true), true, c),
            N::Overline { body, .. } | N::Underline { body, .. } => {
                let mut b = self.node(body, c)?;
                let ct = self.font.constants;
                let under = matches!(n, N::Underline { .. });
                let gap = self.font.value(
                    if under {
                        ct.underbar_vertical_gap()
                    } else {
                        ct.overbar_vertical_gap()
                    },
                    c,
                );
                let thick = self.font.value(
                    if under {
                        ct.underbar_rule_thickness()
                    } else {
                        ct.overbar_rule_thickness()
                    },
                    c,
                );
                let extra = self.font.value(
                    if under {
                        ct.underbar_extra_descender()
                    } else {
                        ct.overbar_extra_ascender()
                    },
                    c,
                );
                let top = if under {
                    -b.depth - gap
                } else {
                    b.ascent + gap + thick
                };
                b.rule(0., top, b.width, thick, c);
                if under {
                    b.depth += extra
                } else {
                    b.ascent += extra
                };
                b.glyph = None;
                Ok(b)
            }
            N::MClass { mclass, body, .. } => {
                let mut b = self.row(body, c)?;
                b.class = mclass_type(mclass);
                Ok(b)
            }
            N::SpacingNode { text, .. } => Ok(Box {
                width: space(text)? * c.scale,
                ..Default::default()
            }),
            N::Kern { dimension, .. } => Ok(Box {
                width: measurement(dimension, c)?,
                ..Default::default()
            }),
            N::Styling { style, body, .. } => {
                let mut new = c.clone();
                new.display = matches!(style, StyleStr::Display);
                let desired = match style {
                    StyleStr::Script => 1,
                    StyleStr::Scriptscript => 2,
                    _ => 0,
                };
                // Absolute math styles, including a return from script to text.
                let ratio = |level| match level {
                    1 => f64::from(self.font.constants.script_percent_scale_down()) / 100.,
                    2 => f64::from(self.font.constants.script_script_percent_scale_down()) / 100.,
                    _ => 1.,
                };
                new.scale *= ratio(desired) / ratio(c.level);
                new.level = desired;
                self.row(body, &new)
            }
            N::MathChoice {
                display,
                text,
                script,
                scriptscript,
                ..
            } => self.row(
                if c.level == 2 {
                    scriptscript
                } else if c.level == 1 {
                    script
                } else if c.display {
                    display
                } else {
                    text
                },
                c,
            ),
            N::Sizing { size, body, .. } => {
                let multiplier = [0.5, 0.6, 0.7, 0.8, 0.9, 1., 1.2, 1.44, 1.728, 2.074, 2.488]
                    .get(size.saturating_sub(1) as usize)
                    .ok_or("无效数学字号")?;
                let mut new = c.clone();
                new.scale *= multiplier / new.sizing;
                new.sizing = *multiplier;
                self.row(body, &new)
            }
            N::Phantom { body, .. } => {
                let mut b = self.row(body, c)?;
                b.items.clear();
                b.glyph = None;
                Ok(b)
            }
            N::VPhantom { body, .. } => {
                let mut b = self.node(body, c)?;
                b.items.clear();
                b.width = 0.;
                b.glyph = None;
                Ok(b)
            }
            N::Smash {
                body,
                smash_height,
                smash_depth,
                ..
            } => {
                let mut b = self.node(body, c)?;
                if *smash_height {
                    b.ascent = 0.
                }
                if *smash_depth {
                    b.depth = 0.
                }
                Ok(b)
            }
            N::HBox { body, .. } => self.row(body, c),
            N::Internal { .. } | N::NoNumber { .. } => Ok(Box::default()),
            _ => {
                let kind = serde_json::to_value(n)
                    .ok()
                    .and_then(|v| v["type"].as_str().map(str::to_owned))
                    .unwrap_or_default();
                Err(format!(
                    "OpenType 数学排版尚不支持命令类型 {kind}；可使用默认 ratex-katex 后端"
                ))
            }
        }
    }
    fn symbol(&mut self, text: &str, mode: Mode, c: &Context) -> MathResult<Box> {
        if text.starts_with('\\') {
            return self.font.glyph(symbol(text, mode)?, c);
        }
        let mut result = Box::default();
        let mut x = 0.;
        let mut single = None;
        for ch in text.chars() {
            let b = self.font.glyph(ch, c)?;
            let width = b.width;
            result.italic = b.italic;
            result.accent = b.accent;
            single = b.glyph;
            result.add(b, x, 0.);
            x += width;
        }
        result.width = x;
        if text.chars().count() == 1 {
            result.glyph = single;
        }
        Ok(result)
    }
    fn scripts(
        &mut self,
        base: Option<&N>,
        sup: Option<&N>,
        sub: Option<&N>,
        c: &Context,
    ) -> MathResult<Box> {
        let base = base
            .map(|n| self.node(n, c))
            .transpose()?
            .unwrap_or_default();
        let script = c.script(self.font);
        let sup = sup.map(|n| self.node(n, &script)).transpose()?;
        let mut subctx = script.clone();
        subctx.cramped = true;
        let sub = sub.map(|n| self.node(n, &subctx)).transpose()?;
        if base.limits {
            return self.limits(base, sup, sub, c);
        }
        let ct = self.font.constants;
        let mut up = 0.;
        let mut down = 0.;
        if let Some(b) = &sup {
            up = self
                .font
                .value(
                    if c.cramped {
                        ct.superscript_shift_up_cramped()
                    } else {
                        ct.superscript_shift_up()
                    },
                    c,
                )
                .max(base.ascent - self.font.value(ct.superscript_baseline_drop_max(), c))
                .max(b.depth + self.font.value(ct.superscript_bottom_min(), c));
        }
        if let Some(b) = &sub {
            down = self
                .font
                .value(ct.subscript_shift_down(), c)
                .max(base.depth + self.font.value(ct.subscript_baseline_drop_min(), c))
                .max(b.ascent - self.font.value(ct.subscript_top_max(), c));
        }
        if let (Some(sup), Some(sub)) = (&sup, &sub) {
            let min = self.font.value(ct.sub_superscript_gap_min(), c);
            let deficit = (min - (up + down - sup.depth - sub.ascent)).max(0.);
            let preferred = self
                .font
                .value(ct.superscript_bottom_max_with_subscript(), c);
            let rise = deficit.min((preferred - (up - sup.depth)).max(0.));
            up += rise;
            down += deficit - rise;
        }
        let mut result = Box {
            class: base.class,
            ..Default::default()
        };
        if let Some(b) = sup {
            let kern = self.script_kern(&base, &b, up, true);
            result.add(b, base.width + base.italic + kern, up);
        }
        if let Some(b) = sub {
            let kern = self.script_kern(&base, &b, -down, false);
            result.add(b, base.width + kern, -down);
        }
        result.add(base, 0., 0.);
        result.width += self.font.value(ct.space_after_script(), c);
        Ok(result)
    }
    fn script_kern(&self, base: &Box, script: &Box, shift: f64, upper: bool) -> f64 {
        let Some((gid, bs)) = base.glyph else {
            return 0.;
        };
        let Some((sgid, ss)) = script.glyph else {
            return 0.;
        };
        let infos = self
            .font
            .face
            .tables()
            .math
            .and_then(|m| m.glyph_info)
            .and_then(|i| i.kern_infos);
        let bi = infos.and_then(|i| i.get(gid));
        let si = infos.and_then(|i| i.get(sgid));
        let (bk, sk) = if upper {
            (bi.and_then(|i| i.top_right), si.and_then(|i| i.bottom_left))
        } else {
            (bi.and_then(|i| i.bottom_right), si.and_then(|i| i.top_left))
        };
        let sample = |kern: &Option<math::Kern<'_>>, height: f64, scale: f64| {
            let Some(kern) = kern else { return 0. };
            let h = height * self.font.upem / scale;
            let index = (0..kern.count())
                .find(|i| kern.height(*i).is_some_and(|v| h < f64::from(v.value)))
                .unwrap_or(kern.count());
            kern.kern(index)
                .map_or(0., |v| f64::from(v.value) * scale / self.font.upem)
        };
        let (a, b) = if upper {
            (base.ascent, shift - script.depth)
        } else {
            (-base.depth, shift + script.ascent)
        };
        (sample(&bk, a, bs) + sample(&sk, a - shift, ss))
            .min(sample(&bk, b, bs) + sample(&sk, b - shift, ss))
    }
    fn limits(
        &self,
        base: Box,
        sup: Option<Box>,
        sub: Option<Box>,
        c: &Context,
    ) -> MathResult<Box> {
        let ct = self.font.constants;
        let width = base
            .width
            .max(sup.as_ref().map_or(0., |b| b.width))
            .max(sub.as_ref().map_or(0., |b| b.width));
        let mut result = Box {
            width,
            class: Class::Op,
            ..Default::default()
        };
        if let Some(b) = sup {
            let up = base.ascent
                + (self.font.value(ct.upper_limit_gap_min(), c) + b.depth)
                    .max(self.font.value(ct.upper_limit_baseline_rise_min(), c));
            let x = (width - b.width) / 2. + base.italic / 2.;
            result.add(b, x, up);
        }
        if let Some(b) = sub {
            let down = base.depth
                + (self.font.value(ct.lower_limit_gap_min(), c) + b.ascent)
                    .max(self.font.value(ct.lower_limit_baseline_drop_min(), c));
            let x = (width - b.width) / 2. - base.italic / 2.;
            result.add(b, x, -down);
        }
        let x = (width - base.width) / 2.;
        result.add(base, x, 0.);
        Ok(result)
    }
    fn fraction(
        &mut self,
        numer: &N,
        denom: &N,
        bar: bool,
        thickness: Option<&Measurement>,
        c: &Context,
    ) -> MathResult<Box> {
        let nc = c.fraction(self.font);
        let mut dc = nc.clone();
        dc.cramped = true;
        let n = self.node(numer, &nc)?;
        let d = self.node(denom, &dc)?;
        let ct = self.font.constants;
        let axis = self.font.value(ct.axis_height(), c);
        let thick = if bar {
            thickness
                .map(|m| measurement(m, c))
                .transpose()?
                .unwrap_or(self.font.value(ct.fraction_rule_thickness(), c))
        } else {
            0.
        };
        let mut up = self.font.value(
            if c.display {
                ct.fraction_numerator_display_style_shift_up()
            } else {
                ct.fraction_numerator_shift_up()
            },
            c,
        );
        let mut down = self.font.value(
            if c.display {
                ct.fraction_denominator_display_style_shift_down()
            } else {
                ct.fraction_denominator_shift_down()
            },
            c,
        );
        if bar {
            let ngap = self.font.value(
                if c.display {
                    ct.fraction_num_display_style_gap_min()
                } else {
                    ct.fraction_numerator_gap_min()
                },
                c,
            );
            let dgap = self.font.value(
                if c.display {
                    ct.fraction_denom_display_style_gap_min()
                } else {
                    ct.fraction_denominator_gap_min()
                },
                c,
            );
            up = up.max(axis + thick / 2. + ngap + n.depth);
            down = down.max(d.ascent - axis + thick / 2. + dgap);
        } else {
            up = self.font.value(
                if c.display {
                    ct.stack_top_display_style_shift_up()
                } else {
                    ct.stack_top_shift_up()
                },
                c,
            );
            down = self.font.value(
                if c.display {
                    ct.stack_bottom_display_style_shift_down()
                } else {
                    ct.stack_bottom_shift_down()
                },
                c,
            );
            let gap = self.font.value(
                if c.display {
                    ct.stack_display_style_gap_min()
                } else {
                    ct.stack_gap_min()
                },
                c,
            );
            let extra = (gap - (up + down - n.depth - d.ascent)).max(0.) / 2.;
            up += extra;
            down += extra;
        }
        let width = n.width.max(d.width) + 0.2 * c.scale;
        let mut result = Box {
            width,
            class: Class::Inner,
            ..Default::default()
        };
        let x = (width - n.width) / 2.;
        result.add(n, x, up);
        let x = (width - d.width) / 2.;
        result.add(d, x, -down);
        if bar {
            result.rule(0., axis + thick / 2., width, thick, c);
        }
        Ok(result)
    }
    fn radical(&mut self, body: &N, index: Option<&N>, c: &Context) -> MathResult<Box> {
        let mut cramped = c.clone();
        cramped.cramped = true;
        let body = self.node(body, &cramped)?;
        let ct = self.font.constants;
        let gap = self.font.value(
            if c.display {
                ct.radical_display_style_vertical_gap()
            } else {
                ct.radical_vertical_gap()
            },
            c,
        );
        let thick = self.font.value(ct.radical_rule_thickness(), c);
        let extra = self.font.value(ct.radical_extra_ascender(), c);
        let bar_top = body.ascent + gap + thick;
        let mut radical = self.font.stretch(
            '√',
            body.ascent + body.depth + gap + thick,
            true,
            &c.font(Alphabet::Roman),
        )?;
        let rwidth = radical.width;
        let radical_height = radical.ascent + radical.depth;
        let y = body.ascent + gap + thick - radical.ascent;
        let mut leading = 0.;
        let mut result = Box::default();
        if let Some(index) = index {
            let mut degree = c.script(self.font).script(self.font);
            degree.cramped = false;
            let b = self.node(index, &degree)?;
            let before = self.font.value(ct.radical_kern_before_degree(), c);
            let after = self.font.value(ct.radical_kern_after_degree(), c);
            leading = (before + b.width + after).max(0.);
            let raise = f64::from(ct.radical_degree_bottom_raise_percent()) / 100. * radical_height;
            let degree_y = y - radical.depth + raise + b.depth;
            result.add(b, before.max(0.), degree_y);
        }
        radical.glyph = None;
        result.add(radical, leading, y);
        let width = body.width;
        result.add(body, leading + rwidth, 0.);
        result.rule(leading + rwidth, bar_top, width, thick, c);
        result.ascent += extra;
        Ok(result)
    }
    fn delimiter(&self, text: &str, target: f64, c: &Context) -> MathResult<Box> {
        if text == "." {
            return Ok(Box::default());
        }
        let ch = symbol(text, Mode::Math)?;
        let b = self
            .font
            .stretch(ch, target, true, &c.font(Alphabet::Roman))?;
        let y = self.font.value(self.font.constants.axis_height(), c) - (b.ascent - b.depth) / 2.;
        let mut result = Box::default();
        result.add(b, 0., y);
        Ok(result)
    }
    fn delimit(&self, body: Box, left: &str, right: &str, c: &Context) -> MathResult<Box> {
        if left == "." && right == "." {
            return Ok(body);
        }
        let axis = self.font.value(self.font.constants.axis_height(), c);
        let target = 2.
            * (body.ascent - axis).max(body.depth + axis).max(
                f64::from(self.font.constants.delimited_sub_formula_min_height()) / self.font.upem
                    * c.scale
                    / 2.,
            );
        let l = self.delimiter(left, target, c)?;
        let r = self.delimiter(right, target, c)?;
        let lw = l.width;
        let bw = body.width;
        let mut result = Box {
            class: Class::Inner,
            ..Default::default()
        };
        result.add(l, 0., 0.);
        result.add(body, lw, 0.);
        result.add(r, lw + bw, 0.);
        Ok(result)
    }
    fn accent(
        &mut self,
        label: &str,
        base: &N,
        stretchy: bool,
        under: bool,
        c: &Context,
    ) -> MathResult<Box> {
        let mut b = self.node(base, c)?;
        let ch = match label {
            "\\hat" | "\\widehat" => '\u{0302}',
            "\\tilde" | "\\widetilde" => '\u{0303}',
            "\\bar" | "\\overline" => '\u{0304}',
            "\\vec" | "\\overrightarrow" => '\u{20D7}',
            "\\dot" => '\u{0307}',
            "\\ddot" => '\u{0308}',
            "\\breve" => '\u{0306}',
            "\\check" => '\u{030C}',
            "\\acute" => '\u{0301}',
            "\\grave" => '\u{0300}',
            _ => return Err(format!("OpenType 数学排版尚不支持重音 {label}")),
        };
        let mut accent = self.font.stretch(
            ch,
            if stretchy { b.width } else { 0. },
            false,
            &c.font(Alphabet::Roman),
        )?;
        let gap = 0.04 * c.scale;
        let attach = b.accent.unwrap_or(b.width / 2.);
        let x = if stretchy {
            (b.width - accent.width) / 2.
        } else {
            attach - accent.accent.unwrap_or(accent.width / 2.)
        };
        // Combining accents can have their entire ink above the baseline.
        // Their logical depth (zero) is not the ink's lower edge.
        let mut bottom = f64::INFINITY;
        let mut top = f64::NEG_INFINITY;
        for item in &accent.items {
            if let Item::Glyph { id, y, scale, .. } = item {
                if let Some(r) = self.font.face.glyph_bounding_box(*id) {
                    bottom = bottom.min(y + f64::from(r.y_min) * scale / self.font.upem);
                    top = top.max(y + f64::from(r.y_max) * scale / self.font.upem);
                }
            }
        }
        if !bottom.is_finite() || !top.is_finite() {
            return Err("数学重音缺少轮廓".into());
        }
        let y = if under {
            -b.depth - gap - top
        } else {
            b.ascent
                .max(self.font.value(self.font.constants.accent_base_height(), c))
                + gap
                - bottom
        };
        accent.ascent = top;
        accent.depth = -bottom;
        let width = b.width;
        accent.glyph = None;
        b.add(accent, x, y);
        b.width = width;
        b.glyph = None;
        Ok(b)
    }
    fn array(
        &mut self,
        rows: &[Vec<N>],
        cols: Option<&[AlignSpec]>,
        gaps: &[Option<Measurement>],
        stretch: f64,
        c: &Context,
    ) -> MathResult<Box> {
        if !stretch.is_finite() || stretch <= 0. {
            return Err("无效数组行距".into());
        }
        let align: Vec<_> = cols
            .unwrap_or(&[])
            .iter()
            .filter(|s| matches!(s.align_type, AlignType::Align))
            .collect();
        if cols.is_some_and(|cols| {
            cols.iter()
                .any(|s| matches!(s.align_type, AlignType::Separator))
        }) {
            return Err("OpenType 数学排版尚不支持数组竖线".into());
        }
        let mut cellctx = c.clone();
        cellctx.display = false;
        let cells = rows
            .iter()
            .map(|r| {
                r.iter()
                    .map(|n| self.node(n, &cellctx))
                    .collect::<MathResult<Vec<_>>>()
            })
            .collect::<MathResult<Vec<_>>>()?;
        let count = cells.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0_f64; count];
        for row in &cells {
            for (i, b) in row.iter().enumerate() {
                widths[i] = widths[i].max(b.width);
            }
        }
        let mut result = Box::default();
        let gap = c.scale;
        let mut y = 0.;
        for (r, row) in cells.into_iter().enumerate() {
            let ascent = row.iter().map(|b| b.ascent).fold(0.7 * c.scale, f64::max) * stretch;
            let depth = row.iter().map(|b| b.depth).fold(0.3 * c.scale, f64::max) * stretch;
            y -= ascent;
            let mut x = 0.;
            for (i, b) in row.into_iter().enumerate() {
                let offset = match align.get(i).and_then(|s| s.align.as_deref()).unwrap_or("c") {
                    "l" => 0.,
                    "r" => widths[i] - b.width,
                    _ => (widths[i] - b.width) / 2.,
                };
                result.add(b, x + offset, y);
                x += widths[i] + gap;
            }
            y -= depth + 0.2 * c.scale;
            if let Some(Some(extra)) = gaps.get(r) {
                y -= measurement(extra, c)?;
            }
        }
        let height = (-y - 0.2 * c.scale).max(0.);
        result.depth = result.depth.max(height);
        let shift = height / 2. + self.font.value(self.font.constants.axis_height(), c);
        let mut centered = Box {
            width: widths.iter().sum::<f64>() + gap * count.saturating_sub(1) as f64,
            ..Default::default()
        };
        centered.add(result, 0., shift);
        centered.class = Class::Inner;
        Ok(centered)
    }
}
fn symbol(text: &str, mode: Mode) -> MathResult<char> {
    let mode = if mode == Mode::Math {
        ratex_font::Mode::Math
    } else {
        ratex_font::Mode::Text
    };
    ratex_font::get_symbol(text, mode)
        .and_then(|s| s.codepoint)
        .or_else(|| {
            let mut chars = text.chars();
            let ch = chars.next()?;
            if chars.next().is_none() {
                Some(ch)
            } else {
                None
            }
        })
        .ok_or_else(|| format!("无法解析数学符号 {text}"))
}
fn class(n: &N) -> Class {
    match n {
        N::Atom { family, .. } => match family {
            AtomFamily::Bin => Class::Bin,
            AtomFamily::Rel => Class::Rel,
            AtomFamily::Open => Class::Open,
            AtomFamily::Close => Class::Close,
            AtomFamily::Punct => Class::Punct,
            AtomFamily::Inner => Class::Inner,
        },
        N::OpToken { .. } => Class::Op,
        _ => Class::Ord,
    }
}
fn mclass_type(s: &str) -> Class {
    match s {
        "mop" => Class::Op,
        "mbin" => Class::Bin,
        "mrel" => Class::Rel,
        "mopen" => Class::Open,
        "mclose" => Class::Close,
        "mpunct" => Class::Punct,
        "minner" => Class::Inner,
        _ => Class::Ord,
    }
}
fn spacing(a: Class, b: Class, script: bool) -> f64 {
    use Class::*;
    // TeX's eight atom classes, expressed in mu (1/18 em).
    let i = |c| match c {
        Ord => 0,
        Op => 1,
        Bin => 2,
        Rel => 3,
        Open => 4,
        Close => 5,
        Punct => 6,
        Inner => 7,
    };
    const TABLE: [[u8; 8]; 8] = [
        [0, 3, 4, 5, 0, 0, 0, 3],
        [3, 3, 0, 5, 0, 0, 0, 3],
        [4, 4, 0, 0, 4, 0, 0, 4],
        [5, 5, 0, 0, 5, 0, 0, 5],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 3, 4, 5, 0, 0, 0, 3],
        [3, 3, 0, 3, 3, 3, 3, 3],
        [3, 3, 4, 5, 3, 0, 3, 3],
    ];
    if script && !matches!((a, b), (Ord, Op) | (Op, Ord) | (Op, Op) | (Close, Op)) {
        return 0.;
    }
    f64::from(TABLE[i(a)][i(b)]) / 18.
}
fn font_style(s: &str) -> MathResult<Alphabet> {
    use Alphabet::*;
    match s.trim_start_matches('\\') {
        "mathnormal" => Ok(Auto),
        "text" => Ok(Roman),
        "mathrm" | "textrm" | "textnormal" | "textmd" | "textup" | "rm" => Ok(Roman),
        "mathit" | "textit" | "emph" | "it" => Ok(Italic),
        "mathbf" | "textbf" | "bf" => Ok(Bold),
        "boldsymbol" | "bm" => Ok(BoldItalic),
        "mathsf" | "textsf" | "sf" => Ok(Sans),
        "mathsfit" => Ok(SansItalic),
        "mathtt" | "texttt" | "tt" => Ok(Mono),
        "mathcal" | "mathscr" | "cal" => Ok(Script),
        "mathfrak" | "frak" => Ok(Fraktur),
        "mathbb" => Ok(Double),
        x => Err(format!("OpenType 数学排版尚不支持字体样式 {x}")),
    }
}
fn space(s: &str) -> MathResult<f64> {
    match s {
        "\\," | "\\thinspace" => Ok(3. / 18.),
        "\\:" | "\\medspace" => Ok(4. / 18.),
        "\\;" | "\\thickspace" => Ok(5. / 18.),
        "\\!" | "\\negthinspace" => Ok(-3. / 18.),
        "\\quad" => Ok(1.),
        "\\qquad" => Ok(2.),
        "\\ " | "~" | "\\space" | "\\nobreakspace" => Ok(0.25),
        "\\enspace" => Ok(0.5),
        _ => Err(format!("不支持的数学空白 {s}")),
    }
}
fn measurement(m: &Measurement, c: &Context) -> MathResult<f64> {
    let scale = match m.unit.as_str() {
        "em" => c.scale,
        "ex" => c.x_height * c.scale,
        "mu" => c.scale / 18.,
        "pt" => 1. / c.em_pt,
        "mm" => 72. / 25.4 / c.em_pt,
        "cm" => 72. / 2.54 / c.em_pt,
        "in" => 72. / c.em_pt,
        "" => c.scale,
        _ => return Err(format!("不支持的数学长度单位 {}", m.unit)),
    };
    if !m.number.is_finite() {
        return Err("无效数学长度".into());
    }
    Ok(m.number * scale)
}
