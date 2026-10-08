//! Compound constructions use the selected face throughout, including labels.
use super::*;
use kurbo::{BezPath, Shape};
use ratex_parser::parse_node::{ArrayTag, ProofBranch, ProofLineStyle};

impl Box {
    fn line(&mut self, a: (f64, f64), b: (f64, f64), thickness: f64, c: &Context) {
        let mut path = BezPath::new();
        path.move_to(a);
        path.line_to(b);
        self.path(path, None, Some(c.color.clone()), thickness);
    }
    fn path(
        &mut self,
        path: BezPath,
        fill: Option<String>,
        stroke: Option<String>,
        thickness: f64,
    ) {
        let r = path.bounding_box();
        let extra = if stroke.is_some() { thickness / 2. } else { 0. };
        self.width = self.width.max(r.x1 + extra);
        self.ascent = self.ascent.max(r.y1 + extra);
        self.depth = self.depth.max(-r.y0 + extra);
        self.items.push(Item::Path {
            path,
            fill,
            stroke,
            thickness,
        });
    }
    fn dashed(&mut self, x: f64, y: f64, width: f64, thickness: f64, c: &Context) {
        let dash = (3. * thickness).max(0.12 * c.scale);
        let mut start = 0.;
        while start < width {
            self.rule(
                x + start,
                y + thickness / 2.,
                dash.min(width - start),
                thickness,
                c,
            );
            start += dash * 1.8;
        }
    }
}

impl Engine<'_, '_> {
    pub(super) fn advanced(&mut self, n: &N, c: &Context) -> MathResult<Box> {
        match n {
            N::HtmlMathMl { html, .. } => self.row(html, c),
            N::Html {
                attributes, body, ..
            } => {
                if let Some(bond) = attributes
                    .get("style")
                    .and_then(|s| super::super::parse::ChemicalBond::from_tag(s))
                {
                    self.chemical_bond(bond, c)
                } else {
                    self.html(attributes, body, c)
                }
            }
            N::ProofTree { tree, .. } => Ok(self.proof(tree, c)?.node),
            N::XArrow {
                label, body, below, ..
            } => self.arrow(label, Some(body), below.as_deref(), c),
            N::HorizBrace {
                label,
                base,
                is_over,
                ..
            } => {
                let b = self.node(base, c)?;
                self.brace(label, b, *is_over, c)
            }
            N::Middle { delim, .. } => {
                let mut b = self.delimiter(
                    delim,
                    c.middle_height.ok_or("middle 必须位于 left/right 内")?,
                    c,
                )?;
                b.class = Class::Rel;
                Ok(b)
            }
            N::Rule {
                shift,
                width,
                height,
                ..
            } => {
                let (w, h) = (measurement(width, c)?, measurement(height, c)?);
                if w < 0. || h < 0. {
                    return Ok(Box {
                        width: w,
                        ..Default::default()
                    });
                }
                let y = shift
                    .as_ref()
                    .map(|v| measurement(v, c))
                    .transpose()?
                    .unwrap_or(0.);
                let mut b = Box::default();
                b.rule(0., y + h, w, h, c);
                Ok(b)
            }
            N::Lap {
                alignment, body, ..
            } => {
                let b = self.node(body, c)?;
                let x = match alignment.as_str() {
                    "llap" => -b.width,
                    "clap" => -b.width / 2.,
                    _ => 0.,
                };
                let mut out = Box::default();
                out.add(b, x, 0.);
                out.width = 0.;
                Ok(out)
            }
            N::RaiseBox { dy, body, .. } => {
                let b = self.node(body, c)?;
                let mut out = Box::default();
                out.add(b, 0., measurement(dy, c)?);
                Ok(out)
            }
            N::VCenter { body, .. } => {
                let b = self.node(body, c)?;
                let y = self.font.value(self.font.constants.axis_height(), c)
                    - (b.ascent - b.depth) / 2.;
                let mut out = Box::default();
                out.add(b, 0., y);
                Ok(out)
            }
            N::Verb { body, star, .. } => self.literal(
                &if *star {
                    body.replace(' ', "␣")
                } else {
                    body.clone()
                },
                &c.font(Alphabet::Mono),
            ),
            N::Pmb { mclass, body, .. } => {
                let b = self.row(body, c)?;
                let mut out = Box {
                    class: mclass_type(mclass),
                    ..Default::default()
                };
                out.add(b.clone(), 0., 0.);
                out.add(b, 0.025 * c.scale, 0.);
                Ok(out)
            }
            N::Enclose {
                label,
                body,
                background_color,
                border_color,
                ..
            } => {
                let b = self.node(body, c)?;
                self.enclose(
                    label,
                    b,
                    background_color.as_deref(),
                    border_color.as_deref(),
                    c,
                )
            }
            N::Tag { body, tag, .. } => {
                let b = self.row(body, c)?;
                let t = self.row(tag, &c.font(Alphabet::Roman))?;
                let x = b.width + c.scale;
                let mut out = Box::default();
                out.add(b, 0., 0.);
                out.add(t, x, 0.);
                Ok(out)
            }
            N::CdLabel { label, .. } => self.node(label, &c.script(self.font)),
            N::CdLabelParent { fragment, .. } => self.node(fragment, c),
            N::CdArrow {
                direction,
                label_above,
                label_below,
                ..
            } => self.cd_arrow(direction, label_above.as_deref(), label_below.as_deref(), c),
            N::AccentToken { text, mode, .. } => self.symbol(text, *mode, c),
            N::Cr { .. } => Ok(Box::default()), // row() handles line breaks before dispatch.
            N::Href { body, .. } => self.row(body, c),
            N::Url { url, .. } => self.symbol(url, Mode::Text, &c.font(Alphabet::Mono)),
            N::Raw { string, .. } => self.symbol(string, Mode::Text, &c.font(Alphabet::Roman)),
            N::ColorToken { .. }
            | N::Size { .. }
            | N::LeftRightRight { .. }
            | N::Environment { .. }
            | N::Infix { .. } => Err("解析器返回了未消解的数学节点".into()),
            N::IncludeGraphics { .. } => Err("公式中不支持外部图像".into()),
            _ => Err("未处理的数学节点".into()),
        }
    }

