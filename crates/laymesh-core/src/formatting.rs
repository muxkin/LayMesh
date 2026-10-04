use crate::model::V;
use regex::Regex;
use std::sync::LazyLock;
static FORMAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(.)([<^>])|([<^>]))?([+ -])?(0)?(\d+)?(?:\.(\d+))?([fFeEgG%d])?$").unwrap()
});

pub fn value(
    v: &V,
    spec: &str,
    conversion: Option<char>,
    dpi: f64,
) -> std::result::Result<String, String> {
    let (mut number, mut text, suffix) = match v {
        V::Number(n, u) => {
            let physical = !u.is_empty() && u != "deg";
            let n = if physical {
                crate::model::value_length(v, "mm", dpi).unwrap_or(*n)
            } else {
                *n
            };
            (
                Some(n),
                n.to_string(),
                if physical {
                    " mm"
                } else if u == "deg" {
                    " deg"
                } else {
                    ""
                },
            )
        }
        V::Text(s, _) => (None, s.clone(), ""),
        V::Bool(b) => (None, b.to_string(), ""),
        _ => return Err("插值需要字符串、布尔值或数字".into()),
    };
    if let Some(c) = conversion {
        if c == 'r' && matches!(v, V::Text(..)) {
            text = serde_json::to_string(&text).unwrap();
        }
        number = None;
    }
    if spec.is_empty() {
        return Ok(format!("{text}{suffix}"));
    }
    let caps = FORMAT
        .captures(spec)
        .ok_or_else(|| format!("不支持的格式 {spec}"))?;
    let get = |i| caps.get(i).map(|s| s.as_str());
    let width = get(6)
        .unwrap_or("0")
        .parse::<usize>()
        .map_err(|_| "格式宽度超过 10000")?;
    let precision = get(7)
        .unwrap_or("6")
        .parse::<usize>()
        .map_err(|_| "格式精度超过 100")?;
    if width > 10000 || precision > 100 {
        return Err("格式宽度或精度超出限制".into());
    }
    let kind = get(8).unwrap_or("");
    if let Some(n) = number {
        let abs = n.abs();
        text = match kind {
            "d" if n.fract() != 0. => return Err("d 格式需要整数".into()),
            "f" | "F" => format!("{abs:.precision$}"),
            "%" => format!("{:.precision$}%", abs * 100.),
            "e" | "E" => exponential(abs, precision),
            "g" | "G" => {
                let p = precision.max(1);
                let exponent = if abs == 0. {
                    0
                } else {
                    abs.log10().floor() as i32
                };
                if exponent < -6 || exponent >= p as i32 {
                    exponential(abs, p - 1)
                } else {
                    format!("{:.*}", (p as i32 - 1 - exponent).max(0) as usize, abs)
                }
            }
            _ => abs.to_string(),
        };
        if matches!(kind, "E" | "F" | "G") {
            text = text.to_uppercase();
        }
        text = format!(
            "{}{text}",
            if n < 0. {
                "-"
            } else {
                get(4).filter(|s| *s == "+" || *s == " ").unwrap_or("")
            }
        );
    } else if !kind.is_empty() || get(7).is_some() {
        return Err("数值格式需要数字".into());
    }
    let zero = get(5).is_some();
    let ch = get(1).unwrap_or(if zero { "0" } else { " " });
    let align = get(2)
        .or(get(3))
        .unwrap_or(if number.is_some() { ">" } else { "<" });
    let padding = width.saturating_sub(text.encode_utf16().count());
    text = match align {
        "<" => format!("{text}{}", ch.repeat(padding)),
        "^" => format!(
            "{}{text}{}",
            ch.repeat(padding / 2),
            ch.repeat(padding - padding / 2)
        ),
        _ if zero && text.starts_with(['+', '-', ' ']) => {
            format!("{}{}{}", &text[..1], ch.repeat(padding), &text[1..])
        }
        _ => format!("{}{text}", ch.repeat(padding)),
    };
    Ok(format!("{text}{suffix}"))
}
fn exponential(n: f64, precision: usize) -> String {
    let text = format!("{n:.precision$e}");
    let (m, e) = text.split_once('e').unwrap();
    let e = e.parse::<i32>().unwrap();
    format!("{m}e{e:+}")
}
