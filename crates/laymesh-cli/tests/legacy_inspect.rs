//! Preserve the command/API equality contract for transformed broken-axis plots.
use std::{path::PathBuf, process::Command};

#[test]
fn rotated_broken_plots_inspect_cli_matches_scene() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let capture: serde_json::Value = serde_json::from_str(
        include_str!("../../../migration/corpus/cases-1044564.jsonl")
            .lines()
            .nth(17)
            .unwrap(),
    )
    .unwrap();
    let mut source = capture["source"].as_str().unwrap().to_owned();
    for name in capture["files"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|n| n.ends_with(".ttf"))
    {
        source = source.replace(
            name,
            &root
                .join("tests/fonts/DejaVuSans.ttf")
                .to_str()
                .unwrap()
                .replace('\\', "/"),
        );
    }
    let file = std::env::temp_dir().join(format!(
        "laymesh-inspect-rotated-{}.lay",
        std::process::id()
    ));
    std::fs::write(&file, source).unwrap();
    struct RemoveFile(PathBuf);
    impl Drop for RemoveFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = RemoveFile(file.clone());
    let scene = laymesh_core::engine::compile_file(file.to_str().unwrap()).unwrap();
    let expected = laymesh_language::inspect::inspect_scene(&scene);
    // The original contract compares two JSON round trips, so normalize the
    // in-memory floats through the same representation as the CLI output.
    let expected: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&expected).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("inspect")
        .arg(&file)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(actual["plots"], expected["plots"]);
    assert_eq!(actual["plots"].as_array().unwrap().len(), 2);
}