    pub(super) fn left_right(
        &mut self,
        nodes: &[N],
        left: &str,
        right: &str,
        c: &Context,
    ) -> MathResult<Box> {
        fn has_middle(value: &serde_json::Value) -> bool {
            if value["type"] == "leftright" {
                return false;
            }
            if value["type"] == "middle" {
                return true;
            }
            match value {
                serde_json::Value::Array(values) => values.iter().any(has_middle),
                serde_json::Value::Object(values) => values.values().any(has_middle),
                _ => false,
            }
        }
        if !has_middle(&serde_json::to_value(nodes).map_err(|e| e.to_string())?) {
            let b = self.row(nodes, c)?;
            return self.delimit(b, left, right, c);
        }
        let mut probe = c.clone();
        probe.middle_height = Some(0.);
        let b = self.row(nodes, &probe)?;
        let axis = self.font.value(self.font.constants.axis_height(), c);
        let target = 2.
            * (b.ascent - axis).max(b.depth + axis).max(
                f64::from(self.font.constants.delimited_sub_formula_min_height()) / self.font.upem
                    * c.scale
                    / 2.,
            );
        probe.middle_height = Some(target);
        let b = self.row(nodes, &probe)?;
        self.delimit(b, left, right, c)
    }

    pub(super) fn multiline(&mut self, nodes: &[N], c: &Context) -> MathResult<Box> {
        let mut out = Box::default();
        let mut start = 0;
        let mut y = 0.;
        let mut last_depth = 0.;
        for end in 0..=nodes.len() {
            if end < nodes.len() && !matches!(nodes[end], N::Cr { new_line: true, .. }) {
                continue;
            }
            let b = self.row(&nodes[start..end], c)?;
            if start > 0 {
                y -= last_depth + b.ascent + 0.3 * c.scale;
            }
            last_depth = b.depth;
            out.add(b, 0., y);
            if let Some(N::Cr {
                size: Some(size), ..
            }) = nodes.get(end)
            {
                y -= measurement(size, c)?;
            }
            start = end + 1;
        }
        Ok(out)
    }

    pub(super) fn arrow(
        &mut self,
        label: &str,
        above: Option<&N>,
        below: Option<&N>,
        c: &Context,
    ) -> MathResult<Box> {
        let sc = c.script(self.font);
        let a = above.map(|n| self.node(n, &sc)).transpose()?;
        let b = below.map(|n| self.node(n, &sc)).transpose()?;
        let target = a
            .as_ref()
            .map_or(0., |b| b.width)
            .max(b.as_ref().map_or(0., |b| b.width))
            + 0.7 * c.scale;
        let ch = arrow_char(label)?;
        let mut shaft = self.stretch_arrow(ch, target.max(1.4 * c.scale), c)?;
        let w = shaft.width;
        let axis = self.font.value(self.font.constants.axis_height(), c);
        let center = (shaft.ascent - shaft.depth) / 2.;
        let mut out = Box {
            class: Class::Rel,
            ..Default::default()
        };
        let top = axis + shaft.ascent - center;
        let bottom = axis - shaft.depth - center;
        shaft.glyph = None;
        out.add(shaft, 0., axis - center);
        let gap = self
            .font
            .value(self.font.constants.upper_limit_gap_min(), c)
            .max(0.15 * c.scale);
        if let Some(a) = a {
            let y = top + gap + a.depth;
            let x = (w - a.width) / 2.;
            out.add(a, x, y);
        }
        if let Some(b) = b {
            let y = bottom - gap - b.ascent;
            let x = (w - b.width) / 2.;
            out.add(b, x, y);
        }
        Ok(out)
    }

