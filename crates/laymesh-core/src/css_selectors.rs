//! Shared selector validation, matching and specificity for appearance/motion.
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug, Default)]
pub struct Element {
    pub kind: String,
    pub class: String,
}
struct Compound<'a> {
    kind: &'a str,
    classes: Vec<&'a str>,
    attributes: Vec<(&'a str, &'a str, bool)>,
    root: bool,
    weight: usize,
}
fn compound(mut s: &str) -> Option<Compound<'_>> {
    if matches!(
        s,
        ":root" | "html" | "body" | ":where(html)" | ":where(:root)"
    ) {
        return Some(Compound {
            kind: "",
            classes: vec![],
            attributes: vec![],
            root: true,
            weight: if s.starts_with(":where") {
                0
            } else if s == ":root" {
                100
            } else {
                1
            },
        });
    }
    static TYPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[\w-]+|\*)").unwrap());
    static CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\.([\w-]+)").unwrap());
    static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^\[\s*class\s*([*~^$|]?=)\s*(?:'([^']*)'|"([^"]*)"|([\w-]+))\s*([iIsS])?\s*\]"#,
        )
        .unwrap()
    });
    let kind = TYPE.find(s).map(|m| m.as_str()).unwrap_or("");
    s = &s[kind.len()..];
    let mut out = Compound {
        kind,
        classes: vec![],
        attributes: vec![],
        root: false,
        weight: usize::from(!kind.is_empty() && kind != "*"),
    };
    while !s.is_empty() {
        if let Some(c) = CLASS.captures(s) {
            out.classes.push(c.get(1)?.as_str());
            s = &s[c.get(0)?.end()..];
        } else if let Some(c) = ATTRIBUTE.captures(s) {
            let value = c.get(2).or_else(|| c.get(3)).or_else(|| c.get(4))?.as_str();
            out.attributes.push((
                c.get(1)?.as_str(),
                value,
                c.get(5)
                    .is_some_and(|m| m.as_str().eq_ignore_ascii_case("i")),
            ));
            s = &s[c.get(0)?.end()..];
        } else {
            return None;
        }
        out.weight += 100;
    }
    (!kind.is_empty() || !out.classes.is_empty() || !out.attributes.is_empty()).then_some(out)
}
fn tokens(selector: &str) -> Vec<&str> {
    let (mut depth, mut quote, mut escape, mut start) = (0, '\0', false, 0);
    let mut out = vec![];
    for (i, c) in selector.char_indices() {
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
        if matches!(c, '[' | '(') {
            depth += 1;
        }
        if matches!(c, ']' | ')') {
            depth -= 1;
        }
        if depth == 0 && (c.is_whitespace() || c == '>') {
            if start < i {
                out.push(&selector[start..i]);
            }
            if c == '>' {
                out.push(&selector[i..i + 1]);
            }
            start = i + c.len_utf8();
        }
    }
    if start < selector.len() {
        out.push(&selector[start..]);
    }
    out
}
pub fn specificity(selector: &str) -> usize {
    tokens(selector)
        .iter()
        .filter_map(|s| compound(s))
        .map(|c| c.weight)
        .sum()
}
pub fn supported(selector: &str) -> bool {
    let parts = tokens(selector);
    !parts.is_empty()
        && parts.first() != Some(&">")
        && parts.last() != Some(&">")
        && !parts.windows(2).any(|w| w == [">", ">"])
        && parts.iter().all(|s| *s == ">" || compound(s).is_some())
}
fn compound_matches(sel: &str, e: &Element, root: bool) -> bool {
    let Some(c) = compound(sel) else { return false };
    if c.root {
        return root;
    }
    if !c.kind.is_empty() && c.kind != "*" && c.kind != e.kind {
        return false;
    }
    if !c
        .classes
        .iter()
        .all(|class| e.class.split_whitespace().any(|s| s == *class))
    {
        return false;
    }
    c.attributes.iter().all(|(op, value, insensitive)| {
        let (class, value) = if *insensitive {
            (e.class.to_ascii_lowercase(), value.to_ascii_lowercase())
        } else {
            (e.class.clone(), (*value).into())
        };
        match *op {
            "=" => class == value,
            "*=" => !value.is_empty() && class.contains(&value),
            "~=" => !value.is_empty() && class.split_whitespace().any(|s| s == value),
            "^=" => !value.is_empty() && class.starts_with(&value),
            "$=" => !value.is_empty() && class.ends_with(&value),
            "|=" => class == value || class.starts_with(&format!("{value}-")),
            _ => false,
        }
    })
}
pub fn matches(selector: &str, chain: &[Element]) -> bool {
    fn test(parts: &[&str], chain: &[Element]) -> bool {
        if parts.is_empty() {
            return true;
        }
        let Some(last) = chain.last() else {
            return false;
        };
        if !compound_matches(parts[parts.len() - 1], last, chain.len() == 1) {
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
    supported(selector) && test(&tokens(selector), chain)
}

pub(crate) fn scope_applies(scope: Option<&str>, file: &str, entry: Option<&str>) -> bool {
    scope.is_none() || scope == Some(file) || entry.is_some() && scope == entry
}
pub(crate) fn theme_priority(scope: Option<&str>, entry: Option<&str>) -> bool {
    scope.is_none() || entry.is_some() && scope == entry
}
