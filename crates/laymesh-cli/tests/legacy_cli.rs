//! Command-level ports from acceptance.test.mjs and typography.test.mjs.
use std::{path::PathBuf, process::Command};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "LayMesh-中文路径-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn legacy_cli_chinese_paths_exports_and_nonzero_located_failures() {
    let tmp = Temp::new();
    let input = tmp.0.join("layout.lay");
    let output = tmp.0.join("output.png");
    std::fs::write(&input,"page=canvas(size=(10 mm,10 mm))\nbox=rect(size=(5 mm,5 mm),fill=\"#f00\")\none=page.add(box,target=page.center)\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("validate")
        .arg(&input)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("有效"));
    let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("render")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .args(["--dpi", "300"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(output).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    assert_eq!((width, height), (118, 118));
    std::fs::write(&input, "page=canvas(size=(10 mm,10 mm))\nbad=illegal()\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("validate")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("layout.lay:2:5: E_CALL"));
}
#[test]
fn legacy_cli_font_warning_location_and_warning_controls() {
    let tmp = Temp::new();
    let input = tmp.0.join("main.lay");
    std::fs::write(&input,"\npage=canvas(size=(50 mm,25 mm))\na=text(content=\"ABC\",font_size=10 pt)\npage.add(a)\nb=text(content=\"Fallback\",font_family=\"Definitely Missing Font Family\",font_size=10 pt)\npage.add(b,target=page.center)\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("validate")
        .arg(&input)
        .env("LAYMESH_NO_SYSTEM_FONTS", "1")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&result.stderr).contains("main.lay:5:3: W_FONT:"));
    for args in [vec!["--warnings", "hide"], vec![]] {
        let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
            .arg("validate")
            .arg(&input)
            .args(args)
            .env("LAYMESH_WARNINGS", "hide")
            .env("LAYMESH_NO_SYSTEM_FONTS", "1")
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("W_FONT"));
    }
}

#[test]
fn legacy_cli_plot_layout_warnings_do_not_prevent_any_export() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let capture: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(root.join("migration/corpus/cases-1044587.jsonl"))
            .unwrap()
            .lines()
            .nth(54)
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
            root.join("tests/fonts/DejaVuSans.ttf").to_str().unwrap(),
        );
    }
    let tmp = Temp::new();
    let input = tmp.0.join("plot.lay");
    std::fs::write(&input, source).unwrap();
    let validation = Command::new(env!("CARGO_BIN_EXE_laymesh"))
        .arg("validate")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(String::from_utf8_lossy(&validation.stderr).contains("W_PLOT_LAYOUT"));
    for ext in ["svg", "pdf", "png"] {
        let output = tmp.0.join(format!("warning.{ext}"));
        let result = Command::new(env!("CARGO_BIN_EXE_laymesh"))
            .arg("render")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{ext}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("W_PLOT_LAYOUT"));
        let bytes = std::fs::read(output).unwrap();
        assert!(bytes.len() > 100);
        match ext {
            "svg" => assert!(String::from_utf8_lossy(&bytes).contains(">Time</text>")),
            "pdf" => assert!(
                !String::from_utf8_lossy(&bytes)
                    .split_whitespace()
                    .collect::<String>()
                    .contains("/Subtype/Image")
            ),
            "png" => assert_eq!(
                u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
                (180. * 96. / 25.4_f64).round() as u32
            ),
            _ => unreachable!(),
        }
    }
}
