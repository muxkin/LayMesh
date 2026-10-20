pub mod animation;
pub mod arrows;
mod art;
pub mod asset_cache;
pub mod assets;
pub mod backgrounds;
mod collections;
pub mod color;
pub mod colormap;
mod css_selectors;
pub mod css_values;
mod data;
pub mod document;
pub mod endpoints;
pub mod engine;
mod formatting;
pub mod geometry;
mod geometry_cache;
pub mod geometry_query;
mod geometry_recipe;
mod gradient_color;
pub mod gradients;
pub mod media;
pub mod migration;
pub mod model;
mod opentype_math;
pub mod page_transition;
pub mod parser;
pub mod plot;
mod query_path;
pub mod session;
pub mod style;
pub mod text;
mod unit_diagnostics;
mod validation;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm_allocator;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Loc {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub file: String,
    pub loc: Loc,
}
impl Diagnostic {
    pub fn new(code: &str, message: impl Into<String>, file: &str, loc: Loc) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            file: file.into(),
            loc,
        }
    }
}
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}: {}",
            self.file, self.loc.line, self.loc.column, self.code, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}
pub type Result<T> = std::result::Result<T, Diagnostic>;