    pub(super) fn special_accent(
        &mut self,
        label: &str,
        b: Box,
        under: bool,
        c: &Context,
    ) -> MathResult<Box> {
        if label == "\\textcircled" {
            return self.enclose("circle", b, None, None, c);
        }
        if matches!(label, "\\dddot" | "\\ddddot") {
            let n = if label == "\\dddot" { 3 } else { 4 };
            let dot = self.font.glyph('\u{0307}', &c.font(Alphabet::Roman))?;
            let advance = 0.18 * c.scale;
            let w = (n - 1) as f64 * advance;
            let mut dots = Box::default();
            let ink = self
                .font
                .face
                .glyph_bounding_box(dot.glyph.unwrap().0)
                .ok_or("重音缺少轮廓")?;
            let bottom = f64::from(ink.y_min) / self.font.upem * c.scale;
            for i in 0..n {
                dots.add(
                    dot.clone(),
                    i as f64 * advance - dot.accent.unwrap_or(0.),
                    0.,
                );
            }
            let mut out = b;
            let x = (out.width - w) / 2.;
            let y = out.ascent + 0.04 * c.scale - bottom;
            let width = out.width;
            out.add(dots, x, y);
            out.width = width;
            out.glyph = None;
            return Ok(out);
        }
        if matches!(label, "\\overgroup" | "\\undergroup") {
            return self.brace("group", b, !under, c);
        }
        if label.contains("linesegment") {
            let mut out = b;
            let y = if under {
                -out.depth - 0.1 * c.scale
            } else {
                out.ascent + 0.1 * c.scale
            };
            let thick = self
                .font
                .value(self.font.constants.overbar_rule_thickness(), c);
            out.line((0., y), (out.width, y), thick, c);
            out.glyph = None;
            return Ok(out);
        }
        let ch = arrow_char(label)?;
        let accent = self.stretch_arrow(ch, b.width, c)?;
        let bottom = ink_bounds(&accent, self.font).0;
        let top = ink_bounds(&accent, self.font).1;
        let mut out = b;
        let width = out.width;
        let y = if under {
            -out.depth - 0.08 * c.scale - top
        } else {
            out.ascent + 0.08 * c.scale - bottom
        };
        out.add(accent.clone(), (width - accent.width) / 2., y);
        out.width = width;
        out.glyph = None;
        Ok(out)
    }

    pub(super) fn brace(
        &mut self,
        label: &str,
        b: Box,
        over: bool,
        c: &Context,
    ) -> MathResult<Box> {
        let w = b.width;
        let gap = 0.12 * c.scale;
        let mut out = b;
        let y = if over {
            out.ascent + gap
        } else {
            -out.depth - gap
        };
        let thick = self
            .font
            .value(self.font.constants.overbar_rule_thickness(), c);
        if label.contains("bracket") {
            let tip = if over {
                0.18 * c.scale
            } else {
                -0.18 * c.scale
            };
            out.line((0., y), (w, y), thick, c);
            out.line((0., y), (0., y - tip), thick, c);
            out.line((w, y), (w, y - tip), thick, c);
        } else {
            let ch = if over { '⏞' } else { '⏟' };
            let brace = self.font.stretch(ch, w, false, &c.font(Alphabet::Roman))?;
            let (bottom, top) = ink_bounds(&brace, self.font);
            let by = if over { y - bottom } else { y - top };
            out.add(brace.clone(), (w - brace.width) / 2., by);
        }
        out.class = Class::Op;
        out.limits = true;
        out.glyph = None;
        Ok(out)
    }

    pub(super) fn enclose(
        &mut self,
        label: &str,
        b: Box,
        bg: Option<&str>,
        border: Option<&str>,
        c: &Context,
    ) -> MathResult<Box> {
        let thick = self
            .font
            .value(self.font.constants.overbar_rule_thickness(), c);
        let pad = if label.contains("cancel") { 0.1 } else { 0.2 } * c.scale;
        let (w, top, bottom) = (b.width + 2. * pad, b.ascent + pad, -b.depth - pad);
        let mut out = Box::default();
        if let Some(bg) = bg {
            out.path(
                kurbo::Rect::new(0., bottom, w, top).to_path(1e-4),
                Some(bg.into()),
                None,
                0.,
            );
        }
        out.add(b, pad, 0.);
        let mut pen = c.clone();
        if let Some(border) = border {
            pen.color = border.into();
        }
        match label {
            "\\cancel" | "\\bcancel" | "\\xcancel" => {
                if label != "\\bcancel" {
                    out.line((0., bottom), (w, top), thick, &pen);
                }
                if label != "\\cancel" {
                    out.line((0., top), (w, bottom), thick, &pen);
                }
            }
            "\\sout" => out.line(
                (0., (top + bottom) / 2.),
                (w, (top + bottom) / 2.),
                thick,
                &pen,
            ),
            "\\angl" => {
                out.line((0., top), (w, top), thick, &pen);
                out.line((w, top), (w, bottom), thick, &pen);
            }
            "\\phase" => {
                out.line((0., bottom), (w, bottom), thick, &pen);
                out.line((0., bottom), (0.3 * c.scale, top), thick, &pen);
            }
            "circle" => out.path(
                kurbo::Ellipse::new(
                    (w / 2., (top + bottom) / 2.),
                    (w / 2., (top - bottom) / 2.),
                    0.,
                )
                .to_path(1e-4),
                None,
                Some(pen.color.clone()),
                thick,
            ),
            _ => out.path(
                kurbo::Rect::new(0., bottom, w, top).to_path(1e-4),
                None,
                border.map(str::to_owned).or_else(|| {
                    if bg.is_none() {
                        Some(pen.color.clone())
                    } else {
                        None
                    }
                }),
                thick,
            ),
        }
        out.width = w;
        Ok(out)
    }

