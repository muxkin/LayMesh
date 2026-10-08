//! Keep mhchem's compound bonds semantic at the RaTeX parser boundary.
//!
//! RaTeX 0.1.14 exposes per-parser macro hooks but no chemical-bond AST node.
//! Wrap its original ce expander (including its shared recursion budget) and
//! decode the canonical bond productions emitted by mhchem/texify.rs, before
//! their lap/raise/text-style recipes are expanded. Only ce's emitted tokens
//! are decoded; user source, ordinary overlays and the default backend stay
//! untouched. A private htmlStyle wrapper transports the construction tag.
use ratex_parser::{
    Mode, ParseNode, Parser,
    macro_expander::{FnMacroHandler, MacroDefinition, MacroExpander},
};
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub(super) enum ChemicalBond {
    Dashed,
    PartialDouble,
    PartialTriple,
    DashedMiddle,
}
impl ChemicalBond {
    fn tag(self) -> &'static str {
        match self {
            Self::Dashed => "--laymesh-math-construction:chemical-dash",
            Self::PartialDouble => "--laymesh-math-construction:chemical-partial-double",
            Self::PartialTriple => "--laymesh-math-construction:chemical-partial-triple",
            Self::DashedMiddle => "--laymesh-math-construction:chemical-dashed-middle",
        }
    }
    fn expansion(self) -> String {
        format!("\\htmlStyle{{{}}}{{-}}", self.tag())
    }
    pub(super) fn from_tag(tag: &str) -> Option<Self> {
        [
            Self::Dashed,
            Self::PartialDouble,
            Self::PartialTriple,
            Self::DashedMiddle,
        ]
        .into_iter()
        .find(|b| b.tag() == tag)
    }
}

fn token_texts(tex: &str) -> Vec<String> {
    let mut lexer = MacroExpander::new(tex, Mode::Math);
    let mut result = vec![];
    loop {
        let token = lexer.pop_token();
        if token.is_eof() {
            break;
        }
        result.push(token.text);
    }
    result
}

static ORIGINAL_CE: OnceLock<FnMacroHandler> = OnceLock::new();
static BONDS: OnceLock<Vec<(Vec<String>, ChemicalBond)>> = OnceLock::new();

pub(crate) fn parse(source: &str) -> ratex_parser::ParseResult<Vec<ParseNode>> {
    let mut parser = Parser::new(source);
    ORIGINAL_CE.get_or_init(|| match parser.gullet.get_macro("\\ce") {
        Some(MacroDefinition::Function(handler)) => *handler,
        _ => unreachable!("pinned RaTeX ce macro must be a function"),
    });
    BONDS.get_or_init(|| {
        [
            // These are grammar productions of the pinned upstream emitter,
            // not matches against example formulas or font names.
            (
                r"{\mathrlap{\raisebox{-.1em}{$-$}}\raisebox{.1em}{$\tripledash$}}",
                ChemicalBond::PartialDouble,
            ),
            (
                r"{\mathrlap{\raisebox{-.2em}{$-$}}\mathrlap{\raisebox{.2em}{$\tripledash$}}-}",
                ChemicalBond::PartialTriple,
            ),
            (
                r"{\mathrlap{\raisebox{-.2em}{$-$}}\mathrlap{\raisebox{.2em}{$-$}}\tripledash}",
                ChemicalBond::DashedMiddle,
            ),
        ]
        .into_iter()
        .map(|(tex, bond)| (token_texts(tex), bond))
        .collect()
    });
    parser.gullet.set_macro(
        "\\ce".into(),
        MacroDefinition::Function(|gullet| {
            let mut tokens = ORIGINAL_CE.get().expect("ce hook initialized")(gullet)?;
            tokens.reverse(); // upstream returns stack order
            let mut output = vec![];
            let mut i = 0;
            while i < tokens.len() {
                let production =
                    BONDS
                        .get()
                        .expect("bond grammar initialized")
                        .iter()
                        .find(|(pattern, _)| {
                            tokens.get(i..i + pattern.len()).is_some_and(|slice| {
                                slice.iter().zip(pattern).all(|(t, p)| t.text == *p)
                            })
                        });
                if let Some((pattern, bond)) = production {
                    let expansion = bond.expansion();
                    let mut lexer = MacroExpander::new(&expansion, Mode::Math);
                    loop {
                        let token = lexer.pop_token();
                        if token.is_eof() {
                            break;
                        }
                        output.push(token);
                    }
                    i += pattern.len();
                } else {
                    output.push(tokens[i].clone());
                    i += 1;
                }
            }
            output.reverse();
            Ok(output)
        }),
    );
    parser.gullet.set_macro(
        "\\tripledash".into(),
        MacroDefinition::Text(ChemicalBond::Dashed.expansion()),
    );
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_overlays_and_non_bond_chemistry_keep_the_upstream_ast() {
        for source in [
            r"\mathrlap{\raisebox{-.2em}{$-$}}-",
            r"\htmlStyle{color:red}{x}+\text{-}",
            r"\ce{H2 + O2 -> H2O}",
            r"\ce{A\bond{1}B\bond{2}C\bond{3}D\bond{...}E}",
            r"\pu{123 kJ mol-1}",
        ] {
            assert_eq!(
                serde_json::to_value(parse(source).unwrap()).unwrap(),
                serde_json::to_value(ratex_parser::parse(source).unwrap()).unwrap(),
                "{source}"
            );
        }
    }

    #[test]
    fn ce_hook_preserves_upstreams_shared_recursion_limit() {
        for depth in [1, 15, 29, 30, 31, 32, 40] {
            let source = format!("{}H{}", r"\ce{".repeat(depth), "}".repeat(depth));
            let native = parse(&source);
            let upstream = ratex_parser::parse(&source);
            assert_eq!(native.is_ok(), upstream.is_ok(), "depth {depth}");
            if let (Err(native), Err(upstream)) = (native, upstream) {
                assert_eq!(native.to_string(), upstream.to_string(), "depth {depth}");
            }
        }
    }
}
