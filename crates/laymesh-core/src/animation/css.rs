//! CSS syntax is tokenized by cssparser. Drawing declarations retain their
//! original byte positions; animation rules are compiled separately from layout.
use crate::{Diagnostic, Loc, Result};
use cssparser::{Parser, Token};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub rules: Vec<Rule>,
    pub keyframes: Vec<Keyframes>,
    pub scope: Option<String>,
    #[serde(skip)]
    pub imports: Vec<(String, Loc)>,
    #[serde(skip)]
    pub warnings: Vec<Diagnostic>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub media: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub supports: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
    pub file: String,
    pub loc: Loc,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframes {
    pub name: String,
    pub frames: Vec<(f64, Vec<Declaration>)>,
    pub prefixed: bool,
    pub file: String,
    pub loc: Loc,
}
struct Block {
    head: String,
    body: Option<String>,
    start: usize,
    body_start: usize,
}
fn location(source: &str, origin: Loc, offset: usize) -> Loc {
    let prefix = &source[..offset];
    let n = prefix.matches('\n').count();
    Loc {
        line: origin.line + n,
        column: if n == 0 {
            origin.column + prefix.encode_utf16().count()
        } else {
            prefix.rsplit('\n').next().unwrap().encode_utf16().count() + 1
        },
        offset: origin.offset + offset,
    }
}
fn blocks(source: &str) -> std::result::Result<Vec<Block>, String> {
    let mut p = Parser::new(source);
    let mut out = vec![];
    while !p.is_exhausted() {
        p.skip_whitespace();
        let start = p.position();
        let mut ended = false;
        while !p.is_exhausted() {
            let before = p.position();
            let t = p
                .next_including_whitespace_and_comments()
                .map_err(|e| format!("CSS syntax: {e:?}"))?
                .clone();
            match t {
                Token::CurlyBracketBlock => {
                    let head = clean(p.slice(start..before));
                    let body_start = p.position().byte_index();
                    let body = p
                        .parse_nested_block(|q| {
                            let s = q.position();
                            while q.next_including_whitespace_and_comments().is_ok() {}
                            Ok::<_, cssparser::ParseError<()>>(q.slice_from(s).to_string())
                        })
                        .map_err(|e| format!("CSS block: {e:?}"))?;
                    out.push(Block {
                        head,
                        body: Some(body),
                        start: start.byte_index(),
                        body_start,
                    });
                    ended = true;
                    break;
                }
                Token::ParenthesisBlock | Token::SquareBracketBlock | Token::Function(_) => {
                    p.parse_nested_block(|q| {
                        while q.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<()>>(())
                    })
                    .map_err(|e| format!("CSS header: {e:?}"))?;
                }
                Token::Semicolon => {
                    out.push(Block {
                        head: clean(p.slice(start..before)),
                        body: None,
                        start: start.byte_index(),
                        body_start: 0,
                    });
                    ended = true;
                    break;
                }
                Token::BadString(_)
                | Token::BadUrl(_)
                | Token::CloseCurlyBracket
                | Token::CloseParenthesis
                | Token::CloseSquareBracket => {
                    ended = true;
                    break;
                }
                _ => {}
            }
        }
        if !ended && !p.slice_from(start).trim().is_empty() {
            out.push(Block {
                head: clean(p.slice_from(start)),
                body: None,
                start: start.byte_index(),
                body_start: 0,
            });
        }
    }
    Ok(out)
}
fn clean(s: &str) -> String {
    let mut bytes = s.as_bytes().to_vec();
    let (mut i, mut quote) = (0, 0);
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if quote != 0 {
            if bytes[i] == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if matches!(bytes[i], b'\'' | b'"') {
            quote = bytes[i];
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            for byte in &mut bytes[start..i] {
                if !matches!(*byte, b'\n' | b'\r') {
                    *byte = b' ';
                }
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(bytes).unwrap().trim().into()
}
pub(crate) fn declarations(
    body: &str,
    file: &str,
    origin: Loc,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<Declaration> {
    let mut p = Parser::new(body);
    let mut result = vec![];
    while !p.is_exhausted() {
        p.skip_whitespace();
        if p.try_parse(|p| p.expect_semicolon()).is_ok() {
            continue;
        }
        let start = p.position();
        let loc = location(body, origin, start.byte_index());
        let parsed = p.parse_until_before(cssparser::Delimiter::Semicolon, |p| {
            let key = p.expect_ident_cloned()?.to_string();
            p.expect_colon()?;
            let value_start = p.position();
            while let Ok(token) = p.next_including_whitespace_and_comments().cloned() {
                if matches!(
                    token,
                    Token::BadString(_)
                        | Token::BadUrl(_)
                        | Token::CloseCurlyBracket
                        | Token::CloseParenthesis
                        | Token::CloseSquareBracket
                ) {
                    return Err(custom_error("Invalid CSS value".to_string()));
                }
                if matches!(
                    token,
                    Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock
                ) {
                    p.parse_nested_block(|q| {
                        while q.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<String>>(())
                    })?;
                }
            }
            let value = clean(p.slice_from(value_start));
            static IMPORTANT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
                regex::Regex::new(r"(?i)\s*!\s*important\s*$").unwrap()
            });
            let important = IMPORTANT.is_match(&value);
            let value = IMPORTANT.replace(&value, "").trim().to_string();
            if value.is_empty() && !key.starts_with("--") {
                return Err(custom_error("Empty CSS value".to_string()));
            }
            let property = if key.starts_with("--") {
                key
            } else {
                key.to_ascii_lowercase()
            };
            Ok(Declaration {
                property: property
                    .strip_prefix("-webkit-")
                    .unwrap_or(&property)
                    .into(),
                value,
                important,
                file: file.into(),
                loc,
            })
        });
        match parsed {
            Ok(d) => result.push(d),
            Err(e) => warnings.push(Diagnostic::new(
                "W_CSS",
                format!("Ignored invalid declaration: {e:?}"),
                file,
                loc,
            )),
        }
        let _ = p.try_parse(|p| p.expect_semicolon());
    }
    result
}
pub fn is_motion_property(key: &str) -> bool {
    let k = key.strip_prefix("-webkit-").unwrap_or(key);
    k == "animation"
        || k.starts_with("animation-")
        || k.starts_with("transition-")
        || matches!(
            k,
            "transform"
                | "transform-origin"
                | "backface-visibility"
                | "visibility"
                | "translate"
                | "rotate"
                | "scale"
        )
        || k.starts_with("-laymesh-")
}
/// One token parser serves both static and animation styles. Unsupported CSS
/// is recorded and recovered at a declaration or rule boundary.
pub fn extract(
    source: &str,
    file: &str,
    origin: Loc,
    scope: Option<&str>,
) -> Result<(String, Sheet)> {
    let mut sheet = Sheet {
        scope: scope.map(str::to_owned),
        ..Sheet::default()
    };
    parse_rules(source, file, origin, "", "", &mut sheet);
    Ok((String::new(), sheet))
}
fn parse_rules(
    source: &str,
    file: &str,
    origin: Loc,
    media: &str,
    supports: &str,
    sheet: &mut Sheet,
) {
    let rules = match blocks(source) {
        Ok(v) => v,
        Err(m) => {
            sheet
                .warnings
                .push(Diagnostic::new("W_CSS", m, file, origin));
            return;
        }
    };
    for b in rules {
        let loc = location(source, origin, b.start);
        let Some(body) = &b.body else {
            if b.head.starts_with("@charset") {
                continue;
            }
            if let Some(import) = b.head.strip_prefix("@import") {
                let import = import.trim();
                let import = import
                    .strip_prefix("url(")
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(import)
                    .trim()
                    .trim_matches(['\'', '"']);
                if (import.ends_with(".css") || import.ends_with(".lcss"))
                    && !import.contains(':')
                    && !import.starts_with("//")
                {
                    sheet.imports.push((import.into(), loc));
                    continue;
                }
            }
            sheet.warnings.push(Diagnostic::new(
                "W_CSS",
                format!("Ignored CSS rule: {}", b.head),
                file,
                loc,
            ));
            continue;
        };
        let body_loc = location(source, origin, b.body_start);
        if b.head.starts_with("@keyframes ") || b.head.starts_with("@-webkit-keyframes ") {
            let name = b
                .head
                .split_once(char::is_whitespace)
                .unwrap()
                .1
                .trim()
                .trim_matches(['\'', '"'])
                .to_string();
            let parsed = (|| -> std::result::Result<Vec<(f64, Vec<Declaration>)>, String> {
                if name.is_empty() {
                    return Err("Missing animation name".into());
                }
                let mut frames = vec![];
                for frame in blocks(body)? {
                    let fb = frame.body.ok_or("Expected keyframe block")?;
                    let ds = declarations(
                        &fb,
                        file,
                        location(body, body_loc, frame.body_start),
                        &mut sheet.warnings,
                    );
                    for at in split(&frame.head, ',') {
                        let at = match at.trim() {
                            "from" => 0.,
                            "to" => 1.,
                            x => x
                                .strip_suffix('%')
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100.)
                                .ok_or("Keyframe requires from/to or percentage")?,
                        };
                        if !(0.0..=1.).contains(&at) {
                            return Err("Keyframe outside 0–100%".into());
                        }
                        frames.push((at, ds.iter().filter(|d| !d.important).cloned().collect()));
                    }
                }
                Ok(frames)
            })();
            match parsed {
                Ok(frames) => sheet.keyframes.push(Keyframes {
                    name,
                    frames,
                    prefixed: b.head.starts_with("@-"),
                    file: file.into(),
                    loc,
                }),
                Err(m) => sheet.warnings.push(Diagnostic::new("W_CSS", m, file, loc)),
            }
        } else if let Some(query) = b.head.strip_prefix("@media ") {
            // Nested conditions are retained, including each comma alternative.
            let query = if media.is_empty() {
                query.to_string()
            } else {
                split(media, ',')
                    .iter()
                    .flat_map(|a| {
                        split(query, ',')
                            .into_iter()
                            .map(move |b| format!("{a} and {b}"))
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            };
            parse_rules(body, file, body_loc, &query, supports, sheet);
        } else if let Some(query) = b.head.strip_prefix("@supports ") {
            let query = if supports.is_empty() {
                query.to_string()
            } else {
                format!("{supports} and {query}")
            };
            parse_rules(body, file, body_loc, media, &query, sheet);
        } else if b.head.starts_with('@') {
            sheet.warnings.push(Diagnostic::new(
                "W_CSS",
                format!("Ignored at-rule: {}", b.head),
                file,
                loc,
            ));
        } else {
            let ds = declarations(body, file, body_loc, &mut sheet.warnings);
            for sel in split(&b.head, ',') {
                sheet.rules.push(Rule {
                    selector: sel,
                    declarations: ds.clone(),
                    media: media.into(),
                    supports: supports.into(),
                });
            }
        }
    }
}
pub fn split(s: &str, delimiter: char) -> Vec<String> {
    let (mut depth, mut quote, mut escape, mut start) = (0, '\0', false, 0);
    let mut out = vec![];
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if quote != '\0' {
            if c == quote {
                quote = '\0';
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
        if depth == 0 && c == delimiter {
            out.push(s[start..i].trim().into());
            start = i + c.len_utf8();
        }
    }
    out.push(s[start..].trim().into());
    out
}
pub fn substitute(s: &str, vars: &BTreeMap<String, String>) -> std::result::Result<String, String> {
    crate::style::substitute(s, vars, &mut vec![])
}
pub fn media_matches(query: &str, reduced: bool) -> bool {
    media_matches_environment(query, reduced, false)
}
pub fn media_matches_environment(query: &str, reduced: bool, print: bool) -> bool {
    query.is_empty()
        || split(query, ',').iter().any(|q| match q.trim() {
            "all" => true,
            "screen" => !print,
            "print" => print,
            "(prefers-reduced-motion: reduce)" => reduced,
            "(prefers-reduced-motion: no-preference)" => !reduced,
            _ => false,
        })
}

pub use crate::css_selectors::{Element, matches, specificity};

fn custom_error(s: String) -> cssparser::ParseError<String> {
    cssparser::ParseError {
        kind: cssparser::ParseErrorKind::Custom(s),
    }
}