    pub(super) fn composite_symbol(&mut self, text: &str, c: &Context) -> MathResult<Option<Box>> {
        let ch = if text.starts_with('\\') {
            symbol(text, Mode::Math).ok()
        } else {
            let mut chars = text.chars();
            let first = chars.next();
            if chars.next().is_none() { first } else { None }
        };
        if ch == Some('\u{E020}') {
            let mut b = Box::default();
            let axis = self.font.value(self.font.constants.axis_height(), c);
            let thick = self
                .font
                .value(self.font.constants.overbar_rule_thickness(), c);
            b.line(
                (0.08 * c.scale, axis - 0.4 * c.scale),
                (0.55 * c.scale, axis + 0.4 * c.scale),
                thick,
                c,
            );
            b.width = 0.;
            return Ok(Some(b));
        }
        let (base, slash, variant) = match ch.map(|c| c as u32) {
            Some(0xE00D) => (Some('≩'), false, true),
            Some(0xE00C) => (Some('≨'), false, true),
            Some(0xE00E) => (Some('≧'), true, false),
            Some(0xE011) => (Some('≦'), true, false),
            Some(0xE00F) => (Some('⩾'), true, false),
            Some(0xE010) => (Some('⩽'), true, false),
            Some(0xE006) => (Some('∣'), true, false),
            Some(0xE007) => (Some('∥'), true, false),
            Some(0xE016) => (Some('⫅'), true, false),
            Some(0xE018) => (Some('⫆'), true, false),
            Some(0xE01A) => (Some('⊊'), false, true),
            Some(0xE01B) => (Some('⊋'), false, true),
            Some(0xE017) => (Some('⫋'), false, true),
            Some(0xE019) => (Some('⫌'), false, true),
            Some(0xE131) => (Some('𝚤'), false, false),
            Some(0xE237) => (Some('𝚥'), false, false),
            Some(0xE258) => (Some('≘'), false, false),
            Some(0xE25E) => (Some('≞'), false, false),
            _ => (None, false, false),
        };
        let Some(base) = base else {
            return Ok(None);
        };
        let mut b = self.font.glyph(base, &c.font(Alphabet::Roman))?;
        if variant {
            if let Some(id) = self.font.face.glyph_variation_index(base, '\u{FE00}') {
                b = self
                    .font
                    .glyph_id(id, Some(base as u32), &c.font(Alphabet::Roman));
            }
        }
        if slash {
            let thick = self
                .font
                .value(self.font.constants.overbar_rule_thickness(), c);
            b.line(
                (0.15 * b.width, -b.depth - 0.1 * c.scale),
                (0.85 * b.width, b.ascent + 0.1 * c.scale),
                thick,
                c,
            );
            b.glyph = None;
        }
        Ok(Some(b))
    }

    fn html(
        &mut self,
        attributes: &std::collections::HashMap<String, String>,
        body: &[N],
        c: &Context,
    ) -> MathResult<Box> {
        let mut ctx = c.clone();
        let mut bg = None;
        let mut underline = false;
        if let Some(style) = attributes.get("style") {
            for declaration in style.split(';') {
                let Some((key, value)) = declaration.split_once(':') else {
                    continue;
                };
                let value = value.trim();
                match key.trim() {
                    "color" => ctx.color = value.into(),
                    "background-color" => bg = Some(value),
                    "font-weight" if value == "bold" => ctx = ctx.font(Alphabet::Bold),
                    "font-style" if value == "italic" => ctx = ctx.font(Alphabet::Italic),
                    "font-size" => {
                        for unit in ["px", "pt", "em"] {
                            if let Some(n) = value
                                .strip_suffix(unit)
                                .and_then(|s| s.trim().parse::<f64>().ok())
                            {
                                if !n.is_finite() || n <= 0. {
                                    return Err("无效 HTML 数学字号".into());
                                }
                                ctx.scale = if unit == "em" {
                                    ctx.scale * n
                                } else {
                                    n * if unit == "px" { 0.75 } else { 1. } / ctx.em_pt
                                };
                                break;
                            }
                        }
                    }
                    "text-decoration" => underline = value.contains("underline"),
                    _ => {}
                }
            }
        }
        let mut b = self.row(body, &ctx)?;
        if let Some(bg) = bg {
            let mut out = Box::default();
            out.path(
                kurbo::Rect::new(0., -b.depth, b.width, b.ascent).to_path(1e-4),
                Some(bg.into()),
                None,
                0.,
            );
            out.add(b, 0., 0.);
            b = out;
        }
        if underline {
            let y = -b.depth - 0.1 * ctx.scale;
            b.line(
                (0., y),
                (b.width, y),
                self.font
                    .value(self.font.constants.underbar_rule_thickness(), &ctx),
                &ctx,
            );
        }
        Ok(b)
    }
}

