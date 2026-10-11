//! Reproducible tokenizer/parser/compiler timings; no raster or GPU work.
use laymesh_core::{engine::compile_preview_source, model::Host, parser};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Instant};

fn main() {
    let mut rows = vec![];
    for file in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&file).unwrap();
        for iteration in 0..3 {
            let start = Instant::now();
            let tokens = black_box(parser::tokenize(&source, &file).unwrap());
            let tokenize_ms = start.elapsed().as_secs_f64() * 1000.;
            let token_count = tokens.len();
            drop(tokens);
            let start = Instant::now();
            let statements = black_box(parser::parse(&source, &file).unwrap());
            let parse_ms = start.elapsed().as_secs_f64() * 1000.;
            drop(statements);
            let start = Instant::now();
            let scene = compile_preview_source(
                &source,
                &file,
                Host {
                    native: true,
                    ..Host::default()
                },
            )
            .unwrap();
            let compile_ms = start.elapsed().as_secs_f64() * 1000.;
            let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&scene).unwrap()));
            rows.push(
                json!({"file":file,"iteration":iteration,"source_bytes":source.len(),
                "tokens":token_count,"tokenize_ms":tokenize_ms,"parse_ms":parse_ms,
                "compile_ms":compile_ms,"scene_sha256":digest}),
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "definition":"Three sequential native runs per source, same process. Parse includes tokenize; compile includes parse. Timings overlap and must not be added. Scene hashing is outside timing.",
        "rows":rows
    })).unwrap());
}
