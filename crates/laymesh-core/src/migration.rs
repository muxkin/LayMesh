//! Explicit source migration for identifiable legacy line-arrow calls.
pub fn legacy_arrow_parameter(name: &str) -> bool {
    name.starts_with("line_") || matches!(name, "start_head" | "end_head" | "start_cap" | "end_cap")
}
pub fn arrow_edits(source: &str) -> Vec<(usize, usize, String)> {
    let Ok(ts) = crate::parser::tokenize(source, "/migration.lay") else {
        return vec![];
    };
    if ts.windows(2).any(|w| {
        (w[0].text == "function" && w[1].text == "arrow")
            || (w[0].text == "arrow" && w[1].text == "=")
    }) {
        return vec![];
    }
    let mut edits = vec![];
    for i in 0..ts.len().saturating_sub(1) {
        if ts[i].string
            || ts[i].text != "arrow"
            || ts[i + 1].text != "("
            || i > 0 && ts[i - 1].text == "."
        {
            continue;
        }
        let mut depth = 0;
        for end in i + 1..ts.len() {
            if ts[end].string {
                continue;
            }
            if ts[end].text == "(" {
                depth += 1
            }
            if ts[end].text == ")" {
                depth -= 1;
                if depth == 0 {
                    let mut nesting = 0;
                    let mut keys = Vec::new();
                    for k in i + 2..end {
                        if ts[k].string {
                            continue;
                        }
                        match ts[k].text.as_str() {
                            "(" | "[" | "{" => nesting += 1,
                            ")" | "]" | "}" => nesting -= 1,
                            _ if nesting == 0 && ts.get(k + 1).is_some_and(|t| t.text == "=") => {
                                keys.push(ts[k].text.as_str())
                            }
                            _ => {}
                        }
                    }
                    let line = crate::engine::API["api"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|v| v["name"] == "line")
                        .unwrap();
                    if !keys.iter().any(|k| legacy_arrow_parameter(k))
                        || keys.iter().any(|k| {
                            !line["parameters"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|p| p["name"] == *k)
                        })
                    {
                        break;
                    }
                    let from = ts[i].loc.offset;
                    let to = ts[end].end;
                    let args = &source[ts[i + 1].end..ts[end].loc.offset];
                    edits.push((
                        from,
                        to,
                        if keys.contains(&"end_head") {
                            format!("line({args})")
                        } else {
                            format!("line(end_head=head(), {args})")
                        },
                    ));
                    break;
                }
            }
        }
    }
    edits
}
pub fn migrate_arrows(source: &str) -> String {
    let mut result = source.to_string();
    for (from, to, text) in arrow_edits(source).into_iter().rev() {
        result.replace_range(from..to, &text)
    }
    result
}
