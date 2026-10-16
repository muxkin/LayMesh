use crate::animation::{
    css::Declaration as StyleDeclaration,
    values::{self, Context},
};
use crate::css_values;
use crate::{
    Diagnostic, Loc, Result,
    engine::{API, Engine},
    model::*,
};
use serde_json::json;
use std::collections::BTreeMap;

fn split(s: &str, delimiter: char) -> Vec<String> {
    let (mut depth, mut quote, mut escaped, mut start) = (0, '\0', false, 0);
    let mut out = vec![];
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if quote != '\0' {
            if c == quote {
                quote = '\0'
            }
            continue;
        }
        if matches!(c, '\'' | '"') {
            quote = c;
            continue;
        }
        if matches!(c, '(' | '[') {
            depth += 1;
        }
        if matches!(c, ')' | ']') {
            depth -= 1;
        }
        if c == delimiter && depth == 0 {
            out.push(s[start..i].trim().into());
            start = i + c.len_utf8();
        }
    }
    out.push(s[start..].trim().into());
    out
}
fn css_color(s: &str) -> &str {
    match s.to_ascii_lowercase().as_str() {
        "black" => "#000000",
        "white" => "#ffffff",
        "red" => "#ff0000",
        "green" => "#008000",
        "blue" => "#0000ff",
        "gray" | "grey" => "#808080",
        "yellow" => "#ffff00",
        "orange" => "#ffa500",
        "purple" => "#800080",
        "navy" => "#000080",
        "transparent" => "none",
        _ => s,
    }
}
fn matches(selector: &str, kind: &str, args: &Args, parent: &Args) -> bool {
    let element = |kind: &str, args: &Args| crate::animation::css::Element {
        kind: if crate::arrows::is_arrow(kind) {
            "arrow".into()
        } else {
            kind.into()
        },
        class: string(args, "class", ""),
    };
    let mut chain = vec![element(kind, args)];
    let mut current = Some(parent.clone());
    while let Some(p) = current.take() {
        let kind = string(&p, "__style_type", "");
        if !kind.is_empty() {
            chain.push(element(&kind, &p));
        }
        current = match p.get("__style_parent") {
            Some(V::Map(v)) => Some((**v).clone()),
            _ => None,
        };
    }
    chain.reverse();
    crate::animation::css::matches(selector, &chain)
}

