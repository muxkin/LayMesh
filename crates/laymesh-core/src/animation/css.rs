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
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub media: String,
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
    end: usize,
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
                    let head = p.slice(start..before).trim().to_string();
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
                        end: p.position().byte_index(),
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
                        head: p.slice(start..before).trim().to_string(),
                        body: None,
                        start: start.byte_index(),
                        end: p.position().byte_index(),
                        body_start: 0,
                    });
                    ended = true;
                    break;
                }
                Token::BadString(_)
                | Token::BadUrl(_)
                | Token::CloseCurlyBracket
                | Token::CloseParenthesis
                | Token::CloseSquareBracket => return Err("Invalid CSS token".into()),
                _ => {}
            }
        }
        if !ended && !p.slice_from(start).trim().is_empty() {
            return Err("Expected CSS rule block or semicolon".into());
        }
    }
    Ok(out)
}
fn declarations(body: &str, file: &str, origin: Loc) -> Result<Vec<Declaration>> {
    let mut p = Parser::new(body);
    let mut result = vec![];
    while !p.is_exhausted() {
        p.skip_whitespace();
        let start = p.position();
        let loc = location(body, origin, start.byte_index());
        let fail = |s: String| Diagnostic::new("E_LCSS", s, file, loc);
        let token = p
            .next()
            .map_err(|e| fail(format!("CSS declaration: {e:?}")))?
            .clone();
        if token == Token::Semicolon {
            continue;
        }
        let Token::Ident(key) = token else {
            return Err(fail("Expected CSS property".into()));
        };
        p.expect_colon().map_err(|_| fail("Expected ':'".into()))?;
        let value_start = p.position();
        let mut end = value_start;
        while !p.is_exhausted() {
            let before = p.position();
            let token = p
                .next_including_whitespace_and_comments()
                .map_err(|e| fail(format!("CSS value: {e:?}")))?
                .clone();
            if token == Token::Semicolon {
                end = before;
                break;
            }
            // Consume function/block tokens, so slicing includes the complete value.
            if matches!(
                token,
                Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock
            ) {
                p.parse_nested_block(|q| {
                    while q.next_including_whitespace_and_comments().is_ok() {}
                    Ok::<_, cssparser::ParseError<()>>(())
                })
                .map_err(|e| fail(format!("CSS value: {e:?}")))?;
            }
            end = p.position();
        }
        let mut value = p.slice(value_start..end).trim().to_string();
        static IMPORTANT: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\s*!\s*important\s*$").unwrap());
        let important = &*IMPORTANT;
        let imp = important.is_match(&value);
        value = important.replace(&value, "").trim().to_string();
        if value.is_empty() {
            return Err(fail("Empty CSS value".into()));
        }
        let property = if key.starts_with("--") {
            key.to_string()
        } else {
            key.to_ascii_lowercase()
        };
        result.push(Declaration {
            property: property
                .strip_prefix("-webkit-")
                .unwrap_or(&property)
                .into(),
            value,
            important: imp,
            file: file.into(),
            loc,
        });
    }
    Ok(result)
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
/// Returns a whitespace-masked source for the legacy drawing cascade and the
/// complete motion stylesheet. Offsets in the masked source are unchanged.
pub fn extract(
    source: &str,
    file: &str,
    origin: Loc,
    scope: Option<&str>,
) -> Result<(String, Sheet)> {
    let mut legacy = source.as_bytes().to_vec();
    let mut sheet = Sheet {
        scope: scope.map(str::to_owned),
        ..Sheet::default()
    };
    let mut blank = |a: usize, b: usize| {
        for c in &mut legacy[a..b] {
            if !matches!(*c, b'\n' | b'\r') {
                *c = b' ';
            }
        }
    };
    parse_rules(source, file, origin, "", &mut sheet, &mut blank, 0)?;
    Ok((String::from_utf8(legacy).unwrap(), sheet))
}
fn parse_rules(
    source: &str,
    file: &str,
    origin: Loc,
    media: &str,
    sheet: &mut Sheet,
    blank: &mut dyn FnMut(usize, usize),
    base: usize,
) -> Result<()> {
    let rules = blocks(source).map_err(|m| Diagnostic::new("E_LCSS", m, file, origin))?;
    for b in rules {
        let loc = location(source, origin, b.start);
        let Some(body) = &b.body else {
            if b.head.starts_with("@charset") {
                blank(base + b.start, base + b.end);
            } else if !b.head.starts_with("@import") {
                return Err(Diagnostic::new(
                    "E_LCSS",
                    "Unsupported CSS at-rule",
                    file,
                    loc,
                ));
            }
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
            if name.is_empty() {
                return Err(Diagnostic::new(
                    "E_LCSS",
                    "Missing animation name",
                    file,
                    loc,
                ));
            }
            let mut frames = vec![];
            for frame in blocks(body).map_err(|m| Diagnostic::new("E_LCSS", m, file, body_loc))? {
                let fb = frame.body.ok_or_else(|| {
                    Diagnostic::new("E_LCSS", "Expected keyframe block", file, body_loc)
                })?;
                let ds = declarations(&fb, file, location(body, body_loc, frame.body_start))?;
                for at in split(&frame.head, ',') {
                    let at = match at.trim() {
                        "from" => 0.,
                        "to" => 1.,
                        x => x
                            .strip_suffix('%')
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| v / 100.)
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    "E_LCSS",
                                    "Keyframe requires from/to or percentage",
                                    file,
                                    loc,
                                )
                            })?,
                    };
                    if !(0.0..=1.0).contains(&at) {
                        return Err(Diagnostic::new(
                            "E_LCSS",
                            "Keyframe outside 0–100%",
                            file,
                            loc,
                        ));
                    }
                    frames.push((at, ds.iter().filter(|d| !d.important).cloned().collect()));
                }
            }
            sheet.keyframes.push(Keyframes {
                name,
                frames,
                prefixed: b.head.starts_with("@-"),
                file: file.into(),
                loc,
            });
            blank(base + b.start, base + b.end);
        } else if let Some(query) = b.head.strip_prefix("@media ") {
            if !media.is_empty() {
                return Err(Diagnostic::new(
                    "E_LCSS",
                    "Nested media queries are not supported",
                    file,
                    loc,
                ));
            }
            if split(query, ',').iter().any(|q| {
                !matches!(
                    q.trim(),
                    "screen"
                        | "all"
                        | "print"
                        | "(prefers-reduced-motion: reduce)"
                        | "(prefers-reduced-motion: no-preference)"
                )
            }) {
                return Err(Diagnostic::new(
                    "E_LCSS",
                    "Unsupported animation media query",
                    file,
                    loc,
                ));
            }
            parse_rules(
                body,
                file,
                body_loc,
                query,
                sheet,
                blank,
                base + b.body_start,
            )?;
            blank(base + b.start, base + b.end);
        } else if b.head.starts_with('@') {
            return Err(Diagnostic::new(
                "E_LCSS",
                "Unsupported CSS at-rule",
                file,
                loc,
            ));
        } else {
            let ds = declarations(body, file, body_loc)?;
            for sel in split(&b.head, ',') {
                sheet.rules.push(Rule {
                    selector: sel,
                    declarations: ds.clone(),
                    media: media.into(),
                });
            }
            if !media.is_empty() || b.head.contains(":root") || b.head.contains('[') {
                blank(base + b.start, base + b.end);
                continue;
            }
            // Mask only motion declarations; keep normal drawing declarations and
            // their exact source offsets, including existing strict diagnostics.
            for d in &ds {
                if is_motion_property(&d.property) {
                    let start = d.loc.offset - origin.offset;
                    let relative = start - b.body_start;
                    let mut p = Parser::new(&body[relative..]);
                    while !p.is_exhausted() {
                        if matches!(
                            p.next_including_whitespace_and_comments(),
                            Ok(Token::Semicolon)
                        ) {
                            break;
                        }
                    }
                    blank(base + start, base + start + p.position().byte_index());
                }
            }
        }
    }
    Ok(())
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

