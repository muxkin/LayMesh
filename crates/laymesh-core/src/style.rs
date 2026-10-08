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
fn compound_match(selector: &str, kind: &str, args: &Args) -> bool {
    let parts: Vec<_> = selector.split('.').collect();
    let typ = parts[0];
    (typ.is_empty() || typ == "*" || typ == kind)
        && parts[1..].iter().all(|c| {
            string(args, "class", "")
                .split_whitespace()
                .any(|v| v == *c)
        })
}
fn matches(selector: &str, kind: &str, args: &Args, parent: &Args) -> bool {
    let normalized = selector.replace('>', " > ");
    let mut items: Vec<_> = normalized.split_whitespace().collect();
    let Some(last) = items.pop() else {
        return false;
    };
    if !compound_match(last, kind, args) {
        return false;
    }
    let mut current = Some(parent.clone());
    while let Some(item) = items.pop() {
        let direct = item == ">";
        let item = if direct {
            let Some(s) = items.pop() else { return false };
            s
        } else {
            item
        };
        let mut found = false;
        while let Some(p) = current.take() {
            let typ = string(&p, "__style_type", "");
            let ok = compound_match(item, &typ, &p);
            current = match p.get("__style_parent") {
                Some(V::Map(v)) => Some(v.clone()),
                _ => None,
            };
            if ok {
                found = true;
                break;
            }
            if direct {
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug)]
pub struct StyleDeclaration {
    property: String,
    value: String,
    file: String,
    loc: Loc,
}
#[derive(Clone, Debug)]
pub struct StyleRule {
    selector: String,
    declarations: Vec<StyleDeclaration>,
    scope: Option<String>,
    specificity: usize,
}
const INHERITED: [&str; 7] = [
    "math_font",
    "font_family",
    "font_size",
    "font_weight",
    "font_style",
    "color",
    "line_height",
];
fn location(source: &str, origin: Loc, offset: usize) -> Loc {
    let prefix = &source[..offset];
    let lines = prefix.bytes().filter(|b| *b == b'\n').count();
    Loc {
        line: origin.line + lines,
        column: if lines == 0 {
            origin.column + prefix.encode_utf16().count()
        } else {
            prefix
                .rsplit('\n')
                .next()
                .unwrap_or("")
                .encode_utf16()
                .count()
                + 1
        },
        offset: origin.offset + offset,
    }
}
/// Find a delimiter outside quotes and function arguments. Preserving byte
/// offsets here makes diagnostics point to declarations in imported stylesheets.
fn delimiter(source: &str, from: usize, delimiters: &[char]) -> Option<(usize, char)> {
    let (mut depth, mut quote, mut escaped) = (0usize, '\0', false);
    for (offset, ch) in source[from..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if quote != '\0' {
            if ch == quote {
                quote = '\0';
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = ch;
            continue;
        }
        if depth == 0 && delimiters.contains(&ch) {
            return Some((from + offset, ch));
        }
        if matches!(ch, '(' | '[') {
            depth += 1;
        }
        if matches!(ch, ')' | ']') {
            depth = depth.saturating_sub(1);
        }
    }
    None
}
fn uncomment(source: &str, file: &str, origin: Loc) -> Result<String> {
    let mut bytes = source.as_bytes().to_vec();
    let (mut at, mut quote) = (0, 0);
    while at < bytes.len() {
        if bytes[at] == b'\\' {
            at += 2;
            continue;
        }
        if quote != 0 {
            if bytes[at] == quote {
                quote = 0;
            }
            at += 1;
            continue;
        }
        if matches!(bytes[at], b'\'' | b'"') {
            quote = bytes[at];
            at += 1;
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            let start = at;
            at += 2;
            while at + 1 < bytes.len() && &bytes[at..at + 2] != b"*/" {
                at += 1;
            }
            if at + 1 == bytes.len() {
                return Err(Diagnostic::new(
                    "E_LCSS",
                    "未闭合样式注释",
                    file,
                    location(source, origin, start),
                ));
            }
            at += 2;
            for byte in &mut bytes[start..at] {
                if *byte != b'\n' && *byte != b'\r' {
                    *byte = b' ';
                }
            }
        } else {
            at += 1;
        }
    }
    Ok(String::from_utf8(bytes).unwrap())
}
fn substitute(
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
    let resolved = vars
        .get(name)
        .map(String::as_str)
        .or_else(|| (args.len() > 1).then_some(fallback.as_str()))
        .ok_or_else(|| format!("未定义样式变量 {name}"))?;
    seen.push(name.into());
    let middle = substitute(resolved, vars, seen)?;
    seen.pop();
    Ok(format!(
        "{}{}{}",
        &value[..start],
        middle,
        substitute(&value[end + 1..], vars, seen)?
    ))
}
/// Parse rules without importing files, evaluating DSL, or discovering fonts.
fn parse_stylesheet(
    source: &str,
    file: &str,
    origin: Loc,
    scope: Option<&str>,
) -> Result<(Vec<StyleRule>, Vec<(String, Loc)>)> {
    let css = uncomment(source, file, origin)?;
    let fail = |message: &str, offset| {
        Diagnostic::new("E_LCSS", message, file, location(&css, origin, offset))
    };
    let mut at = 0;
    let mut parsed = vec![];
    let mut imports = vec![];
    while at < css.len() {
        at += css[at..].len() - css[at..].trim_start().len();
        if at == css.len() {
            break;
        }
        let (open, token) = delimiter(&css, at, &['{', ';', '}'])
            .ok_or_else(|| fail("需要有效的 LCSS 规则", at))?;
        if css[at..].starts_with('@') {
            if token != ';' {
                return Err(fail("不支持的样式 at-rule", at));
            }
            let import = css[at..open]
                .trim()
                .strip_prefix("@import")
                .ok_or_else(|| fail("不支持的样式 at-rule", at))?
                .trim();
            let import = import
                .strip_prefix("url(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(import)
                .trim()
                .trim_matches(['\'', '"']);
            if !import.ends_with(".lcss")
                || import.contains(':')
                || import.starts_with("//")
                || import.is_empty()
            {
                return Err(fail("@import 需要本地 .lcss 路径", at));
            }
            imports.push((import.to_string(), location(&css, origin, at)));
            at = open + 1;
            continue;
        }
        if token != '{' {
            return Err(fail("需要有效的 LCSS 规则", at));
        }
        let (close, token) =
            delimiter(&css, open + 1, &['{', '}']).ok_or_else(|| fail("未闭合样式规则", open))?;
        if token != '}' {
            return Err(fail("样式规则仅接受属性声明", close));
        }
        let mut declarations = vec![];
        let mut pos = open + 1;
        while pos < close {
            pos += css[pos..close].len() - css[pos..close].trim_start().len();
            if pos == close {
                break;
            }
            let end = delimiter(&css[..close], pos, &[';']).map_or(close, |(at, _)| at);
            if pos == end {
                pos += 1;
                continue;
            }
            let (colon, _) =
                delimiter(&css[..end], pos, &[':']).ok_or_else(|| fail("需要属性声明", pos))?;
            let key = css[pos..colon].trim();
            let custom = key.starts_with("--");
            let key = if custom {
                key.into()
            } else {
                key.replace('-', "_")
            };
            if !custom
                && !matches!(key.as_str(), "line" | "border")
                && !API["styleProperties"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == &key)
            {
                return Err(fail(
                    &format!("不支持属性 {key}；尺寸和定位在 .lay 中设置"),
                    pos,
                ));
            }
            let value = css[colon + 1..end].trim();
            if value.is_empty() {
                return Err(fail("空样式值", pos));
            }
            if value.to_ascii_lowercase().contains("!important") {
                return Err(fail("LCSS 不支持 !important", pos));
            }
            declarations.push(StyleDeclaration {
                property: key,
                value: value.into(),
                file: file.into(),
                loc: location(&css, origin, pos),
            });
            pos = end + 1;
        }
        for selector in split(&css[at..open], ',') {
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
                {
                    return Err(fail(&format!("未知图形部件 {name}"), at));
                }
                if !regex::Regex::new(r"^[\w-]+(?:\([\w-]+\))?$")
                    .unwrap()
                    .is_match(part)
                {
                    return Err(fail("无效图形部件选择器", at));
                }
                plain
            } else {
                selector.as_str()
            };
            let normal = plain.replace('>', " > ");
            let tokens: Vec<_> = normal.split_whitespace().collect();
            if tokens.is_empty()
                || tokens.first() == Some(&">")
                || tokens.last() == Some(&">")
                || tokens.windows(2).any(|w| w == [">", ">"])
            {
                return Err(fail("无效所属关系选择器", at));
            }
            let compound = regex::Regex::new(r"^(?:[\w-]+|\*)?(?:\.[\w-]+)*$").unwrap();
            if tokens.iter().any(|s| *s != ">" && !compound.is_match(s)) {
                return Err(fail("不支持选择器；使用类型、类、所属关系或 ::部件", at));
            }
            let specificity = selector.matches('.').count() * 100
                + tokens
                    .iter()
                    .filter(|s| {
                        s.chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                    })
                    .count()
                + usize::from(selector.contains("::"));
            parsed.push(StyleRule {
                selector,
                declarations: declarations.clone(),
                scope: scope.map(str::to_owned),
                specificity,
            });
        }
        at = close + 1;
    }
    Ok((parsed, imports))
}

/// Static stylesheet validation used by editors. Imports are syntax checked but
/// never read; paint/variable evaluation remains a compilation responsibility.
pub fn validate_stylesheet(source: &str, file: &str, origin: Loc) -> Result<()> {
    let (rules, _) = parse_stylesheet(source, file, origin, None)?;
    for rule in rules {
        for declaration in rule.declarations {
            let key = &declaration.property;
            if matches!(key.as_str(), "effects" | "path" | "warp" | "extrude")
                && !declaration.value.contains("var(")
            {
                crate::art::css_config(
                    &declaration.value,
                    &declaration.file,
                    declaration.loc,
                    "mm",
                    96.,
                )?;
                continue;
            }
            if key.starts_with("--")
                || declaration.value.contains("var(")
                || matches!(
                    key.as_str(),
                    "fill"
                        | "text_fill"
                        | "effects"
                        | "path"
                        | "warp"
                        | "extrude"
                        | "background"
                        | "border"
                        | "line"
                        | "font_family"
                        | "math_font"
                )
                || key.ends_with("color")
            {
                continue;
            }
            scalar_value(key, &declaration.value, &declaration.file, declaration.loc)?;
        }
    }
    Ok(())
}

fn scalar_value(key: &str, s: &str, file: &str, l: Loc) -> Result<V> {
    let fail = |m: &str| Diagnostic::new("E_LCSS", m, file, l);
    let numeric = regex::Regex::new(r"^([+-]?(?:\d*\.)?\d+)(mm|cm|in|inch|pt|px)?$").unwrap();
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
        let (parsed, imports) = parse_stylesheet(source, file, origin, scope)?;
        for (import, loc) in imports {
            let path = resolve(file, &import);
            if stack.contains(&path) {
                return Err(Diagnostic::new("E_LCSS", "样式导入循环", file, loc));
            }
            let result = self.host.read(&path, file, loc).and_then(|data| {
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
        self.styles.extend(parsed);
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
        if s.starts_with("linear-gradient(") || s.starts_with("radial-gradient(") {
            let radial = s.starts_with("radial");
            let mut args = split(&s[s.find('(').unwrap() + 1..s.len() - 1], ',');
            let mut angle = 180.;
            if !radial
                && args
                    .first()
                    .is_some_and(|s| s.starts_with("to ") || s.ends_with("deg"))
            {
                let direction = args.remove(0);
                angle = match direction.as_str() {
                    "to top" => 0.,
                    "to right" => 90.,
                    "to bottom" => 180.,
                    "to left" => 270.,
                    _ => direction
                        .trim_end_matches("deg")
                        .parse()
                        .map_err(|_| fail("无效渐变角度"))?,
                };
            }
            if radial
                && args
                    .first()
                    .is_some_and(|s| s.starts_with("circle") || s.starts_with("ellipse"))
            {
                args.remove(0);
            }
            let mut stops = vec![];
            let mut previous = -1.;
            for (i, arg) in args.iter().enumerate() {
                let tokens: Vec<_> = arg.split_whitespace().collect();
                if tokens.is_empty()
                    || tokens.len() > 2
                    || (tokens.len() == 2 && !tokens[1].ends_with('%'))
                {
                    return Err(fail("渐变色标需要颜色及可选百分比"));
                }
                self.validate_color(&V::text(css_color(tokens[0])), l, "E_LCSS")
                    .map_err(|mut e| {
                        e.file = file.into();
                        e
                    })?;
                let at = if tokens.len() > 1 {
                    tokens[1]
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .map_err(|_| fail("无效渐变色标"))?
                        / 100.
                } else {
                    i as f64 / (args.len() - 1).max(1) as f64
                };
                if !(0.0..=1.0).contains(&at) || at < previous {
                    return Err(fail("渐变色标必须有序且在 0–100% 之间"));
                }
                previous = at;
                stops.push(json!({"at":at,"color":css_color(tokens[0]),"opacity":1.}));
            }
            if stops.len() < 2 {
                return Err(fail("渐变至少需要两个色标"));
            }
            let j = if radial {
                json!({"kind":"radialGradient","center":[0.5,0.5],"radius":0.5,"stops":stops})
            } else {
                let dx: f64 = (angle * std::f64::consts::PI / 180.).sin();
                let dy: f64 = -(angle * std::f64::consts::PI / 180.).cos();
                let scale = dx.abs().max(dy.abs());
                json!({"kind":"linearGradient","start":[0.5-dx/scale/2.,0.5-dy/scale/2.],"end":[0.5+dx/scale/2.,0.5+dy/scale/2.],"stops":stops})
            };
            return Ok(V::from_json(&j));
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
                value_length(&spacing, "mm", 96.).ok_or_else(|| fail("纹理长度需要单位"))?;
            let width = value_length(&width, "mm", 96.).ok_or_else(|| fail("纹理长度需要单位"))?;
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
        let file = original
            .get("__style_file")
            .map(V::as_str)
            .unwrap_or_else(|| self.file.clone());
        let mut output: Args = parent
            .iter()
            .filter(|(k, _)| INHERITED.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut vars: BTreeMap<String, String> = match parent.get("__style_variables") {
            Some(V::Map(vars)) => vars.iter().map(|(k, v)| (k.clone(), v.as_str())).collect(),
            _ => BTreeMap::new(),
        };
        let mut rules = vec![];
        for (order, rule) in self.styles.iter().enumerate() {
            if rule.scope.as_ref().is_some_and(|scope| scope != &file) {
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
                rules.push((rule.scope.is_none(), rule.specificity, order, rule));
            }
        }
        // Module styles provide local defaults; explicitly attached external
        // stylesheets override them, matching the original LCSS cascade.
        rules.sort_by_key(|(global, specificity, order, _)| (*global, *specificity, *order));
        let mut declarations: Vec<&StyleDeclaration> = vec![];
        for (_, _, _, rule) in rules {
            for declaration in &rule.declarations {
                if declaration.property.starts_with("--") {
                    vars.insert(declaration.property.clone(), declaration.value.clone());
                } else {
                    declarations.retain(|d| d.property != declaration.property);
                    declarations.push(declaration);
                }
            }
        }
        for d in declarations {
            let value = substitute(&d.value, &vars, &mut vec![])
                .map_err(|m| Diagnostic::new("E_LCSS", m, &d.file, d.loc))?;
            if matches!(d.property.as_str(), "border" | "line") {
                for token in value.split_whitespace() {
                    let suffix = if matches!(
                        token,
                        "solid" | "none" | "dashed" | "dotted" | "dash_dot" | "double" | "triple"
                    ) {
                        "style"
                    } else if token.starts_with('#') || css_color(token) != token {
                        "color"
                    } else {
                        "width"
                    };
                    let key = format!("{}_{}", d.property, suffix);
                    output.insert(key.clone(), self.css_value(&key, token, &d.file, d.loc)?);
                }
            } else {
                output.insert(
                    d.property.clone(),
                    self.css_value(&d.property, &value, &d.file, d.loc)?,
                );
            }
        }
        output.extend(original.clone());
        output.insert(
            "__style_variables".into(),
            V::Map(vars.into_iter().map(|(k, v)| (k, V::text(v))).collect()),
        );
        output.insert("__style_type".into(), V::text(kind));
        output.insert("__style_file".into(), V::text(file));
        output.insert("__style_parent".into(), V::Map(parent.clone()));
        Ok(output)
    }
}