#[derive(Clone, Debug)]
pub struct StyleRule {
    selector: String,
    declarations: Vec<StyleDeclaration>,
    scope: Option<String>,
    specificity: usize,
    media: String,
    supports: String,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct StylesheetMetadata {
    pub name: String,
    pub version: String,
    pub source: String,
    pub sha256: String,
    pub license: String,
}
impl StylesheetMetadata {
    pub fn path(&self) -> String {
        format!("@laymesh/{}.css", self.name)
    }
}
/// Editor help and the engine's distributed libraries use the pinned manifest.
pub fn bundled_stylesheets() -> &'static [StylesheetMetadata] {
    static MANIFEST: std::sync::LazyLock<Vec<StylesheetMetadata>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_str(include_str!("../assets/styles/manifest.json")).unwrap()
        });
    &MANIFEST
}
pub fn bundled_stylesheet(path: &str) -> Option<&'static str> {
    match path {
        "@laymesh/bootstrap.css" => Some(include_str!("../assets/styles/bootstrap.css")),
        "@laymesh/open-props.css" => Some(include_str!("../assets/styles/open-props.css")),
        "@laymesh/animate.css" => Some(include_str!("../assets/styles/animate.css")),
        "@laymesh/gl-transitions.css" => Some(include_str!("../assets/styles/gl-transitions.css")),
        _ => None,
    }
}
pub fn property_supported(key: &str) -> bool {
    key.starts_with("--")
        || crate::page_transition::supported(key)
        || API["cssProperties"]
            .as_array()
            .is_some_and(|props| props.iter().any(|v| v["name"] == key.replace('_', "-")))
}
fn selector(selector: &str) -> Option<(String, usize)> {
    let root = matches!(
        selector,
        ":root" | "html" | "body" | ":where(html)" | ":where(:root)"
    );
    if root {
        return Some((
            "canvas".into(),
            if selector.starts_with(":where") {
                0
            } else if selector == ":root" {
                100
            } else {
                1
            },
        ));
    }
    let plain = if let Some((plain, part)) = selector.split_once("::") {
        let name = part.split('(').next().unwrap();
        if ![
            "area",
            "axis",
            "axis-label",
            "tick-label",
            "legend",
            "colorbar",
            "marker",
            "grid",
        ]
        .contains(&name)
            || !regex::Regex::new(r"^[\w-]+(?:\([\w-]+\))?$")
                .unwrap()
                .is_match(part)
        {
            return None;
        }
        plain
    } else {
        selector
    };
    crate::css_selectors::supported(plain).then(|| {
        (
            selector.into(),
            crate::animation::css::specificity(plain) + usize::from(selector.contains("::")),
        )
    })
}
fn parse_sheet(
    source: &str,
    file: &str,
    origin: Loc,
    scope: Option<&str>,
) -> Result<(Vec<StyleRule>, crate::animation::css::Sheet)> {
    let (_, mut sheet) = crate::animation::css::extract(source, file, origin, scope)?;
    let mut parsed = vec![];
    for rule in &sheet.rules {
        let declarations: Vec<_> = rule
            .declarations
            .iter()
            .filter_map(|d| {
                if crate::animation::css::is_motion_property(&d.property) {
                    return None;
                }
                if !property_supported(&d.property) {
                    sheet.warnings.push(Diagnostic::new(
                        "W_CSS",
                        format!("Ignored property {}; geometry belongs in .lay", d.property),
                        file,
                        d.loc,
                    ));
                    return None;
                }
                let mut d = d.clone();
                if !d.property.starts_with("--") {
                    d.property = d.property.replace('-', "_");
                }
                Some(d)
            })
            .collect();
        if let Some((selector, specificity)) = selector(&rule.selector) {
            parsed.push(StyleRule {
                selector,
                declarations,
                scope: scope.map(str::to_owned),
                specificity,
                media: rule.media.clone(),
                supports: rule.supports.clone(),
            });
        } else if !declarations.is_empty() {
            sheet.warnings.push(Diagnostic::new(
                "W_CSS",
                format!("Ignored selector {}", rule.selector),
                file,
                declarations[0].loc,
            ));
        }
    }
    Ok((parsed, sheet))
}
/// Editor validation follows the same recovery policy as compilation.
pub fn validate_stylesheet(source: &str, file: &str, origin: Loc) -> Result<()> {
    let (_, sheet) = parse_sheet(source, file, origin, None)?;
    if let Some(d) = sheet.warnings.iter().find(|d| {
        d.message.starts_with("Ignored invalid declaration") || d.message.starts_with("CSS syntax")
    }) {
        return Err(Diagnostic::new("E_LCSS", &d.message, file, d.loc));
    }
    Ok(())
}
pub fn stylesheet_diagnostics(source: &str, file: &str, origin: Loc) -> Vec<Diagnostic> {
    let mut engine = Engine::new(Host::default());
    engine.file = file.into();
    if let Ok((rules, sheet)) = parse_sheet(source, file, origin, None) {
        for d in sheet.warnings {
            engine.css_warning(d);
        }
        for rule in &rules {
            for d in &rule.declarations {
                if d.property.starts_with("--") || d.value.contains("var(") {
                    continue;
                }
                if let Err(e) = engine.css_expansion(d, Context::default(), "#000000") {
                    engine.css_warning(e);
                }
            }
        }
    }
    engine.css_diagnostics()
}
const INHERITED: [&str; 9] = [
    "math_font",
    "math_text_fallback",
    "font_family",
    "font_size",
    "font_weight",
    "font_style",
    "color",
    "line_height",
    "text_shadow",
];
pub(crate) fn substitute(
    value: &str,
    vars: &BTreeMap<String, String>,
    seen: &mut Vec<String>,
) -> std::result::Result<String, String> {
    let Some(start) = value.find("var(") else {
        return Ok(value.into());
    };
    let mut depth = 1;
    let mut end = None;
    for (offset, ch) in value[start + 4..].char_indices() {
        if ch == '(' {
            depth += 1;
        }
        if ch == ')' {
            depth -= 1;
            if depth == 0 {
                end = Some(start + 4 + offset);
                break;
            }
        }
    }
    let end = end.ok_or("未闭合 var()")?;
    let args = split(&value[start + 4..end], ',');
    let name = args[0].trim();
    if !name.starts_with("--") {
        return Err("var() 需要样式变量名".into());
    }
    if seen.iter().any(|s| s == name) {
        return Err(format!("样式变量循环：{} → {name}", seen.join(" → ")));
    }
    if seen.len() >= 128 {
        return Err("样式变量嵌套超过 128 层".into());
    }
    let fallback = args[1..].join(",");
    let resolved = vars.get(name).map(String::as_str);
    seen.push(name.into());
    let middle = match resolved {
        Some(resolved) => substitute(resolved, vars, seen),
        None => Err(format!("未定义样式变量 {name}")),
    };
    seen.pop();
    let middle = match middle {
        Ok(value) => value,
        Err(_) if args.len() > 1 => substitute(&fallback, vars, seen)?,
        Err(e) => return Err(e),
    };
    Ok(format!(
        "{}{}{}",
        &value[..start],
        middle,
        substitute(&value[end + 1..], vars, seen)?
    ))
}
fn scalar_value(key: &str, s: &str, file: &str, l: Loc) -> Result<V> {
    let fail = |m: &str| Diagnostic::new("E_LCSS", m, file, l);
    if matches!(
        key,
        "shaft_width" | "head_size" | "start_head_size" | "end_head_size"
    ) && (s.starts_with('[') || s.starts_with('('))
    {
        fn literal(expr: crate::parser::Expr) -> Option<V> {
            match expr.kind {
                crate::parser::ExprKind::Number(n, u) => Some(V::Number(n, u)),
                crate::parser::ExprKind::List(v) => Some(V::List(
                    v.into_iter().map(literal).collect::<Option<Vec<_>>>()?,
                )),
                _ => None,
            }
        }
        return literal(crate::parser::expression(s, file).map_err(|_| fail("无效箭头尺寸列表"))?)
            .ok_or_else(|| fail("箭头尺寸列表只接受数字及长度"));
    }
    if key == "math_text_fallback" {
        return match s.trim() {
            "true" => Ok(V::Bool(true)),
            "false" => Ok(V::Bool(false)),
            _ => Err(fail("math_text_fallback 需要 true 或 false")),
        };
    }
    static NUMERIC: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^([+-]?(?:\d*\.)?\d+)(mm|cm|in|inch|pt|px)?$").unwrap()
    });
    let numeric = &*NUMERIC;
    let mut values = vec![];
    for token in s.split_whitespace() {
        if let Some(capture) = numeric.captures(token) {
            let n: f64 = capture[1].parse().map_err(|_| fail("无效数值"))?;
            let unit = capture.get(2).map_or("", |s| s.as_str());
            if unit.is_empty()
                && n != 0.
                && !matches!(
                    key,
                    "opacity"
                        | "font_weight"
                        | "border_miter_limit"
                        | "line_miter_limit"
                        | "border_opacity"
                        | "line_opacity"
                )
            {
                return Err(fail(&format!("{key} 的长度需要单位")));
            }
            values.push(V::Number(n, unit.into()));
        } else {
            values.push(V::text(token.trim_matches(['\'', '"'])));
        }
    }
    if values.is_empty() {
        Err(fail("空样式值"))
    } else if values.len() == 1 {
        Ok(values.remove(0))
    } else {
        Ok(V::List(values))
    }
}

#[derive(Default)]
pub(crate) struct StyleCache {
    values: BTreeMap<String, (Args, f64, Args)>,
    dependencies: BTreeMap<String, String>,
}
impl StyleCache {
    pub fn clear(&mut self) {
        self.values.clear();
        self.dependencies.clear();
    }
}

