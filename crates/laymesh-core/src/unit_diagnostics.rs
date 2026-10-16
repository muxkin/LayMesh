//! Suggestions are limited to literal arithmetic and validated without executing
//! user expressions again. Diagnostics never change dimensional evaluation.
use crate::{
    Diagnostic, Loc, Result,
    model::V,
    parser::{Expr, ExprKind},
};

fn dimension(unit: &str) -> &str {
    match unit {
        "" => "无量纲数",
        "s" | "ms" => "时间",
        "deg" | "rad" => "角度",
        _ => "长度",
    }
}

fn render(expr: &Expr, parent: u8) -> Option<String> {
    let (text, precedence) = match &expr.kind {
        ExprKind::Number(n, u) => (format!("{n}{u}"), 4),
        ExprKind::Unary(op, e) if op == "-" => (format!("-{}", render(e, 3)?), 3),
        ExprKind::Binary(op, a, b) if matches!(op.as_str(), "+" | "-" | "*" | "/") => {
            let p = if matches!(op.as_str(), "+" | "-") {
                1
            } else {
                2
            };
            // Parenthesize equal-precedence right operands to retain the AST,
            // including floating point order, rather than guessing association.
            (format!("{}{op}{}", render(a, p)?, render(b, p + 1)?), p)
        }
        _ => return None,
    };
    Some(if precedence < parent {
        format!("({text})")
    } else {
        text
    })
}

fn literal(expr: &Expr) -> Option<(f64, &str)> {
    match &expr.kind {
        ExprKind::Number(n, unit) => Some((*n, unit)),
        ExprKind::Unary(op, e) if op == "-" => literal(e).map(|(n, u)| (-n, u)),
        _ => None,
    }
}

fn pure_eval(expr: &Expr, binary: &impl Fn(&str, V, V, Loc) -> Result<V>) -> Option<V> {
    match &expr.kind {
        ExprKind::Number(n, u) if n.is_finite() => Some(if u == "rad" {
            V::Number(n.to_degrees(), "deg".into())
        } else {
            V::Number(*n, u.clone())
        }),
        ExprKind::Unary(op, e) if op == "-" => match pure_eval(e, binary)? {
            V::Number(n, u) => Some(V::Number(-n, u)),
            _ => None,
        },
        ExprKind::Binary(op, a, b) if matches!(op.as_str(), "+" | "-" | "*" | "/") => {
            binary(op, pure_eval(a, binary)?, pure_eval(b, binary)?, expr.loc).ok()
        }
        _ => None,
    }
}

// Moving a unit into addition could create a mismatch. Only offer this shorter
// form for multiplication/division chains with scalar literal operands.
fn attach_first(expr: &mut Expr, unit: &str) -> bool {
    match &mut expr.kind {
        ExprKind::Number(_, u) if u.is_empty() => {
            *u = unit.into();
            true
        }
        ExprKind::Unary(op, e) if op == "-" => attach_first(e, unit),
        ExprKind::Binary(op, a, _) if matches!(op.as_str(), "*" | "/") => attach_first(a, unit),
        _ => false,
    }
}

pub(crate) fn annotate(
    op: &str,
    a: &Expr,
    b: &Expr,
    left: &V,
    right: &V,
    mut error: Diagnostic,
    binary: impl Fn(&str, V, V, Loc) -> Result<V>,
) -> Diagnostic {
    let (V::Number(_, lu), V::Number(_, ru)) = (left.value(), right.value()) else {
        return error;
    };
    if error.code != "E_UNIT" {
        return error;
    }
    if matches!(op, "+" | "-" | "==" | "!=" | "<" | ">" | "<=" | ">=")
        && dimension(lu) != dimension(ru)
    {
        error.message = format!(
            "运算维度不匹配：左侧为{}，右侧为{}。裸数字只在尺寸等参数消费时应用默认单位；运算时请显式写出相同量纲的单位。",
            dimension(lu),
            dimension(ru)
        );
        return error;
    }
    if op != "/" || !lu.is_empty() || ru.is_empty() || error.message == "除数不能为零" {
        return error;
    }
    error.message = format!("运算结果为倒数{}，LayMesh 不支持这种量纲。", dimension(ru));
    if let Some((n, unit)) = literal(b) {
        error.message += &format!("\n\n{unit} 属于数字 {n}，因此这里计算的是：");
        if let (Some(a_text), Some(b_text)) = (render(a, 0), render(b, 0)) {
            error.message += &format!("\n({a_text})/({b_text})");
        } else {
            error.message += &format!("无量纲数除以 {n}{unit}。");
        }
        if matches!(pure_eval(a, &binary), Some(V::Number(_, ref u)) if u.is_empty()) {
            let bare_divisor = Expr {
                identity: 0,
                site: 0,
                kind: ExprKind::Number(n, String::new()),
                loc: b.loc,
            };
            let quotient = Expr {
                identity: 0,
                site: 0,
                kind: ExprKind::Binary(
                    "/".into(),
                    Box::new(a.clone()),
                    Box::new(bare_divisor.clone()),
                ),
                loc: error.loc,
            };
            let generic = Expr {
                identity: 0,
                site: 0,
                kind: ExprKind::Binary(
                    "*".into(),
                    Box::new(quotient),
                    Box::new(Expr {
                        identity: 0,
                        site: 0,
                        kind: ExprKind::Number(1., unit.into()),
                        loc: b.loc,
                    }),
                ),
                loc: error.loc,
            };
            let mut first = a.clone();
            let mut suggestions = Vec::new();
            if attach_first(&mut first, unit) {
                let shorter = Expr {
                    identity: 0,
                    site: 0,
                    kind: ExprKind::Binary("/".into(), Box::new(first), Box::new(bare_divisor)),
                    loc: error.loc,
                };
                if pure_eval(&shorter, &binary).is_some() {
                    suggestions.push(render(&shorter, 0).unwrap());
                }
            }
            if pure_eval(&generic, &binary).is_some() {
                // The extra parentheses explicitly separate the scalar ratio.
                suggestions.push(format!("({}/{n})*1{unit}", render(a, 2).unwrap()));
            }
            if !suggestions.is_empty() {
                error.message += &format!(
                    "\n\n如果你想计算以 {unit} 表示的{}，可以写：\n{}",
                    dimension(unit),
                    suggestions.join("\n或：\n")
                );
                return error;
            }
        }
    }
    error.message += &format!(
        "\n\n请检查除数的单位：计算{}比例应让同类量相除；计算{}应让无量纲比例乘以目标单位。复杂表达式需明确各部分的量纲。",
        dimension(ru),
        dimension(ru)
    );
    error
}
