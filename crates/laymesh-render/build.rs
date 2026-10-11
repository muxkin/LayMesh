fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux")
        || std::env::var_os("CARGO_FEATURE_NATIVE").is_none()
    {
        return;
    }
    // Keep the bundled CLI independent of optional desktop font libraries.
    // Skia uses these internally even though LayMesh shapes text in Rust.
    let output = std::process::Command::new("pkg-config")
        .args(["--static", "--libs", "fontconfig", "freetype2"])
        .output()
        .expect("Install pkg-config and the Fontconfig/FreeType development packages");
    assert!(
        output.status.success(),
        "Fontconfig/FreeType development libraries are required"
    );
    let flags = String::from_utf8(output.stdout).unwrap();
    let mut seen = std::collections::HashSet::new();
    for flag in flags.split_whitespace() {
        if let Some(directory) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={directory}");
        } else if let Some(name) = flag.strip_prefix("-l") {
            if seen.insert(name) {
                let kind = if matches!(name, "m" | "pthread" | "dl" | "rt") {
                    "dylib"
                } else {
                    "static"
                };
                println!("cargo:rustc-link-lib={kind}={name}");
            }
        }
    }
}