/// Selector context is a root-to-target chain of material type and class lists.
#[derive(Clone, Debug, Default)]
pub struct Element {
    pub kind: String,
    pub class: String,
}
fn compound(sel: &str, e: &Element, root: bool) -> bool {
    if sel == ":root" {
        return root;
    }
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"\[class\s*([*~^$|]?=)\s*['\"]([^'\"]+)['\"]\s*\]"#).unwrap()
    });
    let re = &*RE;
    for c in re.captures_iter(sel) {
        let v = &c[2];
        if !match &c[1] {
            "=" => e.class == v,
            "*=" => e.class.contains(v),
            "~=" => e.class.split_whitespace().any(|x| x == v),
            "^=" => e.class.starts_with(v),
            "$=" => e.class.ends_with(v),
            "|=" => e.class == v || e.class.starts_with(&format!("{v}-")),
            _ => false,
        } {
            return false;
        }
    }
    let s = re.replace_all(sel, "");
    if s.contains(['[', ']', ':', '#']) {
        return false;
    }
    let mut parts = s.split('.');
    let typ = parts.next().unwrap_or("");
    (typ.is_empty() || typ == "*" || typ == e.kind)
        && parts.all(|c| e.class.split_whitespace().any(|x| x == c))
}
pub fn matches(selector: &str, chain: &[Element]) -> bool {
    fn test(parts: &[&str], chain: &[Element]) -> bool {
        if parts.is_empty() {
            return true;
        }
        if chain.is_empty() {
            return false;
        }
        if !compound(
            parts[parts.len() - 1],
            chain.last().unwrap(),
            chain.len() == 1,
        ) {
            return false;
        }
        if parts.len() == 1 {
            return true;
        }
        if parts[parts.len() - 2] == ">" {
            test(&parts[..parts.len() - 2], &chain[..chain.len() - 1])
        } else {
            (1..chain.len())
                .rev()
                .any(|n| test(&parts[..parts.len() - 1], &chain[..n]))
        }
    }
    // Split combinators only outside attribute selectors and quoted values.
    let normalized = selector.replace('>', " > ");
    test(
        &split(&normalized, ' ')
            .iter()
            .filter(|s| !s.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>(),
        chain,
    )
}
pub fn specificity(selector: &str) -> usize {
    selector.matches('.').count() * 100
        + selector.matches('[').count() * 100
        + selector.matches(":root").count() * 100
        + selector
            .split([' ', '>'])
            .filter(|s| s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
            .count()
}