/// A selector query also observes rules that become applicable after an edit.
/// Matching declaration order is retained; unrelated rules are not inputs.
#[derive(Clone)]
pub(crate) struct StyleRead {
    pub key: String,
    kind: String,
    original: Args,
    parent: Args,
    part: Option<String>,
    part_name: Option<String>,
    file: String,
    fingerprint: String,
    explicit: std::collections::BTreeSet<String>,
    layout: serde_json::Value,
}
pub(crate) fn appearance_property(key: &str) -> bool {
    matches!(
        key,
        "fill"
            | "background"
            | "color"
            | "border_color"
            | "line_color"
            | "opacity"
            | "border_opacity"
            | "line_opacity"
    )
}
impl StyleRead {
    fn layout_signature(&self, output: &Args) -> serde_json::Value {
        args_json(
            &output
                .iter()
                .filter(|(key, _)| {
                    !key.starts_with("__")
                        && !self.explicit.contains(*key)
                        && !appearance_property(key)
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }
    pub fn paint_only_change(&self, engine: &Engine) -> bool {
        crate::session::queries::without_recording(|| {
            engine.styled_part(&self.kind, &self.original, &self.parent, self.part.as_deref(), self.part_name.as_deref())
                .is_ok_and(|next| !next.values().any(|v|matches!(v,V::Map(a) if a.get("kind").is_some_and(|k|k.as_str()=="imagePaint"))) && self.layout_signature(&next) == self.layout)
        })
    }
    pub fn current(&self, engine: &Engine) -> std::rc::Rc<Self> {
        let mut read = self.clone();
        read.fingerprint = self.fingerprint(engine);
        std::rc::Rc::new(read)
    }
    fn fingerprint(&self, engine: &Engine) -> String {
        if let Some(value) = engine.style_cache.borrow().dependencies.get(&self.key) {
            return value.clone();
        }
        let declarations = engine.matching_style_declarations(
            &self.kind,
            &self.original,
            &self.parent,
            self.part.as_deref(),
            self.part_name.as_deref(),
            &self.file,
        );
        let fingerprint = format!(
            "{:?}",
            declarations
                .iter()
                .map(|(important, priority, specificity, _, _, d)| (
                    *important,
                    *priority,
                    *specificity,
                    &d.property,
                    &d.value,
                    &d.file
                ))
                .collect::<Vec<_>>()
        );
        let mut cache = engine.style_cache.borrow_mut();
        {
            cache
                .dependencies
                .insert(self.key.clone(), fingerprint.clone());
        }
        fingerprint
    }
    pub fn matches(&self, engine: &Engine) -> bool {
        self.fingerprint == self.fingerprint(engine)
    }
    pub fn bytes(&self) -> usize {
        self.key.len()
            + self.fingerprint.len()
            + args_json(&self.original).to_string().len()
            + args_json(&self.parent).to_string().len()
            + self.layout.to_string().len()
            + self
                .explicit
                .iter()
                .map(|key| key.len() + 64)
                .sum::<usize>()
            + 512
    }
}

impl Engine {
    pub fn stylesheet(&mut self, css: &str, file: &str, origin: Loc) -> Result<()> {
        self.stylesheet_scoped(css, file, origin, None)
    }
    pub fn stylesheet_scoped(
        &mut self,
        source: &str,
        file: &str,
        origin: Loc,
        scope: Option<&str>,
    ) -> Result<()> {
        self.stylesheet_stack(source, file, origin, scope, vec![])
    }
    fn stylesheet_stack(
        &mut self,
        source: &str,
        file: &str,
        origin: Loc,
        scope: Option<&str>,
        mut stack: Vec<String>,
    ) -> Result<()> {
        stack.push(file.into());
        let (parsed, motion) = parse_sheet(source, file, origin, scope)?;
        for d in &motion.warnings {
            self.css_warning(d.clone());
        }
        for rule in &parsed {
            if css_values::media(&rule.media, self.css_context(&Args::new()), false, false)
                .is_none()
            {
                if let Some(d) = rule.declarations.first() {
                    self.css_warning(Diagnostic::new(
                        "W_CSS",
                        format!("Ignored unsupported media condition {}", rule.media),
                        file,
                        d.loc,
                    ));
                }
            }
        }
        for (import, loc) in &motion.imports {
            let path = resolve(file, import);
            if stack.contains(&path) {
                return Err(Diagnostic::new("E_LCSS", "样式导入循环", file, *loc));
            }
            let result = self.host.read(&path, file, *loc).and_then(|data| {
                self.stylesheet_stack(
                    &String::from_utf8_lossy(&data),
                    &path,
                    Loc {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    scope,
                    stack.clone(),
                )
            });
            result?;
        }
        self.animation_sheets.push(motion);
        self.styles.extend(parsed);
        self.style_cache.borrow_mut().clear();
        Ok(())
    }
    fn css_value(&self, key: &str, s: &str, file: &str, l: Loc) -> Result<V> {
        let fail = |m: &str| Diagnostic::new("E_LCSS", m, file, l);
        let s = s.trim();
        if key == "math_font" {
            let name = s.trim_matches(['\'', '"']);
            if name.is_empty() {
                return Err(fail("math_font 需要非空字体名称或路径"));
            }
            return Ok(V::text(
                if name.contains('/')
                    || [".ttf", ".otf", ".ttc", ".otc"]
                        .iter()
                        .any(|ext| name.contains(ext))
                {
                    resolve(file, name)
                } else {
                    name.into()
                },
            ));
        }
        if matches!(key, "start_head" | "end_head") {
            return crate::endpoints::css_head(s, file, l);
        }
        if matches!(key, "effects" | "path" | "warp" | "extrude") {
            return crate::art::css_config(s, file, l, &self.unit, self.dpi);
        }
        if key == "font_family" {
            return Ok(V::List(
                split(s, ',')
                    .iter()
                    .map(|s| {
                        let s = s.trim_matches(['\'', '"']);
                        V::text(
                            if s.contains('/')
                                || s.contains(".ttf")
                                || s.contains(".otf")
                                || s.contains(".ttc")
                                || s.contains(".otc")
                            {
                                resolve(file, s)
                            } else {
                                s.into()
                            },
                        )
                    })
                    .collect(),
            ));
        }
        if key == "font_weight" && matches!(s, "normal" | "bold") {
            return Ok(V::num(if s == "bold" { 700. } else { 400. }));
        }
        if s.starts_with("hatch(") {
            let args = split(&s[6..s.len() - 1], ',');
            let get = |i, d: &str| args.get(i).cloned().unwrap_or(d.into());
            if !matches!(get(0, "slash").as_str(), "slash" | "cross" | "dots") {
                return Err(fail("纹理须为 slash/cross/dots"));
            }
            let spacing = self.css_value("spacing", &get(2, "2mm"), file, l)?;
            let width = self.css_value("line_width", &get(3, "0.5pt"), file, l)?;
            let spacing =
                value_length(&spacing, "mm", self.dpi).ok_or_else(|| fail("纹理长度需要单位"))?;
            let width =
                value_length(&width, "mm", self.dpi).ok_or_else(|| fail("纹理长度需要单位"))?;
            if spacing <= 0. || width <= 0. {
                return Err(fail("纹理间距和线宽必须为正"));
            }
            return Ok(V::from_json(
                &json!({"kind":"pattern","pattern":get(0,"slash"),"color":css_color(&get(1,"#666666")),"background":css_color(&get(4,"none")),"spacing":spacing,"lineWidth":width,"angle":0.}),
            ));
        }
        if s.starts_with("url(") || s.starts_with("image(") {
            let args = split(&s[s.find('(').unwrap() + 1..s.len() - 1], ',');
            let src = args[0].trim_matches(['\'', '"']);
            if src.contains(':') || src.starts_with("//") {
                return Err(fail("图片填充只接受本地资源"));
            }
            let fit = args.get(1).map(String::as_str).unwrap_or("cover");
            if !matches!(fit, "contain" | "cover" | "stretch") {
                return Err(fail("图片填充适配为 contain/cover/stretch"));
            }
            let path = resolve(file, src);
            let data = self.host.read(&path, file, l)?;
            let asset = crate::assets::load(&data, &path, file, l)?;
            return Ok(V::from_json(
                &json!({"kind":"imagePaint","rasterKey":asset["rasterKey"],"sourceJpegHash":asset["sourceJpegHash"],"data":asset["data"],"mime":asset["mime"],"width":asset["width"],"height":asset["height"],"fit":args.get(1).map(String::as_str).unwrap_or("cover")}),
            ));
        }
        if key.ends_with("color") || matches!(key, "fill" | "background") {
            let v = V::text(css_color(s));
            self.validate_color(&v, l, "E_LCSS").map_err(|mut e| {
                e.file = file.into();
                e
            })?;
            return Ok(v);
        }
        scalar_value(key, s, file, l)
    }

    fn matching_style_declarations<'a>(
        &'a self,
        kind: &str,
        original: &Args,
        parent: &Args,
        part: Option<&str>,
        part_name: Option<&str>,
        file: &str,
    ) -> Vec<(bool, bool, usize, usize, usize, &'a StyleDeclaration)> {
        let ctx = self.css_context(parent);
        let mut declarations = vec![];
        for (order, rule) in self.styles.iter().enumerate() {
            if !crate::css_selectors::scope_applies(
                rule.scope.as_deref(),
                &file,
                self.css_entry_file.as_deref(),
            ) {
                continue;
            }
            if css_values::media(&rule.media, ctx, false, false) != Some(true)
                || !self.css_supports(&rule.supports, ctx)
            {
                continue;
            }
            let (plain, selector_part) = rule
                .selector
                .split_once("::")
                .map(|(a, b)| (a, Some(b)))
                .unwrap_or((&rule.selector, None));
            let part_matches = match (selector_part, part) {
                (None, None) => true,
                (Some(s), Some(p)) => {
                    let (name, named) = s
                        .split_once('(')
                        .map(|(a, b)| (a, Some(b.trim_end_matches(')'))))
                        .unwrap_or((s, None));
                    name == p && named.is_none_or(|n| Some(n) == part_name)
                }
                _ => false,
            };
            if part_matches && matches(plain, kind, original, parent) {
                for (index, d) in rule.declarations.iter().enumerate() {
                    declarations.push((
                        d.important,
                        crate::css_selectors::theme_priority(
                            rule.scope.as_deref(),
                            self.css_entry_file.as_deref().or(Some(&self.file)),
                        ),
                        rule.specificity,
                        order,
                        index,
                        d,
                    ));
                }
            }
        }
        declarations
    }
    pub fn styled(&self, kind: &str, original: &Args, parent: &Args) -> Result<Args> {
        self.styled_part(kind, original, parent, None, None)
    }
    pub fn styled_part(
        &self,
        kind: &str,
        original: &Args,
        parent: &Args,
        part: Option<&str>,
        part_name: Option<&str>,
    ) -> Result<Args> {
        let _profile = crate::session::profile("style");
        let file = original
            .get("__style_file")
            .map(V::as_str)
            .unwrap_or_else(|| self.file.clone());
        // Style matching reads class ancestry, inherited/computed properties,
        // font size and color. Geometry arguments are merged after the cascade
        // and cannot change its result. Cache this derivation once per context.
        let selected: Args = ["class", "font_size", "color", "__style_file"]
            .into_iter()
            .filter_map(|k| original.get(k).map(|v| (k.into(), v.clone())))
            .collect();
        let key = {
            crate::session::queries::signature(&V::Map(parent.clone().into()))
                .zip(crate::session::queries::signature(&V::Map(selected.into())))
                .map(|(parent, selected)| {
                    json!([
                        self.styles.len(),
                        kind,
                        part,
                        part_name,
                        file,
                        self.css_entry_file,
                        self.css_viewport,
                        self.unit,
                        self.dpi,
                        parent,
                        selected
                    ])
                    .to_string()
                })
        };
        let observe = |output: &Args| {
            if crate::session::queries::tracking_styles() {
                if let Some(key) = &key {
                    let mut read = StyleRead {
                        key: key.clone(),
                        kind: kind.into(),
                        original: original
                            .iter()
                            .filter(|(k, _)| {
                                matches!(
                                    k.as_str(),
                                    "class" | "font_size" | "color" | "__style_file"
                                )
                            })
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                        parent: parent.clone(),
                        part: part.map(str::to_owned),
                        part_name: part_name.map(str::to_owned),
                        file: file.clone(),
                        fingerprint: String::new(),
                        explicit: original.keys().cloned().collect(),
                        layout: serde_json::Value::Null,
                    };
                    read.fingerprint = read.fingerprint(self);
                    read.layout = read.layout_signature(output);
                    crate::session::queries::style(std::rc::Rc::new(read));
                } else {
                    crate::session::queries::effect();
                }
            }
        };
        if let Some(key) = &key {
            let cached = self.style_cache.borrow().values.get(key).cloned();
            if let Some((base, root_font, vars)) = cached {
                let mut output = base;
                output.extend(original.clone());
                output.insert("__css_root_font".into(), V::mm(root_font));
                output.insert("__style_variables".into(), V::Map(vars.into()));
                output.insert("__style_type".into(), V::text(kind));
                output.insert("__style_file".into(), V::text(&file));
                output.insert("__style_parent".into(), V::Map(parent.clone().into()));
                observe(&output);
                return Ok(output);
            }
        }
        let mut output: Args = parent
            .iter()
            .filter(|(k, _)| INHERITED.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut vars: BTreeMap<String, String> = match parent.get("__style_variables") {
            Some(V::Map(vars)) => vars.iter().map(|(k, v)| (k.clone(), v.as_str())).collect(),
            _ => BTreeMap::new(),
        };
        let mut ctx = self.css_context(parent);
        let mut declarations =
            self.matching_style_declarations(kind, original, parent, part, part_name, &file);
        declarations.sort_by_key(|(imp, global, spec, order, index, _)| {
            (*imp, *global, *spec, *order, *index)
        });
        // Custom properties cascade before ordinary declarations and keep empty values.
        for (_, _, _, _, _, d) in &declarations {
            if d.property.starts_with("--") {
                vars.insert(d.property.clone(), d.value.clone());
            }
        }
        // Resolve inherited variables now. A child changing --a must not retroactively
        // change an inherited --b:var(--a) that was computed on its parent.
        let raw_vars = vars.clone();
        for (name, value) in &raw_vars {
            match substitute(value, &raw_vars, &mut vec![name.clone()]) {
                Ok(value) => {
                    vars.insert(name.clone(), value);
                }
                Err(_) => {
                    vars.remove(name);
                }
            }
        }
        let parent_ctx = ctx;
        let current = string(&output, "color", "#000000");
        let mut resolved = vec![];
        for (_, _, _, _, _, d) in declarations {
            if d.property.starts_with("--") {
                continue;
            }
            let value = match substitute(&d.value, &vars, &mut vec![]) {
                Ok(v) => v,
                Err(m) => {
                    let message = substitute(&d.value, &raw_vars, &mut vec![])
                        .err()
                        .unwrap_or(m);
                    self.css_warning(Diagnostic::new("W_CSS", message, &d.file, d.loc));
                    continue;
                }
            };
            let mut d = d.clone();
            d.value = value;
            resolved.push(d);
        }
        // Compute the font before expanding em-valued shorthands (notably shadows).
        for d in &resolved {
            if !matches!(d.property.as_str(), "font" | "font_size") {
                continue;
            }
            if let Ok(parts) = self.css_expansion(d, parent_ctx, &current) {
                for (key, value) in parts {
                    if key == "font_size" {
                        if value == "inherit" || value == "unset" {
                            continue;
                        }
                        if let Ok(v) = self
                            .css_computed_value(&key, &value, &d.file, d.loc, parent_ctx, &current)
                        {
                            ctx.font = value_length(&v, "pt", self.dpi).unwrap_or(ctx.font);
                        }
                    }
                }
            }
        }
        if let Some(v) = original.get("font_size") {
            ctx.font = value_length(v, "pt", self.dpi).unwrap_or(ctx.font);
        }
        if kind == "canvas" {
            ctx.root_font = ctx.font;
        }
        let mut expanded = BTreeMap::new();
        for d in resolved {
            match self.css_expansion(
                &d,
                if d.property == "font_size" || d.property == "font" {
                    parent_ctx
                } else {
                    ctx
                },
                &current,
            ) {
                Ok(values) => {
                    for (property, value) in values {
                        let mut long = d.clone();
                        long.property = property.clone();
                        long.value = value;
                        expanded.insert(property, long);
                    }
                }
                Err(e) if matches!(e.code.as_str(), "E_YIELD" | "E_CANCELLED") => return Err(e),
                Err(e) => self.css_warning(e),
            }
        }
        for key in expanded.keys().cloned().collect::<Vec<_>>() {
            let value = expanded[&key].value.clone();
            if matches!(value.as_str(), "inherit" | "unset") {
                if value == "inherit" || INHERITED.contains(&key.as_str()) {
                    if let Some(v) = parent.get(&key) {
                        output.insert(key.clone(), v.clone());
                    }
                } else {
                    output.remove(&key);
                }
                expanded.remove(&key);
            }
        }
        // Font size resolves against the parent's font; all other em lengths use
        // this element's computed font. Root rem is 16 layout pixels until declared.
        if let Some(d) = expanded.get("font_size") {
            match self.css_computed_value(
                "font_size",
                &d.value,
                &d.file,
                d.loc,
                parent_ctx,
                &current,
            ) {
                Ok(v) => {
                    ctx.font = value_length(&v, "pt", self.dpi).unwrap_or(ctx.font);
                    output.insert("font_size".into(), v);
                }
                Err(e) if matches!(e.code.as_str(), "E_YIELD" | "E_CANCELLED") => return Err(e),
                Err(e) => self.css_warning(e),
            }
        }
        if let Some(v) = original.get("font_size") {
            ctx.font = value_length(v, "pt", self.dpi).unwrap_or(ctx.font);
        }
        if kind == "canvas" {
            ctx.root_font = ctx.font;
        }
        if let Some(d) = expanded.get("color") {
            if let Ok(v) = self.css_computed_value("color", &d.value, &d.file, d.loc, ctx, &current)
            {
                output.insert("color".into(), v);
            }
        }
        if let Some(v) = original.get("color") {
            output.insert("color".into(), v.clone());
        }
        let current = string(&output, "color", "#000000");
        for (key, d) in &expanded {
            if css_values::SHADOW_PARTS.contains(&key.as_str())
                || key == "font_size"
                || key == "color"
                || key == "__css_shadow_layers"
            {
                continue;
            }
            match self.css_computed_value(key, &d.value, &d.file, d.loc, ctx, &current) {
                Ok(v) => {
                    output.insert(key.clone(), v);
                }
                // Required transition textures must preserve E_ASSET so the
                // browser worker can fulfill the compiler's lazy resource request.
                Err(e)
                    if matches!(e.code.as_str(), "E_YIELD" | "E_CANCELLED")
                        || key.starts_with("_laymesh_transition_texture_") =>
                {
                    return Err(e);
                }
                Err(e) if matches!(e.code.as_str(), "E_YIELD" | "E_CANCELLED") => return Err(e),
                Err(e) => self.css_warning(e),
            }
        }
        match css_values::box_shadows(&expanded, ctx, &current) {
            Ok(v) if !v.is_empty() => {
                output.insert("__css_box_shadows".into(), V::from_json(&json!(v)));
            }
            Err(m) => {
                if let Some(d) = expanded.get("box_shadow_offset") {
                    self.css_warning(Diagnostic::new("W_CSS", m, &d.file, d.loc));
                }
            }
            _ => {}
        }
        if let Some(v) = output.remove("background_image") {
            if v.as_str() != "none" {
                output.insert("background".into(), v);
            }
        }
        if let Some(v) = output.remove("background_color") {
            output.entry("background".into()).or_insert(v);
        }
        if !matches!(
            kind,
            "image" | "text" | "formula" | "group" | "plot" | "legend" | "colorbar" | "canvas"
        ) {
            if let Some(v) = output.remove("background") {
                output.insert("fill".into(), v);
            }
        }
        if let Some(key) = &key {
            let variables: Args = vars.iter().map(|(k, v)| (k.clone(), V::text(v))).collect();
            {
                let mut cache = self.style_cache.borrow_mut();
                cache
                    .values
                    .insert(key.clone(), (output.clone(), ctx.root_font, variables));
            }
        }
        output.extend(original.clone());
        output.insert("__css_root_font".into(), V::mm(ctx.root_font));
        output.insert(
            "__style_variables".into(),
            V::Map(vars.into_iter().map(|(k, v)| (k, V::text(v))).collect()),
        );
        output.insert("__style_type".into(), V::text(kind));
        output.insert("__style_file".into(), V::text(&file));
        output.insert("__style_parent".into(), V::Map(parent.clone().into()));
        observe(&output);
        Ok(output)
    }
}

impl Engine {
    /// The cascade reads only these inherited values and class ancestry unless
    /// an explicit inherit/variable expression can select another property.
    pub(crate) fn style_parent_inputs(&self, args: &Args) -> Args {
        let arbitrary = self.styles.iter().any(|r| {
            r.declarations
                .iter()
                .any(|d| d.value.contains("inherit") || d.value.contains("var("))
        });
        args.iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "__call_origin" | "__arg_locations" | "__placement_origin"
                ) && (arbitrary
                    || INHERITED.contains(&k.as_str())
                    || matches!(
                        k.as_str(),
                        "class"
                            | "__style_file"
                            | "__style_type"
                            | "__css_root_font"
                            | "__style_parent"
                            | "__style_variables"
                    ))
            })
            .map(|(k, v)| {
                (
                    k.clone(),
                    if k == "__style_parent" {
                        if let V::Map(a) = v {
                            V::Map(self.style_parent_inputs(a).into())
                        } else {
                            v.clone()
                        }
                    } else {
                        v.clone()
                    },
                )
            })
            .collect()
    }
}
impl Engine {
    pub(crate) fn css_warning(&self, mut d: Diagnostic) {
        crate::session::queries::effect();
        d.code = "W_CSS".into();
        self.css_warnings
            .borrow_mut()
            .insert((d.file.clone(), d.loc.offset, d.message.clone()), d);
    }
    pub fn css_diagnostics(&self) -> Vec<Diagnostic> {
        let mut files: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
        for d in self.css_warnings.borrow().values() {
            files.entry(d.file.clone()).or_default().push(d.clone());
        }
        files
            .into_iter()
            .map(|(file, ds)| {
                let details = ds
                    .iter()
                    .take(6)
                    .map(|d| format!("{}:{} {}", d.loc.line, d.loc.column, d.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                Diagnostic::new(
                    "W_CSS",
                    format!(
                        "Ignored {} unsupported or invalid CSS declarations/rules: {details}{}",
                        ds.len(),
                        if ds.len() > 6 { "; …" } else { "" }
                    ),
                    &file,
                    ds[0].loc,
                )
            })
            .collect()
    }
    pub(crate) fn css_context(&self, parent: &Args) -> Context {
        let default = 16. * 25.4 / self.dpi;
        Context {
            dpi: self.dpi,
            font: parent
                .get("font_size")
                .and_then(|v| value_length(v, "pt", self.dpi))
                .unwrap_or(default),
            root_font: parent
                .get("__css_root_font")
                .and_then(|v| value_length(v, "mm", self.dpi))
                .unwrap_or(default),
            viewport: self.css_viewport,
            reference: self.css_viewport[0],
        }
    }
    pub(crate) fn css_supports(&self, query: &str, ctx: Context) -> bool {
        css_values::condition(query, &|s| {
            let (property, value) = s.split_once(':')?;
            let property = property.trim().replace('-', "_");
            if !property_supported(&property) {
                return Some(false);
            }
            let d = StyleDeclaration {
                property,
                value: value.trim().into(),
                file: "<supports>".into(),
                loc: Loc::default(),
                important: false,
            };
            Some(self.css_expansion(&d, ctx, "#000000").is_ok())
        }) == Some(true)
    }
    fn css_expansion(
        &self,
        d: &StyleDeclaration,
        ctx: Context,
        current: &str,
    ) -> Result<Vec<(String, String)>> {
        let key = &d.property;
        let s = d.value.trim();
        if crate::page_transition::supported(key) {
            let initial = match key.as_str() {
                "_laymesh_transition" => "none",
                "_laymesh_transition_duration" => "700ms",
                "_laymesh_transition_timing_function" => "ease-in-out",
                _ => s,
            };
            return Ok(vec![(
                key.clone(),
                if matches!(s, "initial" | "revert" | "revert-layer") {
                    initial
                } else {
                    s
                }
                .into(),
            )]);
        }
        let fail = |m: String| {
            Diagnostic::new(
                "W_CSS",
                format!("Ignored {}: {m}", key.replace('_', "-")),
                &d.file,
                d.loc,
            )
        };
        if matches!(s, "inherit" | "unset") {
            let keys: Vec<String> = match key.as_str() {
                "border" | "line" => ["width", "color", "style"]
                    .map(|p| format!("{key}_{p}"))
                    .to_vec(),
                "background" => vec!["background_image".into(), "background_color".into()],
                "font" => [
                    "font_size",
                    "font_family",
                    "font_style",
                    "font_weight",
                    "line_height",
                ]
                .map(str::to_string)
                .to_vec(),
                _ => vec![key.clone()],
            };
            return Ok(keys.into_iter().map(|k| (k, s.into())).collect());
        }
        if matches!(s, "initial" | "revert" | "revert-layer") {
            let value = match key.as_str() {
                "box_shadow" | "text_shadow" | "filter" | "background" | "background_image"
                | "fill" => "none",
                "color" => "black",
                "background_color" => "transparent",
                "font_family" => "system-ui",
                "font_weight" => "400",
                "font_size" => "16px",
                "font_style" => "normal",
                "line_height" => "normal",
                "opacity" | "border_opacity" | "line_opacity" => "1",
                "border" | "line" => "medium none currentColor",
                "box_shadow_offset" => "none",
                "box_shadow_color" => "currentColor",
                "box_shadow_position" => "outset",
                _ => "0",
            };
            let mut initial = d.clone();
            initial.value = value.into();
            return self.css_expansion(&initial, ctx, current);
        }
        if key == "box_shadow" {
            return css_values::shadow_expansion(s, ctx, "currentColor").map_err(fail);
        }
        if matches!(key.as_str(), "border" | "line") {
            let mut width = "medium".to_string();
            let mut ink = "currentColor".to_string();
            let mut style = "none".to_string();
            let (mut w, mut c, mut st) = (false, false, false);
            for word in values::words(s) {
                if !st
                    && matches!(
                        word.as_str(),
                        "solid"
                            | "none"
                            | "hidden"
                            | "dashed"
                            | "dotted"
                            | "dash_dot"
                            | "double"
                            | "triple"
                    )
                {
                    style = if word == "hidden" {
                        "none".into()
                    } else {
                        word
                    };
                    st = true;
                } else if !w
                    && (values::length_context(&word, ctx).is_ok()
                        || matches!(word.as_str(), "thin" | "medium" | "thick"))
                {
                    width = word;
                    w = true;
                } else if !c && css_values::color(&word, current).is_ok() {
                    ink = word;
                    c = true;
                } else {
                    return Err(fail("Invalid border shorthand".into()));
                }
            }
            return Ok(vec![
                (format!("{key}_width"), width),
                (format!("{key}_color"), ink),
                (format!("{key}_style"), style),
            ]);
        }
        if key == "background" {
            if s.contains("gradient(") {
                css_values::gradient(s, ctx, current).map_err(fail)?;
                return Ok(vec![
                    ("background_image".into(), s.into()),
                    ("background_color".into(), "transparent".into()),
                ]);
            }
            if !s.starts_with("hatch(") && !s.starts_with("url(") && !s.starts_with("image(") {
                css_values::color(s, current).map_err(fail)?;
                return Ok(vec![
                    ("background_color".into(), s.into()),
                    ("background_image".into(), "none".into()),
                ]);
            }
        }
        if key == "font" {
            let words = values::words(s);
            let mut size = None;
            let mut style = "normal";
            let mut weight = "normal";
            for (i, word) in words.iter().enumerate() {
                let token = word.split('/').next().unwrap();
                if values::length_context(token, ctx).is_ok() {
                    size = Some(i);
                    break;
                }
                match word.as_str() {
                    "italic" | "oblique" => style = "italic",
                    "bold" => weight = "bold",
                    "normal" => {}
                    _ => {
                        if values::number(word).is_ok() {
                            weight = word;
                        } else {
                            return Err(fail("Unsupported font shorthand".into()));
                        }
                    }
                }
            }
            let i = size.ok_or_else(|| fail("Font shorthand needs size and family".into()))?;
            if i + 1 >= words.len() {
                return Err(fail("Font shorthand needs family".into()));
            }
            let parts: Vec<_> = words[i].split('/').collect();
            let mut out = vec![
                ("font_size".into(), parts[0].into()),
                ("font_style".into(), style.into()),
                ("font_weight".into(), weight.into()),
                ("font_family".into(), words[i + 1..].join(" ")),
            ];
            if let Some(line) = parts.get(1) {
                out.push(("line_height".into(), line.to_string()));
            } else {
                out.push(("line_height".into(), "normal".into()));
            }
            return Ok(out);
        }
        if css_values::SHADOW_PARTS.contains(&key.as_str()) {
            for token in split(s, ',') {
                let words = values::words(&token);
                match key.as_str() {
                    "box_shadow_offset" if token == "none" => {}
                    "box_shadow_offset" if matches!(words.len(), 1 | 2) => {
                        for word in words {
                            if word.contains('%') {
                                return Err(fail("Offset requires lengths".into()));
                            }
                            values::length_context(&word, ctx).map_err(fail)?;
                        }
                    }
                    "box_shadow_color" => {
                        css_values::color(&token, current).map_err(fail)?;
                    }
                    "box_shadow_position" if matches!(token.as_str(), "outset" | "inset") => {}
                    "box_shadow_blur" | "box_shadow_spread" => {
                        if token.contains('%') {
                            return Err(fail("Shadow requires lengths".into()));
                        }
                        let n = values::length_context(&token, ctx).map_err(fail)?;
                        if key == "box_shadow_blur" && n < 0. {
                            return Err(fail("Blur cannot be negative".into()));
                        }
                    }
                    _ => return Err(fail("Invalid shadow value list".into())),
                }
            }
        } else if !s.contains("var(") {
            self.css_computed_value(key, s, &d.file, d.loc, ctx, current)?;
        }
        Ok(vec![(key.clone(), s.into())])
    }
    fn css_computed_value(
        &self,
        key: &str,
        s: &str,
        file: &str,
        loc: Loc,
        ctx: Context,
        current: &str,
    ) -> Result<V> {
        if self.page_transition_override.is_some() && crate::page_transition::supported(key) {
            return Ok(V::Null);
        }
        if key.starts_with("_laymesh_transition_texture_") {
            let path = s
                .trim()
                .strip_prefix("url(")
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.trim().trim_matches(['\'', '"']))
                .filter(|s| !s.is_empty() && !s.contains(":") && !s.starts_with("//"))
                .ok_or_else(|| {
                    Diagnostic::new(
                        "E_TRANSITION",
                        "Transition texture requires a local image url()",
                        file,
                        loc,
                    )
                })?;
            let path = resolve(file, path);
            let bytes = self.host.read(&path, file, loc)?;
            let asset = crate::assets::load(&bytes, &path, file, loc)?;
            return Ok(V::from_json(&asset));
        }
        if crate::page_transition::supported(key) {
            return Ok(V::text(s));
        }
        let fail = |m: String| {
            Diagnostic::new(
                "W_CSS",
                format!("Ignored {}: {m}", key.replace('_', "-")),
                file,
                loc,
            )
        };
        if key == "text_shadow" {
            return css_values::shadow(s, ctx, current, "text")
                .map(|v| V::from_json(&json!(v)))
                .map_err(fail);
        }
        if key == "filter" {
            if s == "none" {
                return Ok(V::List(vec![]));
            }
            let body = s
                .strip_prefix("drop-shadow(")
                .and_then(|s| s.strip_suffix(')'))
                .ok_or_else(|| fail("Only a single drop-shadow() is supported".into()))?;
            let shadows = css_values::shadow(body, ctx, current, "drop").map_err(fail)?;
            if shadows.len() != 1 {
                return Err(fail("Only a single drop-shadow() is supported".into()));
            }
            return Ok(V::from_json(&json!(shadows)));
        }
        if key == "background_image" {
            return if s == "none" {
                Ok(V::text("none"))
            } else {
                css_values::gradient(s, ctx, current)
                    .map(|v| V::from_json(&v))
                    .map_err(fail)
            };
        }
        if key.ends_with("color") || matches!(key, "fill" | "text_fill" | "background") {
            if s.contains("gradient(") {
                return css_values::gradient(s, ctx, current)
                    .map(|v| V::from_json(&v))
                    .map_err(fail);
            }
            if !s.starts_with("hatch(") && !s.starts_with("url(") && !s.starts_with("image(") {
                return css_values::color(s, current).map(V::text).map_err(fail);
            }
        }
        if key == "opacity" || matches!(key, "border_opacity" | "line_opacity") {
            return values::number_percentage(s, 1.)
                .map(|n| V::num(n.clamp(0., 1.)))
                .map_err(fail);
        }
        if key == "font_weight" {
            return match s {
                "normal" => Ok(V::num(400.)),
                "bold" | "bolder" => Ok(V::num(700.)),
                "lighter" => Ok(V::num(300.)),
                _ => values::number(s)
                    .and_then(|n| {
                        if (1.0..=1000.).contains(&n) {
                            Ok(V::num(n))
                        } else {
                            Err("Font weight must be 1–1000".into())
                        }
                    })
                    .map_err(fail),
            };
        }
        if key == "font_style" && !matches!(s, "normal" | "italic" | "oblique") {
            return Err(fail("Font style must be normal/italic/oblique".into()));
        }
        if key == "line_height" && s == "normal" {
            return Ok(V::mm(ctx.font * 1.2));
        }
        if key == "line_height" {
            if let Ok(n) = values::number(s) {
                if n < 0. {
                    return Err(fail("Line height cannot be negative".into()));
                }
                return Ok(V::mm(ctx.font * n));
            }
        }
        let lengths = key.ends_with("_font_size")
            || key == "grid_line_width"
            || matches!(
                key,
                "font_size"
                    | "line_height"
                    | "padding"
                    | "border_radius"
                    | "border_width"
                    | "line_width"
                    | "text_stroke_width"
                    | "line_dash_offset"
                    | "border_dash_offset"
                    | "spacing"
            )
            || key.ends_with("_dash");
        if lengths {
            let mut items = vec![];
            let words = values::words(s);
            if words.is_empty()
                || (!matches!(key, "padding" | "border_radius")
                    && !key.ends_with("_dash")
                    && words.len() != 1)
                || (key == "padding" && words.len() > 4)
            {
                return Err(fail("Invalid length list".into()));
            }
            // The current shape pipeline has one uniform radius. Different corner
            // radii and elliptical radii are ignored as a whole declaration.
            if key == "border_radius" && words.len() != 1 {
                return Err(fail(
                    "Only uniform border-radius is currently supported".into(),
                ));
            }
            if key == "border_radius" && words[0].ends_with('%') {
                let fraction = values::length_context(
                    &words[0],
                    Context {
                        reference: 1.,
                        ..ctx
                    },
                )
                .map_err(fail)?;
                if fraction < 0. {
                    return Err(fail("Radius cannot be negative".into()));
                }
                return Ok(V::Map(
                    [("__css_fraction".into(), V::num(fraction))]
                        .into_iter()
                        .collect(),
                ));
            }
            for word in words {
                let s = match word.as_str() {
                    "thin" if key.ends_with("_width") => "1px",
                    "medium" if key.ends_with("_width") => "3px",
                    "thick" if key.ends_with("_width") => "5px",
                    _ => &word,
                };
                let ref_length = if matches!(key, "font_size" | "line_height") {
                    ctx.font
                } else {
                    ctx.reference
                };
                let n = values::length_context(
                    s,
                    Context {
                        reference: ref_length,
                        ..ctx
                    },
                )
                .map_err(fail)?;
                if n < 0. && !key.ends_with("_dash_offset") {
                    return Err(fail("Length cannot be negative".into()));
                }
                // Preserve absolute source units; computed expressions become mm.
                static LITERAL: std::sync::LazyLock<regex::Regex> =
                    std::sync::LazyLock::new(|| {
                        regex::Regex::new(r"^[+-]?(?:\d*\.)?\d+(mm|cm|in|inch|pt|px)?$").unwrap()
                    });
                let literal = &*LITERAL;
                items.push(if literal.is_match(s) {
                    scalar_value(key, s, file, loc)?
                } else {
                    V::mm(n)
                });
            }
            return Ok(if items.len() == 1 {
                items.remove(0)
            } else {
                V::List(items)
            });
        }
        if let Some(meta) = API["cssProperties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == key.replace('_', "-"))
        {
            if meta["type"] == "enum" && !matches!(key, "font_style" | "start_cap" | "end_cap") {
                if !meta["values"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|v| v == s))
                {
                    return Err(fail("Unsupported keyword".into()));
                }
            }
            if matches!(key, "line_miter_limit" | "border_miter_limit") {
                return values::number(s)
                    .and_then(|n| {
                        if n >= 1. {
                            Ok(V::num(n))
                        } else {
                            Err("Miter limit must be at least one".into())
                        }
                    })
                    .map_err(fail);
            }
        }
        self.css_value(
            key,
            if key == "font_style" && s == "oblique" {
                "italic"
            } else {
                s
            },
            file,
            loc,
        )
    }
}