fn ink_bounds(b: &Box, font: &MathFont<'_>) -> (f64, f64) {
    let (mut bottom, mut top) = (f64::INFINITY, f64::NEG_INFINITY);
    for item in &b.items {
        if let Item::Glyph { id, y, scale, .. } = item {
            if let Some(r) = font.face.glyph_bounding_box(*id) {
                bottom = bottom.min(y + f64::from(r.y_min) * scale / font.upem);
                top = top.max(y + f64::from(r.y_max) * scale / font.upem);
            }
        }
    }
    if bottom.is_finite() {
        (bottom, top)
    } else {
        (-b.depth, b.ascent)
    }
}
fn arrow_char(label: &str) -> MathResult<char> {
    Ok(match label {
        "\\xrightarrow" | "\\overrightarrow" | "\\underrightarrow" | "\\\u{2192}" | "right" => '→',
        "\\xleftarrow" | "\\overleftarrow" | "\\underleftarrow" | "left" => '←',
        "\\xleftrightarrow" | "\\overleftrightarrow" | "\\underleftrightarrow" => '↔',
        "\\xRightarrow" | "\\Overrightarrow" => '⇒',
        "\\xLeftarrow" => '⇐',
        "\\xLeftrightarrow" => '⇔',
        "\\xhookrightarrow" => '↪',
        "\\xhookleftarrow" => '↩',
        "\\xmapsto" => '↦',
        "\\xrightharpoonup" | "\\overrightharpoon" => '⇀',
        "\\xrightharpoondown" => '⇁',
        "\\xleftharpoonup" | "\\overleftharpoon" => '↼',
        "\\xleftharpoondown" => '↽',
        "\\xrightleftharpoons" => '⇌',
        "\\xleftrightharpoons" => '⇋',
        "\\xrightleftarrows" | "\\xtofrom" => '⇄',
        "\\xtwoheadrightarrow" => '↠',
        "\\xtwoheadleftarrow" => '↞',
        "\\xlongequal" | "horiz_eq" => '=',
        "\\xleftequilibrium" => '⇋',
        "\\xrightequilibrium" => '⇌',
        _ => return Err(format!("无法识别伸缩箭头 {label}")),
    })
}

