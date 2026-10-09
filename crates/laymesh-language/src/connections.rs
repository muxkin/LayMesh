//! Static placement checks use scoped bindings; they never execute user code.
use super::{LanguageService, entry, expression_type};
use laymesh_core::{
    endpoints::CONNECTION_PARAMETERS,
    parser::{Expr, ExprKind, Stmt, StmtKind},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn material_geometry(
    service: &LanguageService,
    uri: &str,
    expr: &Expr,
    at: usize,
    seen: &mut BTreeSet<String>,
) -> Option<bool> {
    match &expr.kind {
        ExprKind::Call(parts, args)
            if parts == &["line"] || laymesh_core::arrows::is_arrow(&parts.join(".")) =>
        {
            if service
                .resolve(uri, &parts[0], at, &mut BTreeSet::new())
                .is_some()
            {
                return None;
            }
            let has = |key: &str| args.iter().any(|(k, _)| k.as_deref() == Some(key));
            if parts.len() == 2 {
                return Some(match parts[1].as_str() {
                    "arc" => has("radius") && has("sweep_angle"),
                    "bent" | "uturn" => true,
                    "path" => has("path"),
                    _ => has("length") || has("dx") || has("dy"),
                });
            }
            Some(args.iter().any(|(key, _)| {
                key.as_deref()
                    .is_some_and(|k| ["dx", "dy", "length", "angle"].contains(&k))
            }))
        }
        ExprKind::Ref(parts) if parts.len() == 1 => {
            let symbol = service.resolve(uri, &parts[0], at, &mut BTreeSet::new())?;
            if symbol.kind != "variable" || !seen.insert(format!("{}:{}", symbol.uri, symbol.from))
            {
                return None;
            }
            material_geometry(
                service,
                &symbol.uri,
                symbol.value.as_ref()?,
                symbol.from.saturating_sub(1),
                seen,
            )
        }
        _ => None,
    }
}

pub(super) fn diagnostics(service: &LanguageService, uri: &str, stmts: &[Stmt]) -> Vec<Value> {
    fn check(
        service: &LanguageService,
        uri: &str,
        expr: &Expr,
        args: &[(Option<String>, Expr)],
        out: &mut Vec<Value>,
    ) {
        let keys: Vec<_> = args.iter().filter_map(|(k, _)| k.as_deref()).collect();
        let connection = CONNECTION_PARAMETERS.iter().any(|k| keys.contains(k));
        let mut report = |code: &str, zh: &str, en: &str| {
            out.push(json!({"from":expr.loc.offset,"to":expr.loc.offset+1,
                "severity":"error","code":code,"message":if service.en() { en } else { zh }}));
        };
        if connection {
            if !keys.contains(&"start") || !keys.contains(&"end") {
                report(
                    "E_ARG",
                    "连接需要同时指定 start 和 end",
                    "Connections require both start and end",
                );
            }
            if ["anchor", "target", "rotation", "size"]
                .iter()
                .any(|k| keys.contains(k))
            {
                report(
                    "E_ARG",
                    "双端点连接与 anchor/target/rotation/size 互斥",
                    "Connections are mutually exclusive with anchor/target/rotation/size",
                );
            }
            if args.iter().any(|(key, value)| {
                key.as_deref() == Some("offset_space")
                    && matches!(&value.kind, ExprKind::String(s, _, _) if s != "container")
            }) {
                report(
                    "E_ARG",
                    "双端点连接的整体 offset_space 只支持 container",
                    "The overall offset_space of a connection must be container",
                );
            }
        }
        if let Some(material) = args.iter().find(|(k, _)| k.is_none()).map(|(_, v)| v) {
            if !connection
                && material_geometry(
                    service,
                    uri,
                    material,
                    material.loc.offset,
                    &mut BTreeSet::new(),
                ) == Some(false)
            {
                report(
                    "E_ARG",
                    "素材未定义几何；请在 add 提供 start/end，或在素材中定义完整几何",
                    "Material has no geometry; supply start/end in add or define complete material geometry",
                );
            }
            if connection {
                let typ = expression_type(material, &|name| {
                    service.type_of(uri, name, material.loc.offset)
                });
                if typ.as_deref().is_some_and(|t| {
                    t != "line" && !laymesh_core::arrows::is_arrow(t) && entry(t).is_some()
                }) {
                    report(
                        "E_ARG",
                        "双端点连接参数只适用于 line 或 arrow 素材",
                        "Connection parameters only apply to line or arrow material",
                    );
                }
            }
        }
    }
    fn expression(service: &LanguageService, uri: &str, expr: &Expr, out: &mut Vec<Value>) {
        match &expr.kind {
            ExprKind::Call(parts, args) => {
                if parts.len() > 1 && parts.last().is_some_and(|p| p == "add") {
                    check(service, uri, expr, args, out);
                }
                for (_, arg) in args {
                    expression(service, uri, arg, out);
                }
            }
            ExprKind::Method(receiver, name, args) => {
                if name == "add" {
                    check(service, uri, expr, args, out);
                }
                expression(service, uri, receiver, out);
                for (_, arg) in args {
                    expression(service, uri, arg, out);
                }
            }
            ExprKind::List(values) => {
                for value in values {
                    expression(service, uri, value, out);
                }
            }
            ExprKind::Dict(values) => {
                for (a, b) in values {
                    expression(service, uri, a, out);
                    expression(service, uri, b, out);
                }
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
                expression(service, uri, a, out);
                expression(service, uri, b, out);
            }
            ExprKind::Member(value, _) | ExprKind::Unary(_, value) => {
                expression(service, uri, value, out)
            }
            _ => {}
        }
    }
    fn statements(service: &LanguageService, uri: &str, stmts: &[Stmt], out: &mut Vec<Value>) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Bind(_, value, _) | StmtKind::Expr(value) | StmtKind::Return(value) => {
                    expression(service, uri, value, out)
                }
                StmtKind::SetIndex(a, b) => {
                    expression(service, uri, a, out);
                    expression(service, uri, b, out);
                }
                StmtKind::Function(_, params, body, _) => {
                    for (_, value) in params {
                        if let Some(value) = value {
                            expression(service, uri, value, out);
                        }
                    }
                    statements(service, uri, body, out);
                }
                StmtKind::For(_, value, body) | StmtKind::While(value, body) => {
                    expression(service, uri, value, out);
                    statements(service, uri, body, out);
                }
                StmtKind::If(branches, other) => {
                    for (value, body) in branches {
                        expression(service, uri, value, out);
                        statements(service, uri, body, out);
                    }
                    statements(service, uri, other, out);
                }
                _ => {}
            }
        }
    }
    let mut out = vec![];
    statements(service, uri, stmts, &mut out);
    out
}
