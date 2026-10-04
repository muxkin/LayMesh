//! Explicit source migration for retired builtins; never a runtime compatibility alias.
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
                    let from = ts[i].loc.offset;
                    let to = ts[end].end;
                    let args = &source[ts[i + 1].end..ts[end].loc.offset];
                    edits.push((from, to, format!("line(end_head=head(), {args})")));
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