struct Proof {
    node: Box,
    root_center: f64,
    root_width: f64,
}
impl Engine<'_, '_> {
    fn proof(&mut self, tree: &ProofBranch, c: &Context) -> MathResult<Proof> {
        self.steps += 1;
        if self.steps > 20_000 || c.depth > 128 {
            return Err("证明树超过复杂度限制".into());
        }
        let mut ctx = c.font(Alphabet::Roman);
        ctx.depth += 1;
        let conclusion = self.row(&tree.conclusion, &ctx)?;
        let cw = conclusion.width;
        if tree.premises.is_empty() {
            return Ok(Proof {
                root_center: cw / 2.,
                root_width: cw,
                node: conclusion,
            });
        }
        let premises = tree
            .premises
            .iter()
            .map(|p| self.proof(p, &ctx))
            .collect::<MathResult<Vec<_>>>()?;
        let gap = 1.2 * c.scale;
        let pw = premises.iter().map(|p| p.node.width).sum::<f64>()
            + gap * premises.len().saturating_sub(1) as f64;
        let pa = premises.iter().map(|p| p.node.ascent).fold(0., f64::max);
        let pd = premises.iter().map(|p| p.node.depth).fold(0., f64::max);
        let mut cursor = 0.;
        let mut left = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        for p in &premises {
            left = left.min(cursor + p.root_center - p.root_width / 2.);
            right = right.max(cursor + p.root_center + p.root_width / 2.);
            cursor += p.node.width + gap;
        }
        let center = (left + right) / 2.;
        let rw = (right - left).max(cw).max(0.7 * c.scale);
        let thick = self
            .font
            .value(self.font.constants.overbar_rule_thickness(), c);
        let vgap = 0.25 * c.scale;
        let cy = 0.;
        let rule_y = if tree.root_at_top {
            -conclusion.depth - vgap - thick / 2.
        } else {
            conclusion.ascent + vgap + thick / 2.
        };
        let py = if tree.root_at_top {
            rule_y - thick / 2. - vgap - pa
        } else {
            rule_y + thick / 2. + vgap + pd
        };
        let mut parts = vec![(conclusion, center - cw / 2., cy)];
        cursor = 0.;
        for p in premises {
            parts.push((p.node, cursor, py));
            cursor += parts.last().unwrap().0.width + gap;
        }
        let mut min_x = (center - rw / 2.).min(0.);
        let mut max_x = pw.max(center + rw / 2.);
        for (label, is_left) in [(&tree.left_label, true), (&tree.right_label, false)] {
            if let Some(label) = label {
                let b = self.row(label, &ctx)?;
                let x = if is_left {
                    center - rw / 2. - 0.1 * c.scale - b.width
                } else {
                    center + rw / 2. + 0.1 * c.scale
                };
                let y = rule_y - (b.ascent - b.depth) / 2.;
                min_x = min_x.min(x);
                max_x = max_x.max(x + b.width);
                parts.push((b, x, y));
            }
        }
        let mut out = Box::default();
        for (b, x, y) in parts {
            out.add(b, x - min_x, y);
        }
        if !matches!(tree.line_style, ProofLineStyle::None) {
            let x = center - rw / 2. - min_x;
            if matches!(tree.line_style, ProofLineStyle::Dashed) {
                out.dashed(x, rule_y, rw, thick, c);
            } else {
                out.rule(x, rule_y + thick / 2., rw, thick, c);
            }
        }
        out.width = max_x - min_x;
        Ok(Proof {
            node: out,
            root_center: center - min_x,
            root_width: cw,
        })
    }
}

