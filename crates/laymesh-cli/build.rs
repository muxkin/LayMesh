use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let archive = env::var_os("LAYMESH_WEB_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/web-export/runtime.zip"));
    println!("cargo:rerun-if-env-changed=LAYMESH_WEB_RUNTIME");
    println!("cargo:rerun-if-changed={}", archive.display());
    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("web-runtime.zip");
    fs::write(output, fs::read(archive).unwrap_or_default()).unwrap();
}
