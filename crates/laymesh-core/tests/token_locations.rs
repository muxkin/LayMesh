use laymesh_core::{Loc, parser::tokenize};

#[test]
fn locations_after_skipped_unicode_comments_strings_and_styles() {
    let source = "# 中文😀\r\ntext(\"甲😀\", size=2)\nstyle {\n/* 😃 */ .a { fill: \"蓝\"; }\n}\nvalue = 3\n";
    let tokens = tokenize(source, "positions.lay").unwrap();
    let select = |text: &str| tokens.iter().find(|t| t.text == text).unwrap().loc;
    assert_eq!(
        select("text"),
        Loc {
            line: 2,
            column: 1,
            offset: source.find("text").unwrap()
        }
    );
    assert_eq!(
        select("size"),
        Loc {
            line: 2,
            column: 13,
            offset: source.find("size").unwrap()
        }
    );
    let style = tokens
        .iter()
        .find(|t| t.text.starts_with("@style"))
        .unwrap();
    assert_eq!(
        style.loc,
        Loc {
            line: 3,
            column: 8,
            offset: source.find("{\n").unwrap() + 1
        }
    );
    assert_eq!(
        select("value"),
        Loc {
            line: 6,
            column: 1,
            offset: source.find("value").unwrap()
        }
    );
    assert_eq!(
        tokens.last().unwrap().loc,
        Loc {
            line: 7,
            column: 1,
            offset: source.len()
        }
    );
}

#[test]
fn multiline_raw_and_formatted_strings_keep_following_locations() {
    let source = "r'''甲\n😀''' + f\"{2 + 3}β\"\nnext=4";
    let tokens = tokenize(source, "strings.lay").unwrap();
    let plus = tokens.iter().find(|t| t.text == "+").unwrap();
    assert_eq!(
        plus.loc,
        Loc {
            line: 2,
            column: 7,
            offset: source.find('+').unwrap()
        }
    );
    let next = tokens.iter().find(|t| t.text == "next").unwrap();
    assert_eq!(
        next.loc,
        Loc {
            line: 3,
            column: 1,
            offset: source.find("next").unwrap()
        }
    );
}

#[test]
fn syntax_errors_retain_utf16_column_and_original_style_start() {
    let source = "# skipped\nvalue=\"😀\" $";
    let error = tokenize(source, "invalid.lay").unwrap_err();
    assert_eq!(
        error.loc,
        Loc {
            line: 2,
            column: 12,
            offset: source.find('$').unwrap()
        }
    );
    let source = "text(\"😀\")\n  style { /* 中文 */ .x { opacity: 1; }";
    let error = tokenize(source, "invalid.lay").unwrap_err();
    assert_eq!(
        error.loc,
        Loc {
            line: 2,
            column: 3,
            offset: source.find("style").unwrap()
        }
    );
}