impl Engine<'_, '_> {
    pub(super) fn array_full(&mut self, n: &N, c: &Context) -> MathResult<Box> {
        let N::Array {
            body,
            cols,
            row_gaps,
            arraystretch,
            hlines_before_row,
            tags,
            col_separation_type,
            hskip_before_and_after,
            add_jot,
            leqno,
            ..
        } = n
        else {
            unreachable!()
        };
        if !arraystretch.is_finite() || *arraystretch <= 0. {
            return Err("无效数组行距".into());
        }
        if body.is_empty() {
            return Ok(Box::default());
        }
        let specs = cols.as_deref().unwrap_or(&[]);
        let align: Vec<_> = specs
            .iter()
            .filter(|s| matches!(s.align_type, AlignType::Align))
            .collect();
        let mut cellctx = c.clone();
        cellctx.display = false;
        if col_separation_type.as_deref() == Some("small") {
            cellctx = c.script(self.font);
        }
        let cells = body
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
        let side = if col_separation_type.as_deref() == Some("small") {
            0.14 * c.scale
        } else {
            0.5 * c.scale
        };
        let edge = if *hskip_before_and_after == Some(true) {
            side
        } else {
            0.
        };
        let mut positions = vec![edge];
        for i in 0..count {
            let previous = positions[i] + widths[i];
            let gap = align.get(i).and_then(|s| s.postgap).unwrap_or(side)
                + align.get(i + 1).and_then(|s| s.pregap).unwrap_or(side);
            positions.push(previous + if i + 1 < count { gap } else { edge });
        }
        let grid_width = *positions.last().unwrap_or(&0.);
        let thick = self
            .font
            .value(self.font.constants.fraction_rule_thickness(), c);
        let mut baselines = vec![];
        let mut boundaries = vec![0.];
        let mut y = 0.;
        for (r, row) in cells.iter().enumerate() {
            let asc = row
                .iter()
                .map(|b| b.ascent)
                .fold(0.7 * c.scale * arraystretch, f64::max);
            let dep = row
                .iter()
                .map(|b| b.depth)
                .fold(0.3 * c.scale * arraystretch, f64::max);
            y -= asc;
            baselines.push(y);
            y -= dep;
            if r + 1 < cells.len() {
                y -= if *add_jot == Some(true) {
                    0.3 * c.scale
                } else {
                    0.2 * c.scale
                };
            }
            if let Some(Some(gap)) = row_gaps.get(r) {
                y -= measurement(gap, c)?;
            }
            if let Some(lines) = hlines_before_row.get(r + 1) {
                if lines.len() > 1 {
                    y -= 0.15 * c.scale * (lines.len() - 1) as f64;
                }
            }
            boundaries.push(y);
        }
        let total = (-y).max(0.);
        let mut out = Box::default();
        for (r, row) in cells.into_iter().enumerate() {
            for (i, b) in row.into_iter().enumerate() {
                let offset = match align.get(i).and_then(|s| s.align.as_deref()).unwrap_or("c") {
                    "l" => 0.,
                    "r" => widths[i] - b.width,
                    _ => (widths[i] - b.width) / 2.,
                };
                out.add(b, positions[i] + offset, baselines[r]);
            }
        }
        for (r, lines) in hlines_before_row.iter().enumerate() {
            let Some(y) = boundaries.get(r) else {
                continue;
            };
            for (i, dashed) in lines.iter().enumerate() {
                let y = y + 0.15 * c.scale * i as f64;
                if *dashed {
                    out.dashed(0., y, grid_width, thick, c);
                } else {
                    out.rule(0., y + thick / 2., grid_width, thick, c);
                }
            }
        }
        let mut col = 0;
        let mut duplicate = 0;
        for spec in specs {
            if matches!(spec.align_type, AlignType::Align) {
                col += 1;
                duplicate = 0;
                continue;
            }
            if col > count {
                continue;
            }
            let x = if col == 0 {
                0.
            } else if col == count {
                grid_width
            } else {
                (positions[col] + positions[col - 1] + widths[col - 1]) / 2.
            } + 0.15 * c.scale * duplicate as f64;
            duplicate += 1;
            if spec.align.as_deref() == Some(":") {
                let mut top = 0.;
                while top > -total {
                    out.rule(x, top, thick, (0.15 * c.scale).min(total + top), c);
                    top -= 0.27 * c.scale;
                }
            } else {
                out.rule(x, 0., thick, total, c);
            }
        }
        let mut tag_width = 0_f64;
        let mut tag_boxes = vec![];
        if let Some(tags) = tags {
            for (r, tag) in tags.iter().enumerate() {
                let b = match tag {
                    ArrayTag::Explicit(nodes) => Some(self.row(nodes, &c.font(Alphabet::Roman))?),
                    ArrayTag::Auto(true) => Some(self.symbol(
                        &format!("({})", r + 1),
                        Mode::Text,
                        &c.font(Alphabet::Roman),
                    )?),
                    _ => None,
                };
                if let Some(b) = b {
                    tag_width = tag_width.max(b.width);
                    tag_boxes.push((r, b));
                }
            }
        }
        let shift = if tag_width > 0. && *leqno == Some(true) {
            tag_width + c.scale
        } else {
            0.
        };
        let mut tagged = Box::default();
        tagged.add(out, shift, 0.);
        for (r, b) in tag_boxes {
            if let Some(y) = baselines.get(r) {
                let x = if shift > 0. { 0. } else { grid_width + c.scale };
                tagged.add(b, x, *y);
            }
        }
        tagged.width = grid_width
            + if tag_width > 0. {
                tag_width + c.scale
            } else {
                0.
            };
        let mut centered = Box {
            class: Class::Inner,
            ..Default::default()
        };
        centered.add(
            tagged,
            0.,
            total / 2. + self.font.value(self.font.constants.axis_height(), c),
        );
        Ok(centered)
    }
    fn cd_arrow(
        &mut self,
        direction: &str,
        above: Option<&N>,
        below: Option<&N>,
        c: &Context,
    ) -> MathResult<Box> {
        if direction == "none" {
            return Ok(Box {
                width: c.scale,
                ..Default::default()
            });
        }
        if matches!(direction, "right" | "left" | "horiz_eq") {
            return self.arrow(direction, above, below, c);
        }
        let h = 2. * c.scale;
        let axis = self.font.value(self.font.constants.axis_height(), c);
        let mut out = Box::default();
        if direction == "vert_eq" {
            let thick = self
                .font
                .value(self.font.constants.fraction_rule_thickness(), c);
            out.rule(0.2 * c.scale, axis + h / 2., thick, h, c);
            out.rule(0.4 * c.scale, axis + h / 2., thick, h, c);
        } else {
            let ch = if direction == "up" {
                '↑'
            } else if direction == "down" {
                '↓'
            } else {
                return Err(format!("未知 CD 箭头 {direction}"));
            };
            let b = self.font.stretch(ch, h, true, &c.font(Alphabet::Roman))?;
            let shift = axis - (b.ascent - b.depth) / 2.;
            out.add(b, 0., shift);
        }
        let width = out.width;
        let mut min_x = 0_f64;
        for (label, left) in [(above, true), (below, false)] {
            if let Some(label) = label {
                let b = self.node(label, &c.script(self.font))?;
                let x = if left {
                    -b.width - 0.15 * c.scale
                } else {
                    width + 0.15 * c.scale
                };
                let y = axis - (b.ascent - b.depth) / 2.;
                min_x = min_x.min(x);
                out.add(b, x, y);
            }
        }
        if min_x < 0. {
            let mut shifted = Box::default();
            shifted.add(out, -min_x, 0.);
            out = shifted;
        }
        Ok(out)
    }
}

struct NativeOutline(BezPath);
impl ttf_parser::OutlineBuilder for NativeOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((f64::from(x), f64::from(y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((f64::from(x), f64::from(y)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0
            .quad_to((f64::from(x1), f64::from(y1)), (f64::from(x), f64::from(y)));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            (f64::from(x1), f64::from(y1)),
            (f64::from(x2), f64::from(y2)),
            (f64::from(x), f64::from(y)),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}
impl Engine<'_, '_> {
    /// Use the native minus's advance, ink endpoints, baseline and thickness
    /// for every partial bond. Only the horizontal extent of each of the
    /// three segments changes; all y coordinates and the font's outline stay.
    fn chemical_bond(
        &self,
        bond: super::super::parse::ChemicalBond,
        c: &Context,
    ) -> MathResult<Box> {
        use super::super::parse::ChemicalBond;
        let base = self.font.glyph('−', &c.font(Alphabet::Roman))?;
        let id = base.glyph.ok_or("化学键缺少减号字形")?.0;
        let mut outline = NativeOutline(BezPath::new());
        self.font
            .face
            .outline_glyph(id, &mut outline)
            .ok_or("化学键缺少减号轮廓")?;
        outline
            .0
            .apply_affine(kurbo::Affine::scale(c.scale / self.font.upem));
        let ink = outline.0.bounding_box();
        if ink.width() <= 0. || ink.height() <= 0. {
            return Err("化学键减号轮廓无效".into());
        }
        let segment = ink.width() / 5.;
        let mut out = Box {
            width: base.width,
            ascent: base.ascent,
            depth: base.depth,
            ..Default::default()
        };
        let (solid, dashed): (&[f64], f64) = match bond {
            ChemicalBond::Dashed => (&[], 0.),
            ChemicalBond::PartialDouble => (&[-0.1], 0.1),
            ChemicalBond::PartialTriple => (&[-0.2, 0.], 0.2),
            ChemicalBond::DashedMiddle => (&[-0.2, 0.2], 0.),
        };
        for shift in solid {
            out.add(base.clone(), 0., shift * c.scale);
        }
        for i in 0..3 {
            let mut path = outline.0.clone();
            path.apply_affine(kurbo::Affine::new([
                0.2,
                0.,
                0.,
                1.,
                0.8 * ink.x0 + 2. * f64::from(i) * segment,
                dashed * c.scale,
            ]));
            out.path(path, Some(c.color.clone()), None, 0.);
        }
        Ok(out)
    }

    fn literal(&mut self, text: &str, c: &Context) -> MathResult<Box> {
        let mut out = Box::default();
        let mut x = 0.;
        for ch in text.chars() {
            let b = self.font.glyph(ch, c)?;
            let w = b.width;
            out.add(b, x, 0.);
            x += w;
        }
        out.width = x;
        Ok(out)
    }
    /// Some real fonts have no horizontal recipe (or malformed terminal
    /// connectors). Extend only the shaft region of their native outline;
    /// arrowheads, hooks, stroke thickness and vertical positions stay intact.
    /// This fallback is restricted to horizontal arrows and equality signs.
    fn stretch_arrow(&self, ch: char, target: f64, c: &Context) -> MathResult<Box> {
        let roman = c.font(Alphabet::Roman);
        if let Ok(b) = self.font.stretch(ch, target, false, &roman) {
            if b.width + 1e-9 >= target {
                return Ok(b);
            }
        }
        let base = self.font.glyph(ch, &roman)?;
        if base.width >= target {
            return Ok(base);
        }
        let id = base.glyph.unwrap().0;
        let mut outline = NativeOutline(BezPath::new());
        self.font
            .face
            .outline_glyph(id, &mut outline)
            .ok_or("箭头缺少可导出的轮廓")?;
        outline
            .0
            .apply_affine(kurbo::Affine::scale(c.scale / self.font.upem));
        let bounds = outline.0.bounding_box();
        let cut_left = bounds.x0 + 0.4 * (bounds.x1 - bounds.x0);
        let cut_right = bounds.x0 + 0.6 * (bounds.x1 - bounds.x0);
        let extra = target - base.width;
        if cut_right <= cut_left {
            return Err("箭头字形宽度无效".into());
        }
        let map = |p: kurbo::Point| {
            kurbo::Point::new(
                p.x + extra * ((p.x - cut_left) / (cut_right - cut_left)).clamp(0., 1.),
                p.y,
            )
        };
        let mut path = BezPath::new();
        kurbo::flatten(outline.0.iter(), 0.00005 * c.scale, |el| match el {
            kurbo::PathEl::MoveTo(p) => path.move_to(map(p)),
            kurbo::PathEl::LineTo(p) => path.line_to(map(p)),
            kurbo::PathEl::ClosePath => path.close_path(),
            _ => unreachable!(),
        });
        let mut result = Box::default();
        result.path(path, Some(c.color.clone()), None, 0.);
        result.width = target;
        Ok(result)
    }
}
